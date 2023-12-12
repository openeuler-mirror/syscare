use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::debug;
use uuid::Uuid;

use syscare_abi::{PackageInfo, PatchEntity, PatchFile, PatchInfo, PatchType};
use syscare_common::util::{
    digest,
    ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs},
    os_str::{OsStrExt, OsStringExt},
};

use crate::{
    build_params::BuildParameters, package::PackageBuilderFactory, patch::PatchBuilder, PKG_IMPL,
};

use super::kpatch_helper::{KernelPatchHelper, KPATCH_SUFFIX, VMLINUX_FILE_NAME};

lazy_static! {
    static ref KPATCH_BUILD: ExternCommand = ExternCommand::new("kpatch-build");
}

struct KBuildParameters {
    pkg_build_dir: PathBuf,
    patch_build_root: PathBuf,
    patch_output_dir: PathBuf,
    kernel_source_dir: PathBuf,
    oot_source_dir: Option<PathBuf>,
    config_file: PathBuf,
    vmlinux_file: PathBuf,
    patch_name: String,
    patch_type: PatchType,
    patch_version: String,
    patch_release: u32,
    patch_arch: String,
    patch_target: PackageInfo,
    patch_description: String,
    patch_files: Vec<PatchFile>,
    jobs: usize,
    skip_compiler_check: bool,
}

struct KernelPatchEntity {
    source_dir: PathBuf,
    module_path: Option<PathBuf>,
    patch_entity: PatchEntity,
}

pub struct KernelPatchBuilder;

impl KernelPatchBuilder {
    fn build_prepare(&self, build_params: &BuildParameters) -> Result<KBuildParameters> {
        let (kernel_entry, oot_module_entry) = match &build_params.kernel_build_entry {
            Some(build_entry) => (build_entry, Some(&build_params.build_entry)),
            None => (&build_params.build_entry, None),
        };

        debug!("- Finding kernel directories");
        let pkg_build_dir = build_params.pkg_build_root.build.clone();
        let patch_build_root = build_params.workdir.patch.build.clone();
        let patch_output_dir: PathBuf = build_params.workdir.patch.output.clone();

        let kernel_pkg = &kernel_entry.target_pkg;
        let kernel_source_dir: PathBuf = PKG_IMPL
            .find_source_directory(
                &kernel_entry.build_source,
                &format!(
                    "linux-{}-{}.{}",
                    kernel_pkg.version, kernel_pkg.release, kernel_pkg.arch
                ),
            )
            .context("Cannot find kernel source directory")?;
        let kernel_debug_dir = &build_params.workdir.package.debuginfo;
        let oot_source_dir = oot_module_entry.map(|build_entry| build_entry.build_source.clone());

        debug!("- Generating kernel default config");
        KernelPatchHelper::generate_defconfig(&kernel_source_dir)
            .context("Failed to generate default config")?;

        debug!("- Finding kernel config");
        let config_file = KernelPatchHelper::find_kernel_config(&kernel_source_dir)
            .context("Cannot find kernel config")?;

        debug!("- Finding vmlinux");
        let vmlinux_file = KernelPatchHelper::find_vmlinux(kernel_debug_dir)
            .context("Cannot find kernel vmlinux")?;

        if let Some(build_entry) = oot_module_entry {
            debug!("- Building out-of-tree module");
            PackageBuilderFactory::get_builder(PKG_IMPL.format(), &build_params.pkg_build_root)
                .build_binary_package(&build_entry.build_spec, &patch_build_root)
                .context("Failed to build out-of-tree module")?;
        }

        Ok(KBuildParameters {
            pkg_build_dir,
            patch_build_root,
            patch_output_dir,
            kernel_source_dir,
            oot_source_dir,
            config_file,
            vmlinux_file,
            patch_name: build_params.patch_name.to_owned(),
            patch_type: build_params.patch_type.to_owned(),
            patch_version: build_params.patch_version.to_owned(),
            patch_release: build_params.patch_release.to_owned(),
            patch_arch: build_params.patch_arch.to_owned(),
            patch_target: build_params.build_entry.target_pkg.to_owned(),
            patch_description: build_params.patch_description.to_owned(),
            patch_files: build_params.patch_files.to_owned(),
            jobs: build_params.jobs,
            skip_compiler_check: build_params.skip_compiler_check,
        })
    }

    fn generate_kpatch_entities(
        &self,
        kbuild_params: &KBuildParameters,
    ) -> Result<Vec<KernelPatchEntity>> {
        let mut entity_list = Vec::new();

        match &kbuild_params.oot_source_dir {
            // Kernel patch
            None => {
                let entity_uuid = Uuid::new_v4().to_string();
                let entity_target = VMLINUX_FILE_NAME;
                let entity_name = format!("{}-{}", entity_target, entity_uuid);

                entity_list.push(KernelPatchEntity {
                    source_dir: kbuild_params.kernel_source_dir.to_owned(),
                    module_path: None,
                    patch_entity: PatchEntity {
                        uuid: entity_uuid,
                        patch_name: entity_name.into(),
                        patch_target: entity_target.into(),
                        checksum: String::default(),
                    },
                });
            }
            // Kernel module patch
            Some(oot_source_dir) => {
                let module_list =
                    KernelPatchHelper::find_kernel_modules(&kbuild_params.pkg_build_dir)
                        .context("Failed to find any kernel module")?;
                let module_suffix = format!(".{}", KPATCH_SUFFIX);

                for module_path in module_list {
                    let file_name = module_path
                        .file_name()
                        .context("Cannot get patch file name")?;
                    let module_name = file_name
                        .strip_suffix(&module_suffix)
                        .context("Unexpected patch suffix")?
                        .to_string_lossy()
                        .replace('.', "_")
                        .replace('-', "_");

                    let entity_uuid: String = Uuid::new_v4().to_string();
                    let entitiy_name = format!("{}-{}", module_name, entity_uuid);
                    let entity_target = file_name.to_os_string();

                    entity_list.push(KernelPatchEntity {
                        source_dir: oot_source_dir.to_owned(),
                        module_path: Some(module_path),
                        patch_entity: PatchEntity {
                            uuid: entity_uuid,
                            patch_name: entitiy_name.into(),
                            patch_target: entity_target.into(),
                            checksum: String::default(),
                        },
                    });
                }
            }
        }

        Ok(entity_list)
    }

    fn parse_kbuild_cmd_args(
        &self,
        kbuild_params: &KBuildParameters,
        kbuild_entity: &KernelPatchEntity,
    ) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--name")
            .arg(&kbuild_entity.patch_entity.patch_name)
            .arg("--sourcedir")
            .arg(&kbuild_entity.source_dir)
            .arg("--config")
            .arg(&kbuild_params.config_file)
            .arg("--vmlinux")
            .arg(&kbuild_params.vmlinux_file)
            .arg("--jobs")
            .arg(kbuild_params.jobs.to_string())
            .arg("--output")
            .arg(&kbuild_params.patch_output_dir)
            .arg("--skip-cleanup");

        if let Some(oot_module) = &kbuild_entity.module_path {
            cmd_args = cmd_args.arg("--oot-module").arg(oot_module);
        }
        if kbuild_params.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip-compiler-check");
        }
        cmd_args = cmd_args.args(kbuild_params.patch_files.iter().map(|patch| &patch.path));

        cmd_args
    }

    fn parse_kbuild_cmd_envs(&self, build_root: &Path) -> ExternCommandEnvs {
        ExternCommandEnvs::new()
            .env("CACHEDIR", build_root)
            .env("NO_PROFILING_CALLS", "1")
            .env("DISABLE_AFTER_LOAD", "1")
            .env("KEEP_JUMP_LABEL", "1")
    }

    fn invoke_kpatch_build(
        &self,
        kbuild_params: &KBuildParameters,
        kpatch_entities: Vec<KernelPatchEntity>,
    ) -> Result<Vec<PatchEntity>> {
        let mut patch_entities = Vec::with_capacity(kpatch_entities.len());

        // Build each patch entity
        for mut kbuild_entity in kpatch_entities {
            let args = self.parse_kbuild_cmd_args(kbuild_params, &kbuild_entity);
            let envs = self.parse_kbuild_cmd_envs(&kbuild_params.patch_build_root);
            KPATCH_BUILD.execve(args, envs)?.check_exit_code()?;

            let patch_name = kbuild_entity.patch_entity.patch_name.clone();
            let patch_file_name = patch_name.concat(".").concat(KPATCH_SUFFIX);

            let patch_binary = kbuild_params.patch_output_dir.join(patch_file_name);
            let patch_checksum = digest::file(&patch_binary).with_context(|| {
                format!(
                    "Failed to calulate patch \"{}\" checksum",
                    patch_binary.display()
                )
            })?;
            kbuild_entity.patch_entity.checksum = patch_checksum;

            patch_entities.push(kbuild_entity.patch_entity);
        }

        Ok(patch_entities)
    }

    fn generate_patch_info(
        &self,
        kbuild_params: &KBuildParameters,
        patch_entities: Vec<PatchEntity>,
    ) -> Result<Vec<PatchInfo>> {
        // Generate patch info
        let patch_info = PatchInfo {
            uuid: Uuid::new_v4().to_string(),
            name: kbuild_params.patch_name.to_owned(),
            kind: kbuild_params.patch_type,
            version: kbuild_params.patch_version.to_owned(),
            release: kbuild_params.patch_release.to_owned(),
            arch: kbuild_params.patch_arch.to_owned(),
            target: kbuild_params.patch_target.to_owned(),
            entities: patch_entities,
            description: kbuild_params.patch_description.to_owned(),
            patches: kbuild_params.patch_files.to_owned(),
        };

        Ok(vec![patch_info])
    }
}

impl PatchBuilder for KernelPatchBuilder {
    fn build_patch(&self, build_params: &BuildParameters) -> Result<Vec<PatchInfo>> {
        debug!("- Preparing to build patch");
        let kbuild_params = self.build_prepare(build_params)?;

        debug!("- Generating patch entities");
        let kpatch_entities = self
            .generate_kpatch_entities(&kbuild_params)
            .context("Failed to generate patch entity")?;

        debug!("- Building patch");
        let patch_entities = self.invoke_kpatch_build(&kbuild_params, kpatch_entities)?;

        debug!("- Generating patch metadata");
        let patch_info_list = self
            .generate_patch_info(&kbuild_params, patch_entities)
            .context("Failed to generate patch metadata")?;

        Ok(patch_info_list)
    }
}
