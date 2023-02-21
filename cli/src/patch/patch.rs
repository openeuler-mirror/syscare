use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::debug;
use lazy_static::lazy_static;

use crate::util::serde;

use super::patch_info::{PatchInfo, PatchType};
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;
use super::user_patch::UserPatchAdapter;
use super::kernel_patch::KernelPatchAdapter;

pub struct Patch {
    pub info:     PatchInfo,
    pub status:   PatchStatus,
    pub root_dir: PathBuf,
}

impl Patch {
    pub fn new<P: AsRef<Path>>(path_root: P) -> std::io::Result<Self> {
        const PATCH_INFO_FILE_NAME: &str = "patch_info";

        let info     = serde::deserialize::<_, PatchInfo>(path_root.as_ref().join(PATCH_INFO_FILE_NAME))?;
        let status   = PatchStatus::NotApplied;
        let root_dir = path_root.as_ref().to_path_buf();

        Ok(Self { info, status, root_dir })
    }

    pub fn short_name(&self) -> String {
        self.name.to_owned()
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.target.short_name(), self.short_name())
    }
}

impl Patch {
    fn get_adapter(&self) -> Box<dyn PatchActionAdapter + '_> {
        match &self.kind {
            PatchType::UserPatch   => Box::new(UserPatchAdapter::new(self)),
            PatchType::KernelPatch => Box::new(KernelPatchAdapter::new(self)),
        }
    }

    fn set_status(&mut self, status: PatchStatus) {
        let current_status = self.status;
        let target_tatus   = status;

        if current_status == target_tatus {
            return;
        }

        debug!("Patch \"{}\" status changed from \"{}\" to \"{}\"", self, current_status, target_tatus);
        self.status = target_tatus;
    }

    fn do_apply(&mut self) -> std::io::Result<()> {
        debug!("Applying patch \"{}\"", self);

        self.get_adapter().check_compatibility().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Check patch \"{}\" failed, {}", self, e)
            )
        })?;

        self.get_adapter().apply().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" apply failed, {}", self, e)
            )
        })?;

        self.set_status(PatchStatus::Deactived);
        Ok(())
    }

    fn do_remove(&mut self) -> std::io::Result<()> {
        debug!("Removing patch \"{}\"", self);

        self.get_adapter().remove().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" remove failed, {}", self, e)
            )
        })?;

        self.set_status(PatchStatus::NotApplied);
        Ok(())
    }

    fn do_active(&mut self) -> std::io::Result<()> {
        debug!("Activing patch \"{}\"", self);

        self.get_adapter().active().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" active failed, {}", self, e)
            )
        })?;

        self.set_status(PatchStatus::Actived);
        Ok(())
    }

    fn do_deactive(&mut self) -> std::io::Result<()> {
        debug!("Deactiving patch \"{}\"", self);

        self.get_adapter().deactive().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Patch \"{}\" deactive failed, {}", self, e)
            )
        })?;

        self.set_status(PatchStatus::Deactived);
        Ok(())
    }
}

impl Patch {
    pub fn update_status(&mut self) -> std::io::Result<()> {
        let patch_status = self.get_adapter().status()?;

        self.status = patch_status;
        debug!("Patch \"{}\" is \"{}\"", self, patch_status);

        Ok(())
    }

    pub fn apply(&mut self) -> std::io::Result<()> {
        match &self.status {
            PatchStatus::NotApplied => {
                self.do_apply()?;
                self.do_active()?;
            },
            _ => {
                debug!("Patch \"{}\" is already applied", self);
            },
        }

        Ok(())
    }

    pub fn remove(&mut self) -> std::io::Result<()> {
        match &self.status {
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
        }

        Ok(())
    }

    pub fn active(&mut self) -> std::io::Result<()> {
        match &self.status {
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
        }

        Ok(())
    }

    pub fn deactive(&mut self) -> std::io::Result<()> {
        match &self.status {
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

        let transition = (self.status, status);
        debug!("Restoring patch \"{}\" status from \"{}\" to \"{}\"", self, transition.0, transition.1);

        match PATCH_TRANSITION_MAP.get(&transition) {
            Some(action) => action(self)?,
            None         => debug!("Patch \"{}\" status not change", self),
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
