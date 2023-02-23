use std::ffi::{OsString, OsStr};

use crate::constants::*;

use crate::cli::{CliWorkDir, CliArguments};
use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchBuilder, PatchBuilderArguments};

use crate::util::{serde, fs};
use crate::util::os_str::OsStrConcat;
use crate::util::ext_cmd::ExternCommandArgs;

use super::upatch_helper::UserPatchHelper;
use super::upatch_builder_args::UserPatchBuilderArguments;

pub struct UserPatchBuilder<'a> {
    workdir: &'a CliWorkDir
}

impl<'a> UserPatchBuilder<'a> {
    pub fn new(workdir: &'a CliWorkDir) -> Self {
        Self { workdir }
    }

    fn parse_cmd_args(&self, args: &UserPatchBuilderArguments) -> ExternCommandArgs {
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
            .arg("--output-dir")
            .arg(&args.output_dir);

        for debuginfo in &args.debuginfo {
            cmd_args = cmd_args.arg("--debug-info").arg(debuginfo);
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
        let patch_build_root = self.workdir.patch.build.as_path();
        let patch_output_dir = self.workdir.patch.output.as_path();

        let source_pkg_dir = self.workdir.package.source.as_path();
        let debug_pkg_dir  = self.workdir.package.debug.as_path();

        let pkg_build_root   = RpmHelper::find_build_root(source_pkg_dir)?;
        let spec_file        = RpmHelper::find_spec_file(&pkg_build_root.specs)?;
        let patch_source_dir = RpmHelper::find_source_directory(&pkg_build_root.build, patch_info)?;
        let patch_debuginfo  = UserPatchHelper::find_debuginfo_file(debug_pkg_dir)?;

        let build_original_cmd = OsString::from(RPM_BUILD.to_string())
            .concat(" --define '_topdir ")
            .concat(&pkg_build_root)
            .concat("' -bb ")
            .concat("--nodebuginfo ")
            .concat("--noclean ")
            .concat(&spec_file);

        let build_patched_cmd = OsString::from(RPM_BUILD.to_string())
            .concat(" --define '_topdir ")
            .concat(&pkg_build_root)
            .concat("' -bc ")
            .concat("--noprep ")
            .concat("--nodebuginfo ")
            .concat("--noclean ")
            .concat(&spec_file);

        let builder_args = UserPatchBuilderArguments {
            name:                patch_info.name.to_owned(),
            work_dir:            patch_build_root.to_owned(),
            debug_source:        patch_source_dir,
            debuginfo:           patch_debuginfo,
            build_source_cmd:    build_original_cmd,
            build_patch_cmd:     build_patched_cmd,
            output_dir:          patch_output_dir.to_path_buf(),
            skip_compiler_check: args.skip_compiler_check,
            verbose:             args.verbose,
            patch_list:          patch_info.patches.to_owned(),
        };

        Ok(PatchBuilderArguments::UserPatch(builder_args))
    }

    fn build_patch(&self, args: &PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                let exit_status = UPATCH_BUILD.execvp(self.parse_cmd_args(uargs))?;

                let exit_code = exit_status.exit_code();
                if exit_code != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("Process \"{}\" exited unsuccessfully, exit_code={}", UPATCH_BUILD, exit_code),
                    ));
                }

                Ok(())
            },
            PatchBuilderArguments::KernelPatch(_) => unreachable!(),
        }
    }

    fn write_patch_info(&self, patch_info: &mut PatchInfo, args: &PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                let elf_names = serde::deserialize::<_, Vec<OsString>>(
                    uargs.output_dir.join(PATCH_ELF_NAME_FILE)
                )?;

                let src_pkg_dir = self.workdir.package.source.as_path();
                let pkg_name = format!("{}.{}",
                    patch_info.target.full_name(),
                    PKG_FILE_EXTENSION
                );
                let pkg_path = fs::find_file(src_pkg_dir, &pkg_name, false, true)?;
                let pkg_file_list = UserPatchHelper::query_pkg_file_list(pkg_path)?;

                let elf_map = pkg_file_list.into_iter()
                    .filter_map(|elf_path| {
                        elf_path.file_name()
                            .map(OsStr::to_os_string)
                            .and_then(|elf_name| {
                                if !elf_names.contains(&elf_name) {
                                    return None;
                                }
                                Some((elf_name.to_os_string(), elf_path))
                            })
                    })
                    .collect::<Vec<_>>();

                    patch_info.target_elfs.extend(elf_map);

                Ok(())
            },
            PatchBuilderArguments::KernelPatch(_) => unreachable!(),
        }
    }
}
