use std::path::Path;
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
use crate::cmd::ExternCommand;

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
    diff_file: String,
    tool_file: String,
    notes_file: String,
    hack_flag: Arc<AtomicBool>,
    dwarf: Dwarf,
    source_obj: HashMap<String, String>,
    patch_obj: HashMap<String, String>,
}

impl UpatchBuild {
    pub fn new() -> Self {
        Self {
            args: Arguments::new(),
            work_dir: WorkDir::new(),
            compiler: Compiler::new(),
            diff_file: String::new(),
            tool_file: String::new(),
            notes_file: String::new(),
            hack_flag: Arc::new(AtomicBool::new(false)),
            dwarf: Dwarf::new(),
            source_obj: HashMap::new(),
            patch_obj: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.args.check()?;

        self.work_dir.create_dir(self.args.work_dir.clone())?;
        self.init_logger()?;

        // check mod
        self.check_mod()?;

        // find upatch-diff and upatch-tool
        self.diff_file = search_tool(SUPPORT_DIFF)?;
        self.tool_file = search_tool(SUPPORT_TOOL)?;
        self.notes_file = search_tool(SUPPORT_NOTES)?;

        // check compiler
        self.compiler.analyze(self.args.compiler.clone())?;
        if !self.args.skip_compiler_check {
            self.compiler.check_version(self.work_dir.cache_dir(), &self.args.debug_infoes[0])?;
        }

        // copy source
        let project_name = &Path::new(&self.args.debug_source).file_name().unwrap();

        // hack compiler
        info!("Hacking compiler");
        self.unhack_stop();
        self.compiler.hack()?;
        self.hack_flag.store(true, Ordering::Relaxed);

        // build source
        info!("Building original {}", project_name.to_str().unwrap());
        let project = Project::new(self.args.debug_source.clone());
        project.build(CMD_SOURCE_ENTER, self.work_dir.source_dir(), self.work_dir.binary_dir(), self.args.build_source_cmd.clone())?;

        // build patch
        for patch in &self.args.patches {
            info!("Patching file: {}", &patch);
            project.patch(patch.clone(), self.args.verbose)?;
        }

        info!("Building patched {}", project_name.to_str().unwrap());
        project.build(CMD_PATCHED_ENTER, self.work_dir.patch_dir(), self.work_dir.binary_dir(), self.args.build_patch_cmd.clone())?;

        // unhack compiler
        info!("Unhacking compiler");
        self.compiler.unhack()?;
        self.hack_flag.store(false, Ordering::Relaxed);

        info!("Detecting changed objects");
        // correlate obj name
        self.source_obj = self.correlate_obj(self.args.debug_source.as_str(), self.work_dir.source_dir())?;
        self.patch_obj = self.correlate_obj(self.args.debug_source.as_str(), self.work_dir.patch_dir())?;
        self.build_patches()?;
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

    fn correlate_obj(&self, compiler_dir: &str, output_dir: &str) -> Result<HashMap<String, String>> {
        let mut map =  HashMap::new();
        let arr = list_all_files_ext(output_dir, "o", false)?;
        for obj in arr {       
            let name = stringtify(obj);
            let result = self.dwarf.file_in_obj(name.clone())?;
            if result.len() == 1 && result[0].DW_AT_comp_dir.find(compiler_dir) != None {
                map.insert(result[0].get_source(), name.clone());
            }
        }
        Ok(map)
    }

    fn create_diff(&self, binary_obj: &Vec<DwarfCompileUnit>, diff_dir: &str, debug_info: &str) -> Result<()> {
        for path in binary_obj {
            let source_name = path.get_source();
            match &self.patch_obj.get(&source_name) {
                Some(patch) => {
                    let output_dir = format!("{}/{}", diff_dir, file_name(patch)?);
                    match &self.source_obj.get(&source_name) {
                        Some(source) => self.upatch_diff(source, patch, &output_dir, debug_info)?,
                        None => { fs::copy(&patch, output_dir)?; },
                    };
                },
                None => {},
            };
        }
        Ok(())
    }

    fn upatch_diff(&self, source: &str, patch: &str, output_dir: &str, debug_info: &str) -> Result<()> {
        let mut args_list = vec!["-s", &source, "-p", &patch, "-o", &output_dir, "-r", debug_info];
        if self.args.verbose {
            args_list.push("-d");
        }
        let output = ExternCommand::new(&self.diff_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Diff(format!("{}: please look {} for detail.", output.exit_code(), self.work_dir.log_file())))
        };
        Ok(())
    }

    fn upatch_tool(&self, patch: &str, debug_info: &str) -> Result<()> {
        let args_list = vec!["resolve", "-b", debug_info, "-p", patch];
        let output = ExternCommand::new(&self.tool_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::TOOL(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    fn upatch_notes(&self, notes: &str, debug_info: &str) -> Result<()> {
        let args_list = vec!["-r", debug_info, "-o", notes];
        let output = ExternCommand::new(&self.notes_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::NOTES(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    fn build_patch(&self, debug_info: &str, binary: &str, diff_dir: &str) -> Result<()> {
        // choose the binary's obj to create upatch file
        let output_file = format!("{}/{}{}", &self.args.output_dir, &self.args.name, binary);

        let link_args = list_all_files_ext(diff_dir, "o", false)?;
        match link_args.len() {
            0 => {
                info!("Building patch: {}: no functional changes found", &output_file);
                return Ok(());
            },
            _ => info!("Building patch: {}", &output_file),
        };
        let mut link_args = link_args.iter().map(|x| -> String {stringtify(x)}).rev().collect::<Vec<String>>();

        // build notes.o
        let notes = format!("{}/notes.o", diff_dir);
        self.upatch_notes(&notes, debug_info)?;
        link_args.push(notes);

        // ld patchs
        self.compiler.linker(&link_args, &output_file)?;
        self.upatch_tool(&output_file, debug_info)?;
        Ok(())
    }

    fn build_patches(&self) -> Result<()> {
        let binary_files = list_all_files(self.work_dir.binary_dir(), false)?;
        for debug_info in &self.args.debug_infoes {
            let debug_info_name = file_name(debug_info)?;
            let mut not_found = true;
            for binary_file in &binary_files {
                let binary_name = file_name(binary_file)?;
                if debug_info_name.starts_with(&binary_name) {
                    let diff_dir = format!("{}/{}", self.work_dir.output_dir(), &binary_name);
                    fs::create_dir(&diff_dir)?;
                    let binary_obj = self.dwarf.file_in_obj(stringtify(binary_file))?;
                    self.create_diff(&binary_obj, &diff_dir, debug_info)?;
                    self.build_patch(debug_info, &binary_name, &diff_dir)?;
                    not_found = false;
                    break;
                }
            }
            if not_found {
                return Err(Error::Build(format!("don't have binary match {}", debug_info)));
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
