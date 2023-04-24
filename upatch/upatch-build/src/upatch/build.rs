use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::process::exit;
use std::thread;
use std::sync::{Arc, Mutex};

use signal_hook::{iterator::Signals, consts::*};

use crate::elf::check_elf;
use crate::log::*;
use crate::tool::*;
use crate::dwarf::Dwarf;

use super::Arguments;
use super::Compiler;
use super::WorkDir;
use super::Project;
use super::Tool;
use super::LinkMessages;
use super::Result;
use super::Error;
use super::{resolve, create_note};

pub const UPATCH_DEV_NAME: &str = "upatch";
const SYSTEM_MOUDLES: &str = "/proc/modules";
const COMPILER_CMD_SOURCE_ENTER: &str = "CSE";
const COMPILER_CMD_PATCHED_ENTER: &str = "CPE";
const ASSEMBLER_CMD_SOURCE_ENTER: &str = "ASE";
const ASSEMBLER_CMD_PATCHED_ENTER: &str = "APE";

pub struct UpatchBuild {
    args: Arguments,
    work_dir: WorkDir,
    compiler: Compiler,
    tool: Tool,
    hack_flag: Arc<Mutex<bool>>,
    dwarf: Dwarf,
    source_obj: HashMap<PathBuf, PathBuf>,
    patch_obj: HashMap<PathBuf, PathBuf>,
    source_link_messages: LinkMessages,
    patch_link_messages: LinkMessages,
}

impl UpatchBuild {
    pub fn new() -> Self {
        Self {
            args: Arguments::new(),
            work_dir: WorkDir::new(),
            compiler: Compiler::new(),
            tool: Tool::new(),
            hack_flag: Arc::new(Mutex::new(false)),
            dwarf: Dwarf::new(),
            source_obj: HashMap::new(),
            patch_obj: HashMap::new(),
            source_link_messages: LinkMessages::new(),
            patch_link_messages: LinkMessages::new(),
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

        // check patches
        let project = Project::new(&self.args.debug_source);
        project.patch_all(&self.args.patches, Level::Debug)?;
        project.unpatch_all(&self.args.patches, Level::Debug)?;

        // hack compiler
        info!("Hacking compiler");
        self.unhack_stop();
        self.hack_compiler()?;

        let project_name = self.args.debug_source.file_name().unwrap();

        // build source
        info!("Building original {:?}", project_name);
        project.build(COMPILER_CMD_SOURCE_ENTER, ASSEMBLER_CMD_SOURCE_ENTER, self.work_dir.source_dir(), self.args.build_source_cmd.clone())?;

        for i in 0..self.args.debug_infoes.len() {
            self.args.elf_pathes[i] = self.get_binary_elf(&self.args.debug_infoes[i], &self.args.elf_pathes[i])?;
        }

        self.source_link_messages = LinkMessages::from(&self.args.elf_pathes, self.work_dir.source_dir())?;

        // build patch
        project.patch_all(&self.args.patches, Level::Info)?;

        info!("Building patched {:?}", project_name);
        project.build(COMPILER_CMD_PATCHED_ENTER, ASSEMBLER_CMD_PATCHED_ENTER, self.work_dir.patch_dir(), self.args.build_patch_cmd.clone())?;

        self.patch_link_messages = LinkMessages::from(&self.args.elf_pathes, self.work_dir.patch_dir())?;
        // unhack compiler
        info!("Unhacking compiler");
        self.unhack_compiler()?;

        info!("Detecting changed objects");
        // correlate obj name
        self.source_obj = self.correlate_obj(&self.args.debug_source, self.work_dir.source_dir())?;
        self.patch_obj = self.correlate_obj(&self.args.debug_source, self.work_dir.patch_dir())?;
        self.build_patches()?;
        Ok(())
    }

    pub fn hack_compiler(&self) -> Result<()> {
        let mut mutex = self.hack_flag.lock().expect("lock failed");
        if !*mutex {
            self.compiler.hack()?;
            *mutex = true;
        }
        Ok(())
    }

    pub fn unhack_compiler(&self) -> Result<()> {
        let mut mutex = self.hack_flag.lock().expect("lock failed");
        if *mutex {
            self.compiler.unhack()?;
            *mutex = false;
        }
        Ok(())
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
            let result = match self.dwarf.file_in_obj(&obj) {
                Ok(dwarf) => dwarf,
                Err(e) => {
                    debug!("build map: {:?} is not elf, {}", &obj, e);
                    continue;
                },
            };
            match result.len() == 1 && result[0].DW_AT_comp_dir.starts_with(compiler_dir) {
                true => { map.insert(obj, result[0].get_source()); },
                false => debug!("build map: read {:?}'s dwarf failed!", &obj),
            }
        }
        Ok(map)
    }

    fn create_diff<P, Q>(&self, source_link_message: &HashSet<PathBuf>, patch_link_message: &HashSet<PathBuf>, diff_dir: P, debug_info: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let diff_dir = diff_dir.as_ref().to_path_buf();
        for patch_path in patch_link_message {
            let patch_name = match self.patch_obj.get(patch_path) {
                Some(name) => name,
                None => {
                    debug!("read {:?}'s dwarf failed!", patch_path);
                    continue;
                },
            };
            let output = diff_dir.join(file_name(&patch_path)?);
            let mut source_path = None;
            for path in source_link_message {
                let source_name = match self.source_obj.get(path) {
                    Some(name) => name,
                    None => {
                        debug!("read {:?}'s dwarf failed!", path);
                        continue;
                    },
                };
                if patch_name.eq(source_name) {
                    source_path = Some(path);
                    break;
                }
            }

            match source_path {
                Some(source_path) => self.tool.upatch_diff(source_path, patch_path, &output, &debug_info, &self.work_dir.log_file(), self.args.verbose)?,
                None => {
                    debug!("copy {:?} to {:?}!", &patch_path, &output);
                    fs::copy(&patch_path, output)?;
                },
            };
        }
        Ok(())
    }

    fn build_patch<P, Q, D>(&self, debug_info: P, binary: Q, diff_dir: D) -> Result<u32>
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
                info!("Building patch: {:?}: no functional changes found", output_file);
                return Ok(0);
            },
            _ => info!("Building patch: {:?}", output_file),
        };

        // build notes.o
        let notes = diff_dir.join("notes.o");
        debug!("create notes: {:?}", notes);
        create_note(&debug_info, &notes)?;
        link_args.push(notes);

        // ld patchs
        self.compiler.linker(&link_args, &output_file)?;
        debug!("resolve {:?} with {:?}", output_file, debug_info.as_ref());
        resolve(&debug_info, &output_file)?;
        Ok(1)
    }

    fn build_patches(&self) -> Result<()> {
        let mut upatch_num = 0;
        for i in 0..self.args.debug_infoes.len() {
            debug!("\n\nbuild upatches: debuginfo: {:?}(elf_path: {:?})", &self.args.debug_infoes[i], &self.args.elf_pathes[i]);
            let patch_objects = match self.patch_link_messages.get_objects(&self.args.elf_pathes[i]) {
                Some(objects) => objects,
                None => {
                    info!("read {:?}'s patch link_message failed: None", &self.args.elf_pathes[i]);
                    continue;
                },
            };
            let source_objects = match self.source_link_messages.get_objects(&self.args.elf_pathes[i]) {
                Some(objects) => objects,
                None => return Err(Error::Build(format!("read {:?}'s source link_message failed: None", &self.args.elf_pathes[i]))),
            };

            let binary_name = file_name(&self.args.elf_pathes[i])?;
            let diff_dir = self.work_dir.output_dir().to_path_buf().join(&binary_name);
            fs::create_dir(&diff_dir)?;
            self.create_diff(source_objects, patch_objects, &diff_dir, &self.args.debug_infoes[i])?;
            upatch_num += self.build_patch(&self.args.debug_infoes[i], &binary_name, &diff_dir)?;
        }
        if upatch_num.eq(&0) {
            return Err(Error::Build(format!("no upatch is generated!")));
        }
        Ok(())
    }

    fn get_binary_elf<P: AsRef<Path>, B: AsRef<Path>>(&self, debug_info: P, binary_file: B) -> Result<PathBuf> {
        let mut result = Vec::new();
        let pathes = glob(&binary_file)?; // for rpm's "BUILDROOT/*/path"
        if pathes.is_empty() {
            return Err(Error::Build(format!("can't find binary: {:?}", binary_file.as_ref())));
        }
        for path in &pathes {
            if self.check_binary_elf(path)? {
                result.push(path);
            }
        }
        match result.len() {
            0 => Err(Error::Build(format!("{:?} don't match binary: {:?}", debug_info.as_ref(), pathes))),
            1 => Ok(result[0].clone()),
            _ => Err(Error::Build(format!("{:?} match too many binaries: {:?}", debug_info.as_ref(), pathes))),
        }
    }

    fn check_binary_elf<P: AsRef<Path>>(&self, path: P) -> std::io::Result<bool> {
        let file = OpenOptions::new().read(true).open(path)?;
        check_elf(&file)
    }

    fn unhack_stop(&self) {
        let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT]).expect("signal_hook error");
        let hack_flag_clone = self.hack_flag.clone();
        let compiler_clone = self.compiler.clone();
        thread::spawn(move || {
            for signal in signals.forever() {
                let mut mutex = hack_flag_clone.lock().expect("lock failed");
                if *mutex {
                    if let Err(e) = compiler_clone.unhack() {
                        eprintln!("{} after upatch build error", e);
                    }
                    *mutex = false;
                }
                eprintln!("ERROR: receive system signal {}", signal);
                exit(signal);
            }
        });

        let hack_flag_clone = self.hack_flag.clone();
        let compiler_clone = self.compiler.clone();
        std::panic::set_hook(Box::new(move |_| {
            let mut mutex = hack_flag_clone.lock().expect("lock failed");
            if *mutex {
                if let Err(e) = compiler_clone.unhack() {
                    eprintln!("{} after upatch build error", e);
                }
                *mutex = false;
            }
        }));
    }
}
