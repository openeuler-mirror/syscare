use std::ffi::OsStr;

use log::LevelFilter;
use log::{debug, info, warn, error};
use common::os;
use common::util::{fs, serde::serde_versioned};
use common::util::fs::TraverseOptions;

use crate::package::{PackageInfo, PackageType, PKG_FILE_EXT, DEBUGINFO_FILE_EXT};
use crate::package::{RpmHelper, RpmBuilder};
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

        for debuginfo in &mut args.debuginfo {
            if fs::file_ext(&debuginfo) != PKG_FILE_EXT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Debuginfo should be a rpm package"),
                ));
            }
            *debuginfo = fs::canonicalize(&debuginfo)?;
        }

        for patch in &mut args.patches {
            if fs::file_ext(&patch) != PATCH_FILE_EXT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Patches should be patch files"),
                ));
            }
            *patch = fs::canonicalize(&patch)?;
        }

        args.source    = fs::canonicalize(&args.source)?;
        args.workdir   = fs::canonicalize(&args.workdir)?;
        args.output    = fs::canonicalize(&args.output)?;

        Ok(())
    }

    fn collect_package_info(&self) -> std::io::Result<(PackageInfo, Vec<PackageInfo>)> {
        info!("Collecting package info");
        let src_pkg_info = PackageInfo::new(&self.args.source)?;
        if src_pkg_info.pkg_type() != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File \"{}\" is not a source package", &self.args.source.display()),
            ));
        }
        info!("------------------------------");
        info!("Source package");
        info!("------------------------------");
        src_pkg_info.print_log(log::Level::Info);
        info!("------------------------------");

        let mut dbg_pkg_infos = Vec::with_capacity(self.args.debuginfo.len());
        for pkg_path in &self.args.debuginfo {
            let pkg_info = PackageInfo::new(pkg_path)?;
            if pkg_info.pkg_type() != PackageType::BinaryPackage {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("File \"{}\" is not a debuginfo package", pkg_path.display())
                ));
            }
            info!("Debuginfo package");
            info!("------------------------------");
            pkg_info.print_log(log::Level::Info);
            info!("------------------------------");
            dbg_pkg_infos.push(pkg_info);
        }

        Ok((src_pkg_info, dbg_pkg_infos))
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

        RpmHelper::extract_package(
            &self.args.source,
            &self.workdir.package.source,
        )
    }

    fn extract_debuginfo_packages(&self) -> std::io::Result<()> {
        info!("Extracting debuginfo package(s)");

        for debuginfo_pkg in &self.args.debuginfo {
            RpmHelper::extract_package(
                debuginfo_pkg,
                &self.workdir.package.debug
            )?;
        }

        Ok(())
    }

    fn complete_build_params(&mut self, src_pkg_info: &mut PackageInfo, dbg_pkg_infos: &[PackageInfo]) -> std::io::Result<()> {
        info!("Completing build parameters");

        let mut args = &mut self.args;

        // Find source directory from extracted package root
        let pkg_source_dir   = RpmHelper::find_build_root(&self.workdir.package.source)?.sources;
        let pkg_metadata_dir = RpmHelper::metadata_dir(&pkg_source_dir);

        // Try to decompress metadata
        RpmHelper::decompress_medatadata(&pkg_source_dir).ok();

        // Collect patch info from metadata directory
        let patch_info_file = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);
        match serde_versioned::deserialize::<_, PatchInfo>(patch_info_file, PatchInfo::version()) {
            Ok(patch_info) => {
                debug!("Found patch metadata, overrides build parameters");
                // Override path release
                args.patch_release = u32::max(args.patch_release, patch_info.release + 1);
                // Overide path list
                let mut new_patches = PatchHelper::collect_patches(pkg_metadata_dir)?;
                if !new_patches.is_empty() {
                    new_patches.append(&mut args.patches);
                    args.patches = new_patches;
                }
                // Override package info
                *src_pkg_info = patch_info.target.to_owned();
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(e),
        }

        for pkg_info in dbg_pkg_infos {
            if !src_pkg_info.is_source_pkg_of(&pkg_info) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Package \"{}\" is not source package of \"{}\"",
                        src_pkg_info.pkg_name(), pkg_info.pkg_name(),
                    ))
                );
            }
        }

        // Source package arch is meaningless, override with debuginfo's arch
        src_pkg_info.arch = dbg_pkg_infos.get(0).as_ref().unwrap().arch.to_owned();

        // Override other arguments
        args.target_name.get_or_insert(src_pkg_info.name.to_owned());
        args.target_arch.get_or_insert(src_pkg_info.arch.to_owned());
        args.target_epoch.get_or_insert(src_pkg_info.epoch.to_owned());
        args.target_version.get_or_insert(src_pkg_info.version.to_owned());
        args.target_release.get_or_insert(src_pkg_info.release.to_owned());
        args.target_license.get_or_insert(src_pkg_info.license.to_owned());

        Ok(())
    }

    fn check_build_params(&self) -> std::io::Result<()> {
        info!("Checking build parameters");

        let args: &CliArguments = &self.args;

        if OsStr::new(&args.patch_arch) != os::cpu::arch() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch arch \"{}\" is unsupported", args.patch_arch),
            ));
        }

        let target_arch = self.args.target_arch.as_deref().unwrap();
        if args.patch_arch != target_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Target arch \"{}\" does not match patch arch \"{}\"", target_arch, args.patch_arch),
            ));
        }

        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }

        if self.args.skip_cleanup {
            warn!("Warning: Skipped cleanup");
        }

        Ok(())
    }

    fn build_prepare(&self) -> std::io::Result<()> {
        info!("Pareparing to build patch");

        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let pkg_spec_file  = RpmHelper::find_spec_file(&pkg_build_root.specs)?;

        let debug_files = fs::list_files_by_ext(
            &self.workdir.package.debug,
            DEBUGINFO_FILE_EXT,
            TraverseOptions { recursive: true }
        )?;
        if debug_files.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cannot find any debuginfo file",
            ));
        }

        RpmBuilder::new(pkg_build_root).build_prepare(pkg_spec_file)
    }

    fn build_patch(&self, patch_info: &mut PatchInfo) -> std::io::Result<()> {
        info!("Building patch, this may take a while");

        let patch_output_dir = self.workdir.patch.output.as_path();
        let patch_info_file  = patch_output_dir.join(PATCH_INFO_FILE_NAME);

        let patch_builder    = PatchBuilderFactory::get_builder(patch_info.kind, &self.workdir);
        let builder_args     = patch_builder.parse_builder_args(patch_info, &self.args)?;

        patch_builder.build_patch(&builder_args)?;
        patch_builder.write_patch_info(patch_info, &builder_args)?;

        // Write patch info into patch package source directory
        debug!("Writing patch info to {:?}", patch_info_file);
        serde_versioned::serialize(patch_info, patch_info_file, PatchInfo::version())?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building patch package");

        let pkg_builder      = RpmBuilder::new(self.workdir.package.patch.to_owned());
        let pkg_source_dir   = pkg_builder.build_root().sources.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();

        // Copy all build outputs to patch package source directory
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)?;

        // Build package
        pkg_builder.build_binary_package(
            pkg_builder.generate_spec_file(patch_info)?,
            &self.args.output
        )?;

        Ok(())
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building source package");

        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let pkg_source_dir = pkg_build_root.sources.clone();
        let pkg_spec_file  = RpmHelper::find_spec_file(&pkg_build_root.specs)?;
        let pkg_builder    = RpmBuilder::new(pkg_build_root);

        // Create metadata directory in source
        let pkg_metadata_dir = RpmHelper::metadata_dir(&pkg_source_dir);
        if !pkg_metadata_dir.exists() {
            fs::create_dir(&pkg_metadata_dir)?;
        }

        // Copy all patch files into metadata directory
        for patch_file in &patch_info.patches {
            let src_path = patch_file.path.as_path();
            let dst_path = pkg_metadata_dir.join(&patch_file.name);
            if src_path != dst_path {
                fs::copy(src_path, dst_path)?;
            }
        }

        // Copy patch info into metadata directory
        let patch_output_dir    = self.workdir.patch.output.as_path();
        let patch_info_src_path = patch_output_dir.join(PATCH_INFO_FILE_NAME);
        let patch_info_dst_path = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);
        fs::copy(patch_info_src_path, patch_info_dst_path)?;

        // Compress all files
        if !RpmHelper::has_metadata(&pkg_source_dir) {
            // Lacking of metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            RpmHelper::add_metadata_to_spec_file(&pkg_spec_file)?;
        }
        RpmHelper::compress_metadata(&pkg_source_dir)?;

        // Build package
        pkg_builder.build_source_package(patch_info, &pkg_spec_file, &self.args.output)?;

        Ok(())
    }

    fn clean_up(&mut self) -> std::io::Result<()> {
        if self.args.skip_cleanup {
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
        let (mut src_pkg_info, dbg_pkg_infos) = self.collect_package_info()?;
        self.extract_source_package()?;
        self.complete_build_params(&mut src_pkg_info, &dbg_pkg_infos)?;
        self.check_build_params()?;

        let mut patch_info = self.collect_patch_info(src_pkg_info)?;
        self.extract_debuginfo_packages()?;
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
