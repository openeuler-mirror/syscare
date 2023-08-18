use std::path::Path;

use anyhow::{Context, Result};
use lazy_static::lazy_static;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

mod manager;
mod skeleton;
mod transaction;

use manager::{Patch, PatchManager};
pub use skeleton::PatchSkeleton;
use transaction::PatchTransaction;

use crate::rpc::{RpcFunction, RpcResult};

lazy_static! {
    static ref PATCH_MANAGER: PatchManager = PatchManager::new();
}

pub struct PatchSkeletonImpl;

impl PatchSkeletonImpl {
    pub fn initialize<P: AsRef<Path>>(patch_root: P) -> Result<()> {
        PATCH_MANAGER.initialize(patch_root)
    }
}

impl PatchSkeletonImpl {
    fn normalize_identifier(identifier: &mut String) {
        while identifier.ends_with('/') {
            identifier.pop();
        }
    }

    fn parse_state_record(patch: &Patch) -> PatchStateRecord {
        let patch_name = patch.to_string();
        let patch_status = PATCH_MANAGER.get_patch_status(patch).unwrap_or_default();

        PatchStateRecord {
            name: patch_name,
            status: patch_status,
        }
    }

    fn parse_list_record(patch: &Patch) -> PatchListRecord {
        let patch_uuid = patch.uuid.to_owned();
        let patch_name = patch.to_string();
        let patch_status = PATCH_MANAGER.get_patch_status(patch).unwrap_or_default();

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
                &PATCH_MANAGER,
                PatchManager::apply_patch,
                PATCH_MANAGER.match_patch(identifier)?,
            )
            .invoke()
        })
    }

    fn remove_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Remove patch '{}'", identifier),
                &PATCH_MANAGER,
                PatchManager::remove_patch,
                PATCH_MANAGER.match_patch(identifier)?,
            )
            .invoke()
        })
    }

    fn active_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Active patch '{}'", identifier),
                &PATCH_MANAGER,
                PatchManager::active_patch,
                PATCH_MANAGER.match_patch(identifier)?,
            )
            .invoke()
        })
    }

    fn deactive_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Deactive patch '{}'", identifier),
                &PATCH_MANAGER,
                PatchManager::deactive_patch,
                PATCH_MANAGER.match_patch(identifier)?,
            )
            .invoke()
        })
    }

    fn accept_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Accept patch '{}'", identifier),
                &PATCH_MANAGER,
                PatchManager::accept_patch,
                PATCH_MANAGER.match_patch(identifier)?,
            )
            .invoke()
        })
    }

    fn get_patch_list(&self) -> RpcResult<Vec<PatchListRecord>> {
        RpcFunction::call(move || -> Result<Vec<PatchListRecord>> {
            let mut result = Vec::new();
            for patch in PATCH_MANAGER.get_patch_list() {
                result.push(Self::parse_list_record(&patch));
            }
            Ok(result)
        })
    }

    fn get_patch_status(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            let mut result = Vec::new();
            for patch in PATCH_MANAGER.match_patch(identifier)? {
                result.push(Self::parse_state_record(&patch));
            }
            Ok(result)
        })
    }

    fn get_patch_info(&self, mut identifier: String) -> RpcResult<PatchInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PatchInfo> {
            let patch_list = PATCH_MANAGER.match_patch(identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info.as_ref().clone())
        })
    }

    fn get_patch_target(&self, mut identifier: String) -> RpcResult<PackageInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PackageInfo> {
            let patch_list = PATCH_MANAGER.match_patch(identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info.target.clone())
        })
    }

    fn save_patch_status(&self) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            PATCH_MANAGER
                .save_patch_status()
                .context("Failed to save patch status")
        })
    }

    fn restore_patch_status(&self, accepted_only: bool) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            PATCH_MANAGER
                .restore_patch_status(accepted_only)
                .context("Failed to restore patch status")
        })
    }
}
