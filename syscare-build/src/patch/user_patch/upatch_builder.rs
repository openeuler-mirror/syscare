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

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};
use log::{debug, info, trace, Level};
use uuid::Uuid;

use syscare_abi::{PackageInfo, PatchEntity, PatchFile, PatchInfo, PatchType};
use syscare_common::{
    args_os, concat_os, fs,
    process::{Command, CommandArgs, CommandEnvs},
    util::digest,
};

use crate::{build_params::BuildParameters, package::PackageImpl, patch::PatchBuilder};

use super::{file_relation::FileRelation, DEBUGINFO_FILE_EXT};

const UPATCH_BUILD_BIN: &str = "upatch-build";
const RPMBUILD_BIN: &str = "rpmbuild";

struct UBuildParameters {
    pkg_binary_dir: PathBuf,
    pkg_output_dir: PathBuf,
    patch_build_root: PathBuf,
    patch_source_dir: PathBuf,
    patch_output_dir: PathBuf,
    compilers: Vec<PathBuf>,
    file_relations: Vec<FileRelation>,
    prepare_cmd: OsString,
    build_cmd: OsString,
    patch_name: String,
    patch_type: PatchType,
    patch_version: String,
    patch_release: u32,
    patch_arch: String,
    patch_target: PackageInfo,
    patch_description: String,
    patch_files: Vec<PatchFile>,
    override_line_macros: bool,
    skip_compiler_check: bool,
    verbose: bool,
}

pub struct UserPatchBuilder {
    pkg_impl: &'static PackageImpl,
}

impl UserPatchBuilder {
    pub fn new(pkg_impl: &'static PackageImpl) -> Self {
        Self { pkg_impl }
    }
}

impl UserPatchBuilder {
    fn detect_compilers() -> Vec<PathBuf> {
        const COMPILERS: [&str; 2] = ["/usr/bin/cc", "/usr/bin/c++"];

        let mut compiler_set = HashSet::new();
        for compiler in COMPILERS {
            if let Ok(compiler_path) = fs::canonicalize(compiler) {
                compiler_set.insert(compiler_path);
            }
        }
        compiler_set.into_iter().collect()
    }

    fn create_topdir_macro<P: AsRef<Path>>(buildroot: P) -> OsString {
        concat_os!("--define '_topdir ", buildroot.as_ref(), "'")
    }

    fn create_build_macros(jobs: usize) -> OsString {
        args_os!(
            format!("--define '_smp_build_ncpus {}'", jobs),
            "--define '__brp_strip %{nil}'",
            "--define '__brp_strip_comment_note %{nil}'",
            "--define '__brp_strip_static_archive %{nil}'",
            "--define '__brp_compress %{nil}'",
            "--define '__check_files %{nil}'",
            "--define '__find_provides %{nil}'",
            "--define '__find_requires %{nil}'",
            "--define '__spec_install_post %{nil}'",
            "--define '_use_internal_dependency_generator 0'",
        )
    }

    fn parse_file_relations(
        package: &PackageInfo,
        debuginfo_root: &Path,
    ) -> Result<Vec<FileRelation>> {
        let debuginfo_files = fs::list_files_by_ext(
            debuginfo_root,
            DEBUGINFO_FILE_EXT,
            fs::TraverseOptions { recursive: true },
        )?;
        ensure!(
            !debuginfo_files.is_empty(),
            "Cannot find any debuginfo file"
        );

        let mut file_relations = Vec::new();
        for debuginfo in &debuginfo_files {
            // Skip file relation error check may cause unknown error
            if let Ok(file_relation) = FileRelation::parse(debuginfo_root, package, debuginfo) {
                file_relations.push(file_relation);
            }
        }
        Ok(file_relations)
    }
}

impl UserPatchBuilder {
    fn build_prepare(&self, build_params: &BuildParameters) -> Result<UBuildParameters> {
        let pkg_build_root = &build_params.pkg_build_root;
        let pkg_binary_dir = pkg_build_root.buildroot.clone();
        let pkg_output_dir = pkg_build_root.rpms.clone();
        let debuginfo_pkg_root = &build_params.build_root.package.debuginfo;

        let build_entry = &build_params.build_entry;
        let patch_build_root = build_params.build_root.patch.build.clone();
        let patch_output_dir = build_params.build_root.patch.output.clone();
        let patch_source_dir = build_entry.build_source.clone();

        let patch_spec = &build_entry.build_spec;
        let patch_target = &build_entry.target_pkg;

        let topdir_macro = Self::create_topdir_macro(pkg_build_root);
        let build_macros = Self::create_build_macros(build_params.jobs);

        let prepare_cmd = args_os!(RPMBUILD_BIN, &topdir_macro, "-bp", patch_spec);
        let build_cmd = args_os!(
            RPMBUILD_BIN,
            &topdir_macro,
            &build_macros,
            "-bb --noprep --nocheck --nodebuginfo --noclean",
            patch_spec
        );

        info!("- Detecting compilers");
        let compilers = Self::detect_compilers();
        for compiler in &compilers {
            debug!("{}", compiler.display())
        }

        info!("- Parsing elf relations");
        let file_relations = Self::parse_file_relations(patch_target, debuginfo_pkg_root)
            .context("Failed to parse elf relation")?;
        trace!("{:#?}", file_relations);

        let ubuild_params = UBuildParameters {
            pkg_binary_dir,
            pkg_output_dir,
            patch_build_root,
            patch_source_dir,
            patch_output_dir,
            compilers,
            file_relations,
            prepare_cmd,
            build_cmd,
            patch_name: build_params.patch_name.to_owned(),
            patch_type: build_params.patch_type.to_owned(),
            patch_version: build_params.patch_version.to_owned(),
            patch_release: build_params.patch_release.to_owned(),
            patch_arch: build_params.patch_arch.to_owned(),
            patch_target: build_params.build_entry.target_pkg.to_owned(),
            patch_description: build_params.patch_description.to_owned(),
            patch_files: build_params.patch_files.to_owned(),
            override_line_macros: build_params.override_line_macros,
            skip_compiler_check: build_params.skip_compiler_check,
            verbose: build_params.verbose,
        };

        Ok(ubuild_params)
    }

    fn parse_ubuild_cmd_args(&self, ubuild_params: &UBuildParameters) -> CommandArgs {
        let mut cmd_args = CommandArgs::new();

        cmd_args
            .arg("--build-root")
            .arg(&ubuild_params.patch_build_root)
            .arg("--source-dir")
            .arg(&ubuild_params.patch_source_dir)
            .arg("--compiler")
            .args(&ubuild_params.compilers)
            .arg("--prepare-cmd")
            .arg(&ubuild_params.prepare_cmd)
            .arg("--build-cmd")
            .arg(&ubuild_params.build_cmd)
            .arg("--binary-dir")
            .arg(&ubuild_params.pkg_binary_dir)
            .arg("--binary")
            .args(
                ubuild_params
                    .file_relations
                    .iter()
                    .map(|relation| &relation.binary),
            )
            .arg("--debuginfo")
            .args(
                ubuild_params
                    .file_relations
                    .iter()
                    .map(|relation| &relation.debuginfo),
            )
            .arg("--patch")
            .args(ubuild_params.patch_files.iter().map(|patch| &patch.path))
            .arg("--output-dir")
            .arg(&ubuild_params.patch_output_dir);

        if ubuild_params.override_line_macros {
            cmd_args.arg("--override-line-macros");
        }
        if ubuild_params.skip_compiler_check {
            cmd_args.arg("--skip-compiler-check");
        }
        if ubuild_params.verbose {
            cmd_args.arg("--verbose");
        }
        cmd_args.arg("--skip-cleanup");

        cmd_args
    }

    fn parse_ubuild_cmd_envs(&self) -> CommandEnvs {
        let mut cmd_envs = CommandEnvs::new();
        cmd_envs.envs([("OMP_PROC_BIND", "false"), ("QA_RPATHS", "0x0011")]);

        cmd_envs
    }

    fn parse_patch_info(
        &self,
        ubuild_params: &UBuildParameters,
        target_pkg: PackageInfo,
        pkg_file_list: &[PathBuf],
        patch_binary_files: &[PathBuf],
    ) -> Result<PatchInfo> {
        let patch_entity_map = patch_binary_files
            .iter()
            .map(|patch_file| (fs::file_name(patch_file), patch_file.as_path()))
            .collect::<HashMap<_, _>>();

        let mut patch_entities = Vec::new();
        for relation in &ubuild_params.file_relations {
            let elf_file = &relation.binary;
            let elf_name = fs::file_name(elf_file);
            if !pkg_file_list.contains(elf_file) {
                continue;
            }

            if let Some(patch_file) = patch_entity_map.get(&elf_name) {
                let entity_uuid = Uuid::new_v4();
                let entity_name = fs::file_name(patch_file);
                let entity_target = elf_file.to_owned();
                let entity_checksum = digest::file(patch_file).with_context(|| {
                    format!("Failed to calulate patch {} checksum", patch_file.display())
                })?;

                let patch_entity = PatchEntity {
                    uuid: entity_uuid,
                    patch_name: entity_name,
                    patch_target: entity_target,
                    checksum: entity_checksum,
                };
                patch_entities.push(patch_entity);
            }
        }

        let patch_info = PatchInfo {
            uuid: Uuid::new_v4(),
            name: ubuild_params.patch_name.to_owned(),
            kind: ubuild_params.patch_type,
            version: ubuild_params.patch_version.to_owned(),
            release: ubuild_params.patch_release.to_owned(),
            arch: ubuild_params.patch_arch.to_owned(),
            target: target_pkg,
            entities: patch_entities,
            description: ubuild_params.patch_description.to_owned(),
            patches: ubuild_params.patch_files.to_owned(),
        };

        Ok(patch_info)
    }

    fn invoke_upatch_build(&self, ubuild_params: &UBuildParameters) -> Result<()> {
        Command::new(UPATCH_BUILD_BIN)
            .args(self.parse_ubuild_cmd_args(ubuild_params))
            .envs(self.parse_ubuild_cmd_envs())
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }

    fn generate_patch_info(&self, ubuild_params: &UBuildParameters) -> Result<Vec<PatchInfo>> {
        info!("- Finding patch binaries");
        let patch_binary_files = fs::list_files(
            &ubuild_params.patch_output_dir,
            fs::TraverseOptions { recursive: false },
        )
        .context("Failed to find generated patch file")?;

        info!("- Finding output packages");
        let output_pkgs = fs::list_files(
            &ubuild_params.pkg_output_dir,
            fs::TraverseOptions { recursive: true },
        )
        .context("Failed to find generated package file")?;

        info!("- Collecting patch metadata");
        let mut patch_info_list = Vec::new();
        for pkg_file in output_pkgs {
            let mut target_pkg =
                self.pkg_impl
                    .parse_package_info(&pkg_file)
                    .with_context(|| {
                        format!("Failed to parse package {} metadata", pkg_file.display())
                    })?;

            // Override target package release
            target_pkg.release = ubuild_params.patch_target.release.to_owned();

            let pkg_file_list =
                self.pkg_impl
                    .query_package_files(&pkg_file)
                    .with_context(|| {
                        format!("Failed to query package {} file list", pkg_file.display())
                    })?;

            let patch_info = self
                .parse_patch_info(
                    ubuild_params,
                    target_pkg,
                    &pkg_file_list,
                    &patch_binary_files,
                )
                .context("Failed to parse patch info")?;

            // If patch entity is empty, it means there's no change applied to the package
            if !patch_info.entities.is_empty() {
                patch_info_list.push(patch_info);
            }
        }

        Ok(patch_info_list)
    }
}

impl PatchBuilder for UserPatchBuilder {
    fn build_patch(&self, build_params: &BuildParameters) -> Result<Vec<PatchInfo>> {
        info!("- Preparing to build patch");
        let ubuild_params = self.build_prepare(build_params)?;

        info!("- Building patch");
        self.invoke_upatch_build(&ubuild_params)?;

        info!("Generating patch metadata");
        let patch_info_list = self
            .generate_patch_info(&ubuild_params)
            .context("Failed to generate patch metadata")?;

        Ok(patch_info_list)
    }
}
