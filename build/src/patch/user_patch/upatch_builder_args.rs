use crate::patch::PatchFile;

pub struct UserPatchBuilderArguments {
    pub name:                String,
    pub build_root:          String,
    pub elf_name:            String,
    pub source_dir:          String,
    pub debuginfo:           String,
    pub output_dir:          String,
    pub skip_compiler_check: bool,
    pub patch_list:          Vec<PatchFile>,
}
