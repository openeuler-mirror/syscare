use crate::cli::{CliWorkDir, CliArguments};

use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchBuilder, PatchBuilderArguments, PatchBuilderArgumentsParser};

use crate::constants::*;

use super::upatch_helper::UserPatchHelper;
use super::upatch_builder_args::UserPatchBuilderArguments;

// use crate::constants::*;

pub struct UserPatchBuilder;

impl UserPatchBuilder {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_arg_list<'a>(&self, args: &'a UserPatchBuilderArguments) -> Vec<&'a str> {
        let mut arg_list = vec![
            "--name",         args.name.as_str(),
            "--workdir",      args.build_root.as_str(),
            "--debugsource",  args.source_dir.as_str(),
            "--debuginfo",    args.debuginfo.as_str(),
            "--elfname",      args.elf_name.as_str(),
            "--output",       args.output_dir.as_str(),
            "--rpmbuild",
        ];

        if args.skip_compiler_check {
            arg_list.push("--skip-compiler-check");
        }

        for patch in &args.patch_list {
            arg_list.push(patch.get_path())
        }

        arg_list
    }
}

impl PatchBuilderArgumentsParser for UserPatchBuilder {
    fn parse_args(patch_info: &PatchInfo, work_dir: &CliWorkDir, args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        let patch_build_root = work_dir.patch_root().build_root_dir();
        let patch_output_dir = work_dir.patch_root().output_dir();

        let source_pkg_dir = work_dir.package_root().source_pkg_dir();
        let debug_pkg_dir  = work_dir.package_root().debug_pkg_dir();

        let source_build_dir = RpmHelper::find_build_root(source_pkg_dir)?;
        let target_elf_name = args.target_elf_name.as_ref().expect("Target elf name is empty");
        let debuginfo_file  = UserPatchHelper::find_debuginfo_file(debug_pkg_dir, target_elf_name)?;

        let builder_args = UserPatchBuilderArguments {
            name:                patch_info.get_patch().get_name().to_owned(),
            build_root:          patch_build_root.to_owned(),
            source_dir:          source_build_dir.to_string(),
            elf_name:            target_elf_name.to_owned(),
            debuginfo:           debuginfo_file,
            output_dir:          patch_output_dir.to_owned(),
            skip_compiler_check: args.skip_compiler_check,
            patch_list:          patch_info.get_file_list().to_owned(),
        };

        Ok(PatchBuilderArguments::UserPatch(builder_args))
    }
}

impl PatchBuilder for UserPatchBuilder {
    fn build_patch(&self, args: PatchBuilderArguments) -> std::io::Result<()> {
        match args {
            PatchBuilderArguments::UserPatch(uargs) => {
                let exit_status = UPATCH_BUILD.execvp(self.parse_arg_list(&uargs))?;

                let exit_code = exit_status.exit_code();
                if exit_code != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("Process '{}' exited unsuccessfully, exit code: {}", UPATCH_BUILD, exit_code),
                    ));
                }
                Ok(())
            },
            _ => unreachable!(),
        }
    }
}
