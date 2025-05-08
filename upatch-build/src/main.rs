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

use std::{
    env,
    ffi::OsStr,
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process,
};

use anyhow::{bail, ensure, Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode,
};
use indexmap::{IndexMap, IndexSet};
use log::{debug, error, info, trace, warn, Level, LevelFilter, Record};
use object::{write, Object, ObjectKind, ObjectSection, SectionKind};

use syscare_common::{concat_os, fs, os, process::Command};

mod args;
mod build_root;
mod compiler;
mod dwarf;
mod elf;
mod file_relation;
mod project;
mod resolve;

use crate::{
    args::Arguments,
    build_root::BuildRoot,
    compiler::Compiler,
    dwarf::{ProducerParser, ProducerType},
    file_relation::FileRelation,
    project::Project,
};

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
    compiler_map: IndexMap<ProducerType, Compiler>,
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

    fn detect_compilers(&mut self) -> Result<()> {
        let mut c_compilers = 0usize;
        let mut cxx_compilers = 0usize;

        for compiler_path in &self.args.compiler {
            let compiler = Compiler::parse(compiler_path, &self.build_root.build_dir)
                .with_context(|| format!("Failed to detect {}", compiler_path.display()))?;
            match compiler.kind {
                ProducerType::GnuC | ProducerType::ClangC => c_compilers += 1,
                ProducerType::GnuCxx | ProducerType::ClangCxx => cxx_compilers += 1,
                _ => bail!("Unknown compiler type"),
            }
            info!(
                "[{}] name: {}, version: {}",
                compiler.kind,
                compiler,
                compiler.version.to_string_lossy(),
            );
            self.compiler_map.insert(compiler.kind, compiler);
        }

        ensure!(
            c_compilers <= 1 && cxx_compilers <= 1,
            "Cannot define multiple C/C++ compilers"
        );
        self.compiler_map.sort_keys();

        Ok(())
    }

    fn check_compiler_version(&self) -> Result<()> {
        for path in &self.args.debuginfo {
            let producer_parser = ProducerParser::open(path)
                .with_context(|| format!("Failed to open {}", path.display()))?;
            let producer_iter = producer_parser
                .parse()
                .with_context(|| format!("Failed to parse {}", path.display()))?;

            for parse_result in producer_iter {
                let producer = parse_result.context("Failed to parse debuginfo producer")?;
                if producer.is_assembler() {
                    continue;
                }
                let matched = self
                    .compiler_map
                    .get(&producer.kind)
                    .map(|compiler| compiler.version == producer.version)
                    .unwrap_or(false);
                ensure!(matched, "Producer {} mismatched", producer);
            }
        }

        Ok(())
    }

    fn find_linker(&self, debuginfo: &Path) -> Result<&Path> {
        let mut producers = ProducerParser::open(debuginfo)
            .with_context(|| format!("Failed to open {}", debuginfo.display()))?
            .parse()
            .with_context(|| format!("Failed to parse {}", debuginfo.display()))?
            .filter_map(|result| result.ok())
            .collect::<IndexSet<_>>();
        producers.sort();

        let compiler = producers
            .pop()
            .and_then(|producer| self.compiler_map.get(&producer.kind))
            .context("Cannot find linking compiler")?;

        Ok(compiler.linker.as_path())
    }

    fn link_objects(&self, objects: &[PathBuf], debuginfo: &Path, output: &Path) -> Result<()> {
        let linker = self.find_linker(debuginfo).context("Cannot find linker")?;

        Command::new(linker)
            .args(["-r", "-o"])
            .arg(output)
            .args(objects)
            .stdout(Level::Trace)
            .run_with_output()?
            .exit_ok()
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
        let mut objects =
            elf::find_elf_files(&temp_dir, |_, kind| matches!(kind, ObjectKind::Relocatable))?;
        if objects.is_empty() {
            debug!("- No functional changes");
            return Ok(());
        }

        debug!("- Creating patch notes");
        let notes_object = temp_dir.join(concat_os!(patch_name, "-", NOTES_OBJECT_NAME));
        Self::create_note(&debuginfo_file, &notes_object)
            .context("Failed to create patch notes")?;
        objects.push(notes_object);

        debug!("- Linking patch");
        self.link_objects(&objects, &debuginfo_file, &output_file)
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
        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        trace!("{:#?}", self.args);

        info!("Detecting compiler(s)");
        info!("------------------------------");
        info!("Compiler");
        info!("------------------------------");
        self.detect_compilers()?;

        let project = Project::new(&self.args, &self.build_root, &self.compiler_map)?;
        info!("------------------------------");
        info!("Project");
        info!("------------------------------");
        info!("Testing patch file(s)");
        project.test_patches().context("Patch test failed")?;

        info!("Checking compiler version(s)");
        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check!")
        } else {
            self.check_compiler_version()
                .context("Compiler version check failed")?;
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

        let binary_dir = self.args.binary_dir.as_path();
        let object_dir = self.args.object_dir.as_path();
        let binaries = self.args.binary.as_slice();
        let debuginfos = self.args.debuginfo.as_slice();

        let original_dir = self.build_root.original_dir.as_path();
        let patched_dir = self.build_root.patched_dir.as_path();

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

    fn create_note<P: AsRef<Path>, Q: AsRef<Path>>(
        debuginfo_file: P,
        output_file: Q,
    ) -> Result<()> {
        let debuginfo_file = debuginfo_file.as_ref();
        let mmap = fs::mmap(debuginfo_file)
            .with_context(|| format!("Failed to mmap file {}", debuginfo_file.display()))?;
        let file = object::File::parse(mmap.as_ref())
            .with_context(|| format!("Failed to parse {}", debuginfo_file.display()))?;

        let mut new_object =
            write::Object::new(file.format(), file.architecture(), file.endianness());

        for section in file.sections() {
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
