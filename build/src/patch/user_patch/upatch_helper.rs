use std::path::{Path, PathBuf};

use crate::patch::PatchInfo;

use crate::util::fs;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_debuginfo_file<P: AsRef<Path>>(directory: P, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        let target = patch_info.get_target();
        let file_name = format!("{}-{}-{}.{}.debug",
            patch_info.get_target_elf_name(), target.get_version(), target.get_release(), patch_info.get_arch()
        );

        fs::find_file(
            directory,
            file_name.as_str(),
            false,
            true
        )
    }
}
