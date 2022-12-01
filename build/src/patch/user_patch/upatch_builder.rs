use crate::constants::*;
use crate::log::debug;

use crate::cli::{CliWorkDir, CliArguments};
use crate::package::RpmHelper;
use crate::patch::{PatchInfo, PatchFile};
use crate::patch::{PatchBuilder, PatchBuilderArgumentsParser, PatchBuilderArguments};

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
            "--name",             args.name.as_str(),
            "--work-dir",         args.build_root.as_str(),
            "--debug-source",     args.source_dir.as_str(),
            "--build-source-cmd", args.build_source_cmd.as_str(),
            "--build-patch-cmd",  args.build_patch_cmd.as_str(),
            "--debug-info",       args.debuginfo.as_str(),
            "--elf-name",         args.elf_name.as_str(),
            "--output-dir",       args.output_dir.as_str(),
        ];
        if args.skip_compiler_check {
            arg_list.push("--skip-compiler-check");
        }
        if args.verbose {
            arg_list.push("--verbose");
        }
        arg_list.append(&mut args.patch_list.iter().map(PatchFile::get_path).collect());

        arg_list
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

        let target_elf_name = args.target_elfname.as_ref().expect("Target elf name is empty");
        let patch_source_dir = RpmHelper::find_source_directory(pkg_build_dir, patch_info)?;
        debug!("source directory: '{}'", patch_source_dir);

        let spec_file_path = RpmHelper::find_spec_file(pkg_specs_dir)?;
        debug!("spec file: '{}'", spec_file_path);

        let debuginfo_file = UserPatchHelper::find_debuginfo_file(debug_pkg_dir, target_elf_name)?;
        debug!("debuginfo file: '{}'", debuginfo_file);

        let build_original_cmd = format!("{} --define '_topdir {}' -bb {}", RPM_BUILD, pkg_root, spec_file_path);
        let build_patched_cmd  = format!("{} --define '_topdir {}' -bb --noprep {}", RPM_BUILD, pkg_root, spec_file_path);

        let builder_args = UserPatchBuilderArguments {
            name:                 patch_info.get_patch().get_name().to_owned(),
            build_root:           patch_build_root.to_owned(),
            source_dir:           patch_source_dir,
            elf_name:             target_elf_name.to_owned(),
            debuginfo:            debuginfo_file,
            build_source_cmd:     build_original_cmd,
            build_patch_cmd:      build_patched_cmd,
            output_dir:           patch_output_dir.to_owned(),
            skip_compiler_check:  args.skip_compiler_check,
            verbose:              args.verbose,
            patch_list:           patch_info.get_file_list().to_owned(),
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
                        format!("Process '{}' exited unsuccessfully, exit_code={}", UPATCH_BUILD, exit_code),
                    ));
                }
                Ok(())
            },
            _ => unreachable!(),
        }
    }
}
