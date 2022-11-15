use crate::package::{PackageInfo, PackageType};
use crate::package::{RpmSpecGenerator, RpmPatchHelper, RpmHelper, RpmBuildRoot, RpmBuilder};
use crate::patch::{PatchType, PatchInfo, PatchBuilderFactory, PatchBuilderOptions};
use crate::patch::{UserPatchHelper, KernelPatchHelper};

use crate::statics::*;
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
            CliPath::File(file_path) => fs::check_file(file_path)?,
            CliPath::Directory(dir_path) => fs::check_dir(dir_path)?,
        }
        if let Some(debug_info_path) = &args.debug_info {
            match debug_info_path {
                CliPath::File(file_path) => fs::check_file(file_path)?,
                CliPath::Directory(dir_path) => fs::check_dir(dir_path)?,
            }
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
        let pkg_extract_dir = self.work_dir.get_package_extract_dir();

        let pkg_info = PackageInfo::read_from_package(&pkg_path)?;
        if pkg_info.get_type() != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not source package", pkg_path),
            ));
        }

        let build_root = RpmHelper::extract_package(&pkg_path, pkg_extract_dir)?;

        // Find source directory from extracted package root
        // Typically, the source directory name contains package name
        let source_dir = match pkg_info.get_name().eq(KERNEL_PKG_NAME) {
            true  => KernelPatchHelper::find_source_directory(build_root.get_build_path())?,
            false => UserPatchHelper::find_source_directory(build_root.get_build_path(), pkg_info.get_name())?
        };

        // Collect addtional patches from patched source package
        let arg_patch_list = &mut args.patches;
        let mut total_patch_list = Vec::with_capacity(arg_patch_list.len());
        let syscare_patch_dir = format!("{}/{}", build_root.get_source_path(), PKG_DIR_NAME_PATCH);
        if let Ok(patch_list) = fs::list_all_files(syscare_patch_dir, false) {
            total_patch_list.append(
                &mut patch_list.into_iter().map(fs::stringtify_path).collect::<Vec<_>>()
            );
        }
        total_patch_list.append(arg_patch_list);

        // Source package would provide belowing info
        args.source = CliPath::Directory(source_dir);
        args.target_name.get_or_insert(pkg_info.get_name().to_owned());
        args.target_version.get_or_insert(pkg_info.get_version().to_owned());
        args.target_release.get_or_insert(pkg_info.get_release().to_owned());
        args.target_license.get_or_insert(pkg_info.get_license().to_owned());
        args.patches = total_patch_list;

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
            println!("Building kernel, this may take a while");

            KernelPatchHelper::write_kernel_config(
                args.kconfig.as_ref().unwrap(), // kconfig must not be None
                &source_dir
            )?;

            let kernel_file = KernelPatchHelper::build_kernel(&source_dir)?;
            args.debug_info = Some(CliPath::File(kernel_file));
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

    fn build_patched_source_package(&self, build_root: RpmBuildRoot, patch_info: &PatchInfo) -> std::io::Result<()> {
        let args = &self.cli_args;

        println!("Building patched source package");
        let patch_list = RpmPatchHelper::modify_patch_list(patch_info.get_file_list());
        let spec_file_path = build_root.find_spec_file()?;
        RpmPatchHelper::modify_spec_file_by_patches(&spec_file_path, &patch_list)?;

        let rpm_builder = RpmBuilder::from(build_root);
        rpm_builder.copy_patch_file_to_source(&patch_list)?;
        rpm_builder.build_source_package(&spec_file_path, patch_info.get_patch_name(), &args.output_dir)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let args = &self.cli_args;
        let patch_build_root = self.work_dir.get_patch_build_root();
        let patch_output_dir = self.work_dir.get_patch_output_dir();
        let patch_info_path = format!("{}/{}", patch_output_dir, PATCH_INFO_FILE_NAME);
        let package_build_root = self.work_dir.get_package_build_root();

        println!("Building patch, this may take a while");
        PatchBuilderFactory::get_patch_builder(
            patch_info.get_patch_type(), patch_build_root
        ).build_patch(
            PatchBuilderOptions::new(&patch_info, args, patch_output_dir)?
        )?;
        std::fs::write(patch_info_path, format!("{}\n", patch_info))?;

        println!("Building patch package");
        let spec_file_path = RpmSpecGenerator::generate_from_patch_info(
            &patch_info,
            patch_output_dir,
            package_build_root,
        )?;

        RpmBuilder::new(package_build_root).build_binary_package(&spec_file_path, &args.output_dir)?;

        Ok(())
    }

    pub fn run(&mut self) {
        self.check_arguments().expect("Check arguments failed");

        let mut source_package_root = None;
        if self.cli_args.source.is_file() {
            source_package_root = Some(
                self.extract_source_package()
                    .expect("Extract source package failed")
            );
        }

        self.check_build_requirements().expect("Check build requirements failed");

        let patch_info = self.collect_patch_info()
            .expect("Collect patch info failed");

        match patch_info.get_patch_type() {
            PatchType::KernelPatch => {
                self.complete_kernel_patch_requirements()
                    .expect("Complete kernel patch requirements failed");
            },
            _ => {}
        }

        if let Some(build_root) = source_package_root {
            self.build_patched_source_package(build_root, &patch_info)
                .expect("Build patched source package failed");
        }

        self.build_patch_package(&patch_info)
            .expect("Build patch package failed");

        println!("Done");
    }
}
