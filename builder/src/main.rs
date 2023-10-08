use std::path::{Path, PathBuf};
use std::{ops::Deref, process::exit, sync::Arc};

use anyhow::{anyhow, bail, ensure, Context, Result};
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter};

use parking_lot::Mutex;
use syscare_abi::{PackageInfo, PackageType, PatchFile, PatchInfo, PatchType};
use syscare_common::{os, util::fs};

mod args;
mod build_params;
mod logger;
mod package;
mod patch;
mod util;
mod workdir;

use args::Arguments;
use build_params::{BuildEntry, BuildParameters};
use logger::Logger;
use package::{PackageBuildRoot, PackageBuilderFactory, PackageFormat, PackageImpl};
use patch::{PatchBuilderFactory, PatchHelper, PatchMetadata, PATCH_FILE_EXT};
use workdir::WorkDir;

use crate::package::{PackageSpecBuilderFactory, PackageSpecWriterFactory};

const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const KERNEL_PKG_NAME: &str = "kernel";

lazy_static! {
    static ref PKG_IMPL: Arc<PackageImpl> = Arc::new(PackageImpl::new(PackageFormat::RpmPackage));
}

pub struct SyscareBuilder {
    args: Arguments,
    workdir: WorkDir,
}

impl SyscareBuilder {
    fn collect_package_info(&self) -> Result<Vec<PackageInfo>> {
        let mut pkg_list = Vec::new();

        for pkg_path in self.args.source.clone() {
            let mut pkg_info = PKG_IMPL.parse_package_info(&pkg_path)?;
            info!("{}", pkg_info);

            if pkg_info.kind != PackageType::SourcePackage {
                bail!("File \"{}\" is not a source package", pkg_path.display());
            }
            // Source package's arch is meaningless, override it
            pkg_info.arch = self.args.patch_arch.clone();

            pkg_list.push(pkg_info);
        }

        for pkg_path in self.args.debuginfo.clone() {
            let pkg_info = PKG_IMPL.parse_package_info(&pkg_path)?;
            info!("{}", pkg_info);

            if pkg_info.kind != PackageType::BinaryPackage {
                bail!("File \"{}\" is not a debuginfo package", pkg_path.display());
            }
            if !pkg_list
                .iter()
                .any(|src_pkg| src_pkg.is_source_of(&pkg_info))
            {
                bail!(
                    "File \"{}\" cannot match to any source package",
                    pkg_path.display()
                );
            }
        }

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

    fn apply_patch_metadata(
        &mut self,
        metadata_root_dir: &Path,
        patch_target: &mut PackageInfo,
        patch_files: &mut Vec<PatchFile>,
    ) -> Result<()> {
        let metadata_dir = PatchMetadata::metadata_dir(metadata_root_dir);
        let metadata_file = PatchMetadata::metadata_file(metadata_root_dir);

        match PatchMetadata::read_from_file(metadata_file) {
            Ok(saved_patch_info) => {
                let patch_version = &self.args.patch_version;
                let patch_release = &mut self.args.patch_release;

                // Override target package
                *patch_target = saved_patch_info.target;

                // Override patch release
                if patch_version == &saved_patch_info.version {
                    *patch_release = u32::max(*patch_release, saved_patch_info.release + 1);
                }

                // Overide patch list
                let mut new_patches = PatchHelper::collect_patch_files(
                    fs::list_files_by_ext(
                        metadata_dir,
                        PATCH_FILE_EXT,
                        fs::TraverseOptions { recursive: false },
                    )
                    .context("Failed to find patch files")?,
                )
                .context("Failed to collect patch file from metadata directory")?;

                if new_patches.is_empty() {
                    bail!("Cannot find any patch file from metadata");
                }
                new_patches.append(patch_files);

                *patch_files = new_patches;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("Failed to read metadata"),
        }

        Ok(())
    }

    fn prepare_to_build(&mut self) -> Result<BuildParameters> {
        let pkg_root = &self.workdir.package;

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
        let patch_target = &mut build_entry.target_pkg;

        debug!("- Extracting patch metadata");
        let metadata_root_dir = &pkg_build_root.sources;
        if PatchMetadata::decompress_tar_pkg(metadata_root_dir).is_ok() {
            debug!("- Applying patch metadata");
            self.apply_patch_metadata(metadata_root_dir, patch_target, &mut patch_files)
                .context("Failed to apply patch metadata")?;
        };

        debug!("- Generating build parameters");
        let build_params = BuildParameters {
            workdir: self.workdir.to_owned(),
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

    fn check_build_params(&self, build_params: &BuildParameters) -> Result<()> {
        let source_pkg = &build_params.build_entry.target_pkg;
        let patch_arch = build_params.patch_arch.as_str();
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

    fn build_patch_package(&self, patch_info: &PatchInfo) -> Result<()> {
        let pkg_build_root = &self.workdir.package.build_root;
        let pkg_source_dir = &pkg_build_root.sources;
        let pkg_spec_dir = &pkg_build_root.specs;
        let patch_output_dir = &self.workdir.patch.output;

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
        let pkg_build_root = &build_params.pkg_build_root;
        let pkg_source_dir = &pkg_build_root.sources;
        let spec_file = &build_params.build_entry.build_spec;

        debug!("- Creating patch metadata directory");
        let metadata_dir = PatchMetadata::metadata_dir(pkg_source_dir);
        if !metadata_dir.exists() {
            fs::create_dir(&metadata_dir)?;
        }

        debug!("- Copying patch file(s)");
        for patch in &build_params.patch_files {
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
        let patch_info = PatchInfo {
            uuid: String::default(),
            name: build_params.patch_name.to_owned(),
            version: build_params.patch_version.to_owned(),
            release: build_params.patch_release.to_owned(),
            arch: build_params.patch_arch.to_owned(),
            kind: build_params.patch_type,
            target: build_params.build_entry.target_pkg.to_owned(),
            entities: Vec::default(),
            description: build_params.patch_description.to_owned(),
            patches: build_params.patch_files.to_owned(),
        };
        PatchMetadata::write_to_file(&patch_info, metadata_file)
            .context("Failed to write patch metadata")?;
        PatchMetadata::compress_tar_pkg(pkg_source_dir).context("Failed to compress metadata")?;

        debug!("- Building package");
        PackageBuilderFactory::get_builder(PKG_IMPL.format(), pkg_build_root).build_source_package(
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
        info!("Preparing to build patch");
        let build_params = self.prepare_to_build()?;

        info!("Checking build parameters");
        self.check_build_params(&build_params)?;

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

        for patch_info in patch_info_list {
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
