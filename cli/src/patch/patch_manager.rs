use std::collections::HashMap;
use std::path::Path;

use log::{debug, error};

use common::util::{fs, serde::serde_versioned};

use super::package_info::PackageInfo;
use super::patch::Patch;
use super::patch_info::PatchInfo;
use super::patch_status::PatchStatus;

const PATCH_INSTALL_DIR: &str = "/usr/lib/syscare/patches";
const PATCH_STATUS_FILE: &str = "/usr/lib/syscare/patch_status";

pub struct PatchManager {
    patch_list: Vec<Patch>
}

impl PatchManager {
    fn scan_patch_dir<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<Patch>> {
        debug!("Scanning for patches");

        let mut patch_list = Vec::new();
        for patch_root in fs::list_dirs(directory, fs::TraverseOptions { recursive: false })? {
            match Patch::new(&patch_root) {
                Ok(patch) => {
                    debug!("Detected patch {{{}}} ({})", patch, patch.full_name());
                    patch_list.push(patch);
                },
                Err(e) => {
                    error!("Cannot read patch info from \"{}\", {}",
                        patch_root.display(),
                        e.to_string().to_lowercase()
                    );
                }
            }
        }

        debug!("Found {} patch(es)", patch_list.len());
        Ok(patch_list)
    }

    fn is_matched_patch<T: AsRef<Patch>>(patch: &T, pattern: &str) -> bool {
        let patch = patch.as_ref();
        if (pattern != patch.full_name()) && (pattern != patch.uuid) {
            return false;
        }

        debug!("Found patch {{{}}}", patch);
        true
    }

    fn match_patch<I, F, R, T>(iter: I, is_matched: F, pattern: &str) -> std::io::Result<R>
    where
        I: Iterator<Item = R>,
        F: Fn(&R, &str) -> bool,
        R: AsRef<T>,
    {
        debug!("Finding patch by \"{}\"", pattern);

        let mut list = iter.filter(|obj| is_matched(obj, pattern)).collect::<Vec<_>>();
        match list.len().cmp(&1) {
            std::cmp::Ordering::Equal => {
                Ok(list.swap_remove(0))
            },
            std::cmp::Ordering::Less => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Cannot find patch \"{}\"", pattern)
                ))
            },
            std::cmp::Ordering::Greater => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Found multiple patch named \"{}\", please use uuid instead", pattern)
                ))
            },
        }
    }

    fn find_patch(&self, identifier: &str) -> std::io::Result<&Patch> {
        Self::match_patch(
            self.patch_list.iter(),
            Self::is_matched_patch,
            identifier
        )
    }
}

impl PatchManager {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            patch_list: Self::scan_patch_dir(PATCH_INSTALL_DIR)?
        })
    }

    pub fn get_patch_list(&self) -> &[Patch] {
        &self.patch_list
    }

    pub fn get_patch_info(&self, identifier: &str) -> std::io::Result<&PatchInfo> {
        Ok(&self.find_patch(identifier)?.info)
    }

    pub fn get_patch_target(&self, identifier: &str) -> std::io::Result<&PackageInfo> {
        Ok(&self.find_patch(identifier)?.info.target)
    }

    pub fn get_patch_status(&self, identifier: &str) -> std::io::Result<PatchStatus> {
        self.find_patch(identifier)?.status()
    }

    pub fn apply_patch(&self, identifier: &str) -> std::io::Result<()> {
        self.find_patch(identifier)?.apply()
    }

    pub fn remove_patch(&self, identifier: &str) -> std::io::Result<()> {
        self.find_patch(identifier)?.remove()
    }

    pub fn active_patch(&self, identifier: &str) -> std::io::Result<()> {
        self.find_patch(identifier)?.active()
    }

    pub fn deactive_patch(&self, identifier: &str) -> std::io::Result<()> {
        self.find_patch(identifier)?.deactive()
    }

    pub fn save_all_patch_status(&self) -> std::io::Result<()> {
        debug!("Saving all patch status");

        let mut status_map = HashMap::with_capacity(self.patch_list.len());

        for patch in &self.patch_list {
            status_map.insert(&patch.uuid, patch.status()?);
        }
        serde_versioned::serialize(&status_map, PATCH_STATUS_FILE)?;

        Ok(())
    }

    pub fn restore_all_patch_status(&self) -> std::io::Result<()> {
        debug!("Reading all patch status");
        let mut status_map: HashMap<String, PatchStatus> = serde_versioned::deserialize(PATCH_STATUS_FILE)?;
        /*
         * Merge patch status map with current patch list
         * and treat new patch as NOT-APPLIED
         */
        for patch in self.get_patch_list() {
            if !status_map.contains_key(&patch.uuid) {
                status_map.insert(patch.uuid.to_owned(), PatchStatus::NotApplied);
            }
        }
        /*
         * To ensure that we won't load multiple patches for same target at the same time,
         * we take following measures:
         * 1. map DEACTIVED status to NOT-APPLIED
         * 2. sort patch status to make sure we firstly do REMOVE operation
         */
        let mut status_list = status_map.into_iter().map(|(uuid, mut status)| {
            if status == PatchStatus::Deactived {
                status = PatchStatus::NotApplied;
            }
            (uuid, status)
        }).collect::<Vec<_>>();
        status_list.sort_by(|(_, lhs), (_, rhs)| lhs.cmp(rhs));

        for (uuid, status) in status_list {
            match self.find_patch(&uuid) {
                Ok(patch) => {
                    if let Err(e) = patch.restore(status) {
                        error!("{}", e);
                        error!("Patch {{{}}} restore failed", patch);
                        continue;
                    }
                },
                Err(e) => error!("{}", e)
            }
        }

        Ok(())
    }
}
