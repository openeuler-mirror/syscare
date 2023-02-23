use std::collections::VecDeque;

use log::{debug, error};

use crate::util::{fs, serde};

use super::package_info::PackageInfo;
use super::patch::Patch;
use super::patch_info::PatchInfo;
use super::patch_status::PatchStatus;

const PATCH_INSTALL_DIR: &str = "/usr/lib/syscare/patches";
const PATCH_STATUS_FILE: &str = "/usr/lib/syscare/patch_status";

pub struct PatchManager {
    patch_list: Vec<Patch>,
    patch_install_dir: &'static str,
    patch_status_file: &'static str,
}

impl PatchManager {
    fn initialize(&mut self) -> std::io::Result<()> {
        debug!("Scanning patch list");
        self.scan_patch_list()?;

        debug!("Updating all patch status");
        self.update_all_patch_status()?;

        Ok(())
    }

    fn scan_patch_list(&mut self) -> std::io::Result<()> {
        let pkg_list = fs::list_all_dirs(self.patch_install_dir, false)?;
        for pkg_root in &pkg_list {
            for patch_root in fs::list_all_dirs(&pkg_root, false)? {
                match Patch::new(&patch_root) {
                    Ok(patch) => {
                        debug!("Detected patch \"{}\"", patch);
                        self.patch_list.push(patch);
                    },
                    Err(e) => {
                        error!("{}", e);
                        error!("Cannot read patch info from \"{}\"", patch_root.to_string_lossy());
                    }
                }
            }
        }

        Ok(())
    }

    fn update_all_patch_status(&mut self) -> std::io::Result<()> {
        for patch in &mut self.patch_list {
            patch.update_status()?;
        }
        Ok(())
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
        let mut new_instance = Self {
            patch_list:        vec![],
            patch_install_dir: PATCH_INSTALL_DIR,
            patch_status_file: PATCH_STATUS_FILE,
        };

        new_instance.initialize()?;

        Ok(new_instance)
    }

    pub fn get_patch_list(&self) -> &[Patch] {
        self.patch_list.as_slice()
    }

    pub fn get_patch_info(&self, patch_name: &str) -> std::io::Result<&PatchInfo> {
        Ok(&self.find_patch(patch_name)?.info)
    }

    pub fn get_patch_target(&self, patch_name: &str) -> std::io::Result<&PackageInfo> {
        Ok(&self.find_patch(patch_name)?.info.target)
    }

    pub fn get_patch_status(&self, patch_name: &str) -> std::io::Result<PatchStatus> {
        Ok(self.find_patch(patch_name)?.status)
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

    pub fn save_all_patch_status(&self) -> std::io::Result<()> {
        let patch_status_list = self.patch_list.iter().map(|patch| {
            (patch.short_name(), patch.status)
        }).collect::<Vec<_>>();

        serde::serialize(&patch_status_list, self.patch_status_file)?;

        Ok(())
    }

    pub fn restore_all_patch_status(&mut self) -> std::io::Result<()> {
        let saved_patch_status = serde::deserialize::<_, Vec<(String, PatchStatus)>>(self.patch_status_file)?;

        for (patch_name, status) in saved_patch_status {
            match self.find_patch_mut(&patch_name) {
                Ok(patch) => patch.restore(status)?,
                Err(e)    => error!("{}", e)
            }
        }

        Ok(())
    }
}
