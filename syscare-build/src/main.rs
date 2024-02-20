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

use std::path::PathBuf;
use std::{process::exit, sync::Arc};

use anyhow::{bail, ensure, Context, Result};
use lazy_static::lazy_static;
use log::{debug, error, info, LevelFilter};

use parking_lot::Mutex;
use syscare_abi::{PackageInfo, PackageType, PatchInfo, PatchType};
use syscare_common::{os, util::fs};

mod args;
mod build_params;
mod build_root;
mod logger;
mod package;
mod patch;
mod util;

use args::Arguments;
use build_params::{BuildEntry, BuildParameters};
use build_root::BuildRoot;
use logger::Logger;
use package::{PackageBuildRoot, PackageBuilderFactory, PackageFormat, PackageImpl};
use patch::{PatchBuilderFactory, PatchHelper, PatchMetadata, PATCH_FILE_EXT};

use crate::package::{PackageSpecBuilderFactory, PackageSpecWriterFactory};

const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const KERNEL_PKG_NAME: &str = "kernel";

lazy_static! {
    static ref PKG_IMPL: Arc<PackageImpl> = Arc::new(PackageImpl::new(PackageFormat::RpmPackage));
}

pub struct SyscareBuilder {
    args: Arguments,
    build_root: BuildRoot,
}

impl SyscareBuilder {
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
                bail!(
                    "Package \"{}\" is not a source package",
                    pkg_info.short_name()
                );
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
                bail!(
                    "Package \"{}\" is not a debuginfo package",
                    pkg_info.short_name()
                );
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
        let pkg_spec_dir = &pkg_build_root.specs;
        let pkg_build_dir = &pkg_build_root.build;

        let mut build_entries = Vec::new();
        for target_pkg in pkg_info_list {
            let pkg_name = &target_pkg.name;
            let pkg_format = PKG_IMPL.format();

            let spec_file = PKG_IMPL
                .find_spec_file(pkg_spec_dir, pkg_name)
                .with_context(|| format!("Cannot find spec file of package \"{}\"", pkg_name))?;

            PackageBuilderFactory::get_builder(pkg_format, pkg_build_root)
                .build_prepare(&spec_file)?;

            let source_dir = PKG_IMPL
                .find_source_directory(pkg_build_dir, pkg_name)
                .with_context(|| {
                    format!("Cannot find source directory of package \"{}\"", pkg_name)
                })?;

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
            (Some(entry), Some(kernel_entry)) => Ok((
                PatchType::KernelPatch,
                entry.clone(),
                Some(kernel_entry.clone()),
            )),
            (None, Some(entry)) => Ok((PatchType::KernelPatch, entry.clone(), None)),
            (Some(entry), None) => Ok((PatchType::UserPatch, entry.clone(), None)),
            (None, None) => bail!("Cannot find any build entry"),
        }
    }

    fn prepare_to_build(&mut self) -> Result<BuildParameters> {
        let pkg_root = &self.build_root.package;

        debug!("- Collecting patch file(s)");
        let mut patch_files = PatchHelper::collect_patch_files(&self.args.patch)
            .context("Failed to collect patch files")?;

        debug!("- Collecting package info");
        let pkg_info_list = self.collect_package_info()?;

        debug!("- Extracting source package(s)");
        for pkg_path in &self.args.source {
            PKG_IMPL
                .extract_package(pkg_path, &pkg_root.source)
                .with_context(|| format!("Failed to extract package \"{}\"", pkg_path.display()))?;
        }

        debug!("- Extracting debuginfo package(s)");
        for pkg_path in &self.args.debuginfo {
            PKG_IMPL
                .extract_package(pkg_path, &pkg_root.debuginfo)
                .with_context(|| format!("Failed to extract package \"{}\"", pkg_path.display()))?;
        }

        debug!("- Finding package build root");
        let pkg_build_root = PKG_IMPL
            .find_build_root(&pkg_root.source)
            .context("Failed to find package build root")?;

        debug!("- Preparing source code");
        let build_entries = self
            .prepare_source_code(&pkg_build_root, pkg_info_list)
            .context("Failed to prepare source code")?;

        debug!("- Parsing build entry");
        let (patch_type, mut build_entry, kernel_build_entry) = self
            .parse_build_entry(&build_entries)
            .context("Failed to parse build entry")?;

        debug!("- Extracting patch metadata");
        let patch_metadata = PatchMetadata::new(&pkg_build_root.sources);
        if let Ok(saved_patch_info) = patch_metadata.extract() {
            debug!("- Applying patch metadata");
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

        debug!("- Generating build parameters");
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

        debug!("- Copying patch outputs");
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)
            .context("Failed to copy patch outputs")?;

        debug!("- Writing patch metadata");
        patch_metadata
            .write(patch_info, pkg_source_dir)
            .context("Failed to write patch metadata")?;

        debug!("- Generating spec file");
        let new_spec_file = PackageSpecBuilderFactory::get_builder(PKG_IMPL.format())
            .build(
                patch_info,
                &self.args.patch_requires,
                pkg_source_dir,
                pkg_spec_dir,
            )
            .context("Failed to generate spec file")?;

        debug!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root)
            .build_binary_package(&new_spec_file, &self.args.output)
    }

    fn build_source_package(&self, build_params: &BuildParameters) -> Result<()> {
        debug!("- Preparing build requirements");
        let pkg_build_root = &build_params.pkg_build_root;
        let pkg_source_dir = &pkg_build_root.sources;
        let spec_file = &build_params.build_entry.build_spec;
        let patch_metadata = PatchMetadata::new(pkg_source_dir);

        debug!("- Modifying package spec file");
        let metadata_pkg = patch_metadata.package_path.clone();
        if !metadata_pkg.exists() {
            // Lacking of metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            let file_list = vec![metadata_pkg];
            PackageSpecWriterFactory::get_writer(PKG_IMPL.format())
                .add_source_files(spec_file, file_list)
                .context("Failed to modify spec file")?;
        }

        debug!("- Creating patch metadata");
        patch_metadata
            .create(build_params)
            .context("Failed to create patch metadata")?;

        debug!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root).build_source_package(
            build_params,
            spec_file,
            &self.args.output,
        )
    }

    fn clean_up(&mut self) {
        self.build_root.remove().ok();
    }
}

impl SyscareBuilder {
    fn new() -> Result<Self> {
        let mut args = Arguments::new()?;
        Self::check_input_args(&args)?;

        os::umask::set_umask(CLI_UMASK);

        args.build_root = args
            .build_root
            .join(format!("syscare-build.{}", os::process::id()));
        let build_root = BuildRoot::new(&args.build_root)?;

        Logger::initialize(
            &args.build_root,
            LevelFilter::Trace,
            match &args.verbose {
                false => LevelFilter::Info,
                true => LevelFilter::Debug,
            },
        )?;

        Ok(SyscareBuilder { args, build_root })
    }

    fn check_input_args(args: &Arguments) -> Result<()> {
        let pkg_file_ext = PKG_IMPL.extension();

        for source_pkg in &args.source {
            if !source_pkg.is_file() || fs::file_ext(source_pkg) != pkg_file_ext {
                bail!("File \"{}\" is not a rpm package", source_pkg.display());
            }
        }

        for debug_pkg in &args.debuginfo {
            if !debug_pkg.is_file() || fs::file_ext(debug_pkg) != pkg_file_ext {
                bail!("File \"{}\" is not a rpm package", debug_pkg.display());
            }
        }

        for patch_file in &args.patch {
            if !patch_file.is_file() || fs::file_ext(patch_file) != PATCH_FILE_EXT {
                bail!("File \"{}\" is not a patch file", patch_file.display());
            }
        }

        let workdir = &args.work_dir;
        if !workdir.exists() {
            fs::create_dir_all(workdir)?;
        }
        if !workdir.is_dir() {
            bail!("Path \"{}\" is not a directory", workdir.display());
        }

        let output = &args.output;
        if !output.exists() {
            fs::create_dir_all(output)?;
        }
        if !output.is_dir() {
            bail!("Path \"{}\" is not a directory", output.display());
        }

        if args.jobs == 0 {
            bail!("Parallel build job number cannot be zero");
        }

        Ok(())
    }

    fn build_main(mut self, log_file: Arc<Mutex<PathBuf>>) -> Result<()> {
        *log_file.lock() = self.build_root.log_file.clone();

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Preparing to build patch");
        let build_params = self.prepare_to_build()?;

        info!("Building patch, this may take a while");
        let patch_type = build_params.patch_type;
        let patch_info_list = PatchBuilderFactory::get_builder(patch_type)
            .build_patch(&build_params)
            .with_context(|| format!("{}Builder: Failed to build patch", patch_type))?;

        info!("Building patch package(s)");
        ensure!(
            !patch_info_list.is_empty(),
            "Cannot find any patch metadata"
        );

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
            self.clean_up();
        }

        info!("Done");
        Ok(())
    }

    fn setup_signal_handlers() -> Result<()> {
        ctrlc::set_handler(|| {
            error!("Received termination signal");
        })
        .context("Failed to setup signal handler")?;

        Ok(())
    }

    fn start_and_run(log_file: Arc<Mutex<PathBuf>>) -> Result<()> {
        Self::setup_signal_handlers()?;
        Self::new()?.build_main(log_file)
    }
}

fn main() {
    let log_file = Arc::new(Mutex::new(PathBuf::new()));
    let exit_code = match SyscareBuilder::start_and_run(log_file.clone()) {
        Ok(_) => 0,
        Err(e) => {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {}", e);
                }
                true => {
                    error!("Error: {:?}", e);
                    eprintln!(
                        "For more information, please check \"{}\"",
                        log_file.lock().display()
                    );
                }
            }
            1
        }
    };
    exit(exit_code);
}
