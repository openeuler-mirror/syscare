use clap::{clap_app, crate_description, crate_name, crate_version, AppSettings, ArgMatches};

const DEFAULT_WORK_DIR: &str = "~/.upatch";
const DEFAULT_BUILD_PATCH_CMD: &str = "";
const DEFAULT_COMPILERS: &str = "gcc";
const DEFAULT_OUTPUT_DIR: &str = "~/.upatch";

pub struct ArgMatcher;

impl ArgMatcher {
    pub fn get_matched_args() -> ArgMatches<'static> {
        clap_app!(syscare_cli =>
            (name: crate_name!())
            (version: crate_version!())
            (about: crate_description!())
            (global_settings: &[ AppSettings::ArgsNegateSubcommands, AppSettings::DeriveDisplayOrder, AppSettings::UnifiedHelpMessage ])
            (@arg name: -n --name +takes_value value_name("NAME") "Specify output name")
            (@arg work_dir: -w --work_dir +takes_value value_name("WORK_DIR") default_value(DEFAULT_WORK_DIR) "Specify work directory")
            (@arg source_dir: -s --source_dir +required +takes_value value_name("SOURCE_DIR") "Specify source directory")
            (@arg build_source_cmd: -b --build_source_cmd +required +takes_value value_name("BUILD_SOURCE_CMD") "Specify build source command")
            (@arg build_patch_cmd: --build_patch_cmd +takes_value value_name("BUILD_PATCH_CMD") default_value(DEFAULT_BUILD_PATCH_CMD) +hide_default_value "Specify build patched source command [default: <BUILD_SOURCE_CMD>]")
            (@arg debuginfo: -d --debuginfo +required +multiple +takes_value value_name("DEBUGINFO") "Specify debuginfo files")
            (@arg elf_dir: --elf_dir +takes_value value_name("ELF_DIR") "Specify the directory of searching elf [default: <SOURCE_DIR>]")
            (@arg elf_path: --elf_path +required +multiple +takes_value value_name("ELF_PATCH") "Specify elf's relative path relate to 'elf_dir' or absolute patch list")
            (@arg compiler: -c --compiler +multiple +takes_value value_name("COMPILER") default_value(DEFAULT_COMPILERS) "Specify compiler(s)")
            (@arg output_dir: -o --output_dir +takes_value value_name("OUTPUT_DIR") default_value(DEFAULT_OUTPUT_DIR) +hide_default_value "Specify output directory [default: <WORK_DIR>]")
            (@arg verbose: -v --verbose "Provide more detailed info")
            (@arg patches: -p --patch +required +multiple +takes_value value_name("PATCHES") "Patch file(s)")
        ).get_matches()
    }
}
