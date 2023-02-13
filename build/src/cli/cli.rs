use regex::Regex;

use log::LevelFilter;
use log::{info, warn, error};

use crate::log::Logger;

use crate::package::{PackageInfo, PackageType};
use crate::package::{RpmExtractor, RpmHelper, RpmSpecHelper, RpmBuilder};

use crate::patch::PatchInfo;
use crate::patch::{PatchHelper, PatchBuilderFactory};

use crate::constants::*;
use crate::util::{sys, fs};

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
        args.source = fs::realpath(&args.source)?;

        if fs::file_ext(&args.debuginfo)? != PKG_FILE_EXTENSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo should be rpm package"),
            ));
        }
        args.debuginfo = fs::realpath(&args.debuginfo)?;

        args.workdir = fs::realpath(&args.workdir)?;
        args.output  = fs::realpath(&args.output)?;

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

    fn extract_packages(&self) -> std::io::Result<(PackageInfo, PackageInfo)> {
        info!("Extracting source package");
        let src_pkg_info = RpmExtractor::extract_package(
            &self.args.source,
            self.workdir.package_root().source_pkg_dir(),
        )?;
        if src_pkg_info.get_type() != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not a source package", src_pkg_info),
            ));
        }
        info!("------------------------------");
        info!("{}", src_pkg_info);
        info!("------------------------------\n");

        info!("Extracting debuginfo package");
        let dbg_pkg_info = RpmExtractor::extract_package(
            &self.args.debuginfo,
            self.workdir.package_root().debug_pkg_dir()
        )?;
        if dbg_pkg_info.get_type() != PackageType::BinaryPackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not a debuginfo package", dbg_pkg_info),
            ));
        }
        info!("------------------------------");
        info!("{}", dbg_pkg_info);
        info!("------------------------------\n");

        Ok((src_pkg_info, dbg_pkg_info))
    }

    fn collect_patch_info(&self, pkg_info: &PackageInfo) -> std::io::Result<PatchInfo> {
        info!("Collecting patch info");

        let patch_info = PatchInfo::new(pkg_info, &self.args)?;
        info!("------------------------------");
        info!("{}", patch_info);
        info!("------------------------------\n");

        Ok(patch_info)
    }

    fn complete_build_args(&mut self, pkg_info: &mut PackageInfo) -> std::io::Result<()> {
        let mut args = &mut self.args;

        // If the source package is kernel, append target elf name 'vmlinux' to arguments
        if pkg_info.get_name() == KERNEL_PKG_NAME {
            args.target_elfname.get_or_insert(KERNEL_VMLINUX_FILE.to_owned());
        }

        // Find source directory from extracted package root
        let source_pkg_dir = self.workdir.package_root().source_pkg_dir();
        let pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let pkg_source_dir = pkg_build_root.sources_dir();

        // Collect patch info from patched source package
        if let Ok(path) = fs::find_file(pkg_source_dir, PATCH_INFO_FILE_NAME, false, false) {
            let patched_path_info  = PatchInfo::read_from_file(path)?;
            let patched_pkg_info  = patched_path_info.get_target();

            // Override path version
            let arg_version = args.patch_version;
            let new_version = patched_path_info.get_version() + 1;
            args.patch_version = u32::max(arg_version, new_version);
            // Overide path list
            let mut package_patches = PatchHelper::collect_patches(pkg_source_dir)?;
            if !package_patches.is_empty() {
                package_patches.append(&mut args.patches);
                args.patches = package_patches;
            }
            // Override package info
            *pkg_info = patched_pkg_info.to_owned();
        }

        // Override other arguments
        args.target_name.get_or_insert(pkg_info.get_name().to_owned());
        args.target_arch.get_or_insert(pkg_info.get_arch().to_owned());
        args.target_epoch.get_or_insert(pkg_info.get_epoch().to_owned());
        args.target_version.get_or_insert(pkg_info.get_version().to_owned());
        args.target_release.get_or_insert(pkg_info.get_release().to_owned());
        args.target_license.get_or_insert(pkg_info.get_license().to_owned());

        Ok(())
    }

    fn check_build_args(&self, src_pkg_info: &PackageInfo, dbg_pkg_info: &PackageInfo) -> std::io::Result<()> {
        let args = &self.args;

        if !dbg_pkg_info.get_name().contains(src_pkg_info.get_name()) ||
           (src_pkg_info.get_arch()    != dbg_pkg_info.get_arch())    ||
           (src_pkg_info.get_epoch()   != dbg_pkg_info.get_epoch())   ||
           (src_pkg_info.get_version() != dbg_pkg_info.get_version()) ||
           (src_pkg_info.get_release() != dbg_pkg_info.get_release()) {
                return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo package does not match the source package"),
            ));
        }

        let patch_arch = args.patch_arch.as_str();
        if patch_arch != sys::get_cpu_arch() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch arch '{}' is unsupported", patch_arch),
            ));
        }

        let target_arch = args.target_arch.as_deref().unwrap();
        if args.patch_arch.as_str() != target_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Target arch '{}' is not match patch arch '{}'", target_arch, patch_arch),
            ));
        }

        if args.target_elfname.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch target elf name is empty"),
            ));
        }

        if args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }

        Ok(())
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let source_pkg_dir = self.workdir.package_root().source_pkg_dir();
        let pkg_output_dir = &self.args.output;

        info!("Building source package");
        let source_pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let source_pkg_spec_dir  = source_pkg_build_root.specs_dir();

        let spec_file_path = RpmHelper::find_spec_file(source_pkg_spec_dir)?;
        RpmSpecHelper::modify_spec_file_by_patches(&spec_file_path, patch_info)?;

        let rpm_builder = RpmBuilder::new(source_pkg_build_root);
        rpm_builder.copy_patch_file_to_source(patch_info)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;
        rpm_builder.build_source_package(pkg_output_dir)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let workdir = &self.workdir;
        let args    = &self.args;

        let patch_output_dir = self.workdir.patch_root().output_dir();
        let pkg_build_root   = self.workdir.package_root().build_root();
        let pkg_output_dir   = &args.output;

        info!("Building patch, this may take a while");
        let patch_builder = PatchBuilderFactory::get_builder(patch_info);
        let builder_args  = PatchBuilderFactory::parse_args(patch_info, workdir, args)?;
        patch_builder.build_patch(builder_args)?;

        info!("Building patch package");
        let rpm_builder = RpmBuilder::new(pkg_build_root.to_owned());
        rpm_builder.copy_all_files_to_source(patch_output_dir)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;
        rpm_builder.generate_spec_file(patch_info)?;
        rpm_builder.build_binary_package(pkg_output_dir)?;

        Ok(())
    }

    fn build_patch(&mut self) -> std::io::Result<()> {
        info!("==============================");
        info!("{}", CLI_DESCRIPTION);
        info!("==============================\n");
        let (mut src_pkg_info, dbg_pkg_info) = self.extract_packages()?;
        self.complete_build_args(&mut src_pkg_info)?;

        self.check_build_args(&src_pkg_info, &dbg_pkg_info)?;

        let patch_info = self.collect_patch_info(&src_pkg_info)?;
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

        self.build_patch()?;

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
                    eprintln!("{}", e)
                },
                true => {
                    error!("{}", e);
                    eprintln!("For more information, please check '{}'",
                        cli.workdir.log_file_path().display()
                    );
                },
            }
        }
    }
}
