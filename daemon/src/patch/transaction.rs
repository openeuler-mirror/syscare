use std::sync::Arc;

use anyhow::{Context, Result};
use log::{debug, info};

use syscare_abi::{PatchStateRecord, PatchStatus};

use super::manager::{Patch, PatchManager};

type TransationRecord = (Arc<Patch>, PatchStatus);

pub struct PatchTransaction<'a, F> {
    name: String,
    manager: &'a PatchManager,
    action: F,
    patch_list: Vec<Arc<Patch>>,
    finish_list: Vec<TransationRecord>,
}

impl<'a, F> PatchTransaction<'a, F>
where
    F: Fn(&PatchManager, &Patch) -> Result<PatchStatus>,
{
    pub fn new(
        name: String,
        manager: &'a PatchManager,
        action: F,
        patch_list: Vec<Arc<Patch>>,
    ) -> Self {
        let patch_num = patch_list.len();
        let instance = Self {
            name,
            manager,
            action,
            patch_list,
            finish_list: Vec::with_capacity(patch_num),
        };

        debug!("{} is created", instance);
        instance
    }
}

impl<F> PatchTransaction<'_, F>
where
    F: Fn(&PatchManager, &Patch) -> Result<PatchStatus>,
{
    fn start(&mut self) -> Result<Vec<PatchStateRecord>> {
        let mut records = Vec::with_capacity(self.patch_list.len());
        while let Some(patch) = self.patch_list.pop() {
            let old_status = self.manager.get_patch_status(&patch)?;
            let new_status = (self.action)(self.manager, &patch)?;

            records.push(PatchStateRecord {
                name: patch.to_string(),
                status: new_status,
            });
            self.finish_list.push((patch, old_status));
        }
        Ok(records)
    }

    fn rollback(&mut self) -> Result<()> {
        while let Some((patch, status)) = self.finish_list.pop() {
            self.manager.do_status_transition(&patch, status)?;
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

impl<F> std::fmt::Display for PatchTransaction<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Transaction \"{}\"", self.name))
    }
}
