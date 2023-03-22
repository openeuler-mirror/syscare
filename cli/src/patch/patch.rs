use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::{debug, error};
use lazy_static::lazy_static;

use common::util::serde::serde_versioned;

use super::patch_info::{PatchInfo, PatchType};
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;
use super::user_patch::UserPatchAdapter;
use super::kernel_patch::KernelPatchAdapter;

pub struct Patch {
    pub info:     PatchInfo,
    pub root_dir: PathBuf,
}

impl Patch {
    pub fn new<P: AsRef<Path>>(patch_root: P) -> std::io::Result<Self> {
        const PATCH_INFO_FILE_NAME: &str = "patch_info";

        let info = serde_versioned::deserialize::<_, PatchInfo>(
            patch_root.as_ref().join(PATCH_INFO_FILE_NAME),
            PatchInfo::version()
        )?;
        let root_dir = patch_root.as_ref().to_path_buf();

        Ok(Self { info, root_dir })
    }

    pub fn short_name(&self) -> String {
        self.name.to_owned()
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.target.short_name(), self.short_name())
    }

    pub fn status(&self) -> std::io::Result<PatchStatus> {
        self.adapter().status()
    }
}

impl Patch {
    fn adapter(&self) -> Box<dyn PatchActionAdapter + '_> {
        match &self.kind {
            PatchType::UserPatch   => Box::new(UserPatchAdapter::new(self)),
            PatchType::KernelPatch => Box::new(KernelPatchAdapter::new(self)),
        }
    }

    fn do_apply(&self) -> std::io::Result<()> {
        debug!("Applying patch {{{}}}", self);

        self.adapter().check_compatibility().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch {{{}}} check failed", self)
            )
        })?;

        self.adapter().apply().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch {{{}}} apply failed", self)
            )
        })
    }

    fn do_remove(&self) -> std::io::Result<()> {
        debug!("Removing patch {{{}}}", self);

        self.adapter().remove().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch {{{}}} remove failed", self)
            )
        })
    }

    fn do_active(&self) -> std::io::Result<()> {
        debug!("Activing patch {{{}}}", self);

        self.adapter().active().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch {{{}}} active failed", self)
            )
        })
    }

    fn do_deactive(&self) -> std::io::Result<()> {
        debug!("Deactiving patch {{{}}}", self);

        self.adapter().deactive().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(
                e.kind(),
                format!("Patch {{{}}} deactive failed", self)
            )
        })
    }

    fn do_transition(&self, current_status: PatchStatus, target_status: PatchStatus) -> std::io::Result<()> {
        type Transition       = (PatchStatus, PatchStatus);
        type TransitionAction = dyn Fn(&Patch) -> std::io::Result<()> + Sync;

        const PATCH_APPLY:    &TransitionAction = &Patch::do_apply;
        const PATCH_REMOVE:   &TransitionAction = &Patch::do_remove;
        const PATCH_ACTIVE:   &TransitionAction = &Patch::do_active;
        const PATCH_DEACTIVE: &TransitionAction = &Patch::do_deactive;

        lazy_static! {
            static ref PATCH_TRANSITION_MAP: HashMap<Transition, Vec<&'static TransitionAction>> = [
                ( (PatchStatus::NotApplied, PatchStatus::Actived   ), vec![PATCH_APPLY,    PATCH_ACTIVE] ),
                ( (PatchStatus::Actived,    PatchStatus::NotApplied), vec![PATCH_DEACTIVE, PATCH_REMOVE] ),
                ( (PatchStatus::NotApplied, PatchStatus::Deactived ), vec![PATCH_APPLY]                  ),
                ( (PatchStatus::Deactived,  PatchStatus::NotApplied), vec![PATCH_REMOVE]                 ),
                ( (PatchStatus::Deactived,  PatchStatus::Actived   ), vec![PATCH_ACTIVE]                 ),
                ( (PatchStatus::Actived,    PatchStatus::Deactived ), vec![PATCH_DEACTIVE]               ),
            ].into_iter().collect();
        }

        debug!("Switching patch {{{}}} status from {} to {}", self, current_status, target_status);
        match PATCH_TRANSITION_MAP.get(&(current_status, target_status)) {
            Some(action_list) => {
                for action in action_list {
                    action(self)?;
                }
            },
            None => {
                debug!("Patch {{{}}} status does not change", self);
            }
        }

        Ok(())
    }
}

impl Patch {
    pub fn apply(&self) -> std::io::Result<()> {
        let current_status = self.status()?;
        let target_status  = match current_status {
            PatchStatus::Unknown    => unreachable!(),
            PatchStatus::NotApplied => PatchStatus::Actived,
            PatchStatus::Deactived  => PatchStatus::Deactived,
            PatchStatus::Actived    => PatchStatus::Actived,
        };
        self.do_transition(current_status, target_status)
    }

    pub fn remove(&self) -> std::io::Result<()> {
        self.do_transition(self.status()?, PatchStatus::NotApplied)
    }

    pub fn active(&self) -> std::io::Result<()> {
        let current_status = self.status()?;
        if current_status == PatchStatus::NotApplied {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Patch {{{}}} is not applied", self)
            ));
        }
        self.do_transition(current_status, PatchStatus::Actived)
    }

    pub fn deactive(&self) -> std::io::Result<()> {
        let status = self.status()?;
        if status == PatchStatus::NotApplied {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Patch {{{}}} is not applied", self)
            ));
        }
        self.do_transition(status, PatchStatus::Deactived)
    }

    pub fn restore(&self, target_status: PatchStatus) -> std::io::Result<()> {
        let current_status = self.status()?;

        debug!("Restoring patch {{{}}} status to {}", self, target_status);
        self.do_transition(current_status, target_status)
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
        f.write_str(&self.uuid)
    }
}
