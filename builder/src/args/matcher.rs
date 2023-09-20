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
            (@arg patch_name: -n --patch_name +required +takes_value value_name("PATCH_NAME") "Patch name")
            (@arg patch_arch: --patch_arch +takes_value value_name("PATCH_ARCH") default_value(&DEFAULT_PATCH_ARCH)  "Patch architecture")
            (@arg patch_version: --patch_version +takes_value value_name("PATCH_VERSION") default_value(DEFAULT_PATCH_VERSION) "Patch version")
            (@arg patch_release: --patch_release +takes_value value_name("PATCH_RELEASE") default_value(DEFAULT_PATCH_RELEASE) "Patch release")
            (@arg patch_description: --patch_description +takes_value value_name("PATCH_DESCRIPTION") default_value(DEFAULT_PATCH_DESCRIPTION) "Patch description")
            (@arg source: -s --source +required +takes_value value_name("SOURCE") "Source package")
            (@arg debuginfo: -d --debuginfo +required +takes_value +multiple value_name("DEBUGINFO") "Debuginfo package(s)")
            (@arg workdir: --workdir +takes_value value_name("WORKDIR") default_value(DEFAULT_WORK_DIR) "Working directory")
            (@arg output: -o --output +takes_value value_name("OUTPUT") default_value(DEFAULT_OUTPUT_DIR) "Output directory")
            (@arg jobs: -j --jobs +takes_value value_name("JOBS") default_value(&DEFAULT_BUILD_JOBS) "Parllel build jobs")
            (@arg skip_compiler_check: --skip_compiler_check "Skip compiler version check (not recommended)")
            (@arg skip_cleanup: --skip_cleanup "Skip post-build cleanup")
            (@arg verbose: -v --verbose "Provide more detailed info")
            (@arg patches: -p --patch +required +takes_value +multiple value_name("PATCH_FILES") "Patch file(s)")
        ).get_matches()
    }
}
