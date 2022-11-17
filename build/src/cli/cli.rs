use crate::package::{PackageInfo, PackageType};
use crate::package::{RpmSpecHelper, RpmExtractor, RpmBuildRoot, RpmBuilder};

use crate::patch::{PatchInfo, PatchName, PatchType};
use crate::patch::{PatchBuilderFactory, PatchBuilderOptions};
use crate::patch::{PatchHelper, UserPatchHelper, KernelPatchHelper};

use crate::constants::*;
use crate::util::fs;

use super::path::CliPath;
use super::workdir::CliWorkDir;
use super::args::CliArguments;

pub struct PatchBuildCLI {
    work_dir: CliWorkDir,
    cli_args: CliArguments,
}

impl PatchBuildCLI {
    pub fn new() -> Self {
        Self {
            work_dir: CliWorkDir::new(),
            cli_args: CliArguments::new(),
        }
    }

    fn check_arguments(&self) -> std::io::Result<()> {
        let args = &self.cli_args;
        match &args.source {
            CliPath::File(file_path)     => fs::check_file(file_path)?,
            CliPath::Directory(dir_path) => fs::check_dir(dir_path)?,
        }
        if let Some(file_path) = &args.debug_info {
            fs::check_file(file_path)?
        }
        if let Some(file_path) = &args.kconfig {
            fs::check_file(file_path)?;
        }
        for file_path in &args.patches {
            fs::check_file(file_path)?;
        }
        fs::check_dir(&args.output_dir)?;

        Ok(())
    }

    fn extract_source_package(&mut self) -> std::io::Result<RpmBuildRoot> {
        println!("Extracting source package");
        let args = &mut self.cli_args;
        let pkg_path = args.source.to_string();
        let pkg_build_root = self.work_dir.get_package_build_root();

        let mut pkg_info = PackageInfo::read_from_package(&pkg_path)?;
        if pkg_info.get_type() != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not source package", pkg_path),
            ));
        }

        let build_root = RpmExtractor::extract_package(&pkg_path, pkg_build_root)?;
        let rpm_build_dir  = build_root.get_build_path();
        let rpm_source_dir = build_root.get_source_path();

        // Collect patch version info from patched source package
        let patch_version_file = fs::find_file(rpm_source_dir, PKG_PATCH_VERSION_FILE_NAME, false, false);
        if let Ok(file_path) = &patch_version_file {
            let arg_version = args.patch_version.parse::<u32>();
            let pkg_version = fs::read_file_to_string(file_path)?.parse::<u32>();

            if let (Ok(arg_ver), Ok(pkg_ver)) = (arg_version, pkg_version) {
                let max_ver = u32::max(arg_ver, pkg_ver + 1);
                if max_ver > arg_ver {
                    args.patch_version = max_ver.to_string();
                }
            }
        }

        // Collect patch target info from patched source package
        let patch_target_file = fs::find_file(rpm_source_dir, PKG_PATCH_TARGET_FILE_NAME, false, false);
        if let Ok(file_path) = &patch_target_file {
            let patch_target_name = fs::read_file_to_string(file_path)?.parse::<PatchName>()?;

            pkg_info.set_name(patch_target_name.get_name().to_owned());
            pkg_info.set_version(patch_target_name.get_version().to_owned());
            pkg_info.set_release(patch_target_name.get_release().to_owned());
        }

        args.target_name.get_or_insert(pkg_info.get_name().to_owned());
        args.target_version.get_or_insert(pkg_info.get_version().to_owned());
        args.target_release.get_or_insert(pkg_info.get_release().to_owned());
        args.target_license.get_or_insert(pkg_info.get_license().to_owned());

        // Collect patch list from patched source package
        if patch_version_file.is_ok() && patch_target_file.is_ok() {
            let current_patches = &mut args.patches;
            let mut package_patches = PatchHelper::collect_patches(rpm_source_dir)?;

            if !package_patches.is_empty() {
                package_patches.append(current_patches);
                args.patches = package_patches;
            }
        }

        // Find source directory from extracted package root
        // Typically, the source directory name contains package name
        let pkg_source_dir = match pkg_info.is_kernel_package() {
            true  => KernelPatchHelper::find_source_directory(rpm_build_dir)?,
            false => UserPatchHelper::find_source_directory(rpm_build_dir, pkg_info.get_name())?
        };
        args.source = CliPath::Directory(pkg_source_dir);

        Ok(build_root)
    }

    fn collect_patch_info(&self) -> std::io::Result<PatchInfo> {
        println!("Collecting patch info");

        let patch_info = PatchInfo::try_from(&self.cli_args)?;
        println!("=============================");
        println!("{}", patch_info);
        println!("=============================");

        Ok(patch_info)
    }

    fn complete_kernel_patch_requirements(&mut self) -> std::io::Result<()> {
        let args = &mut self.cli_args;
        let source_dir = args.source.to_string();

        if args.kconfig.is_none() {
            match KernelPatchHelper::find_kernel_config(&source_dir) {
                Ok(kconfig_path) => {
                    args.kconfig = Some(kconfig_path);
                },
                Err(_) => {
                    // There is no kernel config file in source directory
                    // Generate a new one from default config
                    println!("Generating kernel config");
                    args.kconfig = Some(KernelPatchHelper::generate_defconfig(&source_dir)?);
                },
            }
        }

        if args.debug_info.is_none() {
            let jobs = args.kjobs;
            println!("Building kernel with {} thread(s), this may take a while", jobs);

            KernelPatchHelper::write_kernel_config(
                args.kconfig.as_ref().unwrap(), // kconfig must not be None
                &source_dir
            )?;

            let kernel_file = KernelPatchHelper::build_kernel(&source_dir, jobs)?;
            args.debug_info = Some(kernel_file);
        }

        Ok(())
    }

    fn check_build_requirements(&self) -> std::io::Result<()> {
        let args = &self.cli_args;

        match args.target_name {
            Some(_) => {
                if args.target_version.is_none() {
                    eprintln!("Warning: Patch target version is not set");
                }
                if args.target_release.is_none() {
                    eprintln!("Warning: Patch target release is not set");
                }
            }
            None => {
                eprintln!("Warning: Patch target name is not set");
                if args.target_version.is_some() {
                    eprintln!("Warning: Ignored patch target version");
                }
                if args.target_release.is_some() {
                    eprintln!("Warning: Ignored patch target release");
                }
            }
        }

        if args.target_license.is_none() {
            eprintln!("Warning: Patch target license is not set");
        }
        if args.skip_compiler_check {
            eprintln!("Warning: Skipped compiler version check");
        }

        Ok(())
    }

    fn build_source_package(&self, build_root: RpmBuildRoot, patch_info: &PatchInfo) -> std::io::Result<()> {
        let spec_file_path  = build_root.find_spec_file()?;
        let cli_output_dir = &self.cli_args.output_dir;

        RpmSpecHelper::modify_spec_file_by_patches(&spec_file_path, patch_info)?;

        println!("Building source package");
        let rpm_builder = RpmBuilder::from(build_root);
        rpm_builder.copy_patch_file_to_source(patch_info)?;
        rpm_builder.write_patch_target_info_to_source(patch_info)?;
        rpm_builder.build_source_package(&spec_file_path, cli_output_dir)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let args               = &self.cli_args;
        let patch_build_root   = self.work_dir.get_patch_build_root();
        let patch_output_dir   = self.work_dir.get_patch_output_dir();
        let package_build_root = self.work_dir.get_package_build_root();
        let cli_output_dir     = &args.output_dir;

        println!("Building patch, this may take a while");
        let build_options = PatchBuilderOptions::new(&patch_info, args, patch_output_dir)?;
        PatchBuilderFactory::get_patch_builder(
            patch_info.get_patch_type(),
            patch_build_root
        ).build_patch(build_options)?;

        println!("Building patch package");
        let rpm_builder = RpmBuilder::new(package_build_root);
        rpm_builder.copy_all_files_to_source(patch_output_dir)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;

        let spec_file_path = rpm_builder.generate_spec_file(patch_info)?;
        rpm_builder.build_binary_package(&spec_file_path, cli_output_dir)?;

        Ok(())
    }

    pub fn run(&mut self) {
        self.check_arguments().expect("Check arguments failed");

        self.work_dir.create(&self.cli_args.work_dir).expect("Create working directory failed");

        let mut source_package_root = None;
        if self.cli_args.source.is_file() {
            source_package_root = Some(
                self.extract_source_package().expect("Extract source package failed")
            );
        }

        self.check_build_requirements().expect("Check build requirements failed");

        let patch_info = self.collect_patch_info().expect("Collect patch info failed");
        match patch_info.get_patch_type() {
            PatchType::KernelPatch => {
                self.complete_kernel_patch_requirements()
                    .expect("Complete kernel patch requirements failed");
            },
            _ => {}
        }

        self.build_patch_package(&patch_info).expect("Build patch package failed");

        if let Some(build_root) = source_package_root {
            self.build_source_package(build_root, &patch_info).expect("Build source package failed");
        }

        self.work_dir.clean_all().expect("Clean working directory failed");
        println!("Done");
    }
}
