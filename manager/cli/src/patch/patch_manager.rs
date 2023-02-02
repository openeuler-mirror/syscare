use std::collections::VecDeque;
use std::io::{BufReader, BufWriter, Write};

use log::{debug, warn};

use crate::util::fs;

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
        debug!("scanning patch list");
        self.scan_patch_list()?;

        debug!("updating all patch status");
        self.update_all_patch_status()?;

        Ok(())
    }

    fn scan_patch_list(&mut self) -> std::io::Result<()> {
        let scan_root  = self.patch_install_dir;
        let patch_list = &mut self.patch_list;

        let pkg_list = fs::list_all_dirs(scan_root, false)?;
        for pkg_root in &pkg_list {
            for patch_root in fs::list_all_dirs(&pkg_root, false)? {
                match Patch::parse_from(&patch_root) {
                    Ok(patch) => {
                        debug!("detected patch \"{}\"", patch);
                        patch_list.push(patch);
                    },
                    Err(e) => {
                        warn!("failed to parse patch from \"{}\", {}", patch_root.display(), e);
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
        let patch_ref   = patch.as_ref();
        let full_name   = patch_ref.get_full_name();
        let simple_name = patch_ref.get_simple_name();

        if (pattern != full_name) && (pattern != simple_name) {
            return false;
        }

        debug!("matched patch \"{}\"", full_name);
        true
    }

    fn match_patch<I, F, R, T>(iter: I, is_matched: F, pattern: &str) -> std::io::Result<R>
    where
        I: Iterator<Item = R>,
        F: Fn(&R, &str) -> bool,
        R: AsRef<T>,
    {
        debug!("matching patch \"{}\"", pattern);

        let mut list = iter.filter(|obj| is_matched(obj, pattern)).collect::<VecDeque<_>>();
        match list.len().cmp(&1) {
            std::cmp::Ordering::Less => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("cannot find patch \"{}\"", pattern)
                ))
            },
            std::cmp::Ordering::Equal => {
                Ok(list.pop_front().unwrap())
            },
            std::cmp::Ordering::Greater => {
                Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("found multiple patch named \"{}\", please use 'pkg_name/patch_name' instead", pattern)
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
        Ok(self.find_patch(patch_name)?.get_info())
    }

    pub fn get_patch_status(&self, patch_name: &str) -> std::io::Result<PatchStatus> {
        Ok(self.find_patch(patch_name)?.get_status())
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
        let patch_status_file = self.patch_status_file;
        let patch_status_list = self.patch_list.iter().map(|patch| {
            (patch.get_full_name(), patch.get_status())
        }).collect::<Vec<_>>();

        let buf = bincode::serialize(&patch_status_list).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("serialize patch status records failed, {}", e.to_string())
            )
        })?;

        let mut writer = BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(patch_status_file)?
        );

        writer.write_all(&buf)?;
        writer.flush()?;
        writer.into_inner()?.sync_all()?;

        Ok(())
    }

    pub fn restore_all_patch_status(&mut self) -> std::io::Result<()> {
        let patch_status_file = self.patch_status_file;
        let reader = BufReader::new(
            match std::fs::File::open(patch_status_file) {
                Ok(file) => file,
                Err(e) => {
                    debug!("cannot open file \"{}\", {}", patch_status_file, e.to_string());
                    return Ok(());
                },
            }
        );

        let saved_patch_status = bincode::deserialize_from::<_, Vec<(String, PatchStatus)>>(reader).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("deserialize patch status records from \"{}\" failed, {}", patch_status_file, e.to_string())
            )
        })?;

        for (patch_name, status) in saved_patch_status {
            match self.find_patch_mut(&patch_name) {
                Ok(patch) => patch.restore(status)?,
                Err(e)    => warn!("{}", e.to_string())
            }
        }

        Ok(())
    }
}
