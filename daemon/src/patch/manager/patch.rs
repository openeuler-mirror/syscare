use std::{path::Path, sync::Arc};

use anyhow::Result;

use syscare_abi::{PatchInfo, PatchType, PATCH_INFO_MAGIC};
use syscare_common::util::serde;

use super::{
    info_ext::{KernelPatchExt, PatchInfoExt, UserPatchExt},
    PATCH_INFO_FILE_NAME,
};

/// Patch management target abstraction
#[derive(Debug)]
pub struct Patch {
    pub uuid: String,
    pub entity_name: String,
    pub patch_name: String,
    pub target_name: String,
    pub target_pkg_name: String,
    pub checksum: String,
    pub info: Arc<PatchInfo>,
    pub info_ext: PatchInfoExt,
}

impl Patch {
    pub fn read_from<P: AsRef<Path>>(patch_root: P) -> Result<Vec<Self>> {
        let patch_root = patch_root.as_ref();
        let patch_info = Arc::new(serde::deserialize_with_magic::<PatchInfo, _, _>(
            patch_root.join(PATCH_INFO_FILE_NAME),
            PATCH_INFO_MAGIC,
        )?);

        let mut patch_list = Vec::with_capacity(patch_info.entities.len());
        for patch_entity in patch_info.entities.iter() {
            let uuid = patch_entity.uuid.clone();
            let checksum = patch_entity.checksum.clone();
            let patch_name = patch_info.name();
            let target_name = patch_info.target.short_name();
            let target_pkg_name = patch_info.target.full_name();

            let patch = match patch_info.kind {
                PatchType::KernelPatch => {
                    let entity_name: String =
                        patch_entity.patch_target.to_string_lossy().to_string();
                    Self {
                        uuid,
                        entity_name: format!("{}/{}/{}", target_name, patch_name, entity_name),
                        patch_name: format!("{}/{}", target_name, patch_name),
                        target_name,
                        target_pkg_name,
                        checksum,
                        info: patch_info.clone(),
                        info_ext: PatchInfoExt::KernelPatch(KernelPatchExt::new(
                            patch_root,
                            patch_entity,
                        )),
                    }
                }
                PatchType::UserPatch => {
                    let entity_name = patch_entity.patch_name.to_string_lossy().to_string();
                    Self {
                        uuid,
                        entity_name: format!("{}/{}/{}", target_name, patch_name, entity_name),
                        patch_name: format!("{}/{}", target_name, patch_name),
                        target_name,
                        target_pkg_name,
                        checksum,
                        info: patch_info.clone(),
                        info_ext: PatchInfoExt::UserPatch(UserPatchExt::new(
                            patch_root,
                            patch_entity,
                        )),
                    }
                }
            };
            patch_list.push(patch);
        }

        Ok(patch_list)
    }
}

impl Patch {
    pub fn kind(&self) -> PatchType {
        self.info.kind
    }
}

impl AsRef<Patch> for Patch {
    fn as_ref(&self) -> &Patch {
        self
    }
}

impl std::cmp::PartialEq for Patch {
    fn eq(&self, other: &Self) -> bool {
        self.uuid.eq(&other.uuid)
    }
}
impl std::cmp::Eq for Patch {}

impl std::cmp::PartialOrd for Patch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.entity_name.partial_cmp(&other.entity_name)
    }
}
impl std::cmp::Ord for Patch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.entity_name.cmp(&other.entity_name)
    }
}

impl std::fmt::Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.entity_name)
    }
}
