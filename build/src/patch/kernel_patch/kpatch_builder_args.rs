use crate::patch::{PatchName, PatchFile};

pub struct KernelPatchBuilderArguments {
    pub build_root:          String,
    pub patch_name:          PatchName,
    pub source_dir:          String,
    pub config:              String,
    pub vmlinux:             String,
    pub jobs:                usize,
    pub output_dir:          String,
    pub skip_compiler_check: bool,
    pub patch_list:          Vec<PatchFile>,
}
