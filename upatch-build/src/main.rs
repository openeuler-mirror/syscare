// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{env, ffi::OsStr, fs::Permissions, os::unix::fs::PermissionsExt, path::Path, process};

use anyhow::{ensure, Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode,
};
use indexmap::{IndexMap, IndexSet};
use log::{debug, error, info, trace, warn, Level, LevelFilter, Record};
use object::{write, Object, ObjectKind, ObjectSection, SectionKind};
use syscare_common::{
    concat_os,
    fs::{self, MappedFile},
    os,
    process::Command,
};

mod args;
mod build_root;
mod compiler;
mod dwarf;
mod elf;
mod file_relation;
mod project;
mod resolve;

use args::Arguments;
use build_root::BuildRoot;
use compiler::CompilerInfo;
use dwarf::{Dwarf, ProducerType};
use file_relation::FileRelation;
use project::Project;

const CLI_NAME: &str = "upatch build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const LOG_FILE_NAME: &str = "build";

const PATH_ENV: &str = "PATH";
const BINARY_INSTALL_PATH: &str = "/usr/libexec/syscare";
const UPATCH_DIFF_BIN: &str = "upatch-diff";

struct UpatchBuild {
    args: Arguments,
    logger: LoggerHandle,
    build_root: BuildRoot,
    compiler_map: IndexMap<ProducerType, CompilerInfo>,
    file_relation: FileRelation,
}

/* Main process */
impl UpatchBuild {
    fn new() -> Result<Self> {
        // Setup environment variable & umask
        let path_env = env::var_os(PATH_ENV)
            .with_context(|| format!("Cannot read environment variable {}", PATH_ENV))?;
        env::set_var(PATH_ENV, concat_os!(BINARY_INSTALL_PATH, ":", path_env));
        os::umask::set_umask(CLI_UMASK);

        // Parse arguments
        let args = Arguments::new()?;
        let build_root = BuildRoot::new(&args.build_root)?;
        fs::create_dir_all(&args.output_dir)?;

        // Initialize logger
        let log_level_max = LevelFilter::Trace;
        let log_level_stdout = match &args.verbose {
            false => LevelFilter::Info,
            true => LevelFilter::Debug,
        };

        let log_spec = LogSpecification::builder().default(log_level_max).build();
        let file_spec = FileSpec::default()
            .directory(&args.build_root)
            .basename(LOG_FILE_NAME)
            .use_timestamp(false);

        let logger = Logger::with(log_spec)
            .log_to_file(file_spec)
            .duplicate_to_stdout(Duplicate::from(log_level_stdout))
            .format(Self::format_log)
            .write_mode(WriteMode::Direct)
            .start()
            .context("Failed to initialize logger")?;

        Ok(Self {
            args,
            logger,
            build_root,
            compiler_map: IndexMap::new(),
            file_relation: FileRelation::new(),
        })
    }

    fn check_debuginfo(&self) -> Result<()> {
        let supported_compilers = self
            .compiler_map
            .values()
            .flat_map(|info| &info.producers)
            .collect::<IndexSet<_>>();
        for debuginfo in &self.args.debuginfo {
            for producer in Dwarf::parse(debuginfo)?.producers() {
                ensure!(
                    supported_compilers.contains(&producer),
                    "{} is not supported",
                    producer.to_string_lossy()
                );
            }
        }

        Ok(())
    }

    fn build_patch(&self, patch_name: &OsStr, binary: &Path, debuginfo: &Path) -> Result<()> {
        const NOTES_OBJECT_NAME: &str = "notes.o";

        let temp_dir = self.build_root.build_dir.join(patch_name);
        let output_dir = self.args.output_dir.as_path();

        let debuginfo_file = temp_dir.join(
            debuginfo
                .file_name()
                .context("Failed to parse debuginfo name")?,
        );
        let output_file = output_dir.join(patch_name);

        fs::create_dir_all(&temp_dir)?;
        fs::copy(debuginfo, &debuginfo_file)?;
        fs::set_permissions(&debuginfo_file, Permissions::from_mode(0o644))?;

        debug!("- Resolving debuginfo");
        resolve::resolve_dynamic(&debuginfo_file).context("Failed to resolve debuginfo")?;

        debug!("- Creating diff objects");
        let binary_objects = self
            .file_relation
            .binary_objects(binary)
            .with_context(|| format!("Failed to find objects of {}", binary.display()))?;

        for (patched_object, original_object) in binary_objects {
            debug!(
                "* {}",
                patched_object
                    .file_name()
                    .unwrap_or(patched_object.as_os_str())
                    .to_string_lossy()
            );
            Self::create_diff_objs(original_object, patched_object, &debuginfo_file, &temp_dir)
                .with_context(|| {
                    format!(
                        "Failed to create diff objects for {}",
                        patch_name.to_string_lossy()
                    )
                })?;
        }

        debug!("- Collecting changes");
        let mut changed_objects =
            elf::find_elf_files(&temp_dir, |_, obj_kind| obj_kind == ObjectKind::Relocatable)?;
        if changed_objects.is_empty() {
            debug!("- No functional changes");
            return Ok(());
        }

        debug!("- Creating patch notes");
        let notes_object = temp_dir.join(NOTES_OBJECT_NAME);
        Self::create_note(&debuginfo_file, &notes_object)
            .context("Failed to create patch notes")?;
        changed_objects.push(notes_object);

        debug!("- Linking patch objects");
        let mut link_compiler = ProducerType::C;
        for object in &changed_objects {
            if Dwarf::parse(object)?
                .producer_types()
                .contains(&ProducerType::Cxx)
            {
                link_compiler = ProducerType::Cxx;
                break;
            }
        }

        let compiler_info = self
            .compiler_map
            .get(&link_compiler)
            .with_context(|| format!("Failed to get link compiler {}", link_compiler))?;
        Self::link_objects(&compiler_info.linker, &changed_objects, &output_file)
            .context("Failed to link patch objects")?;

        debug!("- Resolving patch");
        resolve::resolve_upatch(&output_file, &debuginfo_file)
            .context("Failed to resolve patch")?;

        debug!("- Done");
        Ok(())
    }

    fn build_patches(&self) -> Result<()> {
        for (binary, debuginfo) in self.file_relation.get_files() {
            let binary_name = binary
                .file_name()
                .with_context(|| format!("Failed to parse binary name of {}", binary.display()))?;
            let patch_name = if self.args.prefix.is_empty() {
                binary_name.to_os_string()
            } else {
                concat_os!(&self.args.prefix, "-", binary_name)
            };
            debug!("Generating patch '{}'", patch_name.to_string_lossy());
            self.build_patch(&patch_name, binary, debuginfo)
                .with_context(|| {
                    format!("Failed to build patch '{}'", patch_name.to_string_lossy())
                })?;
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        let compilers = self.args.compiler.as_slice();
        let binary_dir = self.args.binary_dir.as_path();
        let object_dir = self.args.object_dir.as_path();
        let binaries = self.args.binary.as_slice();
        let debuginfos = self.args.debuginfo.as_slice();

        let temp_dir = self.build_root.build_dir.as_path();
        let original_dir = self.build_root.original_dir.as_path();
        let patched_dir = self.build_root.patched_dir.as_path();

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        trace!("{:#?}", self.args);

        info!("Checking compiler(s)");
        self.compiler_map = CompilerInfo::parse(compilers, temp_dir)?;

        info!("------------------------------");
        info!("Compiler");
        info!("------------------------------");
        for (producer_type, compiler_info) in &self.compiler_map {
            info!(
                "[{}] compiler: {}, assembler: {}, linker: {}",
                producer_type,
                compiler_info.binary.display(),
                compiler_info.assembler.display(),
                compiler_info.linker.display(),
            );
        }

        let project = Project::new(&self.args, &self.build_root, &self.compiler_map)?;
        info!("------------------------------");
        info!("Project");
        info!("------------------------------");
        info!("Testing patch file(s)");
        project.test_patches().context("Patch test failed")?;

        info!("Checking debuginfo version(s)");
        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check!")
        } else {
            self.check_debuginfo().context("Debuginfo check failed")?;
        }

        if !self.args.prepare_cmd.is_empty() {
            info!("Preparing '{}'", project);
            project
                .prepare()
                .with_context(|| format!("Failed to prepare {}", project))?;
        }

        if !self.args.clean_cmd.is_empty() {
            info!("Cleaning '{}'", project);
            project
                .clean()
                .with_context(|| format!("Failed to clean {}", project))?;
        }

        if !self.args.keep_line_macros {
            info!("Overriding line macros");
            project
                .override_line_macros()
                .context("Failed to override line macros")?;
        }

        info!("Building '{}'", project);
        project
            .build()
            .with_context(|| format!("Failed to build {}", project))?;
        self.file_relation
            .collect_debuginfo(binary_dir, binaries, debuginfos)?;
        self.file_relation
            .collect_original_build(object_dir, original_dir)?;

        if !self.args.prepare_cmd.is_empty() {
            info!("Preparing '{}'", project);
            project
                .prepare()
                .with_context(|| format!("Failed to prepare {}", project))?;
        }

        if !self.args.clean_cmd.is_empty() {
            info!("Cleaning '{}'", project);
            project
                .clean()
                .with_context(|| format!("Failed to clean {}", project))?;
        }

        info!("Patching '{}'", project);
        project
            .apply_patches()
            .with_context(|| format!("Failed to patch {}", project))?;

        if !self.args.keep_line_macros {
            info!("Overriding line macros");
            project
                .override_line_macros()
                .context("Failed to override line macros")?;
        }

        info!("Building '{}'", project);
        project
            .build()
            .with_context(|| format!("Failed to build {}", project))?;
        self.file_relation
            .collect_patched_build(object_dir, patched_dir)?;
        trace!("{:#?}", self.file_relation);

        info!("------------------------------");
        info!("Patches");
        info!("------------------------------");
        self.build_patches()?;

        if !self.args.skip_cleanup {
            info!("Cleaning up");
            self.build_root.remove().ok();
        }

        info!("Done");
        Ok(())
    }
}

/* Tool functions */
impl UpatchBuild {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> std::io::Result<()> {
        write!(w, "{}", record.args())
    }

    fn create_note<P: AsRef<Path>, Q: AsRef<Path>>(debuginfo: P, output_file: Q) -> Result<()> {
        let debuginfo_file = MappedFile::open(&debuginfo)?;
        let object_file = object::File::parse(debuginfo_file.as_bytes())
            .with_context(|| format!("Failed to parse {}", debuginfo.as_ref().display()))?;

        let mut new_object = write::Object::new(
            object_file.format(),
            object_file.architecture(),
            object_file.endianness(),
        );

        for section in object_file.sections() {
            if section.kind() != SectionKind::Note {
                continue;
            }

            let section_name = section.name().context("Failed to get section name")?;
            let section_data = section.data().context("Failed to get section data")?;
            let section_id =
                new_object.add_section(vec![], section_name.as_bytes().to_vec(), section.kind());

            let new_section = new_object.section_mut(section_id);
            new_section.set_data(section_data, section.align());
            new_section.flags = section.flags();
        }

        let contents = new_object
            .write()
            .context("Failed to serialize note object")?;
        fs::write(output_file, contents)?;

        Ok(())
    }

    fn create_diff_objs(
        original_object: &Path,
        patched_object: &Path,
        debuginfo: &Path,
        output_dir: &Path,
    ) -> Result<()> {
        let ouput_name = original_object.file_name().with_context(|| {
            format!(
                "Failed to parse patch file name of {}",
                original_object.display()
            )
        })?;
        let output_file = output_dir.join(ouput_name);

        let mut command = Command::new(UPATCH_DIFF_BIN);
        command
            .arg("-s")
            .arg(original_object)
            .arg("-p")
            .arg(patched_object)
            .arg("-r")
            .arg(debuginfo)
            .arg("-o")
            .arg(output_file);

        command.stdout(Level::Trace).run_with_output()?.exit_ok()
    }

    fn link_objects<P, I, S, Q>(linker: P, objects: I, output: Q) -> Result<()>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        Q: AsRef<Path>,
    {
        Command::new(linker.as_ref())
            .args(["-r", "-o"])
            .arg(output.as_ref())
            .args(objects)
            .stdout(Level::Trace)
            .run_with_output()?
            .exit_ok()
    }
}

impl Drop for UpatchBuild {
    fn drop(&mut self) {
        self.logger.flush();
        self.logger.shutdown();
    }
}

fn main() {
    let mut builder = match UpatchBuild::new() {
        Ok(instance) => instance,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(-1);
        }
    };

    let log_file = builder.build_root.log_file.clone();
    if let Err(e) = builder.run() {
        error!("Error: {:?}", e);
        error!("For more information, please check {}", log_file.display());

        drop(builder);
        process::exit(-1);
    }
}
