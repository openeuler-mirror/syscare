use common::os;
use common::util::fs::TraverseOptions;
use common::util::{fs, serde};

use log::LevelFilter;
use log::{debug, error, info, warn};

use crate::package::{PackageInfo, PackageType, DEBUGINFO_FILE_EXT, PKG_FILE_EXT};
use crate::package::{RpmBuilder, RpmHelper};
use crate::patch::PatchBuilderFactory;
use crate::patch::{PatchInfo, PATCH_FILE_EXT, PATCH_INFO_FILE_NAME, PATCH_INFO_MAGIC};

use super::args::CliArguments;
use super::logger::Logger;
use super::workdir::CliWorkDir;

const CLI_NAME: &str = "syscare build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

pub struct PatchBuildCLI {
    workdir: CliWorkDir,
    args: CliArguments,
}

impl PatchBuildCLI {
    fn canonicalize_input_args(&mut self) -> std::io::Result<()> {
        let args = &mut self.args;

        let source_rpm = &mut args.source;
        if !source_rpm.is_file() || fs::file_ext(&source_rpm) != PKG_FILE_EXT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source should be a rpm package",
            ));
        }
        *source_rpm = fs::canonicalize(&source_rpm)?;

        for debuginfo_rpm in &mut args.debuginfo {
            if !debuginfo_rpm.is_file() || fs::file_ext(&debuginfo_rpm) != PKG_FILE_EXT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Debuginfo should be a rpm package",
                ));
            }
            *debuginfo_rpm = fs::canonicalize(&debuginfo_rpm)?;
        }

        for patch_file in &mut args.patches {
            if !patch_file.is_file() || fs::file_ext(&patch_file) != PATCH_FILE_EXT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Patches should be patch files",
                ));
            }
            *patch_file = fs::canonicalize(&patch_file)?;
        }

        let workdir = &mut args.workdir;
        if !workdir.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Working directory should be a directory",
            ));
        }
        *workdir = fs::canonicalize(&workdir)?;

        let output = &mut args.output;
        if !output.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Output directory should be a directory",
            ));
        }
        *output = fs::canonicalize(&output)?;

        Ok(())
    }

    fn collect_package_info(&self) -> std::io::Result<(PackageInfo, Vec<PackageInfo>)> {
        info!("Collecting package info");
        let src_pkg_info = PackageInfo::new(&self.args.source)?;
        if src_pkg_info.kind != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "File \"{}\" is not a source package",
                    &self.args.source.display()
                ),
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
            if pkg_info.kind != PackageType::BinaryPackage {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("File \"{}\" is not a debuginfo package", pkg_path.display()),
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

    fn collect_patch_info(&self, target_package: &PackageInfo) -> PatchInfo {
        info!("Collecting patch info");

        let patch_info = PatchInfo::new(&self.args, target_package);
        info!("------------------------------");
        info!("Syscare Patch");
        info!("------------------------------");
        patch_info.print_log(log::Level::Info);
        info!("------------------------------");

        patch_info
    }

    fn complete_build_params(
        &mut self,
        src_pkg_info: &mut PackageInfo,
        dbg_pkg_infos: &[PackageInfo],
    ) -> std::io::Result<()> {
        info!("Completing build parameters");

        let mut args = &mut self.args;
        let source_pkg_root = self.workdir.package.source.as_path();

        debug!("- Extracting source package");
        RpmHelper::extract_package(&args.source, source_pkg_root)?;

        debug!("- Finding package source directory");
        let pkg_source_dir = RpmHelper::find_build_root(source_pkg_root)?.sources;

        debug!("- Decompressing patch metadata");
        if RpmHelper::decompress_medatadata(&pkg_source_dir).is_ok() {
            let pkg_metadata_dir = RpmHelper::metadata_dir(&pkg_source_dir);
            let patch_info_file = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);

            debug!("- Reading patch metadata");
            match serde::deserialize_with_magic::<PatchInfo, _, _>(
                patch_info_file,
                PATCH_INFO_MAGIC,
            ) {
                Ok(patch_info) => {
                    debug!("- Applying patch metadata");

                    // Override path release
                    args.patch_release = u32::max(args.patch_release, patch_info.release + 1);

                    // Overide path list
                    let mut new_patches = fs::list_files_by_ext(
                        pkg_metadata_dir,
                        PATCH_FILE_EXT,
                        fs::TraverseOptions { recursive: false },
                    )?;
                    if !new_patches.is_empty() {
                        new_patches.append(&mut args.patches);
                        args.patches = new_patches;
                    }

                    // Override package info
                    *src_pkg_info = patch_info.target;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        };

        debug!("- Writing build parameters");
        // Source package arch is meaningless, override with debuginfo's arch
        src_pkg_info.arch = dbg_pkg_infos
            .get(0)
            .as_ref()
            .expect("Debuginfo package is empty")
            .arch
            .to_owned();

        Ok(())
    }

    fn check_build_params(
        &self,
        patch_info: &PatchInfo,
        dbg_pkg_infos: &[PackageInfo],
    ) -> std::io::Result<()> {
        info!("Checking build parameters");

        let src_pkg_info = &patch_info.target;
        for pkg_info in dbg_pkg_infos {
            if !src_pkg_info.is_source_of(pkg_info) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Package \"{}\" is not source package of \"{}\"",
                        src_pkg_info.source_pkg, pkg_info.source_pkg,
                    ),
                ));
            }
        }

        let patch_arch = patch_info.arch.as_str();
        if patch_arch != os::cpu::arch() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch arch \"{}\" is unsupported", patch_arch),
            ));
        }

        let target_arch = patch_info.arch.as_str();
        if patch_arch != target_arch {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Patch arch \"{}\" does not match target arch \"{}\"",
                    patch_arch, target_arch
                ),
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

        let pkg_root = &self.workdir.package;

        debug!("- Extracting debuginfo package(s)");
        let debug_pkg_root = pkg_root.debug.as_path();
        for debuginfo_pkg in &self.args.debuginfo {
            RpmHelper::extract_package(debuginfo_pkg, debug_pkg_root)?;
        }

        debug!("- Checking debuginfo files");
        let debuginfo_files = fs::list_files_by_ext(
            debug_pkg_root,
            DEBUGINFO_FILE_EXT,
            TraverseOptions { recursive: true },
        )?;
        if debuginfo_files.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cannot find any debuginfo file",
            ));
        }

        debug!("- Preparing to build");
        let pkg_build_root = RpmHelper::find_build_root(pkg_root.source.as_path())?;
        let pkg_spec_file = RpmHelper::find_spec_file(pkg_build_root.specs.as_path())?;
        RpmBuilder::new(pkg_build_root).build_prepare(pkg_spec_file)?;

        Ok(())
    }

    fn build_patch(&self, patch_info: &mut PatchInfo) -> std::io::Result<()> {
        info!("Building patch, this may take a while");

        debug!("- Preparing build requirements");
        let patch_builder = PatchBuilderFactory::get_builder(patch_info.kind, &self.workdir);
        let builder_args = patch_builder.parse_builder_args(patch_info, &self.args)?;
        let patch_info_file = self.workdir.patch.output.join(PATCH_INFO_FILE_NAME);

        debug!("- Building patch");
        patch_builder.build_patch(&builder_args)?;

        debug!("- Generating patch metadata");
        patch_builder.write_patch_info(patch_info, &builder_args)?;

        debug!("- Writing patch metadata");
        serde::serialize_with_magic(patch_info, patch_info_file, PATCH_INFO_MAGIC)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building patch package");

        debug!("- Preparing build requirements");
        let pkg_builder = RpmBuilder::new(self.workdir.package.patch.to_owned());
        let pkg_source_dir = pkg_builder.build_root().sources.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();

        debug!("- Generating spec file");
        let spec_file = pkg_builder.generate_spec_file(patch_info)?;

        debug!("- Copying patch outputs");
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)?;

        debug!("- Building package");
        pkg_builder.build_binary_package(spec_file, &self.args.output)?;

        Ok(())
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        info!("Building source package");

        debug!("- Preparing build requirements");
        let pkg_build_root = RpmHelper::find_build_root(&self.workdir.package.source)?;
        let pkg_source_dir = pkg_build_root.sources.clone();
        let pkg_spec_file = RpmHelper::find_spec_file(&pkg_build_root.specs)?;
        let pkg_builder = RpmBuilder::new(pkg_build_root);

        debug!("- Checking patch metadata");
        let pkg_metadata_dir = RpmHelper::metadata_dir(&pkg_source_dir);
        if !pkg_metadata_dir.exists() {
            fs::create_dir(&pkg_metadata_dir)?;
        }

        debug!("- Copying patch files");
        for patch_file in &patch_info.patches {
            let src_path = patch_file.path.as_path();
            let dst_path = pkg_metadata_dir.join(&patch_file.name);
            if src_path != dst_path {
                fs::copy(src_path, dst_path)?;
            }
        }

        debug!("- Copying patch metadata");
        let patch_output_dir = self.workdir.patch.output.as_path();
        let patch_info_src_path = patch_output_dir.join(PATCH_INFO_FILE_NAME);
        let patch_info_dst_path = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);
        fs::copy(patch_info_src_path, patch_info_dst_path)?;

        debug!("- Compressing patch metadata");
        let has_metadata = RpmHelper::has_metadata(&pkg_source_dir);
        RpmHelper::compress_metadata(&pkg_source_dir)?;

        if !has_metadata {
            // Lacking of patch metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            debug!("- Modifying spec file");
            RpmHelper::add_metadata_to_spec_file(&pkg_spec_file)?;
        }

        debug!("- Building package");
        pkg_builder.build_source_package(patch_info, &pkg_spec_file, &self.args.output)?;

        Ok(())
    }

    fn clean_up(&mut self) -> std::io::Result<()> {
        if self.args.skip_cleanup {
            return Ok(());
        }
        info!("Cleaning up");
        self.workdir.remove()
    }
}

impl PatchBuildCLI {
    fn initialize(&mut self) -> std::io::Result<()> {
        os::umask::set_umask(CLI_UMASK);
        self.canonicalize_input_args()?;
        self.workdir.create(&self.args.workdir)?;

        Logger::initialize(
            &self.workdir,
            match &self.args.verbose {
                false => LevelFilter::Info,
                true => LevelFilter::Debug,
            },
        )?;

        Ok(())
    }

    fn cli_main(&mut self) -> std::io::Result<()> {
        self.initialize()?;

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        let (mut src_pkg_info, dbg_pkg_infos) = self.collect_package_info()?;
        self.complete_build_params(&mut src_pkg_info, &dbg_pkg_infos)?;

        let mut patch_info = self.collect_patch_info(&src_pkg_info);
        self.check_build_params(&patch_info, &dbg_pkg_infos)?;

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
            args: CliArguments::new(),
        };

        if let Err(e) = cli.cli_main() {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {}", e);
                }
                true => {
                    error!("Error: {}", e);
                    eprintln!(
                        "For more information, please check \"{}\"",
                        cli.workdir.log_file.display()
                    );
                }
            }
            return -1;
        }

        0
    }
}
