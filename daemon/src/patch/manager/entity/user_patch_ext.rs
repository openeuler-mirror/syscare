use std::path::{Path, PathBuf};

use syscare_abi::PatchEntity;

use super::PatchInfoExt;

#[derive(Debug)]
pub struct UserPatchExt {
    pub patch_file: PathBuf,
    pub target_elf: PathBuf,
}

impl UserPatchExt {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_entity: &PatchEntity) -> Self {
        Self {
            patch_file: patch_root
                .as_ref()
                .join(patch_entity.patch_name.as_os_str()),
            target_elf: patch_entity.patch_target.to_path_buf(),
        }
    }
}

impl<'a> From<&'a PatchInfoExt> for &'a UserPatchExt {
    fn from(ext: &'a PatchInfoExt) -> Self {
        match ext {
            PatchInfoExt::UserPatch(ext) => ext,
            _ => panic!("Cannot convert kernel patch ext into user patch ext"),
        }
    }
}
