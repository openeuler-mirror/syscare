use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::debug;
use which::which;

use syscare_abi::PatchInfo;
use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs, ExternCommandEnvs},
    fs,
    os_str::OsStringExt,
};

use crate::args::Arguments;
use crate::package::RpmHelper;
use crate::patch::{PatchBuilder, PatchBuilderArguments, PatchHelper};
use crate::workdir::WorkDir;

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
            .map(OsString::from)
            .into_iter()
            .filter_map(|compiler_name| which(compiler_name).ok())
            .collect::<HashSet<_>>();

        compiler_set.into_iter().collect()
    }

    fn create_topdir_macro<P: AsRef<Path>>(&self, buildroot: P) -> OsString {
        OsString::from("--define \"_topdir")
            .append(buildroot.as_ref())
            .concat("\"")
    }

    fn create_build_macros(&self, args: &Arguments) -> OsString {
        OsString::new()
            .append("--define \"_smp_build_ncpus")
            .append(args.jobs.to_string())
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

    fn build_cmd_args(&self, args: &UserPatchBuilderArguments) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--work-dir")
            .arg(&args.work_dir)
            .arg("--debug-source")
            .arg(&args.debug_source)
            .arg("--elf-dir")
            .arg(&args.elf_dir)
            .arg("--build-source-cmd")
            .arg(&args.build_source_cmd)
            .arg("--build-patch-cmd")
            .arg(&args.build_patch_cmd)
            .arg("--output-dir")
            .arg(&args.output_dir);

        for compiler in &args.compiler_list {
            cmd_args = cmd_args.arg("--compiler").arg(compiler)
        }

        for relation in &args.elf_relations {
            cmd_args = cmd_args
                .arg("--elf-path")
                .arg(OsString::from("*").concat(&relation.elf))
                .arg("--debug-info")
                .arg(&relation.debuginfo)
        }

        if args.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip-compiler-check");
        }
        if args.verbose {
            cmd_args = cmd_args.arg("--verbose");
        }
        cmd_args = cmd_args.args(args.patch_list.iter().map(|patch| &patch.path));

        cmd_args
    }
}

impl PatchBuilder for UserPatchBuilder<'_> {
    fn parse_builder_args(
        &self,
        patch_info: &PatchInfo,
        args: &Arguments,
    ) -> Result<PatchBuilderArguments> {
        const RPMBUILD_CMD: &str = "rpmbuild";
        const RPMBUILD_PERP_FLAGS: &str = "-bp";
        const RPMBUILD_FLAGS: &str = "-bi --noprep --nocheck --nodebuginfo --noclean";

        let source_pkg_dir = self.workdir.package.source.as_path();
        let debuginfo_pkg_dir = self.workdir.package.debuginfo.as_path();

        debug!(
            "Finding package build root from \"{}\"...",
            source_pkg_dir.display()
        );
        let rpmbuild_root = RpmHelper::find_rpmbuild_root(source_pkg_dir).with_context(|| {
            format!(
                "Cannot find package build root from \"{}\"",
                source_pkg_dir.display()
            )
        })?;

        let rpm_spec_dir = rpmbuild_root.specs.as_path();
        let rpm_build_dir = rpmbuild_root.build.as_path();
        let rpm_buildroot_dir = rpmbuild_root.buildroot.as_path();

        debug!(
            "Finding package spec file from \"{}\"...",
            rpm_spec_dir.display()
        );
        let spec_file = RpmHelper::find_spec_file(rpm_spec_dir).with_context(|| {
            format!(
                "Cannot find package spec file from \"{}\"",
                rpm_spec_dir.display()
            )
        })?;

        debug!(
            "Finding package source directory from \"{}\"...",
            rpm_build_dir.display()
        );
        let source_dir =
            RpmHelper::find_build_source(rpm_build_dir, patch_info).with_context(|| {
                format!(
                    "Cannot find package source directory from \"{}\"",
                    rpm_build_dir.display()
                )
            })?;

        debug!(
            "Finding package debuginfos from \"{}\"...",
            debuginfo_pkg_dir.display()
        );
        let debuginfos = RpmHelper::find_debuginfo(debuginfo_pkg_dir).with_context(|| {
            format!(
                "Cannot find package debuginfos from \"{}\"",
                debuginfo_pkg_dir.display()
            )
        })?;

        let target_pkg = &patch_info.target;
        let debug_relations =
            RpmHelper::parse_elf_relations(debuginfos, debuginfo_pkg_dir, target_pkg)
                .context("Cannot parse elf relations")?;

        let patch_build_dir = self.workdir.patch.build.as_path();
        let output_dir = self.workdir.patch.output.as_path();
        let compilers = self.detect_compilers();

        let topdir_macro = self.create_topdir_macro(rpmbuild_root.as_ref());
        let build_macros = self.create_build_macros(args);

        let build_prep_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(RPMBUILD_PERP_FLAGS)
            .append(&spec_file);

        let build_original_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(&spec_file);

        let build_patched_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_FLAGS)
            .append(&spec_file);

        let builder_args = UserPatchBuilderArguments {
            work_dir: patch_build_dir.to_path_buf(),
            debug_source: source_dir,
            elf_dir: rpm_buildroot_dir.to_path_buf(),
            elf_relations: debug_relations,
            build_source_cmd: build_original_cmd.append("&&").append(build_prep_cmd),
            build_patch_cmd: build_patched_cmd,
            compiler_list: compilers,
            output_dir: output_dir.to_path_buf(),
            skip_compiler_check: args.skip_compiler_check,
            verbose: args.verbose,
            patch_list: patch_info.patches.to_owned(),
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

    fn write_patch_info(
        &self,
        patch_info: &mut PatchInfo,
        args: &PatchBuilderArguments,
    ) -> Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                /*
                 * We assume that upatch-build generated patch file is named same as original elf file.
                 * Thus, we can filter all elf names by existing patch file, which is the patch binary.
                 */
                for elf_relation in &uargs.elf_relations {
                    let output_dir = uargs.output_dir.as_path();
                    let elf_file = elf_relation.elf.as_path();
                    let patch_name = fs::file_name(elf_file);

                    if let Ok(patch_file) = fs::find_file(
                        output_dir,
                        patch_name,
                        fs::FindOptions {
                            fuzz: false,
                            recursive: false,
                        },
                    ) {
                        patch_info
                            .entities
                            .push(PatchHelper::parse_patch_entity(patch_file, elf_file)?);
                    }
                }

                Ok(())
            }
            _ => unreachable!(),
        }
    }
}
