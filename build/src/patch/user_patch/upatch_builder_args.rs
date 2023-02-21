use std::ffi::OsString;
use std::path::PathBuf;

use crate::patch::PatchFile;

pub struct UserPatchBuilderArguments {
    pub name:                String,
    pub work_dir:            PathBuf,
    pub debug_source:        PathBuf,
    pub debuginfo:           Vec<PathBuf>,
    pub build_source_cmd:    OsString,
    pub build_patch_cmd:     OsString,
    pub output_dir:          PathBuf,
    pub skip_compiler_check: bool,
    pub verbose:             bool,
    pub patch_list:          Vec<PatchFile>,
}
