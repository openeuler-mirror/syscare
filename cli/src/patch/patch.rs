use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

use log::{debug, error};
use lazy_static::lazy_static;

use crate::util::serde::serde_versioned;

use super::patch_info::{PatchInfo, PatchType};
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;
use super::user_patch::UserPatchAdapter;
use super::kernel_patch::KernelPatchAdapter;

pub struct Patch {
    pub info:     PatchInfo,
    pub root_dir: PathBuf,
    status:       AtomicU8,
}

impl Patch {
    pub fn new<P: AsRef<Path>>(path_root: P) -> std::io::Result<Self> {
        const PATCH_INFO_FILE_NAME: &str = "patch_info";

        let info     = serde_versioned::deserialize::<_, PatchInfo>(path_root.as_ref().join(PATCH_INFO_FILE_NAME))?;
        let root_dir = path_root.as_ref().to_path_buf();
        let status   = AtomicU8::new(PatchStatus::default() as u8);

        Ok(Self { info, status, root_dir })
    }

    pub fn short_name(&self) -> String {
        self.name.to_owned()
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.target.short_name(), self.short_name())
    }

    pub fn status(&self) -> std::io::Result<PatchStatus> {
        let status = PatchStatus::from(self.status.load(Ordering::Relaxed));
        if status == PatchStatus::Unknown {
            let new_status = self.fetch_status()?;

            self.update_status(new_status);
            return Ok(new_status);
        }
        Ok(status)
    }
}

impl Patch {
    fn adapter(&self) -> Box<dyn PatchActionAdapter + '_> {
        match &self.kind {
            PatchType::UserPatch   => Box::new(UserPatchAdapter::new(self)),
            PatchType::KernelPatch => Box::new(KernelPatchAdapter::new(self)),
        }
    }

    fn fetch_status(&self) -> std::io::Result<PatchStatus> {
        debug!("Updating patch \"{}\" status", self);
        self.adapter().status()
    }

    fn update_status(&self, status: PatchStatus) {
        let old_status = PatchStatus::from(self.status.load(Ordering::Relaxed));
        if old_status == status {
            return;
        }

        debug!("Patch \"{}\" status changed from \"{}\" to \"{}\"", self, old_status, status);
        self.status.store(status as u8, Ordering::Relaxed);
    }

    fn do_apply(&mut self) -> std::io::Result<()> {
        debug!("Applying patch \"{}\"", self);

        self.adapter().check_compatibility().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" check failed", self)
            )
        })?;

        self.adapter().apply().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" apply failed", self)
            )
        })?;

        self.update_status(PatchStatus::Deactived);
        Ok(())
    }

    fn do_remove(&mut self) -> std::io::Result<()> {
        debug!("Removing patch \"{}\"", self);

        self.adapter().remove().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" remove failed", self)
            )
        })?;

        self.update_status(PatchStatus::NotApplied);
        Ok(())
    }

    fn do_active(&mut self) -> std::io::Result<()> {
        debug!("Activing patch \"{}\"", self);

        self.adapter().active().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" active failed", self)
            )
        })?;

        self.update_status(PatchStatus::Actived);
        Ok(())
    }

    fn do_deactive(&mut self) -> std::io::Result<()> {
        debug!("Deactiving patch \"{}\"", self);

        self.adapter().deactive().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" deactive failed", self)
            )
        })?;

        self.update_status(PatchStatus::Deactived);
        Ok(())
    }
}

impl Patch {
    pub fn apply(&mut self) -> std::io::Result<()> {
        match self.status()? {
            PatchStatus::NotApplied => {
                self.do_apply()?;
                self.do_active()?;
            },
            PatchStatus::Deactived | PatchStatus::Actived => {
                debug!("Patch \"{}\" is already applied", self);
            },
            _ => unreachable!("Patch status is unknown")
        }

        Ok(())
    }

    pub fn remove(&mut self) -> std::io::Result<()> {
        match self.status()? {
            PatchStatus::NotApplied => {
                debug!("Patch \"{}\" is already removed", self);
            },
            PatchStatus::Deactived => {
                self.do_remove()?;
            },
            PatchStatus::Actived => {
                self.do_deactive()?;
                self.do_remove()?;
            },
            _ => unreachable!("Patch status is unknown")
        }

        Ok(())
    }

    pub fn active(&mut self) -> std::io::Result<()> {
        match self.status()? {
            PatchStatus::NotApplied => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Patch \"{}\" is not applied", self)
                ));
            },
            PatchStatus::Deactived => {
                self.do_active()?;
            },
            PatchStatus::Actived => {
                debug!("Patch \"{}\" is already actived", self);
            },
            _ => unreachable!("Patch status is unknown")
        }

        Ok(())
    }

    pub fn deactive(&mut self) -> std::io::Result<()> {
        match self.status()? {
            PatchStatus::NotApplied => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Patch \"{}\" is not applied", self)
                ));
            },
            PatchStatus::Deactived => {
                debug!("Patch \"{}\" is already deactived", self);
            },
            PatchStatus::Actived => {
                self.do_deactive()?;
            },
            _ => unreachable!("Patch status is unknown")
        }

        Ok(())
    }

    pub fn restore(&mut self, status: PatchStatus) -> std::io::Result<()> {
        type Transition  = (PatchStatus, PatchStatus);
        type TransitionAction = dyn Fn(&mut Patch) -> std::io::Result<()> + Sync;

        const PATCH_APPLY:         &TransitionAction = &Patch::apply;
        const PATCH_REMOVE:        &TransitionAction = &Patch::remove;
        const PATCH_APPLY_ONLY:    &TransitionAction = &Patch::do_apply;
        const PATCH_REMOVE_ONLY:   &TransitionAction = &Patch::do_remove;
        const PATCH_ACTIVE_ONLY:   &TransitionAction = &Patch::do_active;
        const PATCH_DEACTIVE_ONLY: &TransitionAction = &Patch::do_deactive;

        lazy_static! {
            static ref PATCH_TRANSITION_MAP: HashMap<Transition, &'static TransitionAction> = [
                ( (PatchStatus::NotApplied, PatchStatus::Actived   ), PATCH_APPLY         ),
                ( (PatchStatus::Actived,    PatchStatus::NotApplied), PATCH_REMOVE        ),
                ( (PatchStatus::NotApplied, PatchStatus::Deactived ), PATCH_APPLY_ONLY    ),
                ( (PatchStatus::Deactived,  PatchStatus::NotApplied), PATCH_REMOVE_ONLY   ),
                ( (PatchStatus::Deactived,  PatchStatus::Actived   ), PATCH_ACTIVE_ONLY   ),
                ( (PatchStatus::Actived,    PatchStatus::Deactived ), PATCH_DEACTIVE_ONLY ),
            ].into_iter().collect();
        }

        let transition = (self.status()?, status);
        debug!("Restoring patch \"{}\" status from \"{}\" to \"{}\"", self, transition.0, transition.1);

        match PATCH_TRANSITION_MAP.get(&transition) {
            Some(action) => {
                action(self)?
            },
            None => debug!("Patch \"{}\" status not change", self),
        }

        Ok(())
    }
}

impl AsRef<Patch> for Patch {
    fn as_ref(&self) -> &Patch {
        self
    }
}

impl std::ops::Deref for Patch {
    type Target = PatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl std::fmt::Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.full_name())
    }
}
