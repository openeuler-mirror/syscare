use std::ffi::OsString;

use crate::constants::*;
use crate::log::debug;

use crate::cli::{CliWorkDir, CliArguments};
use crate::cmd::ExternCommandArgs;
use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchFile};
use crate::patch::{PatchBuilder, PatchBuilderArgumentsParser, PatchBuilderArguments};
use crate::util::os_str::OsStrConcat;

use super::upatch_helper::UserPatchHelper;
use super::upatch_builder_args::UserPatchBuilderArguments;

// use crate::constants::*;

pub struct UserPatchBuilder;

impl UserPatchBuilder {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_cmd_args<'a>(&self, args: &'a UserPatchBuilderArguments) -> ExternCommandArgs {
        let mut cmd_args = ExternCommandArgs::new()
            .arg("--name")
            .arg(&args.name)
            .arg("--work-dir")
            .arg(&args.work_dir)
            .arg("--debug-source")
            .arg(&args.debug_source)
            .arg("--build-source-cmd")
            .arg(&args.build_source_cmd)
            .arg("--build-patch-cmd")
            .arg(&args.build_patch_cmd)
            .arg("--debug-info")
            .arg(&args.debuginfo)
            .arg("--elf-name")
            .arg(&args.elf_name)
            .arg("--output-dir")
            .arg(&args.output_dir);

        if args.skip_compiler_check {
            cmd_args = cmd_args.arg("--skip-compiler-check");
        }
        if args.verbose {
            cmd_args = cmd_args.arg("--verbose");
        }
        cmd_args = cmd_args.args(&mut args.patch_list.iter().map(PatchFile::get_path));

        cmd_args
    }
}

impl PatchBuilderArgumentsParser for UserPatchBuilder {
    fn parse_args(patch_info: &PatchInfo, workdir: &CliWorkDir, args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        let patch_build_root = workdir.patch_root().build_root_dir();
        let patch_output_dir = workdir.patch_root().output_dir();

        let source_pkg_dir = workdir.package_root().source_pkg_dir();
        let debug_pkg_dir  = workdir.package_root().debug_pkg_dir();

        let pkg_root = RpmHelper::find_build_root(source_pkg_dir)?;
        let pkg_build_dir = pkg_root.build_dir();
        let pkg_specs_dir = pkg_root.specs_dir();

        let patch_source_dir = RpmHelper::find_source_directory(pkg_build_dir, patch_info)?;
        debug!("source directory: '{}'", patch_source_dir.display());

        let spec_file_path = RpmHelper::find_spec_file(pkg_specs_dir)?;
        debug!("spec file: '{}'", spec_file_path.display());

        let patch_debuginfo = UserPatchHelper::find_debuginfo_file(debug_pkg_dir, patch_info)?;
        debug!("debuginfo file: '{}'", patch_debuginfo.display());

        let mut build_original_cmd = OsString::from(RPM_BUILD.to_string());
        build_original_cmd.concat(" --define '_topdir ")
            .concat(&pkg_root)
            .concat("' -bc")
            .concat(" --noclean ")
            .concat(&spec_file_path);

        let mut build_patched_cmd  = OsString::from(RPM_BUILD.to_string());
        build_patched_cmd.concat(" --define '_topdir ")
            .concat(&pkg_root)
            .concat("' -bc")
            .concat(" --noclean")
            .concat(" --noprep ")
            .concat(&spec_file_path);

        let builder_args = UserPatchBuilderArguments {
            name:                patch_info.get_name().to_owned(),
            work_dir:            patch_build_root.to_owned(),
            debug_source:        patch_source_dir,
            elf_name:            patch_info.get_target_elf_name().to_owned(),
            debuginfo:           patch_debuginfo,
            build_source_cmd:    build_original_cmd,
            build_patch_cmd:     build_patched_cmd,
            output_dir:          patch_output_dir.to_path_buf(),
            skip_compiler_check: args.skip_compiler_check,
            verbose:             args.verbose,
            patch_list:          patch_info.get_file_list().to_owned(),
        };

        Ok(PatchBuilderArguments::UserPatch(builder_args))
    }
}

impl PatchBuilder for UserPatchBuilder {
    fn build_patch(&self, args: PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                let exit_status = UPATCH_BUILD.execvp(self.parse_cmd_args(&uargs))?;

                let exit_code = exit_status.exit_code();
                if exit_code != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("Process '{}' exited unsuccessfully, exit_code={}", UPATCH_BUILD, exit_code),
                    ));
                }
                Ok(())
            },
            _ => unreachable!(),
        }
    }
}
