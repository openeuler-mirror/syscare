use crate::package::{PackageInfo, PackageType};
use crate::package::{RpmExtractor, RpmHelper, RpmSpecHelper, RpmBuilder};

use crate::patch::{PatchInfo, PatchName};
use crate::patch::{PatchHelper, PatchBuilderFactory};

use crate::constants::*;
use crate::util::fs;

use super::args::CliArguments;
use super::workdir::CliWorkDir;

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

    fn canonicalize_arguments(&mut self) -> std::io::Result<()> {
        let args = &mut self.cli_args;

        fs::check_file(&args.source)?;
        args.source = fs::stringtify(fs::realpath(&args.source)?);

        fs::check_file(&args.debuginfo)?;
        args.debuginfo = fs::stringtify(fs::realpath(&args.debuginfo)?);

        if let Some(pkg_path) = &args.binary {
            fs::check_file(&args.source)?;
            args.binary = Some(fs::stringtify(fs::realpath(pkg_path)?));
        }

        fs::check_dir(&args.work_dir)?;
        args.work_dir = fs::stringtify(fs::realpath(&args.work_dir)?);

        fs::check_dir(&args.output_dir)?;
        args.output_dir = fs::stringtify(fs::realpath(&args.output_dir)?);

        for patch in &mut args.patches {
            fs::check_file(patch.as_str())?;
            *patch = fs::stringtify(fs::realpath(patch.as_str())?);
        }

        Ok(())
    }

    fn extract_source_package(&self) -> std::io::Result<PackageInfo> {
        println!("Extracting source package");

        let pkg_path   = &self.cli_args.source;
        let output_dir = self.work_dir.package_root().source_pkg_dir();

        let pkg_info = RpmExtractor::extract_package(pkg_path, output_dir)?;
        if pkg_info.get_type() != PackageType::SourcePackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not a source package", pkg_path),
            ));
        }

        println!("------------------------------");
        println!("{}", pkg_info);
        println!("------------------------------\n");
        Ok(pkg_info)
    }

    fn extract_debug_package(&self) -> std::io::Result<()> {
        println!("Extracting debuginfo package");

        let pkg_path   = &self.cli_args.debuginfo;
        let output_dir = self.work_dir.package_root().debug_pkg_dir();

        let pkg_info = RpmExtractor::extract_package(pkg_path, output_dir)?;
        if pkg_info.get_type() != PackageType::BinaryPackage {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File '{}' is not a debuginfo package", pkg_path),
            ));
        }

        println!("------------------------------");
        println!("{}", pkg_info);
        println!("------------------------------\n");
        Ok(())
    }

    fn collect_patch_info(&self, pkg_info: &PackageInfo) -> std::io::Result<PatchInfo> {
        println!("Collecting patch info");

        let patch_info = PatchInfo::parse_from(pkg_info, &self.cli_args)?;

        println!("------------------------------");
        println!("{}", patch_info);
        println!("------------------------------\n");
        Ok(patch_info)
    }

    fn complete_build_arguments(&mut self, pkg_info: &PackageInfo) -> std::io::Result<()> {
        let mut args = &mut self.cli_args;

        // If the source package is kernel, append target elf name 'vmlinux' to arguments
        if pkg_info.get_name() == KERNEL_PKG_NAME {
            args.target_elf_name.get_or_insert(KERNEL_FILE_NAME.to_owned());
        }

        // Find source directory from extracted package root
        let source_pkg_dir = self.work_dir.package_root().source_pkg_dir();
        let pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let pkg_source_dir = pkg_build_root.sources_dir();

        // Collect patch version info from patched source package
        let patch_version_file = fs::find_file(pkg_source_dir, PKG_PATCH_VERSION_FILE_NAME, false, false);
        if let Ok(file_path) = &patch_version_file {
            let arg_version = args.version.parse::<u32>();
            let pkg_version = fs::read_file_to_string(file_path)?.parse::<u32>();

            if let (Ok(arg_ver), Ok(pkg_ver)) = (arg_version, pkg_version) {
                let max_ver = u32::max(arg_ver, pkg_ver + 1);
                if max_ver > arg_ver {
                    args.version = max_ver.to_string();
                }
            }
        }

        // Collect patch target info from patched source package
        let patch_target_file = fs::find_file(pkg_source_dir, PKG_PATCH_TARGET_FILE_NAME, false, false);
        match &patch_target_file {
            Ok(file_path) => {
                let patch_target_name = fs::read_file_to_string(file_path)?.parse::<PatchName>()?;
                args.target_name.get_or_insert(patch_target_name.get_name().to_owned());
                args.target_version.get_or_insert(patch_target_name.get_version().to_owned());
                args.target_release.get_or_insert(patch_target_name.get_release().to_owned());
                args.target_license.get_or_insert(pkg_info.get_license().to_owned());
            },
            Err(_) => {
                args.target_name.get_or_insert(pkg_info.get_name().to_owned());
                args.target_version.get_or_insert(pkg_info.get_version().to_owned());
                args.target_release.get_or_insert(pkg_info.get_release().to_owned());
                args.target_license.get_or_insert(pkg_info.get_license().to_owned());
            }
        }

        // Collect patch list from patched source package
        if patch_version_file.is_ok() && patch_target_file.is_ok() {
            let current_patches = &mut args.patches;
            let mut package_patches = PatchHelper::collect_patches(pkg_source_dir)?;

            if !package_patches.is_empty() {
                package_patches.append(current_patches);
                args.patches = package_patches;
            }
        }

        Ok(())
    }

    fn check_build_arguments(&self) -> std::io::Result<()> {
        let args = &self.cli_args;

        if args.target_name.is_none() || args.target_version.is_none() || args.target_release.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch target info is not complete"),
            ));
        }

        if args.target_elf_name.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch target elf name is empty"),
            ));
        }

        if args.target_license.is_none() {
            eprintln!("Warning: Patch target license is not set");
        }

        if args.skip_compiler_check {
            eprintln!("Warning: Skipped compiler version check");
        }

        Ok(())
    }

    fn build_source_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let source_pkg_dir = self.work_dir.package_root().source_pkg_dir();
        let pkg_output_dir = &self.cli_args.output_dir;

        let source_pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let source_pkg_spec_dir  = source_pkg_build_root.specs_dir();

        let spec_file_path = RpmHelper::find_spec_file(source_pkg_spec_dir)?;
        RpmSpecHelper::modify_spec_file_by_patches(&spec_file_path, patch_info)?;

        println!("Building source package");
        let rpm_builder = RpmBuilder::new(source_pkg_build_root);
        rpm_builder.copy_patch_file_to_source(patch_info)?;
        rpm_builder.write_patch_target_info_to_source(patch_info)?;
        rpm_builder.build_source_package(pkg_output_dir)?;

        Ok(())
    }

    fn build_patch_package(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let work_dir = &self.work_dir;
        let cli_args = &self.cli_args;

        let patch_output_dir = self.work_dir.patch_root().output_dir();
        let pkg_build_root   = self.work_dir.package_root().build_root();
        let pkg_output_dir   = &cli_args.output_dir;

        println!("Building patch, this may take a while");
        let patch_builder = PatchBuilderFactory::get_builder(patch_info);
        let builder_args  = PatchBuilderFactory::parse_args(patch_info, work_dir, cli_args)?;
        patch_builder.build_patch(builder_args)?;

        println!("Building patch package");
        let rpm_builder = RpmBuilder::new(pkg_build_root.to_owned());
        rpm_builder.copy_all_files_to_source(patch_output_dir)?;
        rpm_builder.write_patch_info_to_source(patch_info)?;
        rpm_builder.generate_spec_file(patch_info)?;
        rpm_builder.build_binary_package(pkg_output_dir)?;

        Ok(())
    }

    pub fn run(&mut self) {
        self.canonicalize_arguments()
            .expect("Check arguments failed");

        self.work_dir.create(&self.cli_args.work_dir)
            .expect("Create working directory failed");

        let pkg_info = self.extract_source_package()
            .expect("Extract source package failed");

        self.extract_debug_package()
            .expect("Extract source package failed");

        self.complete_build_arguments(&pkg_info)
            .expect("Complete build arguments failed");

        self.check_build_arguments()
            .expect("Check build arguments failed");

        let patch_info = self.collect_patch_info(&pkg_info)
            .expect("Collect patch info failed");

        self.build_patch_package(&patch_info)
            .expect("Build patch package failed");

        self.build_source_package(&patch_info)
            .expect("Build source package failed");

        self.work_dir.remove();
        println!("Done");
    }
}
