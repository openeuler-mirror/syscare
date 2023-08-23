use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};

use parking_lot::RwLock;
use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

mod manager;
mod monitor;
mod skeleton;
mod transaction;

use manager::{Patch, PatchManager};
pub use skeleton::PatchSkeleton;
use transaction::PatchTransaction;

use crate::rpc::{RpcFunction, RpcResult};

use self::monitor::PatchMonitor;

pub struct PatchSkeletonImpl {
    patch_manager: Arc<RwLock<PatchManager>>,
    _patch_monitor: PatchMonitor,
}

impl PatchSkeletonImpl {
    pub fn new<P: AsRef<Path>>(patch_root: P) -> Result<Self> {
        let patch_manager = Arc::new(RwLock::new(PatchManager::new(patch_root)?));
        let patch_monitor = PatchMonitor::new(patch_manager.clone())?;

        Ok(Self {
            patch_manager,
            _patch_monitor: patch_monitor,
        })
    }
}

impl PatchSkeletonImpl {
    fn normalize_identifier(identifier: &mut String) {
        while identifier.ends_with('/') {
            identifier.pop();
        }
    }

    fn parse_state_record(&self, patch: &Patch) -> PatchStateRecord {
        let patch_name = patch.to_string();
        let patch_status = self
            .patch_manager
            .write()
            .get_patch_status(patch)
            .unwrap_or_default();

        PatchStateRecord {
            name: patch_name,
            status: patch_status,
        }
    }

    fn parse_list_record(&self, patch: &Patch) -> PatchListRecord {
        let patch_uuid = patch.uuid.to_owned();
        let patch_name = patch.to_string();
        let patch_status = self
            .patch_manager
            .write()
            .get_patch_status(patch)
            .unwrap_or_default();

        PatchListRecord {
            uuid: patch_uuid,
            name: patch_name,
            status: patch_status,
        }
    }
}

impl PatchSkeleton for PatchSkeletonImpl {
    fn apply_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Apply patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::apply_patch,
                identifier,
            )?
            .invoke()
        })
    }

    fn remove_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Remove patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::remove_patch,
                identifier,
            )?
            .invoke()
        })
    }

    fn active_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Active patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::active_patch,
                identifier,
            )?
            .invoke()
        })
    }

    fn deactive_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Deactive patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::deactive_patch,
                identifier,
            )?
            .invoke()
        })
    }

    fn accept_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Accept patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::accept_patch,
                identifier,
            )?
            .invoke()
        })
    }

    fn get_patch_list(&self) -> RpcResult<Vec<PatchListRecord>> {
        RpcFunction::call(move || -> Result<Vec<PatchListRecord>> {
            let patch_list: Vec<Arc<Patch>> = self.patch_manager.read().get_patch_list();

            let mut result = Vec::new();
            for patch in patch_list {
                result.push(self.parse_list_record(&patch));
            }

            Ok(result)
        })
    }

    fn get_patch_status(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            let patch_list = self.patch_manager.read().get_patch_list();

            let mut result = Vec::new();
            for patch in patch_list {
                result.push(self.parse_state_record(&patch));
            }
            Ok(result)
        })
    }

    fn get_patch_info(&self, mut identifier: String) -> RpcResult<PatchInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PatchInfo> {
            let patch_list = self.patch_manager.read().match_patch(&identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info.as_ref().clone())
        })
    }

    fn get_patch_target(&self, mut identifier: String) -> RpcResult<PackageInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PackageInfo> {
            let patch_list = self.patch_manager.read().match_patch(&identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info.target.clone())
        })
    }

    fn save_patch_status(&self) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            self.patch_manager
                .write()
                .save_patch_status()
                .context("Failed to save patch status")
        })
    }

    fn restore_patch_status(&self, accepted_only: bool) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            self.patch_manager
                .write()
                .restore_patch_status(accepted_only)
                .context("Failed to restore patch status")
        })
    }
}
