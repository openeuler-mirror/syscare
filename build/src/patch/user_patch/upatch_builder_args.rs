use crate::patch::{PatchName, PatchFile};

pub struct UserPatchBuilderArguments {
    pub build_root:  String,
    pub patch_name:  PatchName,
    pub source_dir:  String,
    pub kconfig:     String,
    pub vmlinux:     String,
    pub jobs:        usize,
    pub skip_check:  bool,
    pub output_dir:  String,
    pub patch_files: Vec<PatchFile>,
}
