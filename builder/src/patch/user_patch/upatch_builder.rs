use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{debug, trace};
use uuid::Uuid;
use which::which;

use syscare_abi::{PackageInfo, PatchEntity, PatchFile, PatchInfo, PatchType};
use syscare_common::util::{
    digest,
    ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs},
    fs,
    os_str::OsStringExt,
};

use crate::{build_params::BuildParameters, package::ElfRelation, patch::PatchBuilder, PKG_IMPL};

lazy_static! {
    static ref UPATCH_BUILD: ExternCommand =
        ExternCommand::new("/usr/libexec/syscare/upatch-build");
}

struct UBuildParameters {
    work_dir: PathBuf,
    pkg_binary_dir: PathBuf,
    pkg_output_dir: PathBuf,
    patch_build_root: PathBuf,
    patch_source_dir: PathBuf,
    patch_output_dir: PathBuf,
    compiler_list: Vec<PathBuf>,
    elf_relations: Vec<ElfRelation>,
    build_cmd_original: OsString,
    build_cmd_patched: OsString,
    patch_name: String,
    patch_type: PatchType,
    patch_version: String,
    patch_release: u32,
    patch_arch: String,
    patch_target: PackageInfo,
    patch_description: String,
    patch_files: Vec<PatchFile>,
    skip_compiler_check: bool,
    verbose: bool,
}

pub struct UserPatchBuilder;

impl UserPatchBuilder {
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
}

impl UserPatchBuilder {
    fn build_prepare(&self, build_params: &BuildParameters) -> Result<UBuildParameters> {
        const RPMBUILD_CMD: &str = "rpmbuild";
        const RPMBUILD_PERP_FLAGS: &str = "-bp";
        const RPMBUILD_FLAGS: &str = "-bb --noprep --nocheck --nodebuginfo --noclean";

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

        let topdir_macro = self.create_topdir_macro(pkg_build_root.as_os_str());
        let build_macros = self.create_build_macros(build_params.jobs);

        let build_cmd_prep = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(RPMBUILD_PERP_FLAGS)
            .append(patch_spec);

        let build_cmd_original = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(patch_spec)
            .append("&&")
            .append(build_cmd_prep);

        let build_cmd_patched = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(patch_spec);

        debug!("- Detecting compilers");
        let compiler_list = self.detect_compilers();
        for compiler in &compiler_list {
            trace!("{}", compiler.display())
        }

        debug!("- Parsing elf relations");
        let elf_relations = PKG_IMPL
            .parse_elf_relations(patch_target, debuginfo_pkg_root)
            .context("Failed to parse elf relation")?;
        for elf_relation in &elf_relations {
            trace!("{}", elf_relation);
        }

        let ubuild_params = UBuildParameters {
            work_dir: build_params.work_dir.to_owned(),
            pkg_binary_dir,
            pkg_output_dir,
            patch_build_root,
            patch_source_dir,
            patch_output_dir,
            compiler_list,
            elf_relations,
            build_cmd_original,
            build_cmd_patched,
            patch_name: build_params.patch_name.to_owned(),
            patch_type: build_params.patch_type.to_owned(),
            patch_version: build_params.patch_version.to_owned(),
            patch_release: build_params.patch_release.to_owned(),
            patch_arch: build_params.patch_arch.to_owned(),
            patch_target: build_params.build_entry.target_pkg.to_owned(),
            patch_description: build_params.patch_description.to_owned(),
            patch_files: build_params.patch_files.to_owned(),
            skip_compiler_check: build_params.skip_compiler_check,
            verbose: build_params.verbose,
        };

        Ok(ubuild_params)
    }

    fn parse_ubuild_cmd_args(&self, ubuild_params: &UBuildParameters) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--work-dir")
            .arg(&ubuild_params.work_dir)
            .arg("--build-root")
            .arg(&ubuild_params.patch_build_root)
            .arg("--source-dir")
            .arg(&ubuild_params.patch_source_dir)
            .arg("--elf-dir")
            .arg(&ubuild_params.pkg_binary_dir)
            .arg("--build-source-cmd")
            .arg(&ubuild_params.build_cmd_original)
            .arg("--build-patch-cmd")
            .arg(&ubuild_params.build_cmd_patched)
            .arg("--output-dir")
            .arg(&ubuild_params.patch_output_dir);

        for compiler in &ubuild_params.compiler_list {
            cmd_args = cmd_args.arg("--compiler").arg(compiler)
        }

        for relation in &ubuild_params.elf_relations {
            cmd_args = cmd_args
                .arg("--elf-path")
                .arg(OsString::from("*").concat(&relation.elf))
                .arg("--debuginfo")
                .arg(&relation.debuginfo)
        }

        if ubuild_params.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip-compiler-check");
        }
        if ubuild_params.verbose {
            cmd_args = cmd_args.arg("--verbose");
        }
        cmd_args = cmd_args.arg("--patch");
        cmd_args = cmd_args.args(ubuild_params.patch_files.iter().map(|patch| &patch.path));

        cmd_args
    }

    fn parse_ubuild_cmd_envs(&self) -> ExternCommandEnvs {
        ExternCommandEnvs::new()
            .env("OMP_PROC_BIND", "false")
            .env("QA_RPATHS", "0x0011")
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
        for elf_relation in &ubuild_params.elf_relations {
            let elf_file = &elf_relation.elf;
            let elf_name = fs::file_name(elf_file);
            if !pkg_file_list.contains(elf_file) {
                continue;
            }

            if let Some(patch_file) = patch_entity_map.get(&elf_name) {
                let entity_uuid = Uuid::new_v4().to_string();
                let entity_name = fs::file_name(patch_file);
                let entity_target = elf_file.to_owned();
                let entity_checksum = digest::file(patch_file).with_context(|| {
                    format!(
                        "Failed to calulate patch \"{}\" checksum",
                        patch_file.display()
                    )
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
            uuid: Uuid::new_v4().to_string(),
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
        UPATCH_BUILD
            .execve(
                self.parse_ubuild_cmd_args(ubuild_params),
                self.parse_ubuild_cmd_envs(),
            )?
            .check_exit_code()
    }

    fn generate_patch_info(&self, ubuild_params: &UBuildParameters) -> Result<Vec<PatchInfo>> {
        debug!("- Finding patch binaries");
        let patch_binary_files = fs::list_files(
            &ubuild_params.patch_output_dir,
            fs::TraverseOptions { recursive: false },
        )
        .context("Failed to find generated patch file")?;

        debug!("- Finding output packages");
        let output_pkgs = fs::list_files(
            &ubuild_params.pkg_output_dir,
            fs::TraverseOptions { recursive: true },
        )
        .context("Failed to find generated package file")?;

        debug!("- Generating patch metadata");
        let mut patch_info_list = Vec::new();
        for pkg_file in output_pkgs {
            let mut target_pkg = PKG_IMPL.parse_package_info(&pkg_file).with_context(|| {
                format!(
                    "Failed to parse package \"{}\" metadata",
                    pkg_file.display()
                )
            })?;

            // Override target package release
            target_pkg.release = ubuild_params.patch_target.release.to_owned();

            let pkg_file_list = PKG_IMPL.query_package_files(&pkg_file).with_context(|| {
                format!(
                    "Failed to query package \"{}\" file list",
                    pkg_file.display()
                )
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
        debug!("- Preparing to build patch");
        let ubuild_params = self.build_prepare(build_params)?;

        debug!("- Building patch");
        self.invoke_upatch_build(&ubuild_params)?;

        debug!("- Generating patch metadata");
        let patch_info_list = self
            .generate_patch_info(&ubuild_params)
            .context("Failed to generate patch metadata")?;

        Ok(patch_info_list)
    }
}
