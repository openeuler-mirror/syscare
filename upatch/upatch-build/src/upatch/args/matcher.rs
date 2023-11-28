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
            (set_term_width: 120)
            (global_settings: &[
                AppSettings::ArgRequiredElseHelp,
                AppSettings::ColorNever,
                AppSettings::DeriveDisplayOrder,
                AppSettings::UnifiedHelpMessage,
            ])
            (@arg name: short("n") long("name") +takes_value value_name("NAME") "Specify output name")
            (@arg work_dir: short("w") long("work-dir") +takes_value value_name("WORK_DIR") default_value(DEFAULT_WORK_DIR) "Specify working directory")
            (@arg source_dir: short("s") long("source-dir") +required +takes_value value_name("SOURCE_DIR") "Specify source directory")
            (@arg build_source_cmd: short("b") long("build-source-cmd") +required +takes_value value_name("BUILD_SOURCE_CMD") "Specify build source command")
            (@arg build_patch_cmd: long("build-patch-cmd") +takes_value value_name("BUILD_PATCH_CMD") default_value(DEFAULT_BUILD_PATCH_CMD) +hide_default_value "Specify build patched source command [default: <BUILD_SOURCE_CMD>]")
            (@arg debuginfo: short("d") long("debuginfo") +required +multiple +takes_value value_name("DEBUGINFO") "Specify debuginfo files")
            (@arg elf_dir: long("elf-dir") +takes_value value_name("ELF_DIR") "Specify the directory of searching elf [default: <SOURCE_DIR>]")
            (@arg elf_path: long("elf-path") +required +multiple +takes_value value_name("ELF_PATCH") "Specify elf's relative path relate to 'elf_dir' or absolute patch list")
            (@arg compiler: short("c") long("compiler") +multiple +takes_value value_name("COMPILER") default_value(DEFAULT_COMPILERS) "Specify compiler(s)")
            (@arg patch: short("p") long("patch") +required +multiple +takes_value value_name("PATCH") "Patch file(s)")
            (@arg output_dir: short("o") long("output-dir") +takes_value value_name("OUTPUT_DIR") default_value(DEFAULT_OUTPUT_DIR) +hide_default_value "Specify output directory [default: <WORK_DIR>]")
            (@arg skip_compiler_check: long("skip-compiler-check") "Skip compiler version check (not recommended)")
            (@arg verbose: short("v") long("verbose") "Provide more detailed info")
        ).get_matches()
    }
}
