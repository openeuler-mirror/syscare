use crate::patch::PatchFile;

pub struct UserPatchBuilderArguments {
    pub name:                 String,
    pub build_root:           String,
    pub elf_name:             String,
    pub source_dir:           String,
    pub debuginfo:            String,
    pub build_source_cmd:     String,
    pub build_patch_cmd:      String,
    pub output_dir:           String,
    pub skip_compiler_check:  bool,
    pub verbose:              bool,
    pub patch_list:           Vec<PatchFile>,
}
