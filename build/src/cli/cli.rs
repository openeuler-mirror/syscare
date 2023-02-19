use regex::Regex;

use log::LevelFilter;
use log::{info, warn, error};

use crate::log::Logger;

use crate::package::{PackageInfo, PackageType};
use crate::package::{RpmExtractor, RpmHelper, RpmSpecHelper, RpmBuilder};

use crate::patch::PatchInfo;
use crate::patch::{PatchHelper, PatchBuilderFactory};

use crate::constants::*;
use crate::util::{sys, fs, serde};

use super::args::CliArguments;
use super::workdir::CliWorkDir;

pub struct PatchBuildCLI {
    workdir: CliWorkDir,
    args:    CliArguments,
}

impl PatchBuildCLI {
    fn canonicalize_input_args(&mut self) -> std::io::Result<()> {
        let args = &mut self.args;

        if !Regex::new(PATCH_NAME_REGEX_STR).unwrap().is_match(&args.patch_name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch name should not contain any special character except '_' & '-'"),
            ));
        }

        if fs::file_ext(&args.source)? != PKG_FILE_EXTENSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Source should be rpm package"),
            ));
        }

        if fs::file_ext(&args.debuginfo)? != PKG_FILE_EXTENSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo should be rpm package"),
            ));
        }

        args.source    = fs::realpath(&args.source)?;
        args.debuginfo = fs::realpath(&args.debuginfo)?;
        args.workdir   = fs::realpath(&args.workdir)?;
        args.output    = fs::realpath(&args.output)?;

        for patch in &mut args.patches {
            if fs::file_ext(&patch)? != PATCH_FILE_EXTENSION {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Patches should be patch file"),
                ));
            }
            *patch = fs::realpath(&patch)?;
        }

        Ok(())
    }

    fn collect_package_info(&self) -> std::io::Result<(PackageInfo, PackageInfo)> {
        let src_pkg_info = PackageInfo::new(&self.args.source)?;
        let dbg_pkg_info = PackageInfo::new(&self.args.debuginfo)?;
        if src_pkg_info.kind != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File \"{}\" is not a source package", self.args.source.display()),
            ));
        }
        if dbg_pkg_info.kind != PackageType::BinaryPackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File \"{}\" is not a debuginfo package", self.args.debuginfo.display()),
            ));
        }

        info!("Source package");
        info!("------------------------------");
        src_pkg_info.print_log(log::Level::Info);
        info!("------------------------------");
        info!("Debuginfo package");
        info!("------------------------------");
        dbg_pkg_info.print_log(log::Level::Info);
        info!("------------------------------");

        Ok((src_pkg_info, dbg_pkg_info))
    }

    fn collect_patch_info(&self, pkg_info: &PackageInfo) -> std::io::Result<PatchInfo> {
        info!("Collecting patch info");
        let patch_info = PatchInfo::new(pkg_info, &self.args)?;

        info!("------------------------------");
        patch_info.print_log(log::Level::Info);
        info!("------------------------------");

        Ok(patch_info)
    }

    fn extract_packages(&self) -> std::io::Result<()> {
        info!("Extracting source package");
        RpmExtractor::extract_package(
            &self.args.source,
            &self.workdir.package.source,
        )?;

        info!("Extracting debuginfo package");
        RpmExtractor::extract_package(
            &self.args.debuginfo,
            &self.workdir.package.debug
        )?;

        Ok(())
    }

    fn complete_build_args(&mut self, pkg_info: &mut PackageInfo) -> std::io::Result<()> {
        let mut args = &mut self.args;

        // Find source directory from extracted package root
        let source_pkg_dir = self.workdir.package.source.as_path();
        let pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let pkg_source_dir = pkg_build_root.sources.as_path();

        // Collect patch info from patched source package
        if let Ok(path) = fs::find_file(pkg_source_dir, PATCH_INFO_FILE_NAME, false, false) {
            let old_patch_info  = serde::deserialize::<_, PatchInfo>(path)?;

            // Override path version
            args.patch_version = u32::max(args.patch_version, old_patch_info.version + 1);
            // Overide path list
            let mut new_patches = PatchHelper::collect_patches(pkg_source_dir)?;
            if !new_patches.is_empty() {
                new_patches.append(&mut args.patches);
                args.patches = new_patches;
            }
            // Override package info
            *pkg_info = old_patch_info.target.to_owned();
        }

        // Override other arguments
        args.target_name.get_or_insert(pkg_info.name.to_owned());
        args.target_arch.get_or_insert(pkg_info.arch.to_owned());
        args.target_epoch.get_or_insert(pkg_info.epoch.to_owned());
        args.target_version.get_or_insert(pkg_info.version.to_owned());
        args.target_release.get_or_insert(pkg_info.release.to_owned());
        args.target_license.get_or_insert(pkg_info.license.to_owned());

        Ok(())
    }

    fn check_build_args(&self, src_pkg_info: &PackageInfo, dbg_pkg_info: &PackageInfo) -> std::io::Result<()> {
        if !dbg_pkg_info.name.contains(&src_pkg_info.name) ||
           (src_pkg_info.arch    != dbg_pkg_info.arch)    ||
           (src_pkg_info.epoch   != dbg_pkg_info.epoch)   ||
           (src_pkg_info.version != dbg_pkg_info.version) ||
           (src_pkg_info.release != dbg_pkg_info.release) {
                return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo package does not match the source package"),
            ));
        }

        let patch_arch = self.args.patch_arch.as_str();
        if patch_arch != sys::cpu_arch() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch arch \"{}\" is unsupported", patch_arch),
            ));
        }

        let target_arch = self.args.target_arch.as_deref().unwrap();
        if self.args.patch_arch.as_str() != target_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Target arch \"{}\" is not match patch arch \"{}\"", target_arch, patch_arch),
            ));
        }

        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }

        Ok(())
    }

    fn build_patch(&self, patch_info: &mut PatchInfo) -> std::io::Result<()> {
        info!("Pareparing to build patch");
        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let spec_file      = RpmHelper::find_spec_file(&pkg_build_root.specs)?;

        RpmBuilder::new(pkg_build_root).build_prepare(spec_file)?;

        info!("Building patch, this may take a while");
        let patch_builder = PatchBuilderFactory::get_builder(patch_info.kind, &self.workdir);
        let builder_args  = patch_builder.parse_builder_args(patch_info, &self.args)?;

        patch_builder.build_patch(&builder_args)?;
        patch_builder.write_patch_info(patch_info, &builder_args)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building patch package");
        let rpm_builder = RpmBuilder::new(self.workdir.package.patch.to_owned());

        rpm_builder.copy_all_files_to_source(&self.workdir.patch.output)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;
        rpm_builder.build_binary_package(
            rpm_builder.generate_spec_file(patch_info)?,
            &self.args.output
        )?;

        Ok(())
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building source package");
        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let rpm_builder    = RpmBuilder::new(pkg_build_root.to_owned());
        let spec_file      = RpmHelper::find_spec_file(pkg_build_root.specs)?;

        RpmSpecHelper::modify_spec_file_by_patches(&spec_file, patch_info)?;
        rpm_builder.copy_patch_file_to_source(patch_info)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;
        rpm_builder.build_source_package(spec_file, &self.args.output)?;

        Ok(())
    }

    fn build_all(&mut self) -> std::io::Result<()> {
        info!("==============================");
        info!("{}", CLI_DESCRIPTION);
        info!("==============================");
        let (mut src_pkg_info, dbg_pkg_info) = self.collect_package_info()?;
        self.extract_packages()?;
        self.complete_build_args(&mut src_pkg_info)?;
        self.check_build_args(&src_pkg_info, &dbg_pkg_info)?;

        let mut patch_info = self.collect_patch_info(&src_pkg_info)?;
        self.build_patch(&mut patch_info)?;
        self.build_patch_package(&patch_info)?;
        self.build_source_package(&patch_info)?;
        info!("Done");

        Ok(())
    }
}

impl PatchBuildCLI {
    fn cli_main(&mut self) -> std::io::Result<()> {
        self.canonicalize_input_args()?;
        self.workdir.create(&self.args.workdir)?;

        Logger::initialize(
            &self.workdir,
            match &self.args.verbose {
                false => LevelFilter::Info,
                true  => LevelFilter::Debug,
            }
        )?;

        self.build_all()?;

        if !self.args.skip_cleanup {
            self.workdir.remove().ok();
        }

        Ok(())
    }

    pub fn run() {
        let mut cli = Self {
            workdir: CliWorkDir::new(),
            args:    CliArguments::new(),
        };

        if let Err(e) = cli.cli_main() {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {}", e)
                },
                true => {
                    error!("Error: {}", e);
                    eprintln!("For more information, please check \"{}\"",
                        cli.workdir.log_file.display()
                    );
                },
            }
        }
    }
}
