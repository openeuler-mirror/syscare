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

use super::Arg;
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

pub struct UpatchBuild {
    args: Arg,
    work_dir: WorkDir,
    compiler: Compiler,
    diff_file: String,
    tool_file: String,
    hack_flag: Arc<AtomicBool>,
}

impl UpatchBuild {
    pub fn new() -> Self {
        Self {
            args: Arg::new(),
            work_dir: WorkDir::new(),
            compiler: Compiler::new(),
            diff_file: String::new(),
            tool_file: String::new(),
            hack_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.args.read()?;

        // create .upatch directory
        self.work_dir.create_dir(self.args.work_dir.clone())?;
        self.init_logger()?;

        if self.args.patch_name.is_empty() {
            self.args.patch_name.push_str(&self.args.elf_name);
        }
        if self.args.output.is_empty() {
            self.args.output.push_str(self.work_dir.cache_dir());
        }

        // check mod
        self.check_mod()?;

        // find upatch-diff and upatch-tool
        self.diff_file = search_tool(SUPPORT_DIFF)?;
        self.tool_file = search_tool(SUPPORT_TOOL)?;

        // check compiler
        self.compiler.analyze(self.args.compiler_file.clone())?;
        if !self.args.skip_compiler_check {
            self.compiler.check_version(self.work_dir.cache_dir(), &self.args.debug_info)?;
        }

        // copy source
        let project_name = &Path::new(&self.args.source).file_name().unwrap();

        // hack compiler
        info!("Hacking compiler");
        self.unhack_stop();
        self.compiler.hack()?;
        self.hack_flag.store(true, Ordering::Relaxed);

        // build source
        info!("Building original {}", project_name.to_str().unwrap());
        let project = Project::new(self.args.source.clone());
        project.build(CMD_SOURCE_ENTER, self.work_dir.source_dir(), self.args.build_source_command.clone())?;

        // build patch
        for patch in &self.args.diff_file {
            info!("Patching file: {}", &patch);
            project.patch(patch.clone(), self.args.verbose)?;
        }

        info!("Building patched {}", project_name.to_str().unwrap());
        project.build(CMD_PATCHED_ENTER, self.work_dir.patch_dir(), self.args.build_patch_command.clone())?;

        // unhack compiler
        info!("Unhacking compiler");
        self.compiler.unhack()?;
        self.hack_flag.store(false, Ordering::Relaxed);

        info!("Detecting changed objects");
        // correlate obj name
        let mut source_obj: HashMap<String, String> = HashMap::new();
        let mut patch_obj: HashMap<String, String> = HashMap::new();
        let dwarf = Dwarf::new();

        self.correlate_obj(&dwarf, self.args.source.as_str(), self.work_dir.source_dir(), &mut source_obj)?;
        self.correlate_obj(&dwarf, self.args.source.as_str(), self.work_dir.patch_dir(), &mut patch_obj)?;

        // choose the binary's obj to create upatch file
        let binary_obj = dwarf.file_in_binary(self.args.source.clone(), self.args.elf_name.clone())?;
        self.create_diff(&source_obj, &patch_obj, &binary_obj)?;

        // ld patchs
        let output_file = format!("{}/{}", &self.args.output, &self.args.patch_name);
        self.compiler.linker(self.work_dir.output_dir(), &output_file)?;
        self.upatch_tool(&output_file)?;
        info!("Building patch: {}", &output_file);
        Ok(())
    }

    pub fn unhack_compiler(&self){
        if self.hack_flag.load(Ordering::Relaxed) {
            if let Err(_) = self.compiler.unhack() {
                println!("unhack failed after upatch build error");
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

    fn correlate_obj(&self, dwarf: &Dwarf, comp_dir: &str, dir: &str, map: &mut HashMap<String, String>) -> Result<()> {
        let arr = list_all_files_ext(dir, "o", false)?;
        for obj in arr {       
            let name = stringtify(obj);
            let result = dwarf.file_in_obj(name.clone())?;
            if result.len() == 1 && result[0].DW_AT_comp_dir.find(comp_dir) != None{
                map.insert(result[0].get_source(), name.clone());
            }
        }
        Ok(())
    }

    fn create_diff(&self, source_obj: &HashMap<String, String>, patch_obj: &HashMap<String, String>, binary_obj: &Vec<DwarfCompileUnit>) -> Result<()> {
        for path in binary_obj {
            let source_name = path.get_source();
            match &patch_obj.get(&source_name) {
                Some(patch) => {
                    let output_dir = format!("{}/{}", self.work_dir.output_dir(), file_name(patch)?);
                    match &source_obj.get(&source_name) {
                        Some(source) => self.upatch_diff(source, patch, &output_dir)?,
                        None => { fs::copy(&patch, output_dir)?; },
                    };
                },
                None => {},
            };
        }
        Ok(())
    }

    fn upatch_diff(&self, source: &str, patch: &str, output_dir: &str) -> Result<()> {
        let mut args_list = vec!["-s", &source, "-p", &patch, "-o", &output_dir, "-r", &self.args.debug_info];
        if self.args.verbose {
            args_list.push("-d");
        }
        let output = ExternCommand::new(&self.diff_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Diff(format!("{}: please look {} for detail.", output.exit_code(), self.work_dir.log_file())))
        };
        Ok(())
    }

    fn upatch_tool(&self, patch: &str) -> Result<()> {
        let args_list = vec!["resolve", "-b", &self.args.debug_info, "-p", patch];
        let output = ExternCommand::new(&self.tool_file).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::TOOL(format!("{}: {}", output.exit_code(), output.stderr())))
        };
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
                        println!("{} after upatch build error", e);
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
                    println!("{} after upatch build error", e);
                }
                hack_flag_clone.store(false, Ordering::Relaxed);
            }
        }));
    }
}
