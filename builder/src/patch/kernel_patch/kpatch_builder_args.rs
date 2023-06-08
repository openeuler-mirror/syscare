use std::path::PathBuf;

use crate::patch::PatchFile;

pub struct KernelPatchBuilderArguments {
    pub build_root: PathBuf,
    pub patch_name: String,
    pub source_dir: PathBuf,
    pub config: PathBuf,
    pub vmlinux: PathBuf,
    pub jobs: usize,
    pub output_dir: PathBuf,
    pub skip_compiler_check: bool,
    pub patch_list: Vec<PatchFile>,
}
