use clap::{clap_app, crate_description, crate_name, crate_version, AppSettings, ArgMatches};
use lazy_static::lazy_static;

use syscare_common::os;

const DEFAULT_PATCH_VERSION: &str = "1";
const DEFAULT_PATCH_RELEASE: &str = "1";
const DEFAULT_PATCH_DESCRIPTION: &str = "(none)";
const DEFAULT_WORK_DIR: &str = ".";
const DEFAULT_OUTPUT_DIR: &str = ".";

lazy_static! {
    static ref DEFAULT_BUILD_JOBS: String = os::cpu::num().to_string();
    static ref DEFAULT_PATCH_ARCH: String = os::cpu::arch().to_string_lossy().to_string();
}

pub struct ArgMatcher;

impl ArgMatcher {
    pub fn get_matched_args() -> ArgMatches<'static> {
        clap_app!(syscare_cli =>
            (name: crate_name!())
            (version: crate_version!())
            (about: crate_description!())
            (global_settings: &[ AppSettings::DeriveDisplayOrder, AppSettings::UnifiedHelpMessage ])
            (@arg patch_name: short("n") long("patch-name") +required +takes_value value_name("PATCH_NAME") "Patch name")
            (@arg patch_arch: long("patch-arch") +takes_value value_name("PATCH_ARCH") default_value(&DEFAULT_PATCH_ARCH)  "Patch architecture")
            (@arg patch_version: long("patch-version") +takes_value value_name("PATCH_VERSION") default_value(DEFAULT_PATCH_VERSION) "Patch version")
            (@arg patch_release: long("patch-release") +takes_value value_name("PATCH_RELEASE") default_value(DEFAULT_PATCH_RELEASE) "Patch release")
            (@arg patch_description: long("patch-description") +takes_value value_name("PATCH_DESCRIPTION") default_value(DEFAULT_PATCH_DESCRIPTION) "Patch description")
            (@arg patch_requires: long("patch-requires") +takes_value +multiple value_name("PATCH_REQUIRES") "Patch requirments")
            (@arg source: short("s") long("source") +required +takes_value +multiple value_name("SOURCE") "Source package")
            (@arg debuginfo: short("d") long("debuginfo") +required +takes_value +multiple value_name("DEBUGINFO") "Debuginfo package(s)")
            (@arg patch: short("p") long("patch") +required +takes_value +multiple value_name("PATCH") "Patch file(s)")
            (@arg workdir: long("workdir") +takes_value value_name("WORKDIR") default_value(DEFAULT_WORK_DIR) "Working directory")
            (@arg output: short("o") long("output") +takes_value value_name("OUTPUT") default_value(DEFAULT_OUTPUT_DIR) "Output directory")
            (@arg jobs: short("j") long("jobs") +takes_value value_name("JOBS") default_value(&DEFAULT_BUILD_JOBS) "Parallel build jobs")
            (@arg skip_compiler_check: long("skip-compiler-check") "Skip compiler version check (not recommended)")
            (@arg skip_cleanup: long("skip-cleanup") "Skip post-build cleanup")
            (@arg verbose: short("v") long("verbose") "Provide more detailed info")
        ).get_matches()
    }
}
