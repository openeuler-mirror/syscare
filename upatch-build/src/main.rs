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

use anyhow::{ensure, Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode,
};
use indexmap::IndexSet;
use log::{debug, error, info, warn, Level, LevelFilter, Record};
use object::{write, Object, ObjectSection, SectionKind};
use syscare_common::{concat_os, fs, os, process::Command};

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
use compiler::Compiler;
use dwarf::Dwarf;
use file_relation::FileRelation;
use project::Project;

const CLI_NAME: &str = "upatch build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const PATH_ENV_NAME: &str = "PATH";
const PATH_ENV_VALUE: &str = "/usr/libexec/syscare";

const LOG_FILE_NAME: &str = "build";

struct BuildInfo {
    files: FileRelation,
    linker: PathBuf,
    temp_dir: PathBuf,
    output_dir: PathBuf,
    verbose: bool,
}

struct UpatchBuild {
    args: Arguments,
    logger: LoggerHandle,
    build_root: BuildRoot,
}

/* Initialization */
impl UpatchBuild {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> std::io::Result<()> {
        write!(w, "{}", &record.args())
    }

    fn new() -> Result<Self> {
        // Initialize arguments & prepare environments
        os::umask::set_umask(CLI_UMASK);
        if let Some(path_env) = env::var_os(PATH_ENV_NAME) {
            env::set_var(PATH_ENV_NAME, concat_os!(PATH_ENV_VALUE, ":", path_env));
        }

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

        // Initialize signal handler
        ctrlc::set_handler(|| {
            eprintln!("Interrupt");
        })
        .context("Failed to initialize signal handler")?;

        Ok(Self {
            args,
            logger,
            build_root,
        })
    }
}

/* Tool functions */
impl UpatchBuild {
    fn check_debuginfo(compilers: &[Compiler], debuginfos: &[PathBuf]) -> Result<()> {
        let supported_versions = compilers
            .iter()
            .flat_map(|c| c.versions.iter().map(|s| s.as_os_str()))
            .collect::<IndexSet<_>>();

        debug!("Supported versions:");
        for version in &supported_versions {
            debug!("- {}", version.to_string_lossy());
        }

        for debuginfo in debuginfos {
            let versions = Dwarf::parse_compiler_versions(debuginfo).with_context(|| {
                format!("Failed to parse compiler name of {}", debuginfo.display())
            })?;
            for version in versions {
                ensure!(
                    supported_versions.contains(version.as_os_str()),
                    "{} version mismatched, version={}",
                    debuginfo.display(),
                    version.to_string_lossy()
                );
            }
        }
        Ok(())
    }

    fn create_note<P: AsRef<Path>, Q: AsRef<Path>>(debuginfo: P, path: Q) -> Result<()> {
        let debuginfo_elf = unsafe { memmap2::Mmap::map(&std::fs::File::open(debuginfo)?)? };

        let input_obj =
            object::File::parse(&*debuginfo_elf).context("Failed to parse debuginfo")?;
        let mut output_obj = write::Object::new(
            input_obj.format(),
            input_obj.architecture(),
            input_obj.endianness(),
        );

        for input_section in input_obj.sections() {
            if input_section.kind() != SectionKind::Note {
                continue;
            }

            let section_name = input_section.name().context("Failed to get section name")?;
            let section_data = input_section.data().context("Failed to get section data")?;
            let section_id = output_obj.add_section(
                vec![],
                section_name.as_bytes().to_vec(),
                input_section.kind(),
            );

            let output_section = output_obj.section_mut(section_id);
            output_section.set_data(section_data, input_section.align());
            output_section.flags = input_section.flags();
        }

        let contents = output_obj
            .write()
            .context("Failed to serialize note object")?;
        fs::write(path, contents)?;

        Ok(())
    }

    fn create_diff_objs(
        original_object: &Path,
        patched_object: &Path,
        debuginfo: &Path,
        output_dir: &Path,
        verbose: bool,
    ) -> Result<()> {
        const UPATCH_DIFF_BIN: &str = "upatch-diff";

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
            .arg("-o")
            .arg(output_file)
            .arg("-r")
            .arg(debuginfo);

        if verbose {
            command.arg("-d");
        }

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
            .run()?
            .exit_ok()
    }
}

/* Main process */
impl UpatchBuild {
    fn build_patch(
        &self,
        build_info: &BuildInfo,
        binary: &Path,
        debuginfo: &Path,
        output_file: &Path,
    ) -> Result<()> {
        const OBJECT_EXTENSION: &str = "o";
        const NOTES_OBJECT_NAME: &str = "notes.o";

        let binary_name = binary.file_name().context("Failed to parse binary name")?;
        let debuginfo_name = debuginfo
            .file_name()
            .context("Failed to parse debuginfo name")?;
        let temp_dir = build_info.temp_dir.join(binary_name);
        let new_debuginfo = temp_dir.join(debuginfo_name);

        debug!("- Preparing to build patch");
        fs::create_dir_all(&temp_dir)?;
        fs::copy(debuginfo, &new_debuginfo)?;
        fs::set_permissions(&new_debuginfo, Permissions::from_mode(0o644))?;

        debug!("- Resolving debuginfo");
        resolve::resolve_dynamic(&new_debuginfo).context("Failed to resolve debuginfo")?;

        debug!("- Creating diff objects");
        let patched_objects = build_info
            .files
            .get_patched_objects(binary)
            .with_context(|| format!("Failed to find objects of {}", binary.display()))?;

        for patched_object in patched_objects {
            let original_object = build_info
                .files
                .get_original_object(patched_object)
                .with_context(|| {
                    format!(
                        "Failed to find patched object of {}",
                        patched_object.display()
                    )
                })?;

            UpatchBuild::create_diff_objs(
                original_object,
                patched_object,
                &new_debuginfo,
                &temp_dir,
                build_info.verbose,
            )
            .with_context(|| format!("Failed to create diff objects for {}", binary.display()))?;
        }

        debug!("- Collecting changes");
        let mut changed_objects = fs::list_files_by_ext(
            &temp_dir,
            OBJECT_EXTENSION,
            fs::TraverseOptions { recursive: false },
        )?;
        if changed_objects.is_empty() {
            debug!("- No functional changes");
            return Ok(());
        }

        debug!("- Creating patch notes");
        let notes_object = temp_dir.join(NOTES_OBJECT_NAME);
        Self::create_note(&new_debuginfo, &notes_object).context("Failed to create patch notes")?;
        changed_objects.push(notes_object);

        debug!("- Linking patch objects");
        Self::link_objects(&build_info.linker, &changed_objects, output_file)
            .context("Failed to link patch objects")?;

        debug!("- Resolving patch");
        resolve::resolve_upatch(output_file, &new_debuginfo).context("Failed to resolve patch")?;

        debug!("- Patch: {}", output_file.display());
        Ok(())
    }

    fn build_patches(&self, build_info: BuildInfo, name: &OsStr) -> Result<()> {
        for (binary, debuginfo) in build_info.files.get_files() {
            let binary_name = binary
                .file_name()
                .with_context(|| format!("Failed to parse binary name of {}", binary.display()))?;
            let patch_name = if name.is_empty() {
                binary_name.to_os_string()
            } else {
                concat_os!(name, "-", binary_name)
            };
            let output_file = build_info.output_dir.join(&patch_name);

            info!("Generating patch for '{}'", patch_name.to_string_lossy());
            self.build_patch(&build_info, binary, debuginfo, &output_file)
                .with_context(|| {
                    format!("Failed to build patch '{}'", patch_name.to_string_lossy())
                })?;
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        let name = self.args.name.as_os_str();
        let output_dir = self.args.output_dir.as_path();
        let object_dir = self.args.object_dir.as_path();
        let binaries = self.args.elf.as_slice();
        let debuginfos = self.args.debuginfo.as_slice();
        let verbose = self.args.verbose;

        let temp_dir = self.build_root.temp_dir.as_path();
        let original_dir = self.build_root.original_dir.as_path();
        let patched_dir = self.build_root.patched_dir.as_path();

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Checking compiler(s)");

        let compilers = Compiler::parse(&self.args.compiler, temp_dir)?;
        let linker = compilers
            .iter()
            .map(|c| c.linker.clone())
            .collect::<IndexSet<_>>()
            .pop()
            .context("Failed to find any linker")?;

        debug!("------------------------------");
        debug!("Compiler");
        debug!("------------------------------");
        for compiler in &compilers {
            debug!("{}", compiler);
        }
        debug!("------------------------------");

        let project = Project::new(&self.args, &self.build_root);
        info!("------------------------------");
        info!("Project {}", project);
        info!("------------------------------");
        info!("Testing patch file(s)");
        project
            .test_patches(&self.args.patch)
            .context("Patch test failed")?;

        info!("Checking debuginfo version(s)");
        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check!")
        } else {
            Self::check_debuginfo(&compilers, debuginfos).context("Debuginfo check failed")?;
        }

        let mut files = FileRelation::new();

        info!("Preparing {}", project);
        project
            .prepare()
            .with_context(|| format!("Failed to prepare {}", project))?;

        info!("Building {}", project);
        project
            .build()
            .with_context(|| format!("Failed to build {}", project))?;

        info!("Collecting file relations");
        files.collect_debuginfo(binaries, debuginfos)?;
        files.collect_original_build(object_dir, original_dir)?;

        info!("Preparing {}", project);
        project
            .prepare()
            .with_context(|| format!("Failed to prepare {}", project))?;

        info!("Patching {}", project);
        project
            .apply_patches(&self.args.patch)
            .with_context(|| format!("Failed to patch {}", project))?;

        info!("Rebuilding {}", project);
        project
            .rebuild()
            .with_context(|| format!("Failed to rebuild {}", project))?;

        info!("Collecting file relations");
        files.collect_patched_build(object_dir, patched_dir)?;

        let build_info = BuildInfo {
            linker,
            files,
            temp_dir: temp_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            verbose,
        };
        self.build_patches(build_info, name)?;

        if !self.args.skip_cleanup {
            info!("Cleaning up");
            self.build_root.remove().ok();
        }

        info!("Done");
        Ok(())
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
