use std::collections::VecDeque;
use std::path::Path;

use log::{debug, error, warn};

use crate::util::{fs, serde};

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
    fn scan_patch_list<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<Patch>> {
        debug!("Scanning patch list");

        let mut patch_list = Vec::new();
        for pkg_root in fs::list_all_dirs(directory, false)? {
            for patch_root in fs::list_all_dirs(&pkg_root, false)? {
                match Patch::new(&patch_root) {
                    Ok(patch) => {
                        debug!("Detected patch \"{}\"", patch);
                        patch_list.push(patch);
                    },
                    Err(e) => {
                        error!("{}", e);
                        error!("Cannot read patch info from \"{}\"", patch_root.to_string_lossy());
                    }
                }
            }
        }

        Ok(patch_list)
    }

    fn is_matched_patch<T: AsRef<Patch>>(patch: &T, pattern: &str) -> bool {
        let patch = patch.as_ref();
        if (pattern != patch.short_name()) && (pattern != patch.full_name()) {
            return false;
        }

        debug!("Matched patch \"{}\"", patch);
        true
    }

    fn match_patch<I, F, R, T>(iter: I, is_matched: F, pattern: &str) -> std::io::Result<R>
    where
        I: Iterator<Item = R>,
        F: Fn(&R, &str) -> bool,
        R: AsRef<T>,
    {
        debug!("Matching patch \"{}\"", pattern);

        let mut list = iter.filter(|obj| is_matched(obj, pattern)).collect::<VecDeque<_>>();
        match list.len().cmp(&1) {
            std::cmp::Ordering::Less => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Cannot find patch \"{}\"", pattern)
                ))
            },
            std::cmp::Ordering::Equal => {
                Ok(list.pop_front().unwrap())
            },
            std::cmp::Ordering::Greater => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Found multiple patch named \"{}\", please use 'pkg_name/patch_name' instead", pattern)
                ))
            },
        }
    }

    fn find_patch(&self, patch_name: &str) -> std::io::Result<&Patch> {
        Self::match_patch(
            self.patch_list.iter(),
            Self::is_matched_patch,
            patch_name
        )
    }

    fn find_patch_mut(&mut self, patch_name: &str) -> std::io::Result<&mut Patch> {
        Self::match_patch(
            self.patch_list.iter_mut(),
            Self::is_matched_patch,
            patch_name
        )
    }
}

impl PatchManager {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            patch_list: Self::scan_patch_list(PATCH_INSTALL_DIR)?
        })
    }

    pub fn get_patch_list(&self) -> &[Patch] {
        &self.patch_list
    }

    pub fn get_patch_info(&self, patch_name: &str) -> std::io::Result<&PatchInfo> {
        Ok(&self.find_patch(patch_name)?.info)
    }

    pub fn get_patch_target(&self, patch_name: &str) -> std::io::Result<&PackageInfo> {
        Ok(&self.find_patch(patch_name)?.info.target)
    }

    pub fn get_patch_status(&self, patch_name: &str) -> std::io::Result<PatchStatus> {
        self.find_patch(patch_name)?.status()
    }

    pub fn apply_patch(&mut self, patch_name: &str) -> std::io::Result<()> {
        self.find_patch_mut(patch_name)?.apply()
    }

    pub fn remove_patch(&mut self, patch_name: &str) -> std::io::Result<()> {
        self.find_patch_mut(patch_name)?.remove()
    }

    pub fn active_patch(&mut self, patch_name: &str) -> std::io::Result<()> {
        self.find_patch_mut(patch_name)?.active()
    }

    pub fn deactive_patch(&mut self, patch_name: &str) -> std::io::Result<()> {
        self.find_patch_mut(patch_name)?.deactive()
    }

    pub fn save_all_patch_status(&mut self) -> std::io::Result<()> {
        let mut status_list = Vec::with_capacity(self.patch_list.len());

        for patch in &mut self.patch_list {
            status_list.push((patch.short_name(), patch.status()?))
        }
        serde::serialize(&status_list, PATCH_STATUS_FILE)?;

        Ok(())
    }

    pub fn restore_all_patch_status(&mut self) -> std::io::Result<()> {
        let saved_patch_status = serde::deserialize::<_, Vec<(String, PatchStatus)>>(PATCH_STATUS_FILE)?;
        for (patch_name, status) in saved_patch_status {
            match self.find_patch_mut(&patch_name) {
                Ok(patch) => {
                    if let Err(_) = patch.restore(status) {
                        warn!("Patch \"{}\" restore failed", patch);
                        continue;
                    }
                },
                Err(e) => error!("{}", e)
            }
        }

        Ok(())
    }
}
