// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{env, process, sync::Arc};

use anyhow::{bail, ensure, Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode,
};
use lazy_static::lazy_static;
use log::{error, info, LevelFilter, Record};

use syscare_abi::{PackageInfo, PackageType, PatchInfo, PatchType};
use syscare_common::{concat_os, fs, os};

mod args;
mod build_params;
mod build_root;
mod package;
mod patch;

use args::Arguments;
use build_params::{BuildEntry, BuildParameters};
use build_root::BuildRoot;
use package::{
    PackageBuildRoot, PackageBuilderFactory, PackageFormat, PackageImpl, PackageSpecBuilderFactory,
    PackageSpecWriterFactory,
};
use patch::{PatchBuilderFactory, PatchHelper, PatchMetadata, PATCH_FILE_EXT};

const CLI_NAME: &str = "syscare build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const PATH_ENV_NAME: &str = "PATH";
const PATH_ENV_VALUE: &str = "/usr/libexec/syscare";

const LOG_FILE_NAME: &str = "build";
const KERNEL_PKG_NAME: &str = "kernel";

lazy_static! {
    static ref PKG_IMPL: Arc<PackageImpl> = Arc::new(PackageImpl::new(PackageFormat::RpmPackage));
}

struct SyscareBuild {
    args: Arguments,
    logger: LoggerHandle,
    build_root: BuildRoot,
}

/* Initialization */
impl SyscareBuild {
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
        fs::create_dir_all(&args.output)?;

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
impl SyscareBuild {
    fn check_input_args(&self) -> Result<()> {
        ensure!(
            self.args.patch_arch.as_str() == os::cpu::arch(),
            "Cross compilation is unsupported"
        );

        Ok(())
    }

    fn collect_package_info(&self) -> Result<Vec<PackageInfo>> {
        let mut pkg_list = Vec::new();

        for pkg_path in self.args.source.clone() {
            let mut pkg_info = PKG_IMPL.parse_package_info(&pkg_path)?;

            // Source package's arch is meaningless, override it
            pkg_info.arch = self.args.patch_arch.clone();

            info!("------------------------------");
            info!("Source Package");
            info!("------------------------------");
            info!("{}", pkg_info);

            if pkg_info.kind != PackageType::SourcePackage {
                bail!("File {} is not a source package", pkg_info.short_name());
            }

            pkg_list.push(pkg_info);
        }

        for pkg_path in self.args.debuginfo.clone() {
            let pkg_info = PKG_IMPL.parse_package_info(&pkg_path)?;

            info!("------------------------------");
            info!("Debuginfo Package");
            info!("------------------------------");
            info!("{}", pkg_info);

            if pkg_info.kind != PackageType::BinaryPackage {
                bail!("File {} is not a debuginfo package", pkg_info.short_name());
            }
            if pkg_info.arch != self.args.patch_arch {
                bail!(
                    "Debuginfo package arch {} does not match to patch arch {}",
                    pkg_info.arch,
                    self.args.patch_arch
                );
            }
        }
        info!("------------------------------");

        Ok(pkg_list)
    }

    fn prepare_source_code(
        &self,
        pkg_build_root: &PackageBuildRoot,
        pkg_info_list: Vec<PackageInfo>,
    ) -> Result<Vec<BuildEntry>> {
        let pkg_format = PKG_IMPL.format();
        let pkg_spec_dir = &pkg_build_root.specs;
        let pkg_build_dir = &pkg_build_root.build;

        let mut build_entries = Vec::new();
        for target_pkg in pkg_info_list {
            let pkg_name = &target_pkg.name;
            let spec_file = PKG_IMPL
                .find_spec_file(pkg_spec_dir, pkg_name)
                .with_context(|| format!("Cannot find spec file of package {}", pkg_name))?;

            PackageBuilderFactory::get_builder(pkg_format, pkg_build_root)
                .build_prepare(&spec_file)?;

            let source_dir = PKG_IMPL
                .find_source_directory(pkg_build_dir, pkg_name)
                .with_context(|| format!("Cannot find source directory of package {}", pkg_name))?;

            build_entries.push(BuildEntry {
                target_pkg,
                build_source: source_dir,
                build_spec: spec_file,
            });
        }

        Ok(build_entries)
    }

    fn parse_build_entry(
        &self,
        build_entries: &[BuildEntry],
    ) -> Result<(PatchType, BuildEntry, Option<BuildEntry>)> {
        let pkg_entry = build_entries
            .iter()
            .find(|entry| entry.target_pkg.name != KERNEL_PKG_NAME);
        let kernel_entry = build_entries
            .iter()
            .find(|entry| entry.target_pkg.name == KERNEL_PKG_NAME);

        match (pkg_entry, kernel_entry) {
            (Some(p_entry), Some(k_entry)) => Ok((
                PatchType::KernelPatch,
                p_entry.clone(),
                Some(k_entry.clone()),
            )),
            (None, Some(entry)) => Ok((PatchType::KernelPatch, entry.clone(), None)),
            (Some(entry), None) => Ok((PatchType::UserPatch, entry.clone(), None)),
            (None, None) => bail!("Cannot find any build entry"),
        }
    }
}

/* Main process */
impl SyscareBuild {
    fn prepare_to_build(&mut self) -> Result<BuildParameters> {
        let pkg_root = &self.build_root.package;

        info!("- Collecting patch file(s)");
        let mut patch_files = PatchHelper::collect_patch_files(&self.args.patch)
            .context("Failed to collect patch files")?;

        info!("- Collecting package info");
        let pkg_info_list = self.collect_package_info()?;

        info!("- Extracting source package(s)");
        for pkg_path in &self.args.source {
            PKG_IMPL
                .extract_package(pkg_path, &pkg_root.source)
                .with_context(|| format!("Failed to extract package {}", pkg_path.display()))?;
        }

        info!("- Extracting debuginfo package(s)");
        for pkg_path in &self.args.debuginfo {
            PKG_IMPL
                .extract_package(pkg_path, &pkg_root.debuginfo)
                .with_context(|| format!("Failed to extract package {}", pkg_path.display()))?;
        }

        info!("- Finding package build root");
        let pkg_build_root = PKG_IMPL
            .find_build_root(&pkg_root.source)
            .context("Failed to find package build root")?;

        info!("- Preparing source code");
        let build_entries = self
            .prepare_source_code(&pkg_build_root, pkg_info_list)
            .context("Failed to prepare source code")?;

        info!("- Parsing build entry");
        let (patch_type, mut build_entry, kernel_build_entry) = self
            .parse_build_entry(&build_entries)
            .context("Failed to parse build entry")?;

        info!("- Extracting patch metadata");
        let patch_metadata = PatchMetadata::new(&pkg_build_root.sources);
        if let Ok(saved_patch_info) = patch_metadata.extract() {
            info!("- Applying patch metadata");
            build_entry.target_pkg = saved_patch_info.target;

            // Override package arch
            build_entry.target_pkg.arch = self.args.patch_arch.clone();

            // Override package release
            if self.args.patch_version == saved_patch_info.version {
                self.args.patch_release = self.args.patch_release.max(saved_patch_info.release + 1);
            }

            // Override patch list
            let mut new_patch_files = PatchHelper::collect_patch_files(fs::list_files_by_ext(
                &patch_metadata.metadata_dir,
                PATCH_FILE_EXT,
                fs::TraverseOptions { recursive: false },
            )?)
            .context("Failed to collect patch file from metadata directory")?;

            new_patch_files.extend(patch_files);
            patch_files = new_patch_files;
        }

        info!("- Generating build parameters");
        let build_params = BuildParameters {
            work_dir: self.args.work_dir.to_owned(),
            build_root: self.build_root.to_owned(),
            pkg_build_root,
            build_entry,
            kernel_build_entry,
            patch_name: self.args.patch_name.to_owned(),
            patch_version: self.args.patch_version.to_owned(),
            patch_release: self.args.patch_release,
            patch_arch: self.args.patch_arch.to_owned(),
            patch_description: self.args.patch_description.to_owned(),
            patch_type,
            patch_files,
            jobs: self.args.jobs,
            skip_compiler_check: self.args.skip_compiler_check,
            skip_cleanup: self.args.skip_cleanup,
            verbose: self.args.verbose,
        };

        info!("{}", build_params);
        Ok(build_params)
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> Result<()> {
        let pkg_build_root = &self.build_root.package.build_root;
        let pkg_source_dir = &pkg_build_root.sources;
        let pkg_spec_dir = &pkg_build_root.specs;
        let patch_output_dir = &self.build_root.patch.output;
        let patch_metadata = PatchMetadata::new(pkg_source_dir);

        info!("- Copying patch outputs");
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)
            .context("Failed to copy patch outputs")?;

        info!("- Writing patch metadata");
        patch_metadata
            .write(patch_info, pkg_source_dir)
            .context("Failed to write patch metadata")?;

        info!("- Generating spec file");
        let new_spec_file = PackageSpecBuilderFactory::get_builder(PKG_IMPL.format())
            .build(
                patch_info,
                &self.args.patch_requires,
                pkg_source_dir,
                pkg_spec_dir,
            )
            .context("Failed to generate spec file")?;

        info!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root)
            .build_binary_package(&new_spec_file, &self.args.output)
    }

    fn build_source_package(&self, build_params: &BuildParameters) -> Result<()> {
        info!("- Preparing build requirements");
        let pkg_build_root = &build_params.pkg_build_root;
        let pkg_source_dir = &pkg_build_root.sources;
        let spec_file = &build_params.build_entry.build_spec;
        let patch_metadata = PatchMetadata::new(pkg_source_dir);

        info!("- Modifying package spec file");
        let metadata_pkg = patch_metadata.package_path.clone();
        if !metadata_pkg.exists() {
            // Lacking of metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            let file_list = vec![metadata_pkg];
            PackageSpecWriterFactory::get_writer(PKG_IMPL.format())
                .add_source_files(spec_file, file_list)
                .context("Failed to modify spec file")?;
        }

        info!("- Creating patch metadata");
        patch_metadata
            .create(build_params)
            .context("Failed to create patch metadata")?;

        info!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root).build_source_package(
            build_params,
            spec_file,
            &self.args.output,
        )
    }

    fn run(&mut self) -> Result<()> {
        self.check_input_args()?;

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Preparing to build patch");
        let build_params = self.prepare_to_build()?;

        info!("Building patch, this may take a while");
        let patch_info_list = PatchBuilderFactory::get_builder(&PKG_IMPL, build_params.patch_type)
            .build_patch(&build_params)?;
        ensure!(
            !patch_info_list.is_empty(),
            "Cannot find any patch metadata"
        );

        info!("Building patch package(s)");
        for patch_info in &patch_info_list {
            info!("------------------------------");
            info!("Syscare Patch");
            info!("------------------------------");
            info!("{}", patch_info);
            info!("------------------------------");
            self.build_patch_package(patch_info)?;
        }

        info!("Building source package");
        self.build_source_package(&build_params)?;

        if !self.args.skip_cleanup {
            info!("Cleaning up");
            self.build_root.remove().ok();
        }

        info!("Done");
        Ok(())
    }
}

impl Drop for SyscareBuild {
    fn drop(&mut self) {
        self.logger.flush();
        self.logger.shutdown();
    }
}

fn main() {
    let mut builder = match SyscareBuild::new() {
        Ok(instance) => instance,
        Err(e) => {
            eprintln!("Error: {:?}", e);
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
