use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::dwarf::Dwarf;
use crate::elf::check_elf;
use crate::log::*;
use crate::tool::*;

use super::note;
use super::resolve;
use super::Arguments;
use super::BuildRoot;
use super::Error;
use super::LinkMessages;
use super::Project;
use super::Result;
use super::Tool;
use super::{Compiler, CompilerHackGuard};

const UPATCHD_SOCKET_NAME: &str = "upatchd.sock";

pub struct UpatchBuild {
    args: Arguments,
    build_root: BuildRoot,
    compiler: Compiler,
    tool: Tool,
    dwarf: Dwarf,
    source_obj: HashMap<PathBuf, PathBuf>,
    patch_obj: HashMap<PathBuf, PathBuf>,
    source_link_messages: LinkMessages,
    patch_link_messages: LinkMessages,
}

impl UpatchBuild {
    fn new() -> Result<Self> {
        Ok(Self {
            args: Arguments::new()?,
            build_root: BuildRoot::new(),
            compiler: Compiler::new(),
            tool: Tool::new(),
            dwarf: Dwarf::new(),
            source_obj: HashMap::new(),
            patch_obj: HashMap::new(),
            source_link_messages: LinkMessages::new(),
            patch_link_messages: LinkMessages::new(),
        })
    }

    fn setup_signal_handlers() -> Result<()> {
        ctrlc::set_handler(|| {
            error!("Received termination signal");
        })
        .map_err(|e| Error::Mod(e.to_string()))
    }

    fn build_main(mut self) -> Result<()> {
        self.build_root.create_dir(&self.args.build_root)?;
        self.init_logger()?;

        // find upatch-diff
        self.tool.check()?;

        // check patches
        info!("Testing patch file(s)");
        let project = Project::new(&self.args.source_dir);
        project.patch_all(&self.args.patch, Level::Debug)?;
        project.unpatch_all(&self.args.patch, Level::Debug)?;

        // check compiler
        self.compiler.analyze(&self.args.compiler)?;
        if !self.args.skip_compiler_check {
            self.compiler
                .check_version(self.build_root.cache_dir(), &self.args.debuginfo)?;
        }

        // hack compiler
        info!("Hacking compilers");
        let socket_file = self.args.work_dir.join(UPATCHD_SOCKET_NAME);
        let compiler_hacker = CompilerHackGuard::new(&self.compiler, socket_file)?;
        let project_name = self.args.source_dir.file_name().unwrap();

        // build source
        info!("Building original {:?}", project_name);
        project.build(
            self.build_root.source_dir(),
            self.args.build_source_cmd.clone(),
        )?;

        for i in 0..self.args.debuginfo.len() {
            self.args.elf_path[i] =
                self.get_binary_elf(&self.args.debuginfo[i], &self.args.elf_path[i])?;
        }

        // collect source link message and object message
        self.source_link_messages =
            LinkMessages::from(&self.args.elf_path, self.build_root.source_dir())?;
        self.source_obj =
            self.correlate_obj(&self.args.source_dir, self.build_root.source_dir())?;
        if self.source_obj.is_empty() {
            return Err(Error::Build(format!(
                "no valid object in {:?}",
                self.build_root.source_dir()
            )));
        }

        // patch
        project.patch_all(&self.args.patch, Level::Info)?;

        // build patched
        info!("Building patched {:?}", project_name);
        project.build(
            self.build_root.patch_dir(),
            self.args.build_patch_cmd.clone(),
        )?;

        // collect patched link message and object message
        self.patch_link_messages =
            LinkMessages::from(&self.args.elf_path, self.build_root.patch_dir())?;
        self.patch_obj = self.correlate_obj(&self.args.source_dir, self.build_root.patch_dir())?;
        if self.patch_obj.is_empty() {
            return Err(Error::Build(format!(
                "no valid object in {:?}",
                self.build_root.patch_dir()
            )));
        }

        // unhack compiler
        info!("Unhacking compilers");
        drop(compiler_hacker);

        // detecting changed objects
        info!("Detecting changed objects");
        self.build_patches()?;

        info!("Done");
        Ok(())
    }

    pub fn start_and_run() -> Result<()> {
        Self::setup_signal_handlers()?;
        Self::new()?.build_main()
    }
}

impl UpatchBuild {
    fn init_logger(&self) -> Result<()> {
        let mut logger = Logger::new();

        let log_level = match self.args.verbose {
            false => LevelFilter::Info,
            true => LevelFilter::Debug,
        };

        logger.set_print_level(log_level);
        logger.set_log_file(LevelFilter::Trace, self.build_root.log_file())?;
        Logger::init_logger(logger);

        Ok(())
    }

    fn correlate_obj<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        compiler_dir: P,
        output_dir: Q,
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        let compiler_dir = compiler_dir.as_ref();
        let mut map = HashMap::new();
        let arr = list_all_files_ext(output_dir, "o", false)?;
        for obj in arr {
            let result = match self.dwarf.file_in_obj(&obj) {
                Ok(dwarf) => dwarf,
                Err(e) => {
                    debug!("build map: {:?} is not elf, {}", &obj, e);
                    continue;
                }
            };
            match result.len() == 1 && result[0].DW_AT_comp_dir.starts_with(compiler_dir) {
                true => {
                    map.insert(obj, result[0].get_source());
                }
                false => debug!("build map: read {:?}'s dwarf failed!", &obj),
            }
        }
        Ok(map)
    }

    fn create_diff<P, Q>(
        &self,
        source_link_message: &HashSet<PathBuf>,
        patch_link_message: &HashSet<PathBuf>,
        diff_dir: P,
        debug_info: Q,
    ) -> Result<()>
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
                }
            };
            let output = diff_dir.join(file_name(patch_path)?);
            let mut source_path = None;
            for path in source_link_message {
                let source_name = match self.source_obj.get(path) {
                    Some(name) => name,
                    None => {
                        debug!("read {:?}'s dwarf failed!", path);
                        continue;
                    }
                };
                if patch_name.eq(source_name) {
                    source_path = Some(path);
                    break;
                }
            }

            match source_path {
                Some(source_path) => self.tool.upatch_diff(
                    source_path,
                    patch_path,
                    &output,
                    &debug_info,
                    self.build_root.log_file(),
                    self.args.verbose,
                )?,
                None => {
                    debug!("copy {:?} to {:?}!", &patch_path, &output);
                    fs::copy(patch_path, output)?;
                }
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
        let output_dir = self.args.output_dir.as_path();

        let mut patch_name = self.args.name.clone();
        let output_file = match patch_name.is_empty() {
            true => output_dir.join(&binary),
            false => output_dir.join(patch_name.concat("-").concat(&binary)),
        };

        let mut link_args = list_all_files_ext(diff_dir, "o", false)?;
        match link_args.len() {
            0 => {
                info!(
                    "Building patch: {:?}: no functional changes found",
                    output_file
                );
                return Ok(0);
            }
            _ => info!("Building patch: {:?}", output_file),
        };

        // build notes.o
        let notes = diff_dir.join("notes.o");
        debug!("create notes: {:?}", notes);
        note::create_note(&debug_info, &notes)?;
        link_args.push(notes);

        // ld patchs
        self.compiler.linker(&link_args, &output_file)?;
        debug!("resolve {:?} with {:?}", output_file, debug_info.as_ref());
        resolve::resolve_upatch(&debug_info, &output_file)?;
        Ok(1)
    }

    fn build_patches(&self) -> Result<()> {
        let mut upatch_num = 0;
        for i in 0..self.args.debuginfo.len() {
            debug!(
                "\n\nbuild upatches: debuginfo: {:?}(elf_path: {:?})",
                &self.args.debuginfo[i], &self.args.elf_path[i]
            );
            let patch_objects = match self.patch_link_messages.get_objects(&self.args.elf_path[i]) {
                Some(objects) => objects,
                None => {
                    info!(
                        "read {:?}'s patch link_message failed: None",
                        &self.args.elf_path[i]
                    );
                    continue;
                }
            };
            let source_objects = match self
                .source_link_messages
                .get_objects(&self.args.elf_path[i])
            {
                Some(objects) => objects,
                None => {
                    return Err(Error::Build(format!(
                        "read {:?}'s source link_message failed: None",
                        &self.args.elf_path[i]
                    )))
                }
            };

            let binary_name = file_name(&self.args.elf_path[i])?;
            let diff_dir = self
                .build_root
                .output_dir()
                .to_path_buf()
                .join(&binary_name);
            fs::create_dir(&diff_dir)?;

            let new_debug_info = self
                .build_root
                .debuginfo_dir()
                .join(file_name(&self.args.debuginfo[i])?);
            debug!(
                "copy {:?} to {:?}!",
                &self.args.debuginfo[i], &new_debug_info
            );
            fs::copy(&self.args.debuginfo[i], &new_debug_info)?;
            fs::set_permissions(&new_debug_info, fs::Permissions::from_mode(0o644))?;
            resolve::resolve_dynamic(&new_debug_info)?;

            self.create_diff(source_objects, patch_objects, &diff_dir, &new_debug_info)?;
            upatch_num += self.build_patch(&new_debug_info, &binary_name, &diff_dir)?;
        }
        if upatch_num.eq(&0) {
            return Err(Error::Build("no upatch is generated!".to_string()));
        }
        Ok(())
    }

    fn get_binary_elf<P: AsRef<Path>, B: AsRef<Path>>(
        &self,
        debug_info: P,
        binary_file: B,
    ) -> Result<PathBuf> {
        let mut result = Vec::new();
        let pathes = glob(&binary_file)?; // for rpm's "BUILDROOT/*/path"
        if pathes.is_empty() {
            return Err(Error::Build(format!(
                "can't find binary: {:?}",
                binary_file.as_ref()
            )));
        }
        for path in &pathes {
            if self.check_binary_elf(path)? {
                result.push(path);
            }
        }
        match result.len() {
            0 => Err(Error::Build(format!(
                "{:?} don't match binary: {:?}",
                debug_info.as_ref(),
                pathes
            ))),
            1 => Ok(result[0].clone()),
            _ => Err(Error::Build(format!(
                "{:?} match too many binaries: {:?}",
                debug_info.as_ref(),
                pathes
            ))),
        }
    }

    fn check_binary_elf<P: AsRef<Path>>(&self, path: P) -> std::io::Result<bool> {
        if path.as_ref().is_dir() {
            return Ok(false);
        }
        let file = OpenOptions::new().read(true).open(path)?;
        check_elf(&file)
    }
}
