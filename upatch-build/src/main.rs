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
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn, Level, LevelFilter, Record};
use object::{write, Object, ObjectSection, SectionKind};
use syscare_common::{ffi::OsStringExt, fs, os, process::Command};

mod args;
mod build_root;
mod compiler;
mod compiler_hacker;
mod dwarf;
mod elf;
mod file_relations;
mod pattern_path;
mod project;
mod resolve;
mod rpc;

use args::Arguments;
use build_root::BuildRoot;
use compiler::Compiler;
use compiler_hacker::CompilerHacker;
use dwarf::Dwarf;
use file_relations::{BinaryRelation, ObjectRelation};
use project::Project;

const CLI_NAME: &str = "syscare build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const LOG_FILE_NAME: &str = "build";
const UPATCHD_SOCKET_NAME: &str = "upatchd.sock";

struct BuildInfo<'a> {
    compiler_map: IndexMap<&'a OsStr, &'a Compiler>,
    binaries: Vec<BinaryRelation>,
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
    fn check_debuginfo(
        compiler_map: &IndexMap<&OsStr, &Compiler>,
        debuginfos: &[PathBuf],
    ) -> Result<()> {
        for debuginfo in debuginfos {
            let compiler_name = Dwarf::parse_compiler_name(debuginfo)?;
            ensure!(
                compiler_map.contains_key(compiler_name.as_os_str()),
                "{} version mismatched, version={}",
                debuginfo.display(),
                compiler_name.to_string_lossy()
            );
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
        &self,
        objects: &[ObjectRelation],
        debuginfo: &Path,
        output_dir: &Path,
        verbose: bool,
    ) -> Result<()> {
        const UPATCH_DIFF_BIN: &str = "/usr/libexec/syscare/upatch-diff";

        for object in objects {
            let original_object = object.original_object.as_path();
            let patched_object = object.patched_object.as_path();

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

            command.stdout(Level::Trace).run_with_output()?.exit_ok()?
        }

        Ok(())
    }
}

/* Main process */
impl UpatchBuild {
    fn build_patch(
        &self,
        build_info: &BuildInfo,
        binary: &BinaryRelation,
        output_file: &Path,
    ) -> Result<()> {
        const OBJECT_EXTENSION: &str = "o";
        const NOTES_OBJECT_NAME: &str = "notes.o";

        let binary_name = binary
            .path
            .file_name()
            .context("Failed to parse binary name")?;
        let debuginfo_name = binary
            .debuginfo
            .file_name()
            .context("Failed to parse debuginfo name")?;
        let output_dir = build_info.temp_dir.join(binary_name);
        let debuginfo = output_dir.join(debuginfo_name);

        debug!("- Preparing to build patch");
        fs::create_dir_all(&output_dir)?;
        fs::copy(&binary.debuginfo, &debuginfo)?;
        fs::set_permissions(&debuginfo, Permissions::from_mode(0o644))?;

        debug!("- Resolving debuginfo");
        resolve::resolve_dynamic(&debuginfo).context("Failed to resolve debuginfo")?;

        debug!("- Creating diff objects");
        self.create_diff_objs(&binary.objects, &debuginfo, &output_dir, build_info.verbose)
            .with_context(|| format!("Failed to create diff objects {}", binary.path.display()))?;

        debug!("- Collecting changes");
        let mut changed_objects = fs::list_files_by_ext(
            &output_dir,
            OBJECT_EXTENSION,
            fs::TraverseOptions { recursive: false },
        )?;
        if changed_objects.is_empty() {
            debug!("- Patch: No functional changes");
            return Ok(());
        }

        debug!("- Creating patch notes");
        let notes_object = output_dir.join(NOTES_OBJECT_NAME);
        Self::create_note(&debuginfo, &notes_object).context("Failed to create patch notes")?;
        changed_objects.push(notes_object);

        debug!("- Linking patch objects");
        let compiler_name = &binary.compiler;
        let compiler = build_info
            .compiler_map
            .get(compiler_name.as_os_str())
            .with_context(|| format!("Cannot find compiler {}", compiler_name.to_string_lossy()))?;
        compiler
            .link_objects(&changed_objects, output_file)
            .context("Failed to link patch objects")?;

        debug!("- Resolving patch");
        resolve::resolve_upatch(output_file, &debuginfo).context("Failed to resolve patch")?;

        debug!("- Patch: {}", output_file.display());
        Ok(())
    }

    fn build_patches(&self, build_info: BuildInfo, name: &OsStr) -> Result<()> {
        for binary in &build_info.binaries {
            let binary_path = binary.path.as_path();
            let binary_name = binary.path.file_name().with_context(|| {
                format!("Failed to parse binary name of {}", binary_path.display())
            })?;
            let patch_name = match name.is_empty() {
                true => binary_name.to_os_string(),
                false => name.to_os_string().join("-").join(binary_name),
            };
            let output_file = build_info.output_dir.join(&patch_name);

            info!("Generating patch {}", patch_name.to_string_lossy());
            self.build_patch(&build_info, binary, &output_file)
                .with_context(|| {
                    format!("Failed to build patch {}", patch_name.to_string_lossy())
                })?;
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        let socket_file = self.args.work_dir.join(UPATCHD_SOCKET_NAME);
        let name = self.args.name.as_os_str();
        let source_dir = self.args.source_dir.as_path();
        let output_dir = self.args.output_dir.as_path();
        let binaries = self.args.elf.as_slice();
        let debuginfos = self.args.debuginfo.as_slice();
        let verbose = self.args.verbose;

        let temp_dir = self.build_root.output_dir.as_path();
        let original_dir = self.build_root.original_dir.as_path();
        let patched_dir = self.build_root.patched_dir.as_path();

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Checking compiler(s)");
        let compilers = Compiler::parse(&self.args.compiler, temp_dir)?;
        let compiler_map = compilers
            .iter()
            .map(|compiler| (compiler.name.as_os_str(), compiler))
            .collect::<IndexMap<_, _>>();
        debug!("------------------------------");
        debug!("Compiler");
        debug!("------------------------------");
        for compiler in &compilers {
            debug!("{}", compiler);
        }
        debug!("------------------------------");
        let hacker_guard =
            CompilerHacker::new(&compilers, socket_file).context("Failed to hack compilers")?;

        let project = Project::new(source_dir);
        info!("------------------------------");
        info!("Project {}", project.name());
        info!("------------------------------");
        info!("Testing patch file(s)");
        project
            .test_patches(&self.args.patch)
            .context("Patch test failed")?;

        info!("Checking debuginfo version(s)");
        match self.args.skip_compiler_check {
            false => {
                Self::check_debuginfo(&compiler_map, debuginfos)
                    .context("Debuginfo check failed")?;
            }
            true => warn!("Warning: Skipped compiler version check!"),
        }

        // Build unpatched source code
        info!("Building {}", project.name());
        project
            .build(&self.args.build_source_cmd, original_dir)
            .with_context(|| format!("Failed to build {}", project.name()))?;

        // Patch project
        info!("Patching {}", project.name());
        project
            .apply_patches(&self.args.patch)
            .with_context(|| format!("Failed to patch {}", project.name()))?;

        // Build patched source code
        info!("Rebuilding {}", project.name());
        project
            .build(self.args.build_patch_cmd.clone(), patched_dir)
            .with_context(|| format!("Failed to rebuild {}", project.name()))?;

        // Unhack compilers
        drop(hacker_guard);

        info!("Parsing file relations");
        let binaries = BinaryRelation::parse(binaries, debuginfos, original_dir, patched_dir)
            .context("Failed to parse file relations")?;
        for binary in &binaries {
            trace!("{}", binary);
        }

        let build_info = BuildInfo {
            compiler_map,
            binaries,
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
