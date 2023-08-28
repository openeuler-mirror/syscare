use std::path::{Path, PathBuf};

use syscare_abi::PatchEntity;

#[derive(Debug)]
pub enum PatchInfoExt {
    UserPatch(UserPatchExt),
    KernelPatch(KernelPatchExt),
}

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

#[derive(Debug)]
pub struct KernelPatchExt {
    pub patch_file: PathBuf,
    pub sys_file: PathBuf,
}

impl KernelPatchExt {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_entity: &PatchEntity) -> Self {
        const KPATCH_SUFFIX: &str = ".ko";
        const KPATCH_MGNT_DIR: &str = "/sys/kernel/livepatch";
        const KPATCH_MGNT_FILE_NAME: &str = "enabled";

        let patch_file_name = patch_entity.patch_name.to_string_lossy();
        let patch_sys_name = patch_file_name
            .strip_suffix(KPATCH_SUFFIX)
            .expect("Illegal patch suffix")
            .replace('-', "_");

        Self {
            patch_file: patch_root.as_ref().join(patch_file_name.as_ref()),
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
