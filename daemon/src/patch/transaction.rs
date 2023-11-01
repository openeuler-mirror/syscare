use std::sync::Arc;

use anyhow::{Context, Result};
use log::{debug, info};

use parking_lot::RwLock;
use syscare_abi::{PatchStateRecord, PatchStatus};

use super::manager::{Patch, PatchManager, PatchOpFlag};

type TransationRecord = (Arc<Patch>, PatchStatus);

pub struct PatchTransaction<F> {
    name: String,
    patch_manager: Arc<RwLock<PatchManager>>,
    action: F,
    identifier: String,
    flag: PatchOpFlag,
    finish_list: Vec<TransationRecord>,
}

impl<F> PatchTransaction<F>
where
    F: Fn(&mut PatchManager, Arc<Patch>, PatchOpFlag) -> Result<PatchStatus>,
{
    pub fn new(
        name: String,
        patch_manager: Arc<RwLock<PatchManager>>,
        action: F,
        flag: PatchOpFlag,
        identifier: String,
    ) -> Result<Self> {
        let instance = Self {
            name,
            patch_manager,
            action,
            identifier,
            flag,
            finish_list: Vec::new(),
        };

        debug!("{} is created", instance);
        Ok(instance)
    }
}

impl<F> PatchTransaction<F>
where
    F: Fn(&mut PatchManager, Arc<Patch>, PatchOpFlag) -> Result<PatchStatus>,
{
    fn start(&mut self) -> Result<Vec<PatchStateRecord>> {
        let mut patch_manager = self.patch_manager.write();

        let mut patch_list = patch_manager.match_patch(&self.identifier)?;
        let mut records = Vec::with_capacity(patch_list.len());

        while let Some(patch) = patch_list.pop() {
            let old_status = patch_manager.get_patch_status(patch.clone())?;
            let new_status = (self.action)(&mut patch_manager, patch.clone(), self.flag)?;

            records.push(PatchStateRecord {
                name: patch.to_string(),
                status: new_status,
            });
            self.finish_list.push((patch, old_status));
        }
        Ok(records)
    }

    fn rollback(&mut self) -> Result<()> {
        let mut patch_manager = self.patch_manager.write();
        while let Some((patch, status)) = self.finish_list.pop() {
            patch_manager.do_status_transition(patch, status, PatchOpFlag::SkipCheck)?;
        }
        Ok(())
    }

    pub fn invoke(mut self) -> Result<Vec<PatchStateRecord>> {
        info!("{} started...", self);
        match self.start() {
            Ok(result) => {
                info!("{} finished", self);
                Ok(result)
            }
            Err(e) => {
                if !self.finish_list.is_empty() {
                    debug!("{} rolling back...", self);
                    self.rollback()
                        .with_context(|| format!("{} rollback failed", self))?;
                    debug!("{} rolled back", self);
                }
                Err(e.context(format!("{} failed", self)))
            }
        }
    }
}

impl<F> std::fmt::Display for PatchTransaction<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Transaction \"{}\"", self.name))
    }
}
