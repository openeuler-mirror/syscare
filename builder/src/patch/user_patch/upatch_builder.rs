use std::ffi::OsString;
use std::path::Path;

use common::util::os_str::OsStringExt;
use common::util::ext_cmd::{ExternCommand, ExternCommandArgs};
use common::util::fs;

use crate::cli::{CliWorkDir, CliArguments};
use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchBuilder, PatchBuilderArguments};

use super::upatch_builder_args::UserPatchBuilderArguments;

pub struct UserPatchBuilder<'a> {
    workdir: &'a CliWorkDir
}

impl<'a> UserPatchBuilder<'a> {
    pub fn new(workdir: &'a CliWorkDir) -> Self {
        Self { workdir }
    }

    fn detect_compiler_names(&self) -> Vec<OsString> {
        /*
         * This is a temporary solution for compiler detection
         * We assume the compiler would be 'gcc/g++' by default
         * If gcc_secure is installed, the real compiler would be 'gcc_old/g++_old'
         */
        const GCC_SECURE_PKG_NAME:        &str = "gcc_secure";
        const GCC_SECURE_COMPILER_SUFFIX: &str = "_old";

        const GCC_CC_NAME:  &str = "gcc";
        const GCC_CXX_NAME: &str = "g++";

        let mut compiler_list = vec![
            OsString::from(GCC_CC_NAME),
            OsString::from(GCC_CXX_NAME),
        ];

        if RpmHelper::is_package_installed(GCC_SECURE_PKG_NAME) {
            for compiler in &mut compiler_list {
                compiler.push(GCC_SECURE_COMPILER_SUFFIX)
            }
        }

        compiler_list
    }

    fn create_topdir_macro<P: AsRef<Path>>(&self, buildroot: P) -> OsString {
        OsString::from("--define \"_topdir").append(buildroot.as_ref()).concat("\"")
    }

    fn create_build_macros(&self, args: &CliArguments) -> OsString {
        OsString::new()
            .append("--define \"_smp_build_ncpus").append(args.jobs.to_string()).concat("\"")
            .append("--define \"__arch_install_post %{nil}\"")
            .append("--define \"__os_install_post %{nil}\"")
            .append("--define \"__find_provides %{nil}\"")
            .append("--define \"__find_requires %{nil}\"")
    }

    fn parse_cmd_args(&self, args: &UserPatchBuilderArguments) -> ExternCommandArgs {
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

        for compiler in &args.compilers {
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
    fn parse_builder_args(&self, patch_info: &PatchInfo, args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        const RPMBUILD_CMD:            &str = "rpmbuild";
        const RPMBUILD_PERP_FLAGS:     &str = "-bp";
        const RPMBUILD_ORIGINAL_FLAGS: &str = "-bi --noprep --nocheck --nodebuginfo --clean";
        const RPMBUILD_PATCHED_FLAGS:  &str = "-bi --noprep --nocheck --nodebuginfo --noclean";

        let source_pkg_dir = self.workdir.package.source.as_path();
        let debug_pkg_dir  = self.workdir.package.debug.as_path();

        let pkg_build_root    = RpmHelper::find_build_root(source_pkg_dir)?;
        let pkg_spec_dir      = pkg_build_root.specs.as_path();
        let pkg_build_dir     = pkg_build_root.build.as_path();
        let pkg_buildroot_dir = pkg_build_root.build_root.as_path();
        let pkg_spec_file     = RpmHelper::find_spec_file(pkg_spec_dir)?;

        let target_pkg      = &patch_info.target;
        let work_dir        = self.workdir.patch.build.as_path();
        let source_dir      = RpmHelper::find_build_source(pkg_build_dir, patch_info)?;
        let debuginfos      = RpmHelper::find_debuginfo(debug_pkg_dir)?;
        let debug_relations = RpmHelper::parse_elf_relations(debuginfos, debug_pkg_dir, target_pkg)?;
        let compiler_names  = self.detect_compiler_names();
        let output_dir      = self.workdir.patch.output.as_path();

        let topdir_macro   = self.create_topdir_macro(pkg_build_root.as_ref());
        let build_macros   = self.create_build_macros(args);

        let build_prep_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(RPMBUILD_PERP_FLAGS)
            .append(&pkg_spec_file);

        let build_original_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_ORIGINAL_FLAGS)
            .append(&pkg_spec_file);

        let build_patched_cmd = OsString::from(RPMBUILD_CMD)
            .append(&topdir_macro)
            .append(&build_macros)
            .append(RPMBUILD_PATCHED_FLAGS)
            .append(&pkg_spec_file);

        let builder_args = UserPatchBuilderArguments {
            work_dir:            work_dir.to_path_buf(),
            debug_source:        source_dir,
            elf_dir:             pkg_buildroot_dir.to_path_buf(),
            elf_relations:       debug_relations,
            build_source_cmd:    build_original_cmd.append("&&").append(build_prep_cmd),
            build_patch_cmd:     build_patched_cmd,
            compilers:           compiler_names,
            output_dir:          output_dir.to_path_buf(),
            skip_compiler_check: args.skip_compiler_check,
            verbose:             args.verbose,
            patch_list:          patch_info.patches.to_owned(),
        };

        Ok(PatchBuilderArguments::UserPatch(builder_args))
    }

    fn build_patch(&self, args: &PatchBuilderArguments) -> std::io::Result<()> {
        const UPATCH_BUILD: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-build");

        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                UPATCH_BUILD.execvp(
                    self.parse_cmd_args(uargs)
                )?.check_exit_code()
            },
            _ => unreachable!(),
        }
    }

    fn write_patch_info(&self, patch_info: &mut PatchInfo, args: &PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                /*
                 * We assume that upatch-build generated patch file is named same as original elf file.
                 * Thus, we can filter all elf names by existing patch file, which is the patch binary.
                 */
                for elf_relation in &uargs.elf_relations {
                    let output_dir = uargs.output_dir.as_path();
                    let patch_name = fs::file_name(&elf_relation.elf);

                    if fs::find_file(output_dir, &patch_name, fs::FindOptions { fuzz: false, recursive: false }).is_ok() {
                        let elf_path = elf_relation.elf.to_owned();
                        let elf_name = patch_name;

                        patch_info.target_elfs.insert(elf_name, elf_path);
                    }
                }

                Ok(())
            },
            _ => unreachable!(),
        }
    }
}
