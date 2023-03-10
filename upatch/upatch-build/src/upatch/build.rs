use std::ffi::OsString;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::process::exit;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use signal_hook::{iterator::Signals, consts::SIGINT};

use crate::elf::check_elf;
use crate::log::*;
use crate::tool::*;
use crate::dwarf::Dwarf;

use super::Arguments;
use super::Compiler;
use super::WorkDir;
use super::Project;
use super::Tool;
use super::Result;
use super::Error;
use super::{resolve, create_note};

pub const UPATCH_DEV_NAME: &str = "upatch";
const SYSTEM_MOUDLES: &str = "/proc/modules";
const CMD_SOURCE_ENTER: &str = "SE";
const CMD_PATCHED_ENTER: &str = "PE";

pub struct UpatchBuild {
    args: Arguments,
    work_dir: WorkDir,
    compiler: Compiler,
    tool: Tool,
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
            tool: Tool::new(),
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
        self.tool.check()?;

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
        project.build(CMD_SOURCE_ENTER, self.work_dir.source_dir(), self.args.build_source_cmd.clone())?;

        // build patch
        for patch in &self.args.patches {
            info!("Patching file: {}", patch.display());
            project.patch(patch, Level::Info)?;
        }

        info!("Building patched {:?}", project_name);
        project.build(CMD_PATCHED_ENTER, self.work_dir.patch_dir(), self.args.build_patch_cmd.clone())?;

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

    fn create_diff<B, P, Q>(&self, binary_file: B, diff_dir: P, debug_info: Q) -> Result<()>
    where
        B: AsRef<Path>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let diff_dir = diff_dir.as_ref().to_path_buf();
        let binary_obj = self.dwarf.file_in_obj(&binary_file)?;
        for path in &binary_obj {
            let source_name = path.get_source();
            match &self.patch_obj.get(&source_name) {
                Some(patch) => {
                    let output = diff_dir.join(file_name(&patch)?);
                    match &self.source_obj.get(&source_name) {
                        Some(source) => self.tool.upatch_diff(&source, &patch, &output, &debug_info, &self.work_dir.log_file(), self.args.verbose)?,
                        None => { fs::copy(&patch, output)?; },
                    };
                },
                None => {},
            };
        }
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
        create_note(&debug_info, &notes)?;
        link_args.push(notes);

        // ld patchs
        self.compiler.linker(&link_args, &output_file)?;
        resolve(&debug_info, &output_file)?;
        Ok(())
    }

    fn build_patches<P: AsRef<Path>>(&self, debug_infoes: &Vec<P>) -> Result<()> {
        match self.args.elf_pathes.is_empty() {
            true => self.__build_patches(debug_infoes),
            false => self.__build_patches_elf(debug_infoes),
        }?;
        Ok(())
    }

    fn __build_patches_elf<P: AsRef<Path>>(&self, debug_infoes: &Vec<P>) -> Result<()> {
        for i in 0..debug_infoes.len() {
            let binary_path = self.get_binary_elf(&debug_infoes[i], &self.args.elf_pathes[i])?;
            let binary_name = file_name(&binary_path)?;
            let diff_dir = self.work_dir.output_dir().to_path_buf().join(&binary_name);
            fs::create_dir(&diff_dir)?;
            self.create_diff(&binary_path, &diff_dir, &debug_infoes[i])?;
            self.build_patch(&debug_infoes[i], &binary_name, &diff_dir)?;
        }
        Ok(())
    }

    fn get_binary_elf<P: AsRef<Path>, B: AsRef<Path>>(&self, debug_info: P, binary_file: B) -> Result<PathBuf> {
        let mut result = Vec::new();
        let pathes = glob(binary_file)?;
        for binary_file in &pathes {
            if self.check_binary_elf(binary_file)? {
                result.push(binary_file);
            }
        }
        match result.len() {
            0 => Err(Error::Build(format!("no binary match {:?}", debug_info.as_ref()))),
            1 => Ok(result[0].clone()),
            _ => Err(Error::Build(format!("{:?} match too many binaries: {:?}", debug_info.as_ref(), pathes))),
        }
    }

    fn __build_patches<P: AsRef<Path>>(&self, debug_infoes: &Vec<P>) -> Result<()> {
        let binary_files = list_all_files(self.args.elf_dir.as_ref().unwrap(), true)?;
        for debug_info in debug_infoes {
            let debug_info_name = file_name(debug_info)?;
            let (binary_name, binary_file) = self.get_binary(&binary_files, debug_info_name)?;
            let diff_dir = self.work_dir.output_dir().to_path_buf().join(&binary_name);
            fs::create_dir(&diff_dir)?;
            self.create_diff(&binary_file, &diff_dir, debug_info)?;
            self.build_patch(debug_info, &binary_name, &diff_dir)?;
        }
        Ok(())
    }

    fn get_binary<P: AsRef<Path>>(&self, binary_files: &Vec<P>, debug_info_name: OsString) -> Result<(OsString, PathBuf)> {
        let (mut name, mut file) = (OsString::new(), PathBuf::new());
        for binary_file in binary_files {
            let binary_name = file_name(binary_file)?;
            if debug_info_name.contains(binary_name.as_bytes()) && self.check_binary_elf(&binary_file)? {
                match name.is_empty() {
                    true => (name, file) = (binary_name, binary_file.as_ref().to_path_buf()),
                    false => return Err(Error::Build(format!("{:?} match too many binaries: {:?} {:?}, please use --elf-dir or --elf-path parameter to specify one", debug_info_name, file, binary_file.as_ref()))),
                }
            }
        }
        match name.is_empty() {
            true => Err(Error::Build(format!("no binary match {:?}", debug_info_name))),
            false => Ok((name, file)),
        }
    }

    fn check_binary_elf<P: AsRef<Path>>(&self, binary_file: P) -> std::io::Result<bool> {
        let file = OpenOptions::new().read(true).open(binary_file)?;
        check_elf(&file)
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
