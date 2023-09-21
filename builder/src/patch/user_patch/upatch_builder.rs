use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use uuid::Uuid;
use which::which;

use syscare_abi::{PackageInfo, PatchEntity, PatchInfo};
use syscare_common::util::{
    digest,
    ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs},
    fs,
    os_str::OsStringExt,
};

use crate::{
    build_params::BuildParameters,
    patch::{PatchBuilder, PatchBuilderArguments},
    workdir::WorkDir,
    PKG_IMPL,
};

use super::upatch_builder_args::UserPatchBuilderArguments;

pub struct UserPatchBuilder<'a> {
    workdir: &'a WorkDir,
}

impl<'a> UserPatchBuilder<'a> {
    pub fn new(workdir: &'a WorkDir) -> Self {
        Self { workdir }
    }

    fn detect_compilers(&self) -> Vec<PathBuf> {
        const COMPILER_NAMES: [&str; 4] = ["cc", "gcc", "c++", "g++"];

        // Get compiler path and filter invalid one
        let compiler_set = COMPILER_NAMES
            .iter()
            .filter_map(|compiler_name| which(compiler_name).ok())
            .collect::<HashSet<_>>();

        compiler_set.into_iter().collect()
    }

    fn create_topdir_macro<P: AsRef<Path>>(&self, buildroot: P) -> OsString {
        OsString::from("--define \"_topdir")
            .append(buildroot.as_ref())
            .concat("\"")
    }

    fn create_build_macros(&self, jobs: usize) -> OsString {
        OsString::new()
            .append("--define \"_smp_build_ncpus")
            .append(jobs.to_string())
            .concat("\"")
            .append("--define \"__spec_install_post %{nil}\"")
            .append("--define \"__find_provides %{nil}\"")
            .append("--define \"__find_requires %{nil}\"")
            .append("--define \"_use_internal_dependency_generator 0\"")
    }

    fn build_cmd_envs(&self) -> ExternCommandEnvs {
        ExternCommandEnvs::new()
            .env("OMP_PROC_BIND", "false")
            .env("QA_RPATHS", "0x0011")
    }

    fn build_cmd_args(&self, uargs: &UserPatchBuilderArguments) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--work_dir")
            .arg(&uargs.work_dir)
            .arg("--source_dir")
            .arg(&uargs.source_dir)
            .arg("--elf_dir")
            .arg(&uargs.elf_dir)
            .arg("--build_source_cmd")
            .arg(&uargs.build_source_cmd)
            .arg("--build_patch_cmd")
            .arg(&uargs.build_patch_cmd)
            .arg("--output_dir")
            .arg(&uargs.output_dir);

        for compiler in &uargs.compiler_list {
            cmd_args = cmd_args.arg("--compiler").arg(compiler)
        }

        for (elf, debuginfo) in &uargs.debug_relations {
            cmd_args = cmd_args
                .arg("--elf_path")
                .arg(elf)
                .arg("--debuginfo")
                .arg(debuginfo)
        }

        if uargs.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip_compiler_check");
        }
        if uargs.verbose {
            cmd_args = cmd_args.arg("--verbose");
        }
        cmd_args = cmd_args.arg("--patch");
        cmd_args = cmd_args.args(uargs.patch_list.iter().map(|patch| &patch.path));

        cmd_args
    }

    fn parse_patch_info<P, Q>(
        build_params: &BuildParameters,
        target_pkg: PackageInfo,
        pkg_files: &[P],
        patch_uuid: String,
        patch_entities: &[Q],
    ) -> Result<PatchInfo>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let pkg_files = pkg_files.iter().map(|f| f.as_ref()).collect::<Vec<_>>();

        let patch_entity_map = patch_entities
            .iter()
            .map(|patch_file| (fs::file_name(patch_file), patch_file.as_ref()))
            .collect::<HashMap<_, _>>();

        let mut patch_entities = Vec::new();
        for elf_relation in &build_params.elf_relations {
            let elf_file = elf_relation.elf.as_path();
            if !pkg_files.contains(&elf_file) {
                continue;
            }
            if let Some(patch_file) = patch_entity_map.get(&fs::file_name(elf_file)) {
                let patch_entity = PatchEntity {
                    uuid: Uuid::new_v4().to_string(),
                    patch_name: fs::file_name(patch_file),
                    patch_target: elf_file.to_owned(),
                    checksum: digest::file(patch_file)
                        .context("Failed to calulate patch file digest")?,
                };
                patch_entities.push(patch_entity);
            }
        }

        let patch_info = PatchInfo {
            uuid: patch_uuid,
            name: build_params.patch.name.to_owned(),
            kind: build_params.patch.kind,
            version: build_params.patch.version.to_owned(),
            release: build_params.patch.release.to_owned(),
            arch: build_params.patch.arch.to_owned(),
            target: target_pkg,
            entities: patch_entities,
            description: build_params.patch.description.to_owned(),
            patches: build_params.patch.patches.to_owned(),
        };

        Ok(patch_info)
    }
}

impl PatchBuilder for UserPatchBuilder<'_> {
    fn parse_builder_args(&self, build_params: &BuildParameters) -> Result<PatchBuilderArguments> {
        const RPMBUILD_CMD: &str = "rpmbuild";
        const RPMBUILD_PERP_FLAGS: &str = "-bp";
        const RPMBUILD_FLAGS: &str = "-bb --noprep --nocheck --nodebuginfo --noclean";

        let pkg_buildroot = &build_params.build_root;
        let patch_build_dir = self.workdir.patch.build.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();

        let compilers = self.detect_compilers();

        let topdir_macro = self.create_topdir_macro(build_params.build_root.as_ref());
        let build_macros = self.create_build_macros(build_params.jobs);

        let build_prep_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(RPMBUILD_PERP_FLAGS)
            .append(&build_params.spec_file);

        let build_original_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(&build_params.spec_file);

        let build_patched_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(&build_params.spec_file);

        let elf_relations = build_params
            .elf_relations
            .iter()
            .map(|relation| {
                (
                    PathBuf::from(OsString::from("*").concat(&relation.elf)),
                    relation.debuginfo.to_owned(),
                )
            })
            .collect::<Vec<_>>();

        let builder_args = UserPatchBuilderArguments {
            work_dir: patch_build_dir.to_path_buf(),
            source_dir: build_params.source_dir.to_owned(),
            elf_dir: pkg_buildroot.buildroot.to_path_buf(),
            debug_relations: elf_relations,
            build_source_cmd: build_original_cmd.append("&&").append(build_prep_cmd),
            build_patch_cmd: build_patched_cmd,
            compiler_list: compilers,
            output_dir: patch_output_dir.to_path_buf(),
            skip_compiler_check: build_params.skip_compiler_check,
            verbose: build_params.verbose,
            patch_list: build_params.patch.patches.to_owned(),
        };

        Ok(PatchBuilderArguments::UserPatch(builder_args))
    }

    fn build_patch(&self, args: &PatchBuilderArguments) -> Result<()> {
        const UPATCH_BUILD: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-build");

        match args {
            PatchBuilderArguments::UserPatch(uargs) => UPATCH_BUILD
                .execve(self.build_cmd_args(uargs), self.build_cmd_envs())?
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
            PatchBuilderArguments::UserPatch(uargs) => {
                let patch_entities =
                    fs::list_files(&uargs.output_dir, fs::TraverseOptions { recursive: false })
                        .context("Failed to find generated patch file")?;

                let output_pkgs = fs::list_files(
                    &build_params.build_root.rpms,
                    fs::TraverseOptions { recursive: true },
                )
                .context("Failed to find generated package file")?;

                let mut patch_infos = Vec::new();
                for pkg_file in output_pkgs {
                    let mut target_pkg = PKG_IMPL
                        .parse_package_info(&pkg_file)
                        .context("Failed to parse package info")?;
                    target_pkg.release = build_params.patch.target.release.to_owned();

                    let pkg_files = PKG_IMPL
                        .query_package_files(&pkg_file)
                        .context("Failed to query package files")?;

                    let patch_uuid = Uuid::new_v4().to_string();
                    let patch_info = Self::parse_patch_info(
                        build_params,
                        target_pkg,
                        &pkg_files,
                        patch_uuid,
                        &patch_entities,
                    )
                    .context("Failed to parse patch info")?;

                    // If patch entity is empty, it means there's no change applied to the package
                    if !patch_info.entities.is_empty() {
                        patch_infos.push(patch_info);
                    }
                }

                Ok(patch_infos)
            }
            _ => unreachable!(),
        }
    }
}
