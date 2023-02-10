use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::string::String;
use std::process::exit;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use signal_hook::{iterator::Signals, consts::SIGINT};

use crate::log::*;
use crate::tool::*;
use crate::dwarf::{Dwarf, DwarfCompileUnit};
use crate::cmd::*;

use super::Arguments;
use super::Compiler;
use super::WorkDir;
use super::Project;
use super::Result;
use super::Error;

pub const UPATCH_DEV_NAME: &str = "upatch";
const SYSTEM_MOUDLES: &str = "/proc/modules";
const CMD_SOURCE_ENTER: &str = "SE";
const CMD_PATCHED_ENTER: &str = "PE";
const SUPPORT_DIFF: &str = "upatch-diff";
const SUPPORT_TOOL: &str = "upatch-tool";
const SUPPORT_NOTES: &str = "upatch-notes";

pub struct UpatchBuild {
    args: Arguments,
    work_dir: WorkDir,
    compiler: Compiler,
    diff_file: PathBuf,
    tool_file: PathBuf,
    notes_file: PathBuf,
    hack_flag: Arc<AtomicBool>,
    dwarf: Dwarf,
    source_obj: HashMap<PathBuf, PathBuf>,
    patch_obj: HashMap<PathBuf, PathBuf>,
}

impl UpatchBuild {
    pub fn new() -> Self {
        Self {
            args: Arguments::new(),
            work_dir: WorkDir::new(),
            compiler: Compiler::new(),
            diff_file: PathBuf::new(),
            tool_file: PathBuf::new(),
            notes_file: PathBuf::new(),
            hack_flag: Arc::new(AtomicBool::new(false)),
            dwarf: Dwarf::new(),
            source_obj: HashMap::new(),
            patch_obj: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.args.check()?;

        self.work_dir.create_dir(self.args.work_dir.as_ref().unwrap())?;
        self.args.output_dir.get_or_insert(self.work_dir.cache_dir().to_path_buf());
        self.init_logger()?;

        // check mod
        self.check_mod()?;

        // find upatch-diff and upatch-tool
        self.diff_file = search_tool(SUPPORT_DIFF)?;
        self.tool_file = search_tool(SUPPORT_TOOL)?;
        self.notes_file = search_tool(SUPPORT_NOTES)?;

        // check compiler
        self.compiler.analyze(self.args.compiler.as_ref().unwrap())?;
        if !self.args.skip_compiler_check {
            self.compiler.check_version(self.work_dir.cache_dir(), &self.args.debug_infoes[0])?;
        }

        // hack compiler
        info!("Hacking compiler");
        self.unhack_stop();
        self.compiler.hack()?;
        self.hack_flag.store(true, Ordering::Relaxed);

        let project_name = self.args.debug_source.file_name().unwrap();

        // build source
        info!("Building original {:?}", project_name);
        let project = Project::new(&self.args.debug_source);
        project.build(CMD_SOURCE_ENTER, self.work_dir.source_dir(), self.work_dir.binary_dir(), self.args.build_source_cmd.clone())?;

        // build patch
        for patch in &self.args.patches {
            info!("Patching file: {}", patch.display());
            project.patch(patch, self.args.verbose)?;
        }

        info!("Building patched {:?}", project_name);
        project.build(CMD_PATCHED_ENTER, self.work_dir.patch_dir(), self.work_dir.binary_dir(), self.args.build_patch_cmd.clone())?;

        // unhack compiler
        info!("Unhacking compiler");
        self.compiler.unhack()?;
        self.hack_flag.store(false, Ordering::Relaxed);

        info!("Detecting changed objects");
        // correlate obj name
        self.source_obj = self.correlate_obj(&self.args.debug_source, self.work_dir.source_dir())?;
        self.patch_obj = self.correlate_obj(&self.args.debug_source, self.work_dir.patch_dir())?;
        self.build_patches(&self.args.debug_infoes)?;
        Ok(())
    }

    pub fn unhack_compiler(&self){
        if self.hack_flag.load(Ordering::Relaxed) {
            if let Err(_) = self.compiler.unhack() {
                eprintln!("unhack failed after upatch build error");
            }
            self.hack_flag.store(false, Ordering::Relaxed);
        }
    }
}

impl UpatchBuild {
    fn init_logger(&self) -> Result<()> {
        let mut logger = Logger::new();

        let log_level = match self.args.verbose {
            false => LevelFilter::Info,
            true  => LevelFilter::Debug,
        };

        logger.set_print_level(log_level);
        logger.set_log_file(LevelFilter::Trace, self.work_dir.log_file())?;
        Logger::init_logger(logger);

        Ok(())
    }

    fn check_mod(&self) -> Result<()> {
        let mut file = File::open(SYSTEM_MOUDLES)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        match contents.find(UPATCH_DEV_NAME) {
            Some(_) => Ok(()),
            None => Err(Error::Mod(format!("can't find upatch mod in system"))),
        }
    }

    fn correlate_obj<P: AsRef<Path>, Q: AsRef<Path>>(&self, compiler_dir: P, output_dir: Q) -> Result<HashMap<PathBuf, PathBuf>> {
        let compiler_dir = compiler_dir.as_ref();
        let mut map =  HashMap::new();
        let arr = list_all_files_ext(output_dir, "o", false)?;
        for obj in arr {       
            let result = self.dwarf.file_in_obj(&obj)?;
            if result.len() == 1 && result[0].DW_AT_comp_dir.starts_with(compiler_dir) {
                map.insert(result[0].get_source(), obj);
            }
        }
        Ok(map)
    }

    fn create_diff<P: AsRef<Path>, Q: AsRef<Path>>(&self, binary_obj: &Vec<DwarfCompileUnit>, diff_dir: P, debug_info: Q) -> Result<()> {
        let diff_dir = diff_dir.as_ref().to_path_buf();
        for path in binary_obj {
            let source_name = path.get_source();
            match &self.patch_obj.get(&source_name) {
                Some(patch) => {
                    let output_dir = diff_dir.join(file_name(patch)?);
                    match &self.source_obj.get(&source_name) {
                        Some(source) => self.upatch_diff(source, patch, &output_dir, &debug_info)?,
                        None => { fs::copy(&patch, output_dir)?; },
                    };
                },
                None => {},
            };
        }
        Ok(())
    }

    fn upatch_diff<P, Q, O, D>(&self, source: P, patch: Q, output_dir: O, debug_info: D) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        O: AsRef<Path>,
        D: AsRef<Path>,
    {
        let mut args_list = ExternCommandArgs::new()
            .arg("-s").arg(source.as_ref())
            .arg("-p").arg(patch.as_ref())
            .arg("-o").arg(output_dir.as_ref())
            .arg("-r").arg(debug_info.as_ref());
        if self.args.verbose {
            args_list = args_list.arg("-d");
        }
        let output = ExternCommand::new(&self.diff_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Diff(format!("{}: please look {} for detail.", output.exit_code(), self.work_dir.log_file().display())))
        };
        Ok(())
    }

    fn upatch_tool<P, Q>(&self, patch: P, debug_info: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new().args(["resolve", "-b"]).arg(debug_info.as_ref()).arg("-p").arg(patch.as_ref());
        let output = ExternCommand::new(&self.tool_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::TOOL(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    fn upatch_notes<P, Q>(&self, notes: P, debug_info: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new().arg("-r").arg(debug_info.as_ref()).arg("-o").arg(notes.as_ref());
        let output = ExternCommand::new(&self.notes_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::NOTES(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    fn build_patch<P, Q, D>(&self, debug_info: P, binary: Q, diff_dir: D) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        D: AsRef<Path>,
    {
        let diff_dir = diff_dir.as_ref();
        let binary = binary.as_ref().to_path_buf();

        let mut binding = self.args.name.clone();
        let output_file = self.args.output_dir.as_ref().unwrap().join(binding.concat(&binary));

        let mut link_args = list_all_files_ext(diff_dir, "o", false)?;
        match link_args.len() {
            0 => {
                info!("Building patch: {}: no functional changes found", output_file.display());
                return Ok(());
            },
            _ => info!("Building patch: {}", output_file.display()),
        };

        // build notes.o
        let notes = diff_dir.join("notes.o");
        self.upatch_notes(&notes, &debug_info)?;
        link_args.push(notes);

        // ld patchs
        self.compiler.linker(&link_args, &output_file)?;
        self.upatch_tool(&output_file, &debug_info)?;
        Ok(())
    }

    fn build_patches<P: AsRef<Path>>(&self, debug_infoes: &Vec<P>) -> Result<()> {
        let binary_files = list_all_files(self.work_dir.binary_dir(), false)?;
        for debug_info in debug_infoes {
            let debug_info_name = file_name(debug_info)?;
            let mut not_found = true;
            for binary_file in &binary_files {
                let binary_name = file_name(binary_file)?;
                if debug_info_name.contains(binary_name.as_bytes()) {
                    let diff_dir = self.work_dir.output_dir().to_path_buf().join(&binary_name);
                    fs::create_dir(&diff_dir)?;
                    let binary_obj = self.dwarf.file_in_obj(&binary_file)?;
                    self.create_diff(&binary_obj, &diff_dir, debug_info)?;
                    self.build_patch(debug_info, &binary_name, &diff_dir)?;
                    not_found = false;
                    break;
                }
            }
            if not_found {
                return Err(Error::Build(format!("don't have binary match {}", debug_info.as_ref().display())));
            }
        }
        Ok(())
    }

    fn unhack_stop(&self) {
        let mut signals = Signals::new(&[SIGINT]).expect("signal_hook error");
        let hack_flag_clone = self.hack_flag.clone();
        let compiler_clone = self.compiler.clone();
        thread::spawn(move || {
            for signal in signals.forever() {
                if hack_flag_clone.load(Ordering::Relaxed) {
                    if let Err(e) = compiler_clone.unhack() {
                        eprintln!("{} after upatch build error", e);
                    }
                    hack_flag_clone.store(false, Ordering::Relaxed);
                }
                eprintln!("ERROR: receive system signal {}", signal);
                exit(signal);
            }
        });

        let hack_flag_clone = self.hack_flag.clone();
        let compiler_clone = self.compiler.clone();
        std::panic::set_hook(Box::new(move |_| {
            if hack_flag_clone.load(Ordering::Relaxed) {
                if let Err(e) = compiler_clone.unhack() {
                    eprintln!("{} after upatch build error", e);
                }
                hack_flag_clone.store(false, Ordering::Relaxed);
            }
        }));
    }
}
