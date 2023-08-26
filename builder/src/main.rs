use std::process::exit;

use anyhow::{bail, Context, Result};
use log::LevelFilter;
use log::{debug, error, info, warn};
use once_cell::sync::OnceCell;

use syscare_abi::{PackageInfo, PackageType};
use syscare_abi::{PatchInfo, PATCH_INFO_MAGIC};
use syscare_common::{
    os,
    util::{fs, serde},
};

mod args;
mod logger;
mod package;
mod patch;
mod workdir;

use args::Arguments;
use logger::Logger;
use package::{
    PackageHelper, RpmBuilder, RpmHelper, RpmSpecGenerator, DEBUGINFO_FILE_EXT, PKG_FILE_EXT,
};
use patch::{PatchBuilderFactory, PatchHelper, PATCH_FILE_EXT, PATCH_INFO_FILE_NAME};
use workdir::WorkDir;

const CLI_NAME: &str = "syscare build";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

pub struct SyscareBuilder {
    args: Arguments,
    workdir: OnceCell<WorkDir>,
}

impl SyscareBuilder {
    fn check_input_args(&self) -> Result<()> {
        let args = &self.args;

        let source_rpm = &args.source;
        if !source_rpm.is_file() || fs::file_ext(source_rpm) != PKG_FILE_EXT {
            bail!("Path \"{}\" is not a rpm package", source_rpm.display());
        }

        for debuginfo_rpm in &args.debuginfo {
            if !debuginfo_rpm.is_file() || fs::file_ext(debuginfo_rpm) != PKG_FILE_EXT {
                bail!("Path \"{}\" is not a rpm package", debuginfo_rpm.display());
            }
        }

        for patch_file in &args.patches {
            if !patch_file.is_file() || fs::file_ext(patch_file) != PATCH_FILE_EXT {
                bail!("Path \"{}\" is not a patch file", patch_file.display());
            }
        }

        let workdir = &args.workdir;
        if !workdir.exists() {
            fs::create_dir_all(workdir)?;
        } else if !workdir.is_dir() {
            bail!("Path \"{}\" is not a directory", workdir.display());
        }

        let output = &args.output;
        if !output.exists() {
            fs::create_dir_all(output)?;
        } else if !output.is_dir() {
            bail!("Path \"{}\" is not a directory", output.display());
        }

        Ok(())
    }

    fn workdir(&self) -> &WorkDir {
        self.workdir.wait()
    }

    fn collect_package_info(&self) -> Result<(PackageInfo, Vec<PackageInfo>)> {
        let src_pkg_info = PackageHelper::parse_pkg_info(&self.args.source)?;
        if src_pkg_info.kind != PackageType::SourcePackage {
            bail!(
                "File \"{}\" is not a source package",
                &self.args.source.display()
            );
        }
        info!("------------------------------");
        info!("Source package");
        info!("------------------------------");
        PackageHelper::print_pkg_info(&src_pkg_info, log::Level::Info);
        info!("------------------------------");

        let mut dbg_pkg_infos = Vec::with_capacity(self.args.debuginfo.len());
        for pkg_path in &self.args.debuginfo {
            let debug_pkg_info = PackageHelper::parse_pkg_info(pkg_path)?;
            if debug_pkg_info.kind != PackageType::BinaryPackage {
                bail!("File \"{}\" is not a debuginfo package", pkg_path.display());
            }

            info!("Debuginfo package");
            info!("------------------------------");
            PackageHelper::print_pkg_info(&debug_pkg_info, log::Level::Info);
            info!("------------------------------");
            dbg_pkg_infos.push(debug_pkg_info);
        }

        Ok((src_pkg_info, dbg_pkg_infos))
    }

    fn collect_patch_info(&self, target_package: &PackageInfo) -> PatchInfo {
        let patch_info = PatchHelper::parse_patch_info(&self.args, target_package);
        info!("------------------------------");
        info!("Syscare Patch");
        info!("------------------------------");
        PatchHelper::print_patch_info(&patch_info, log::Level::Info);
        info!("------------------------------");

        patch_info
    }

    fn complete_build_params(
        &mut self,
        src_pkg_info: &mut PackageInfo,
        dbg_pkg_infos: &[PackageInfo],
    ) -> Result<()> {
        let source_pkg_root = self.workdir().package.source.clone();
        let args = &mut self.args;

        debug!("- Extracting source package");
        RpmHelper::extract_package(&args.source, &source_pkg_root)
            .context("Failed to extrace source package")?;

        debug!("- Finding package build root");
        let rpmbuild_root = RpmHelper::find_rpmbuild_root(&source_pkg_root)
            .context("Failed to find package build root")?;

        debug!("- Decompressing patch metadata");
        let pkg_source_dir = &rpmbuild_root.sources;
        if RpmHelper::decompress_medatadata(pkg_source_dir).is_ok() {
            let pkg_metadata_dir = RpmHelper::metadata_dir(pkg_source_dir);
            let patch_info_file = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);

            debug!("- Reading patch metadata");
            match serde::deserialize_with_magic::<PatchInfo, _, _>(
                patch_info_file,
                PATCH_INFO_MAGIC,
            ) {
                Ok(patch_info) => {
                    debug!("- Applying patch metadata");

                    // Override path release
                    if args.patch_version == patch_info.version {
                        args.patch_release = u32::max(args.patch_release, patch_info.release + 1);
                    }

                    // Overide path list
                    debug!("- Collecting patches from metadata");
                    let mut new_patches = fs::list_files_by_ext(
                        pkg_metadata_dir,
                        PATCH_FILE_EXT,
                        fs::TraverseOptions { recursive: false },
                    )
                    .context("Failed to find patch files")?;

                    if new_patches.is_empty() {
                        bail!("Cannot find any patch file from metadata");
                    }
                    new_patches.append(&mut args.patches);
                    args.patches = new_patches;

                    // Override package info
                    *src_pkg_info = patch_info.target;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e).context("Failed to read patch metadata"),
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
    ) -> Result<()> {
        let src_pkg_info = &patch_info.target;
        for pkg_info in dbg_pkg_infos {
            if !src_pkg_info.is_source_of(pkg_info) {
                bail!(
                    "Package \"{}\" is not source package of \"{}\"",
                    src_pkg_info.source_pkg,
                    pkg_info.source_pkg
                );
            }
        }

        let patch_arch = patch_info.arch.as_str();
        if patch_arch != os::cpu::arch() {
            bail!("Patch arch \"{}\" is unsupported", patch_arch);
        }

        let target_arch = patch_info.arch.as_str();
        if patch_arch != target_arch {
            bail!(
                "Patch arch \"{}\" does not match target arch \"{}\"",
                patch_arch,
                target_arch
            );
        }

        if self.args.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }

        if self.args.skip_cleanup {
            warn!("Warning: Skipped cleanup");
        }

        Ok(())
    }

    fn build_prepare(&self) -> Result<()> {
        let pkg_root = &self.workdir().package;

        debug!("- Extracting debuginfo package(s)");
        let debug_pkg_root = pkg_root.debuginfo.as_path();
        for debuginfo_pkg in &self.args.debuginfo {
            RpmHelper::extract_package(debuginfo_pkg, debug_pkg_root)
                .context("Failed to extract debuginfo package")?;
        }

        debug!("- Checking debuginfo files");
        let debuginfo_files = fs::list_files_by_ext(
            debug_pkg_root,
            DEBUGINFO_FILE_EXT,
            fs::TraverseOptions { recursive: true },
        )
        .context("Failed to find debuginfo files")?;

        if debuginfo_files.is_empty() {
            bail!("Cannot find any debuginfo file");
        }

        debug!("- Preparing to build");
        let rpmbuild_root = RpmHelper::find_rpmbuild_root(&pkg_root.source)
            .context("Cannot find package build root")?;

        let pkg_spec_file = RpmHelper::find_spec_file(&rpmbuild_root.specs)
            .context("Cannot find package spec file")?;

        RpmBuilder::new(rpmbuild_root).build_prepare(pkg_spec_file)
    }

    fn build_patch(&self, patch_info: &mut PatchInfo) -> Result<()> {
        debug!("- Preparing build requirements");
        let workdir = self.workdir();
        let patch_builder = PatchBuilderFactory::get_builder(patch_info.kind, workdir);
        let builder_args = patch_builder
            .parse_builder_args(patch_info, &self.args)
            .context("Failed to parse build arguments")?;

        debug!("- Building patch");
        patch_builder.build_patch(&builder_args)?;

        debug!("- Generating patch metadata");
        patch_builder
            .write_patch_info(patch_info, &builder_args)
            .context("Failed to generate patch meatadata")?;

        debug!("- Writing patch metadata");
        let patch_info_file = workdir.patch.output.join(PATCH_INFO_FILE_NAME);
        serde::serialize_with_magic(patch_info, patch_info_file, PATCH_INFO_MAGIC)
            .context("Failed to write patch metadata")?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> Result<()> {
        debug!("- Preparing build requirements");
        let workdir = self.workdir();
        let rpmbuild_root = &workdir.package.patch;
        let pkg_source_dir = &rpmbuild_root.sources;
        let pkg_spec_dir = &rpmbuild_root.specs;
        let patch_output_dir = &workdir.patch.output;

        debug!("- Generating spec file");
        let new_spec_file =
            RpmSpecGenerator::generate_spec_file(patch_info, pkg_source_dir, pkg_spec_dir)
                .context("Failed to generate spec file")?;

        debug!("- Copying patch outputs");
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)
            .context("Failed to copy patch outputs")?;

        debug!("- Building package");
        RpmBuilder::new(rpmbuild_root.to_owned())
            .build_binary_package(new_spec_file, &self.args.output)
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> Result<()> {
        debug!("- Preparing build requirements");
        let workdir = self.workdir();
        let source_pkg_dir = &workdir.package.source;

        let rpmbuild_root = RpmHelper::find_rpmbuild_root(source_pkg_dir)
            .context("Cannot find package build root")?;

        let pkg_source_dir = &rpmbuild_root.sources;
        let pkg_spec_file = RpmHelper::find_spec_file(&rpmbuild_root.specs)
            .context("Cannot find package spec file")?;

        debug!("- Checking patch metadata");
        let pkg_metadata_dir = RpmHelper::metadata_dir(pkg_source_dir);
        if !pkg_metadata_dir.exists() {
            fs::create_dir(&pkg_metadata_dir)?;
        }

        debug!("- Copying patch files");
        for patch_file in &patch_info.patches {
            let src_path = &patch_file.path;
            let dst_path = pkg_metadata_dir.join(&patch_file.name);
            if src_path != &dst_path {
                fs::copy(src_path, dst_path).context("Failed to copy patch files")?;
            }
        }

        debug!("- Copying patch metadata");
        let patch_output_dir = &workdir.patch.output;
        let patch_info_src_path = patch_output_dir.join(PATCH_INFO_FILE_NAME);
        let patch_info_dst_path = pkg_metadata_dir.join(PATCH_INFO_FILE_NAME);
        fs::copy(patch_info_src_path, patch_info_dst_path)
            .context("Failed to copy patch metadata")?;

        debug!("- Compressing patch metadata");
        let has_metadata = RpmHelper::has_metadata(pkg_source_dir);
        RpmHelper::compress_metadata(pkg_source_dir)
            .context("Failed to compress patch metadata")?;

        if !has_metadata {
            // Lacking of patch metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            debug!("- Modifying spec file");
            RpmHelper::add_metadata_to_spec(&pkg_spec_file)
                .context("Failed to modify spec file")?;
        }

        debug!("- Building package");
        RpmBuilder::new(rpmbuild_root).build_source_package(
            patch_info,
            &pkg_spec_file,
            &self.args.output,
        )
    }

    fn clean_up(&mut self) {
        self.workdir().remove().ok();
    }
}

impl SyscareBuilder {
    fn initialize(&mut self) -> Result<()> {
        os::umask::set_umask(CLI_UMASK);
        self.check_input_args()?;

        let temp_dir = self
            .args
            .workdir
            .join(format!("syscare-build.{}", os::process::id()));
        self.workdir
            .get_or_try_init(|| WorkDir::new(temp_dir))
            .context("Failed to create working directory")?;

        Logger::initialize(
            self.workdir(),
            match &self.args.verbose {
                false => LevelFilter::Info,
                true => LevelFilter::Debug,
            },
        )?;

        Ok(())
    }

    fn start_and_run(&mut self) -> Result<()> {
        self.initialize()?;

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Collecting package info");
        let (mut src_pkg_info, dbg_pkg_infos) = self
            .collect_package_info()
            .context("Failed to collect package info")?;

        info!("Completing build parameters");
        self.complete_build_params(&mut src_pkg_info, &dbg_pkg_infos)
            .context("Failed to complete build parameters")?;

        info!("Collecting patch info");
        let mut patch_info = self.collect_patch_info(&src_pkg_info);

        info!("Checking build parameters");
        self.check_build_params(&patch_info, &dbg_pkg_infos)
            .context("Build parameters check failed")?;

        info!("Pareparing to build patch");
        self.build_prepare().context("Failed to prepare build")?;

        info!("Building patch, this may take a while");
        self.build_patch(&mut patch_info)
            .context("Failed to build patch")?;

        info!("Building patch package");
        self.build_patch_package(&patch_info)
            .context("Failed to build patch package")?;

        info!("Building source package");
        self.build_source_package(&patch_info)
            .context("Failed to build patch source package")?;

        if !self.args.skip_cleanup {
            info!("Cleaning up");
            self.clean_up();
        }

        info!("Done");
        Ok(())
    }
}

impl SyscareBuilder {
    pub fn name() -> &'static str {
        CLI_NAME
    }

    pub fn version() -> &'static str {
        CLI_VERSION
    }
}

fn main() {
    let mut cli = SyscareBuilder {
        args: Arguments::new(),
        workdir: OnceCell::new(),
    };
    let exit_code = match cli.start_and_run() {
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
                        cli.workdir.wait().log_file.display()
                    );
                }
            }
            1
        }
    };
    exit(exit_code);
}
