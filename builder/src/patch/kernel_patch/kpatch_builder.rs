use anyhow::{bail, Context, Result};
use log::debug;
use uuid::Uuid;

use syscare_abi::{PatchEntity, PatchInfo};
use syscare_common::util::{
    digest,
    ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs},
};

use crate::{
    build_params::BuildParameters,
    patch::kernel_patch::kpatch_helper::VMLINUX_FILE_NAME,
    patch::{PatchBuilder, PatchBuilderArguments},
    workdir::WorkDir,
    PKG_IMPL,
};

use super::{
    kpatch_builder_args::KernelPatchBuilderArguments,
    kpatch_helper::KernelPatchHelper,
    kpatch_helper::{KPATCH_PATCH_PREFIX, KPATCH_PATCH_SUFFIX},
};

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
            .env("NO_PROFILING_CALLS", "1")
            .env("DISABLE_AFTER_LOAD", "1")
            .env("KEEP_JUMP_LABEL", "1")
    }
}

impl PatchBuilder for KernelPatchBuilder<'_> {
    fn parse_builder_args(&self, build_params: &BuildParameters) -> Result<PatchBuilderArguments> {
        let patch_build_root = self.workdir.patch.build.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();
        let debuginfo_pkg_dir = self.workdir.package.debuginfo.as_path();

        let source_pkg = &build_params.patch.target;

        debug!("Finding kernel source directory...");
        let kernel_source_dir = PKG_IMPL
            .find_source_directory(
                &build_params.source_dir,
                &format!(
                    "linux-{}-{}.{}",
                    source_pkg.version, source_pkg.release, source_pkg.arch
                ),
            )
            .context("Cannot find kernel source directory")?;

        debug!("Generating kernel default config...");
        KernelPatchHelper::generate_defconfig(&kernel_source_dir)
            .context("Failed to generate default config")?;

        debug!("Finding kernel config...");
        let kernel_config_file = KernelPatchHelper::find_kernel_config(&kernel_source_dir)
            .context("Cannot find kernel config")?;

        debug!("Finding vmlinux...");
        let vmlinux_file =
            KernelPatchHelper::find_vmlinux(debuginfo_pkg_dir).context("Cannot find vmlinux")?;

        let kernel_patch_uuid = Uuid::new_v4().to_string();
        let kernel_patch_name = format!("{}-{}", KPATCH_PATCH_PREFIX, Uuid::new_v4()); // Use uuid to avoid patch name collision

        let builder_args = KernelPatchBuilderArguments {
            build_root: patch_build_root.to_owned(),
            patch_uuid: kernel_patch_uuid,
            patch_name: kernel_patch_name,
            source_dir: kernel_source_dir.to_owned(),
            config: kernel_config_file,
            vmlinux: vmlinux_file,
            jobs: build_params.jobs,
            output_dir: patch_output_dir.to_owned(),
            debug: build_params.verbose,
            skip_compiler_check: build_params.skip_compiler_check,
            patch_list: build_params.patch.patches.to_owned(),
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

    fn generate_patch_info(
        &self,
        build_params: &BuildParameters,
        args: &PatchBuilderArguments,
    ) -> Result<Vec<PatchInfo>> {
        match args {
            PatchBuilderArguments::KernelPatch(kargs) => {
                /*
                 * Kernel patch does not use target_elf for patch operation,
                 * so we just add it for display purpose.
                 */
                let target_pkg = build_params.patch.target.to_owned();
                let patch_file_name = format!("{}.{}", kargs.patch_name, KPATCH_PATCH_SUFFIX);
                let patch_file = kargs.output_dir.join(&patch_file_name);
                if !patch_file.is_file() {
                    bail!("Failed to find patch file");
                }

                let patch_entity = PatchEntity {
                    uuid: Uuid::new_v4().to_string(),
                    patch_name: patch_file_name.into(),
                    patch_target: VMLINUX_FILE_NAME.into(),
                    checksum: digest::file(patch_file)
                        .context("Failed to calulate patch file digest")?,
                };

                let patch_info = PatchInfo {
                    uuid: kargs.patch_uuid.to_owned(),
                    name: build_params.patch.name.to_owned(),
                    kind: build_params.patch.kind,
                    version: build_params.patch.version.to_owned(),
                    release: build_params.patch.release.to_owned(),
                    arch: build_params.patch.arch.to_owned(),
                    target: target_pkg,
                    entities: vec![patch_entity],
                    description: build_params.patch.description.to_owned(),
                    patches: build_params.patch.patches.to_owned(),
                };

                Ok(vec![patch_info])
            }
            _ => unreachable!(),
        }
    }
}
