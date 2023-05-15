use std::ffi::OsString;
use std::path::PathBuf;

use crate::package::RpmElfRelation;
use crate::patch::PatchFile;

pub struct UserPatchBuilderArguments {
    pub work_dir:            PathBuf,
    pub debug_source:        PathBuf,
    pub elf_dir:             PathBuf,
    pub elf_relations:       Vec<RpmElfRelation>,
    pub build_source_cmd:    OsString,
    pub build_patch_cmd:     OsString,
    pub output_dir:          PathBuf,
    pub compiler:            OsString,
    pub skip_compiler_check: bool,
    pub verbose:             bool,
    pub patch_list:          Vec<PatchFile>,
}
