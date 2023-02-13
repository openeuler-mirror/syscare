use crate::constants::*;
use crate::log::debug;

use crate::cli::{CliWorkDir, CliArguments};
use crate::cmd::{ExternCommandArgs, ExternCommandEnvs};
use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchFile};
use crate::patch::{PatchBuilder, PatchBuilderArgumentsParser, PatchBuilderArguments};

use super::kpatch_helper::KernelPatchHelper;
use super::kpatch_builder_args::KernelPatchBuilderArguments;

pub struct KernelPatchBuilder;

impl KernelPatchBuilder {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_cmd_args(&self, args: &KernelPatchBuilderArguments) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--name")
            .arg(&args.patch_name)
            .arg("--sourcedir")
            .arg(&args.source_dir)
            .arg("--config")
            .arg(&args.config_file)
            .arg("--vmlinux")
            .arg(&args.vmlinux)
            .arg("--jobs")
            .arg(args.jobs.to_string())
            .arg("--output")
            .arg(&args.output_dir)
            .arg("--skip-cleanup");

        if args.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip-compiler-check");
        }
        cmd_args = cmd_args.args(args.patch_list.iter().map(PatchFile::get_path));

        cmd_args
    }

    fn parse_cmd_envs(&self, args: &KernelPatchBuilderArguments) -> ExternCommandEnvs {
        ExternCommandEnvs::new()
            .env("CACHEDIR",           &args.build_root)
            .env("NO_PROFILING_CALLS", &args.build_root)
            .env("DISABLE_AFTER_LOAD", &args.build_root)
            .env("KEEP_JUMP_LABEL",    &args.build_root)
    }
}

impl PatchBuilderArgumentsParser for KernelPatchBuilder {
    fn parse_args(patch_info: &PatchInfo, workdir: &CliWorkDir, args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        let patch_build_root = workdir.patch_root().build_root_dir();
        let patch_output_dir = workdir.patch_root().output_dir();

        let source_pkg_dir = workdir.package_root().source_pkg_dir();
        let debug_pkg_dir  = workdir.package_root().debug_pkg_dir();

        let source_pkg_build_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let source_pkg_build_dir  = source_pkg_build_root.build_dir();

        let kernel_source_dir = RpmHelper::find_source_directory(source_pkg_build_dir, patch_info)?;
        debug!("source directory: '{}'", kernel_source_dir.display());

        KernelPatchHelper::generate_defconfig(&kernel_source_dir)?;
        let kernel_config_file = KernelPatchHelper::find_kernel_config(&kernel_source_dir)?;
        debug!("kernel config: '{}'", kernel_config_file.display());

        let debuginfo_file = KernelPatchHelper::find_debuginfo_file(debug_pkg_dir)?;
        debug!("debuginfo file: '{}'", debuginfo_file.display());

        let builder_args = KernelPatchBuilderArguments {
            build_root:          patch_build_root.to_owned(),
            patch_name:          patch_info.get_name().to_owned(),
            source_dir:          kernel_source_dir,
            config_file:         kernel_config_file,
            vmlinux:             debuginfo_file,
            jobs:                args.kjobs,
            output_dir:          patch_output_dir.to_owned(),
            skip_compiler_check: args.skip_compiler_check,
            patch_list:          patch_info.get_patches().to_owned(),
        };

        Ok(PatchBuilderArguments::KernelPatch(builder_args))
    }
}

impl PatchBuilder for KernelPatchBuilder {
    fn build_patch(&self, args: PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::KernelPatch(kargs) => {
                let exit_status = KPATCH_BUILD.execve(
                    self.parse_cmd_args(&kargs),
                    self.parse_cmd_envs(&kargs)
                )?;

                let exit_code = exit_status.exit_code();
                if exit_code != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("Process '{}' exited unsuccessfully, exit_code={}", KPATCH_BUILD, exit_code),
                    ));
                }
                Ok(())
            },
            _ => unreachable!(),
        }
    }
}
