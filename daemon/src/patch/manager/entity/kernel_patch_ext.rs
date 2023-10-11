use std::path::{Path, PathBuf};

use syscare_abi::PatchEntity;

use super::PatchInfoExt;

#[derive(Debug)]
pub struct KernelPatchExt {
    pub patch_file: PathBuf,
    pub sys_file: PathBuf,
}

impl KernelPatchExt {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_entity: &PatchEntity) -> Self {
        const KPATCH_SUFFIX: &str = "ko";
        const KPATCH_MGNT_DIR: &str = "/sys/kernel/livepatch";
        const KPATCH_MGNT_FILE_NAME: &str = "enabled";

        let patch_name = patch_entity.patch_name.to_string_lossy();
        let patch_sys_name = patch_name.replace('-', "_").replace('.', "_");
        let patch_file_name = format!("{}.{}", patch_name, KPATCH_SUFFIX);

        Self {
            patch_file: patch_root.as_ref().join(patch_file_name),
            sys_file: PathBuf::from(KPATCH_MGNT_DIR)
                .join(patch_sys_name)
                .join(KPATCH_MGNT_FILE_NAME),
        }
    }
}

impl<'a> From<&'a PatchInfoExt> for &'a KernelPatchExt {
    fn from(ext: &'a PatchInfoExt) -> Self {
        match ext {
            PatchInfoExt::KernelPatch(ext) => ext,
            _ => panic!("Cannot convert user patch ext into kernel patch ext"),
        }
    }
}
