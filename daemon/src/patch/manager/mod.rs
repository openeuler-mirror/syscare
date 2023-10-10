use std::{
    cmp::Ordering,
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn};

use syscare_abi::{PatchStatus, PatchType};
use syscare_common::util::{fs, serde};

mod dependency;
mod driver;
mod entity;
mod monitor;

use dependency::PatchManagerDependency;
use driver::{KernelPatchDriver, PatchDriver, UserPatchDriver};
pub use entity::Patch;
pub use monitor::PatchMonitor;

const PATCH_INFO_FILE_NAME: &str = "patch_info";
const PATCH_INSTALL_DIR: &str = "patches";
const PATCH_STATUS_FILE_NAME: &str = "patch_status";

type Transition = (PatchStatus, PatchStatus);
type TransitionAction = &'static (dyn Fn(&mut PatchManager, &Patch) -> Result<()> + Sync);

const PATCH_APPLY: TransitionAction = &PatchManager::driver_apply_patch;
const PATCH_REMOVE: TransitionAction = &PatchManager::driver_remove_patch;
const PATCH_ACTIVE: TransitionAction = &PatchManager::driver_active_patch;
const PATCH_DEACTIVE: TransitionAction = &PatchManager::driver_deactive_patch;
const PATCH_ACCEPT: TransitionAction = &PatchManager::do_patch_accept;
const PATCH_DECLINE: TransitionAction = &PatchManager::do_patch_decline;

const PATCH_INIT_RESTORE_ACCEPTED_ONLY: bool = true;

lazy_static! {
    static ref DRIVER_MAP: IndexMap<PatchType, Box<dyn PatchDriver>> = IndexMap::from([
        (
            PatchType::KernelPatch,
            Box::new(KernelPatchDriver) as Box<dyn PatchDriver>
        ),
        (
            PatchType::UserPatch,
            Box::new(UserPatchDriver) as Box<dyn PatchDriver>
        ),
    ]);
    static ref TRANSITION_MAP: IndexMap<Transition, Vec<TransitionAction>> = IndexMap::from([
        (
            (PatchStatus::NotApplied, PatchStatus::Deactived),
            vec![PATCH_APPLY]
        ),
        (
            (PatchStatus::NotApplied, PatchStatus::Actived),
            vec![PATCH_APPLY, PATCH_ACTIVE]
        ),
        (
            (PatchStatus::NotApplied, PatchStatus::Accepted),
            vec![PATCH_APPLY, PATCH_ACTIVE, PATCH_ACCEPT]
        ),
        (
            (PatchStatus::Deactived, PatchStatus::NotApplied),
            vec![PATCH_REMOVE]
        ),
        (
            (PatchStatus::Deactived, PatchStatus::Actived),
            vec![PATCH_ACTIVE]
        ),
        (
            (PatchStatus::Deactived, PatchStatus::Accepted),
            vec![PATCH_ACTIVE, PATCH_ACCEPT]
        ),
        (
            (PatchStatus::Actived, PatchStatus::NotApplied),
            vec![PATCH_DEACTIVE, PATCH_REMOVE]
        ),
        (
            (PatchStatus::Actived, PatchStatus::Deactived),
            vec![PATCH_DEACTIVE]
        ),
        (
            (PatchStatus::Actived, PatchStatus::Accepted),
            vec![PATCH_ACCEPT]
        ),
        (
            (PatchStatus::Accepted, PatchStatus::NotApplied),
            vec![PATCH_DECLINE, PATCH_DEACTIVE, PATCH_REMOVE]
        ),
        (
            (PatchStatus::Accepted, PatchStatus::Deactived),
            vec![PATCH_DECLINE, PATCH_DEACTIVE]
        ),
        (
            (PatchStatus::Accepted, PatchStatus::Actived),
            vec![PATCH_DECLINE]
        ),
    ]);
}

struct PatchEntry {
    patch: Arc<Patch>,
    status: PatchStatus,
}

pub struct PatchManager {
    _dependency: PatchManagerDependency,
    patch_install_dir: PathBuf,
    patch_status_file: PathBuf,
    entry_map: IndexMap<String, PatchEntry>,
}

impl PatchManager {
    pub fn new<P: AsRef<Path>>(patch_root: P) -> Result<Self> {
        let _dependency = PatchManagerDependency::new()?;
        let patch_install_dir = patch_root.as_ref().join(PATCH_INSTALL_DIR);
        let patch_status_file = patch_root.as_ref().join(PATCH_STATUS_FILE_NAME);
        let entry_map = Self::scan_patches(&patch_install_dir)?;

        let mut instance = Self {
            _dependency,
            patch_install_dir,
            patch_status_file,
            entry_map,
        };

        instance.restore_patch_status(PATCH_INIT_RESTORE_ACCEPTED_ONLY)?;

        Ok(instance)
    }

    fn finallize(&mut self) {
        if let Err(e) = self.save_patch_status() {
            error!("{:?}", e)
        }
    }
}

impl PatchManager {
    pub fn match_patch(&self, identifier: &str) -> Result<Vec<Arc<Patch>>> {
        debug!("Matching patch by \"{}\"...", identifier);
        let match_result = match self.find_patch_by_uuid(identifier) {
            Ok(patch) => vec![patch],
            Err(_) => self.find_patch_by_name(identifier)?,
        };

        for patch in &match_result {
            debug!("Matched \"{}\"", patch)
        }
        debug!("Matched {} patch(es)", match_result.len());

        Ok(match_result)
    }

    pub fn get_patch_list(&self) -> Vec<Arc<Patch>> {
        self.entry_map
            .values()
            .map(|entry| entry.patch.clone())
            .collect::<Vec<_>>()
    }

    pub fn get_patch_status(&mut self, patch: &Patch) -> Result<PatchStatus> {
        let mut status = self
            .entry_map
            .get(&patch.uuid)
            .with_context(|| format!("Cannot find patch \"{}\"", patch))?
            .status;

        if status == PatchStatus::Unknown {
            status = self.driver_get_patch_status(patch)?;
            self.set_patch_status(patch, status)
                .with_context(|| format!("Failed to set patch \"{}\" status", patch))?;
        }

        Ok(status)
    }

    pub fn apply_patch(&mut self, patch: &Patch) -> Result<PatchStatus> {
        info!("Apply patch \"{}\"", patch);
        self.do_status_transition(patch, PatchStatus::Actived)
    }

    pub fn remove_patch(&mut self, patch: &Patch) -> Result<PatchStatus> {
        info!("Remove patch \"{}\"", patch);
        self.do_status_transition(patch, PatchStatus::NotApplied)
    }

    pub fn active_patch(&mut self, patch: &Patch) -> Result<PatchStatus> {
        info!("Active patch \"{}\"", patch);
        let current_status = self.get_patch_status(patch)?;
        let target_status = PatchStatus::Actived;

        if current_status == PatchStatus::NotApplied {
            bail!("Patch \"{}\" is not applied", patch);
        }
        self.do_status_transition(patch, target_status)
    }

    pub fn deactive_patch(&mut self, patch: &Patch) -> Result<PatchStatus> {
        info!("Deactive patch \"{}\"", patch);
        let current_status = self.get_patch_status(patch)?;
        let target_status = PatchStatus::Deactived;

        if current_status == PatchStatus::NotApplied {
            bail!("Patch \"{}\" is not applied", patch);
        }
        self.do_status_transition(patch, target_status)
    }

    pub fn accept_patch(&mut self, patch: &Patch) -> Result<PatchStatus> {
        info!("Accept patch \"{}\"", patch);
        let current_status = self.get_patch_status(patch)?;
        let target_status = PatchStatus::Accepted;

        if current_status != PatchStatus::Actived {
            bail!("Patch \"{}\" is not actived", patch);
        }
        self.do_status_transition(patch, target_status)
    }

    pub fn save_patch_status(&mut self) -> Result<()> {
        info!("Saving all patch status...");

        debug!("Updating all patch status...");
        for patch in self.get_patch_list() {
            debug!("Update patch \"{}\" status", patch);
            self.get_patch_status(&patch)?;
        }

        let mut status_map = HashMap::new();
        for (uuid, entry) in self.entry_map.iter() {
            status_map.insert(uuid, entry.status);
        }

        debug!("Writing patch status file");
        serde::serialize(&status_map, &self.patch_status_file)
            .context("Failed to write patch status file")?;

        fs::sync();
        info!("All patch status were saved");
        Ok(())
    }

    pub fn restore_patch_status(&mut self, accepted_only: bool) -> Result<()> {
        info!("Restoring all patch status...");

        debug!("Reading patch status...");
        let status_file = &self.patch_status_file;
        let status_map: HashMap<String, PatchStatus> = match status_file.exists() {
            true => serde::deserialize(status_file).context("Failed to read patch status")?,
            false => {
                warn!("Cannot find patch status file");
                return Ok(());
            }
        };

        /*
         * To ensure that we won't load multiple patches for same target at the same time,
         * we take a sort operation of the status to make sure do REMOVE operation at first
         */
        let mut restore_list = status_map
            .into_iter()
            .filter_map(|(uuid, status)| match self.find_patch_by_uuid(&uuid) {
                Ok(patch) => {
                    if accepted_only && (status != PatchStatus::Accepted) {
                        debug!(
                            "Skipped patch \"{}\", status is not \"{}\"",
                            patch,
                            PatchStatus::Accepted
                        );
                        return None;
                    }
                    Some((patch, status))
                }
                Err(e) => {
                    error!("{:?}", e);
                    None
                }
            })
            .collect::<Vec<_>>();

        restore_list.sort_by(|(lhs_patch, lhs_status), (rhs_patch, rhs_status)| {
            match lhs_status.cmp(rhs_status) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => lhs_patch.cmp(rhs_patch),
                Ordering::Greater => Ordering::Greater,
            }
        });

        for (patch, target_status) in restore_list {
            debug!(
                "Restore patch \"{}\" status to \"{}\"",
                patch, target_status
            );
            if let Err(e) = self.do_status_transition(&patch, target_status) {
                error!("{}", e);
            }
        }
        info!("All patch status were restored");

        Ok(())
    }

    pub fn rescan_patches(&mut self) -> Result<()> {
        let entry_map = &mut self.entry_map;
        let new_patch_list = Self::scan_patches(&self.patch_install_dir)?;

        // Delete already removed patch
        for uuid in entry_map.keys().cloned().collect::<Vec<_>>() {
            if !new_patch_list.contains_key(&uuid) {
                trace!("Remove patch {{{}}} from patch manager", uuid);
                entry_map.remove(&uuid);
            }
        }
        // Insert new installed patch
        for (uuid, entry) in new_patch_list {
            if !entry_map.contains_key(&uuid) {
                trace!("Insert patch {{{}}} from patch manager", uuid);
                entry_map.insert(uuid, entry);
            }
        }
        // Sort patches by its entity name
        entry_map.sort_by(|_, lhs_entry, _, rhs_entry| {
            lhs_entry
                .patch
                .entity_name
                .cmp(&rhs_entry.patch.entity_name)
        });

        Ok(())
    }

    pub(super) fn do_status_transition(
        &mut self,
        patch: &Patch,
        status: PatchStatus,
    ) -> Result<PatchStatus> {
        let current_status = self.get_patch_status(patch)?;
        let target_status = status;
        if current_status == target_status {
            debug!(
                "Patch \"{}\": Current status is already \"{}\"",
                patch, target_status,
            );
            return Ok(target_status);
        }

        match TRANSITION_MAP.get(&(current_status, target_status)) {
            Some(action_list) => {
                debug!(
                    "Patch \"{}\": Switching status from \"{}\" to \"{}\"",
                    patch, current_status, status
                );
                for action in action_list {
                    action(self, patch)?;
                }
            }
            None => {
                warn!(
                    "Patch \"{}\": Ignored invalid status transition from \"{}\" to \"{}\"",
                    patch, current_status, status
                );
            }
        }

        let new_status = self.get_patch_status(patch)?;
        if new_status != status {
            bail!("Patch \"{}\" does not reached \"{}\" status", patch, status);
        }

        Ok(new_status)
    }
}

impl PatchManager {
    fn scan_patches<P: AsRef<Path>>(directory: P) -> Result<IndexMap<String, PatchEntry>> {
        const TRAVERSE_OPTION: fs::TraverseOptions = fs::TraverseOptions { recursive: false };

        let mut patch_map = IndexMap::new();

        info!(
            "Scanning patches from \"{}\"...",
            directory.as_ref().display()
        );
        for patch_root in fs::list_dirs(directory, TRAVERSE_OPTION)? {
            let read_result = Patch::read_from(&patch_root).with_context(|| {
                format!(
                    "Failed to load patch metadata from \"{}\"",
                    patch_root.display()
                )
            });
            match read_result {
                Ok(patches) => {
                    for patch in patches {
                        debug!("Detected patch \"{}\"", patch);
                        patch_map.insert(
                            patch.uuid.clone(),
                            PatchEntry {
                                patch: Arc::new(patch),
                                status: PatchStatus::Unknown,
                            },
                        );
                    }
                }
                Err(e) => error!("{:?}", e),
            }
        }
        info!("Found {} patch(es)", patch_map.len());

        Ok(patch_map)
    }

    fn find_patch_by_uuid(&self, uuid: &str) -> Result<Arc<Patch>> {
        self.entry_map
            .get(uuid)
            .map(|entry| entry.patch.clone())
            .with_context(|| format!("Cannot find patch by uuid {{{}}}", uuid))
    }

    fn find_patch_by_name(&self, identifier: &str) -> Result<Vec<Arc<Patch>>> {
        let match_result = self
            .entry_map
            .values()
            .filter_map(|entry| {
                let patch = &entry.patch;
                let is_matched = (identifier == patch.entity_name)
                    || (identifier == patch.patch_name)
                    || (identifier == patch.target_name);
                match is_matched {
                    true => Some(patch.clone()),
                    false => None,
                }
            })
            .collect::<Vec<_>>();

        if match_result.is_empty() {
            bail!("Cannot match any patch named \"{}\"", identifier);
        }
        Ok(match_result)
    }

    fn set_patch_status(&mut self, patch: &Patch, value: PatchStatus) -> Result<()> {
        if value == PatchStatus::Unknown {
            bail!("Cannot set patch {} status to {}", patch, value);
        }
        self.entry_map
            .get_mut(&patch.uuid)
            .with_context(|| format!("Cannot find patch \"{}\"", patch))?
            .status = value;

        Ok(())
    }
}

impl PatchManager {
    fn call_driver<T, U>(&self, patch: &Patch, driver_action: T) -> Result<U>
    where
        T: FnOnce(&'static dyn PatchDriver, &Patch) -> Result<U>,
    {
        let patch_type = patch.kind();
        let driver = DRIVER_MAP
            .get(&patch_type)
            .map(Box::deref)
            .with_context(|| format!("Failed to get driver of {}", patch_type))?;

        driver_action(driver, patch)
    }

    fn driver_get_patch_status(&self, patch: &Patch) -> Result<PatchStatus> {
        self.call_driver(patch, PatchDriver::status)
            .with_context(|| format!("Driver: Failed to get patch \"{}\" status", patch))
    }

    fn driver_apply_patch(&mut self, patch: &Patch) -> Result<()> {
        self.call_driver(patch, PatchDriver::check)
            .with_context(|| format!("Driver: Patch \"{}\" check failed", patch))?;

        self.call_driver(patch, PatchDriver::apply)
            .with_context(|| format!("Driver: Failed to apply patch \"{}\"", patch))?;

        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn driver_remove_patch(&mut self, patch: &Patch) -> Result<()> {
        self.call_driver(patch, PatchDriver::remove)
            .with_context(|| format!("Driver: Failed to remove patch \"{}\"", patch))?;

        self.set_patch_status(patch, PatchStatus::NotApplied)
    }

    fn driver_active_patch(&mut self, patch: &Patch) -> Result<()> {
        self.call_driver(patch, PatchDriver::active)
            .with_context(|| format!("Driver: Failed to active patch \"{}\"", patch))?;

        self.set_patch_status(patch, PatchStatus::Actived)
    }

    fn driver_deactive_patch(&mut self, patch: &Patch) -> Result<()> {
        self.call_driver(patch, PatchDriver::deactive)
            .with_context(|| format!("Driver: Failed to deactive patch \"{}\"", patch))?;

        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn do_patch_accept(&mut self, patch: &Patch) -> Result<()> {
        self.set_patch_status(patch, PatchStatus::Accepted)
    }

    fn do_patch_decline(&mut self, patch: &Patch) -> Result<()> {
        self.set_patch_status(patch, PatchStatus::Actived)
    }
}

impl Drop for PatchManager {
    fn drop(&mut self) {
        self.finallize()
    }
}
