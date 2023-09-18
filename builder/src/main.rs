use std::path::PathBuf;
use std::{ops::Deref, process::exit, sync::Arc};

use anyhow::{anyhow, bail, Context, Result};
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter};

use parking_lot::Mutex;
use syscare_abi::{PackageType, PatchInfo, PatchType};
use syscare_common::{os, util::fs};

mod args;
mod build_params;
mod logger;
mod package;
mod patch;
mod util;
mod workdir;

use args::Arguments;
use build_params::BuildParameters;
use logger::Logger;
use package::{PackageBuilderFactory, PackageFormat, PackageImpl};
use patch::{PatchBuilderFactory, PatchHelper, PatchMetadata, PATCH_FILE_EXT};
use workdir::WorkDir;

use crate::package::{PackageSpecBuilderFactory, PackageSpecWriterFactory};
use crate::patch::PatchBuilderArguments;

const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

lazy_static! {
    static ref PKG_IMPL: Arc<PackageImpl> = Arc::new(PackageImpl::new(PackageFormat::RpmPackage));
}

pub struct SyscareBuilder {
    args: Arguments,
    workdir: WorkDir,
}

impl SyscareBuilder {
    fn collect_build_params(&self) -> Result<BuildParameters> {
        let args = &self.args;
        let pkg_root = &self.workdir.package;
        let source_pkg_root = &pkg_root.source;
        let debuginfo_pkg_root = &pkg_root.debuginfo;

        debug!("- Collecting package info");
        let source_pkg = PKG_IMPL.parse_package_info(&self.args.source)?;
        if source_pkg.kind != PackageType::SourcePackage {
            bail!(
                "File \"{}\" is not a source package",
                &self.args.source.display()
            );
        }
        info!("{}", source_pkg);

        let mut debuginfo_pkgs = Vec::with_capacity(self.args.debuginfo.len());
        for pkg_path in &self.args.debuginfo {
            let debuginfo_pkg = PKG_IMPL.parse_package_info(pkg_path)?;
            if debuginfo_pkg.kind != PackageType::BinaryPackage {
                bail!("File \"{}\" is not a debuginfo package", pkg_path.display());
            }
            info!("{}", debuginfo_pkg);
            debuginfo_pkgs.push(debuginfo_pkg);
        }

        debug!("- Collecting patch files");
        let patch_files = PatchHelper::collect_patch_files(&self.args.patches)?;
        let mut patch_info = PatchInfo {
            uuid: String::default(),
            name: args.patch_name.to_owned(),
            version: args.patch_version.to_owned(),
            release: args.patch_release.to_owned(),
            arch: args.patch_arch.to_owned(),
            kind: match source_pkg.name == "kernel" {
                true => PatchType::KernelPatch,
                false => PatchType::UserPatch,
            },
            target: source_pkg,
            entities: Vec::default(),
            description: args.patch_description.to_owned(),
            patches: patch_files,
        };

        debug!("- Extracting source package");
        PKG_IMPL
            .extract_package(&args.source, source_pkg_root)
            .context("Failed to extrace source package")?;

        debug!("- Finding build root");
        let pkg_buildroot = PKG_IMPL
            .find_build_root(source_pkg_root)
            .context("Failed to find package build root")?;

        debug!("- Finding spec file");
        let spec_file = PKG_IMPL
            .find_spec_file(&pkg_buildroot.specs)
            .context("Cannot find package spec file")?;

        debug!("- Extracting patch metadata");
        let pkg_source_dir = &pkg_buildroot.sources;
        if PatchMetadata::decompress_tar_pkg(pkg_source_dir).is_ok() {
            debug!("- Applying patch metadata");
            PatchHelper::apply_patch_metadata(&mut patch_info, pkg_source_dir)
                .context("Failed to apply patch metadata")?;
        };

        debug!("- Preparing source code");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), &pkg_buildroot)
            .build_prepare(&spec_file)?;

        debug!("- Finding source directory");
        let source_dir = PKG_IMPL
            .find_source_directory(&pkg_buildroot.build, &patch_info.target.name)
            .context("Cannot find package source directory")?;

        debug!("- Extracting debuginfo package");
        for debuginfo_pkg in &self.args.debuginfo {
            PKG_IMPL
                .extract_package(debuginfo_pkg, debuginfo_pkg_root)
                .context("Failed to extract debuginfo package")?;
        }

        debug!("- Parsing elf relations");
        let elf_relations = PKG_IMPL
            .parse_elf_relations(&patch_info.target, debuginfo_pkg_root)
            .context("Failed to parse elf relation")?;

        debug!("- Generating build parameters");
        // Source package arch is meaningless, override it
        patch_info.arch = debuginfo_pkgs
            .get(0)
            .context("Debuginfo package is empty")?
            .arch
            .clone();

        let build_params = BuildParameters {
            patch: patch_info,
            build_root: pkg_buildroot,
            source_dir,
            spec_file,
            debuginfo_pkgs,
            elf_relations,
            jobs: args.jobs,
            skip_compiler_check: args.skip_compiler_check,
            skip_cleanup: args.skip_cleanup,
            verbose: args.verbose,
        };

        info!("{}", build_params);
        Ok(build_params)
    }

    fn check_build_params(&self, build_params: &BuildParameters) -> Result<()> {
        let source_pkg = &build_params.patch.target;
        for debuginfo_pkg in &build_params.debuginfo_pkgs {
            if !source_pkg.is_source_of(debuginfo_pkg) {
                bail!(
                    "Package \"{}\" is not source package of \"{}\"",
                    source_pkg.source_pkg,
                    debuginfo_pkg.source_pkg
                );
            }
        }

        let patch_arch = build_params.patch.arch.as_str();
        if patch_arch != os::cpu::arch() {
            bail!("Patch arch \"{}\" is unsupported", patch_arch);
        }
        let target_arch = source_pkg.arch.as_str();
        if patch_arch != target_arch {
            bail!(
                "Patch arch \"{}\" does not match target arch \"{}\"",
                patch_arch,
                target_arch
            );
        }
        if build_params.skip_compiler_check {
            warn!("Warning: Skipped compiler version check");
        }
        if build_params.skip_cleanup {
            warn!("Warning: Skipped cleanup");
        }

        Ok(())
    }

    fn build_patch(&self, build_params: &BuildParameters) -> Result<Vec<PatchInfo>> {
        debug!("- Preparing build requirements");
        let patch_type = build_params.patch.kind;
        let patch_builder = PatchBuilderFactory::get_builder(patch_type, &self.workdir);
        let builder_args: PatchBuilderArguments = patch_builder
            .parse_builder_args(build_params)
            .context("Failed to parse build arguments")?;

        debug!("- Building patch");
        patch_builder
            .build_patch(&builder_args)
            .with_context(|| format!("{}Builder: Failed to build patch", patch_type))?;

        debug!("- Generating patch metadata");
        let patch_infos = patch_builder
            .generate_patch_info(build_params, &builder_args)
            .context("Failed to generate patch meatadata")?;

        Ok(patch_infos)
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> Result<()> {
        let workdir = &self.workdir;
        let pkg_build_root = &workdir.package.patch;
        let pkg_source_dir = &pkg_build_root.sources;
        let pkg_spec_dir = &pkg_build_root.specs;
        let patch_output_dir = &workdir.patch.output;

        debug!("- Writing patch metadata");
        let metadata_file = pkg_source_dir.join(PatchMetadata::metadata_file_name());
        PatchMetadata::write_to_file(patch_info, metadata_file)
            .context("Failed to write patch metadata")?;

        debug!("- Copying patch outputs");
        fs::copy_dir_contents(patch_output_dir, pkg_source_dir)
            .context("Failed to copy patch outputs")?;

        debug!("- Generating spec file");
        let new_spec_file = PackageSpecBuilderFactory::get_builder(PKG_IMPL.format())
            .build(patch_info, pkg_source_dir, pkg_spec_dir)
            .context("Failed to generate spec file")?;

        debug!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root)
            .build_binary_package(&new_spec_file, &self.args.output)
    }

    fn build_source_package(&self, build_params: &BuildParameters) -> Result<()> {
        debug!("- Preparing build requirements");
        let pkg_buildroot = &build_params.build_root;
        let pkg_source_dir = &pkg_buildroot.sources;
        let spec_file = &build_params.spec_file;

        debug!("- Creating patch metadata directory");
        let metadata_dir = PatchMetadata::metadata_dir(pkg_source_dir);
        if !metadata_dir.exists() {
            fs::create_dir(&metadata_dir)?;
        }

        debug!("- Copying patch files");
        for patch in &build_params.patch.patches {
            let src_path = &patch.path;
            let dst_path = metadata_dir.join(&patch.name);
            if src_path != &dst_path {
                fs::copy(src_path, dst_path).context("Failed to copy patch files")?;
            }
        }

        debug!("- Modifying package spec file");
        let metadata_pkg = PatchMetadata::metadata_pkg(pkg_source_dir);
        if !metadata_pkg.exists() {
            // Lacking of metadata means that the package is not patched
            // Thus, we should add a 'Source' tag into spec file
            let file_list = vec![metadata_pkg];
            PackageSpecWriterFactory::get_writer(PKG_IMPL.format())
                .add_source_files(spec_file, file_list)
                .context("Failed to modify spec file")?;
        }

        debug!("- Writing patch metadata");
        let metadata_file = PatchMetadata::metadata_file(pkg_source_dir);
        PatchMetadata::write_to_file(&build_params.patch, metadata_file)
            .context("Failed to write patch metadata")?;
        PatchMetadata::compress_tar_pkg(pkg_source_dir).context("Failed to compress metadata")?;

        debug!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_buildroot).build_source_package(
            build_params,
            spec_file,
            &self.args.output,
        )
    }

    fn clean_up(&mut self) {
        self.workdir.remove().ok();
    }
}

impl SyscareBuilder {
    fn new() -> Result<Self> {
        let args = Arguments::new()?;
        Self::check_input_args(&args)?;

        os::umask::set_umask(CLI_UMASK);
        let workdir = WorkDir::new(
            args.workdir
                .join(format!("syscare-build.{}", os::process::id())),
        )?;

        Logger::initialize(
            workdir.deref(),
            LevelFilter::Trace,
            match &args.verbose {
                false => LevelFilter::Info,
                true => LevelFilter::Debug,
            },
        )?;

        Ok(SyscareBuilder { args, workdir })
    }

    fn check_input_args(args: &Arguments) -> Result<()> {
        let pkg_file_ext = PKG_IMPL.extension();

        let source_pkg = &args.source;
        if !source_pkg.is_file() || fs::file_ext(source_pkg) != pkg_file_ext {
            bail!("Path \"{}\" is not a rpm package", source_pkg.display());
        }

        for debuginfo_pkg in &args.debuginfo {
            if !debuginfo_pkg.is_file() || fs::file_ext(debuginfo_pkg) != pkg_file_ext {
                bail!("Path \"{}\" is not a rpm package", debuginfo_pkg.display());
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

        Ok(())
    }

    fn build_main(mut self, log_file: Arc<Mutex<PathBuf>>) -> Result<()> {
        *log_file.lock() = self.workdir.log_file.clone();

        info!("==============================");
        info!("{}", CLI_ABOUT);
        info!("==============================");
        info!("Collecting build parameters");
        let build_params = self.collect_build_params()?;

        info!("Checking build parameters");
        self.check_build_params(&build_params)?;

        info!("Building patch, this may take a while");
        let patch_infos = self.build_patch(&build_params)?;

        info!("Building patch package(s)");
        for patch_info in patch_infos {
            info!("{}", patch_info);
            self.build_patch_package(&patch_info)?;
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

        let build_thread =
            std::thread::spawn(move || -> Result<()> { Self::new()?.build_main(log_file) });

        match build_thread.join() {
            Ok(build_result) => build_result,
            Err(_) => Err(anyhow!("Failed to join build thread")),
        }
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
