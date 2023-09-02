use std::ffi::OsString;

use anyhow::{Context, Result};
use log::debug;
use syscare_abi::PatchInfo;
use syscare_common::util::ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs};
use syscare_common::util::fs;

use crate::args::Arguments;
use crate::patch::{PatchBuilder, PatchBuilderArguments, PatchHelper};
use crate::workdir::WorkDir;
use crate::PKG_IMPL;

use super::kpatch_builder_args::KernelPatchBuilderArguments;
use super::kpatch_helper::KernelPatchHelper;
use super::kpatch_helper::{KPATCH_PATCH_PREFIX, KPATCH_PATCH_SUFFIX, VMLINUX_FILE_NAME};

pub struct KernelPatchBuilder<'a> {
    workdir: &'a WorkDir,
}

impl<'a> KernelPatchBuilder<'a> {
    pub fn new(workdir: &'a WorkDir) -> Self {
        Self { workdir }
    }
}

impl KernelPatchBuilder<'_> {
    fn parse_cmd_args(&self, args: &KernelPatchBuilderArguments) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--name")
            .arg(&args.patch_name)
            .arg("--sourcedir")
            .arg(&args.source_dir)
            .arg("--config")
            .arg(&args.config)
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
        cmd_args = cmd_args.args(args.patch_list.iter().map(|patch| &patch.path));

        cmd_args
    }

    fn parse_cmd_envs(&self, args: &KernelPatchBuilderArguments) -> ExternCommandEnvs {
        ExternCommandEnvs::new()
            .env("CACHEDIR", &args.build_root)
            .env("NO_PROFILING_CALLS", &args.build_root)
            .env("DISABLE_AFTER_LOAD", &args.build_root)
            .env("KEEP_JUMP_LABEL", &args.build_root)
    }
}

impl PatchBuilder for KernelPatchBuilder<'_> {
    fn parse_builder_args(
        &self,
        patch_info: &PatchInfo,
        args: &Arguments,
    ) -> Result<PatchBuilderArguments> {
        let patch_build_root = self.workdir.patch.build.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();
        let source_pkg_dir = self.workdir.package.source.as_path();
        let debuginfo_pkg_dir = self.workdir.package.debuginfo.as_path();

        debug!("Finding package build root from...");
        let pkg_build_root = PKG_IMPL
            .find_buildroot(source_pkg_dir)
            .context("Cannot find package build root")?;
        let source_pkg_build_dir = pkg_build_root.build.as_path();

        debug!("Finding kernel source directory...");
        let kernel_source_dir = PKG_IMPL
            .find_source_directory(source_pkg_build_dir, "linux-")
            .context("Cannot find kernel source directory")?;

        debug!("Generating kernel default config...");
        KernelPatchHelper::generate_defconfig(&kernel_source_dir)
            .context("Failed to generate default config")?;

        debug!("Finding kernel config...");
        let kernel_config_file = KernelPatchHelper::find_kernel_config(&kernel_source_dir)
            .context("Cannot find kernel config")?;

        debug!("Finding vmlinux...");
        let vmlinux_file =
            KernelPatchHelper::find_vmlinux(&debuginfo_pkg_dir).context("Cannot find vmlinux")?;

        let kernel_patch_name = format!("{}-{}", KPATCH_PATCH_PREFIX, patch_info.uuid); // Use uuid to avoid patch name collision
        let builder_args = KernelPatchBuilderArguments {
            build_root: patch_build_root.to_owned(),
            patch_name: kernel_patch_name,
            source_dir: kernel_source_dir,
            config: kernel_config_file,
            vmlinux: vmlinux_file,
            jobs: args.jobs,
            output_dir: patch_output_dir.to_owned(),
            skip_compiler_check: args.skip_compiler_check,
            patch_list: patch_info.patches.to_owned(),
        };

        Ok(PatchBuilderArguments::KernelPatch(builder_args))
    }

    fn build_patch(&self, args: &PatchBuilderArguments) -> Result<()> {
        const KPATCH_BUILD: ExternCommand = ExternCommand::new("kpatch-build");

        match args {
            PatchBuilderArguments::KernelPatch(kargs) => KPATCH_BUILD
                .execve(self.parse_cmd_args(kargs), self.parse_cmd_envs(kargs))?
                .check_exit_code()?,
            _ => unreachable!(),
        }

        Ok(())
    }

    fn write_patch_info(
        &self,
        patch_info: &mut PatchInfo,
        args: &PatchBuilderArguments,
    ) -> Result<()> {
        match args {
            PatchBuilderArguments::KernelPatch(kargs) => {
                /*
                 * Kernel patch does not use target_elf for patch operation,
                 * so we just add it for display purpose.
                 */
                let output_dir = kargs.output_dir.as_path();
                let patch_name = format!(
                    "{}-{}.{}",
                    KPATCH_PATCH_PREFIX, patch_info.uuid, KPATCH_PATCH_SUFFIX
                );

                if let Ok(patch_file) = fs::find_file(
                    output_dir,
                    patch_name,
                    fs::FindOptions {
                        fuzz: false,
                        recursive: false,
                    },
                ) {
                    patch_info.entities.push(PatchHelper::parse_patch_entity(
                        patch_file,
                        OsString::from(VMLINUX_FILE_NAME),
                    )?);
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
