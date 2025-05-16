// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{info, Level};
use uuid::Uuid;

use syscare_abi::{PackageInfo, PatchEntity, PatchFile, PatchInfo, PatchType};
use syscare_common::{
    concat_os,
    ffi::OsStrExt,
    fs,
    process::{Command, CommandArgs, CommandEnvs},
    util::digest,
};

use crate::{
    build_params::BuildParameters,
    package::{PackageBuilderFactory, PackageImpl},
    patch::PatchBuilder,
};

use super::kpatch_helper::{KernelPatchHelper, KPATCH_SUFFIX, VMLINUX_FILE_NAME};

const KPATCH_BUILD_BIN: &str = "kpatch-build";
const GENERATED_KCONFIG_NAME: &str = ".config";

struct KBuildParameters {
    pkg_build_dir: PathBuf,
    patch_build_root: PathBuf,
    patch_output_dir: PathBuf,
    kernel_source_dir: PathBuf,
    kmod_source_dir: Option<PathBuf>,
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
    verbose: bool,
}

struct KernelPatchEntity {
    source_dir: PathBuf,
    kmod_path: Option<PathBuf>,
    patch_entity: PatchEntity,
}

pub struct KernelPatchBuilder {
    pkg_impl: &'static PackageImpl,
}

impl KernelPatchBuilder {
    pub fn new(pkg_impl: &'static PackageImpl) -> Self {
        Self { pkg_impl }
    }
}

impl KernelPatchBuilder {
    fn build_prepare(&self, build_params: &BuildParameters) -> Result<KBuildParameters> {
        let (kernel_entry, oot_module_entry) = match &build_params.kernel_build_entry {
            Some(build_entry) => (build_entry, Some(&build_params.build_entry)),
            None => (&build_params.build_entry, None),
        };

        info!("- Finding kernel directories");
        let pkg_build_dir = build_params.pkg_build_root.build.clone();
        let patch_build_root = build_params.build_root.patch.build.clone();
        let patch_output_dir: PathBuf = build_params.build_root.patch.output.clone();

        let kernel_pkg = &kernel_entry.target_pkg;
        let kernel_source_dir: PathBuf = fs::find_dir(
            &kernel_entry.build_source,
            format!("linux-{}", kernel_pkg.version),
            fs::FindOptions {
                fuzz: true,
                recursive: true,
            },
        )
        .context("Cannot find kernel source directory")?;
        let kmod_source_dir = oot_module_entry.map(|entry| entry.build_source.clone());
        let kernel_debug_dir = &build_params.build_root.package.debuginfo;

        /*
         * Kernel config:
         * If it's a valid path, use it directly as an exteral file.
         * Otherwise, we treat it as a kernel config name.
         */
        let config_file = match fs::canonicalize(&build_params.kernel_config).ok() {
            Some(file_path) => {
                info!(
                    "- Using kernel config file '{}'",
                    build_params.kernel_config.to_string_lossy()
                );
                file_path
            }
            None => {
                info!(
                    "- Using kernel config '{}'",
                    build_params.kernel_config.to_string_lossy()
                );
                KernelPatchHelper::generate_config_file(
                    &kernel_source_dir,
                    &build_params.kernel_config,
                )
                .context("Failed to generate kernel config")?;

                kernel_source_dir.join(GENERATED_KCONFIG_NAME)
            }
        };

        info!("- Finding vmlinux");
        let vmlinux_file = KernelPatchHelper::find_vmlinux(kernel_debug_dir)
            .context("Cannot find kernel vmlinux")?;

        if let Some(build_entry) = oot_module_entry {
            info!("- Building out-of-tree module");
            PackageBuilderFactory::get_builder(
                self.pkg_impl.format(),
                &build_params.pkg_build_root,
            )
            .build_binary_package(&build_entry.build_spec, &patch_build_root)
            .context("Failed to build out-of-tree module")?;
        }

        Ok(KBuildParameters {
            pkg_build_dir,
            patch_build_root,
            patch_output_dir,
            kernel_source_dir,
            kmod_source_dir,
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
            verbose: build_params.verbose,
        })
    }

    fn generate_kpatch_entities(
        &self,
        kbuild_params: &KBuildParameters,
    ) -> Result<Vec<KernelPatchEntity>> {
        let mut entity_list = Vec::new();

        match &kbuild_params.kmod_source_dir {
            // Kernel patch
            None => {
                let uuid = Uuid::new_v4();
                let uuid_short = uuid
                    .to_string()
                    .split_once('-')
                    .map(|s| s.0.to_string())
                    .expect("Unexpected kernel patch uuid");

                let patch_entity = PatchEntity {
                    uuid,
                    patch_name: concat_os!(VMLINUX_FILE_NAME, "-", uuid_short),
                    patch_target: VMLINUX_FILE_NAME.into(),
                    checksum: String::new(),
                };
                entity_list.push(KernelPatchEntity {
                    source_dir: kbuild_params.kernel_source_dir.to_owned(),
                    kmod_path: None,
                    patch_entity,
                });
            }
            // Kernel module patch
            Some(kmod_source_dir) => {
                let kmod_list =
                    KernelPatchHelper::find_kernel_modules(&kbuild_params.pkg_build_dir)
                        .context("Failed to find any kernel module")?;
                for kmod_path in kmod_list {
                    let uuid = Uuid::new_v4();
                    let uuid_short = uuid
                        .to_string()
                        .split_once('-')
                        .map(|s| s.0.to_string())
                        .expect("Unexpected kernel module uuid");
                    let file_name = kmod_path
                        .file_name()
                        .expect("Unexpected kernel module file name");
                    let module_name = {
                        let mut kmod_file = kmod_path.clone();
                        kmod_file.set_extension("");

                        kmod_file
                            .file_name()
                            .expect("Unexpected kernel module name")
                            .replace(['.', '-'], "_")
                    };

                    let patch_entity = PatchEntity {
                        uuid,
                        patch_name: concat_os!(module_name, "-", uuid_short),
                        patch_target: file_name.into(),
                        checksum: String::new(),
                    };
                    entity_list.push(KernelPatchEntity {
                        source_dir: kmod_source_dir.to_owned(),
                        kmod_path: Some(kmod_path),
                        patch_entity,
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
    ) -> CommandArgs {
        let mut cmd_args = CommandArgs::new();

        cmd_args
            .arg("--name")
            .arg(&kbuild_entity.patch_entity.patch_name)
            .arg("--sourcedir")
            .arg(&kbuild_entity.source_dir)
            .arg("--config")
            .arg(&kbuild_params.config_file)
            .arg("--vmlinux")
            .arg(&kbuild_params.vmlinux_file);

        if let Some(kmod_path) = &kbuild_entity.kmod_path {
            cmd_args.arg("--oot-module").arg(kmod_path);
            cmd_args
                .arg("--oot-module-src")
                .arg(&kbuild_entity.source_dir);
        }

        cmd_args
            .arg("--jobs")
            .arg(kbuild_params.jobs.to_string())
            .arg("--non-replace")
            .arg("--skip-cleanup");

        if kbuild_params.skip_compiler_check {
            cmd_args.arg("--skip-compiler-check");
        }
        if kbuild_params.verbose {
            cmd_args.arg("--debug");
        }

        cmd_args
            .arg("--output")
            .arg(&kbuild_params.patch_output_dir)
            .args(kbuild_params.patch_files.iter().map(|f| &f.path));

        cmd_args
    }

    fn parse_kbuild_cmd_envs(&self, kbuild_params: &KBuildParameters) -> CommandEnvs {
        let mut cmd_envs = CommandEnvs::new();
        cmd_envs
            .env("CACHEDIR", &kbuild_params.patch_build_root)
            .env("NO_PROFILING_CALLS", "yes")
            .env("DISABLE_AFTER_LOAD", "yes")
            .env("KEEP_JUMP_LABEL", "yes");
        if let Some(kmod_source_dir) = &kbuild_params.kmod_source_dir {
            cmd_envs.env("OOT_MODULE", "yes");
            cmd_envs.env("USERMODBUILDDIR", kmod_source_dir);
        }
        cmd_envs
    }

    fn invoke_kpatch_build(
        &self,
        kbuild_params: &KBuildParameters,
        kpatch_entities: Vec<KernelPatchEntity>,
    ) -> Result<Vec<PatchEntity>> {
        let mut patch_entities = Vec::with_capacity(kpatch_entities.len());

        // Build each patch entity
        for mut kbuild_entity in kpatch_entities {
            Command::new(KPATCH_BUILD_BIN)
                .args(self.parse_kbuild_cmd_args(kbuild_params, &kbuild_entity))
                .envs(self.parse_kbuild_cmd_envs(kbuild_params))
                .stdout(Level::Debug)
                .run_with_output()?
                .exit_ok()?;

            let patch_name = kbuild_entity.patch_entity.patch_name.clone();
            let patch_file_name = concat_os!(patch_name, ".", KPATCH_SUFFIX);

            let patch_binary = kbuild_params.patch_output_dir.join(patch_file_name);
            let patch_checksum = digest::file(&patch_binary).with_context(|| {
                format!(
                    "Failed to calulate patch {} checksum",
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
            uuid: Uuid::new_v4(),
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
        info!("- Preparing to build patch");
        let kbuild_params = self.build_prepare(build_params)?;

        info!("- Generating patch entities");
        let kpatch_entities = self
            .generate_kpatch_entities(&kbuild_params)
            .context("Failed to generate patch entity")?;

        info!("- Building patch");
        let patch_entities = self.invoke_kpatch_build(&kbuild_params, kpatch_entities)?;

        info!("Generating patch metadata");
        let patch_info_list = self
            .generate_patch_info(&kbuild_params, patch_entities)
            .context("Failed to generate patch metadata")?;

        Ok(patch_info_list)
    }
}
