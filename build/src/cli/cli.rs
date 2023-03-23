use std::ffi::OsStr;

use log::LevelFilter;
use log::{info, warn, error};
use common::os;
use common::util::{fs, serde::serde_versioned};

use crate::package::{PackageInfo, PackageType, PKG_FILE_EXT};
use crate::package::{RpmExtractor, RpmHelper, RpmSpecHelper, RpmBuilder};
use crate::patch::{PatchInfo, PATCH_FILE_EXT, PATCH_INFO_FILE_NAME};
use crate::patch::{PatchHelper, PatchBuilderFactory};

use super::logger::Logger;
use super::args::CliArguments;
use super::workdir::CliWorkDir;

const CLI_NAME:    &str = "syscare build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT:   &str = env!("CARGO_PKG_DESCRIPTION");

pub struct PatchBuildCLI {
    workdir: CliWorkDir,
    args:    CliArguments,
}

impl PatchBuildCLI {
    fn canonicalize_input_args(&mut self) -> std::io::Result<()> {
        let args = &mut self.args;

        if fs::file_ext(&args.source) != PKG_FILE_EXT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Source should be a rpm package"),
            ));
        }

        if fs::file_ext(&args.debuginfo) != PKG_FILE_EXT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo should be a rpm package"),
            ));
        }

        args.source    = fs::canonicalize(&args.source)?;
        args.debuginfo = fs::canonicalize(&args.debuginfo)?;
        args.workdir   = fs::canonicalize(&args.workdir)?;
        args.output    = fs::canonicalize(&args.output)?;

        for patch in &mut args.patches {
            if fs::file_ext(&patch) != PATCH_FILE_EXT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Patches should be patch files"),
                ));
            }
            *patch = fs::canonicalize(&patch)?;
        }

        Ok(())
    }

    fn collect_package_info(&self) -> std::io::Result<(PackageInfo, PackageInfo)> {
        info!("Collecting package info");
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

        info!("------------------------------");
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

    fn collect_patch_info(&self, target_pkg_info: PackageInfo) -> std::io::Result<PatchInfo> {
        info!("Collecting patch info");

        let patch_info = PatchInfo::new(target_pkg_info, &self.args)?;
        info!("------------------------------");
        info!("Syscare Patch");
        info!("------------------------------");
        patch_info.print_log(log::Level::Info);
        info!("------------------------------");

        Ok(patch_info)
    }

    fn extract_source_package(&self) -> std::io::Result<()> {
        info!("Extracting source package");
        RpmExtractor::extract_package(
            &self.args.source,
            &self.workdir.package.source,
        )
    }

    fn extract_debuginfo_package(&self) -> std::io::Result<()> {
        info!("Extracting debuginfo package");
        RpmExtractor::extract_package(
            &self.args.debuginfo,
            &self.workdir.package.debug
        )
    }

    fn complete_build_args(&mut self, src_pkg_info: &mut PackageInfo, dbg_pkg_info: &PackageInfo) -> std::io::Result<()> {
        let mut args = &mut self.args;

        // Find source directory from extracted package root
        let pkg_source_dir = RpmHelper::find_build_root(
            &self.workdir.package.source
        )?.sources;

        // Collect patch info from patched source package
        if let Ok(path) = fs::find_file(&pkg_source_dir, PATCH_INFO_FILE_NAME, fs::FindOptions { fuzz: false, recursive: false }) {
            let old_patch_info = serde_versioned::deserialize::<_, PatchInfo>(path, PatchInfo::version())?;
            // Override path release
            args.patch_release = u32::max(args.patch_release, old_patch_info.release + 1);
            // Overide path list
            let mut new_patches = PatchHelper::collect_patches(&pkg_source_dir)?;
            if !new_patches.is_empty() {
                new_patches.append(&mut args.patches);
                args.patches = new_patches;
            }
            // Override package info
            *src_pkg_info = old_patch_info.target.to_owned();
        }

        /*
         * Package info matching has to be after package extraction,
         * since the package info may be replaced by the one from
         * 'patched' source package.
         */
        let dbg_pkg_name = format!("{}-debuginfo", src_pkg_info.name);
        if (dbg_pkg_info.name    != dbg_pkg_name)         ||
           (dbg_pkg_info.epoch   != src_pkg_info.epoch)   ||
           (dbg_pkg_info.version != src_pkg_info.version) ||
           (dbg_pkg_info.release != src_pkg_info.release) {
                return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Debuginfo package does not match to the source package"),
            ));
        }

        // Override other arguments
        args.target_name.get_or_insert(src_pkg_info.name.to_owned());
        args.target_arch.get_or_insert(dbg_pkg_info.arch.to_owned());
        args.target_epoch.get_or_insert(src_pkg_info.epoch.to_owned());
        args.target_version.get_or_insert(src_pkg_info.version.to_owned());
        args.target_release.get_or_insert(src_pkg_info.release.to_owned());
        args.target_license.get_or_insert(src_pkg_info.license.to_owned());

        Ok(())
    }

    fn check_build_args(&self) -> std::io::Result<()> {
        let args = &self.args;

        let system_arch = os::cpu::arch();
        if OsStr::new(&args.patch_arch) != system_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch arch \"{}\" is unsupported", args.patch_arch),
            ));
        }

        let target_arch = self.args.target_arch.as_deref().unwrap();
        if args.patch_arch != target_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Target arch \"{}\" is not match patch arch \"{}\"", target_arch, args.patch_arch),
            ));
        }

        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }

        Ok(())
    }

    fn build_prepare(&self) -> std::io::Result<()> {
        info!("Pareparing to build patch");

        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let spec_file      = RpmHelper::find_spec_file(&pkg_build_root.specs)?;

        RpmBuilder::new(pkg_build_root).build_prepare(spec_file)
    }

    fn build_patch(&self, patch_info: &mut PatchInfo) -> std::io::Result<()> {
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
        rpm_builder.build_source_package(patch_info, spec_file, &self.args.output)?;

        Ok(())
    }

    fn clean_up(&mut self) -> std::io::Result<()> {
        if self.args.skip_cleanup {
            warn!("Warning: Skipped cleanup");
            return Ok(())
        }
        info!("Cleaning up");
        self.workdir.remove()
    }
}

impl PatchBuildCLI {
    fn initialize(&mut self) -> std::io::Result<()> {
        self.canonicalize_input_args()?;
        self.workdir.create(&self.args.workdir)?;

        Logger::initialize(
            &self.workdir,
            match &self.args.verbose {
                false => LevelFilter::Info,
                true  => LevelFilter::Debug,
            }
        )?;

        Ok(())
    }

    fn cli_main(&mut self) -> std::io::Result<()> {
        self.initialize()?;

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        let (mut src_pkg_info, dbg_pkg_info) = self.collect_package_info()?;
        self.extract_source_package()?;
        self.complete_build_args(&mut src_pkg_info, &dbg_pkg_info)?;

        let mut patch_info = self.collect_patch_info(src_pkg_info)?;
        self.check_build_args()?;
        self.extract_debuginfo_package()?;

        self.build_prepare()?;
        self.build_patch(&mut patch_info)?;
        self.build_patch_package(&patch_info)?;
        self.build_source_package(&patch_info)?;

        self.clean_up()?;

        info!("Done");
        Ok(())
    }
}

impl PatchBuildCLI {
    pub fn name() -> &'static str {
        CLI_NAME
    }

    pub fn version() -> &'static str {
        CLI_VERSION
    }

    pub fn run() -> i32 {
        let mut cli = Self {
            workdir: CliWorkDir::new(),
            args:    CliArguments::new(),
        };

        if let Err(e) = cli.cli_main() {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {}", e);
                },
                true => {
                    error!("Error: {}", e);
                    eprintln!("For more information, please check \"{}\"",
                        cli.workdir.log_file.display()
                    );
                },
            }
            return -1;
        }
        return 0;
    }
}
