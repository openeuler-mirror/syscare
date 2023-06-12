use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lazy_static::lazy_static;
use log::{debug, error, info};

use common::util::{fs, serde};

use super::kernel_patch::KernelPatchAdapter;
use super::patch_action::PatchActionAdapter;
use super::patch_info::{PatchInfo, PatchType, PATCH_INFO_MAGIC};
use super::patch_status::PatchStatus;
use super::user_patch::UserPatchAdapter;

pub struct Patch {
    info: Rc<PatchInfo>,
    root_dir: PathBuf,
    accept_flag: PathBuf,
    adapter: Box<dyn PatchActionAdapter>,
}

impl Patch {
    pub fn new<P: AsRef<Path>>(patch_root: P) -> std::io::Result<Self> {
        const PATCH_INFO_NAME: &str = "patch_info";
        const PATCH_ACCEPT_FLAG_NAME: &str = "accept_flag";

        let patch_root = patch_root.as_ref().to_path_buf();
        let patch_info = Rc::new(serde::deserialize_with_magic::<PatchInfo, _, _>(
            patch_root.join(PATCH_INFO_NAME),
            PATCH_INFO_MAGIC,
        )?);

        let patch_accept_flag = patch_root.join(PATCH_ACCEPT_FLAG_NAME);
        let patch_adapter: Box<dyn PatchActionAdapter> = match patch_info.kind {
            PatchType::UserPatch => {
                Box::new(UserPatchAdapter::new(&patch_root, patch_info.clone()))
            }
            PatchType::KernelPatch => {
                Box::new(KernelPatchAdapter::new(&patch_root, patch_info.clone()))
            }
        };

        Ok(Self {
            info: patch_info,
            root_dir: patch_root,
            accept_flag: patch_accept_flag,
            adapter: patch_adapter,
        })
    }

    pub fn short_name(&self) -> String {
        self.name.to_owned()
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.target.short_name(), self.short_name())
    }

    pub fn info(&self) -> &PatchInfo {
        &self.info
    }

    pub fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
    }

    pub fn status(&self) -> std::io::Result<PatchStatus> {
        self.adapter.status().map(|status| {
            if status == PatchStatus::Actived && self.accept_flag.exists() {
                return PatchStatus::Accepted;
            }
            status
        })
    }
}

impl Patch {
    fn do_apply(&self) -> std::io::Result<()> {
        debug!("Applying patch {{{}}}", self);
        self.adapter.check().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} check failed", self))
        })?;

        self.adapter.apply().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} apply failed", self))
        })
    }

    fn do_remove(&self) -> std::io::Result<()> {
        debug!("Removing patch {{{}}}", self);

        self.adapter.remove().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} remove failed", self))
        })
    }

    fn do_active(&self) -> std::io::Result<()> {
        debug!("Activing patch {{{}}}", self);

        self.adapter.active().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} active failed", self))
        })
    }

    fn do_deactive(&self) -> std::io::Result<()> {
        debug!("Deactiving patch {{{}}}", self);

        self.adapter.deactive().map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} deactive failed", self))
        })
    }

    fn do_accept(&self) -> std::io::Result<()> {
        debug!("Accepting patch {{{}}}", self);

        fs::create_file(&self.accept_flag).map(|_| ()).map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} accept failed", self))
        })
    }

    fn do_decline(&self) -> std::io::Result<()> {
        debug!("Declining patch {{{}}}", self);

        fs::remove_file(&self.accept_flag).map_err(|e| {
            error!("{}", e);
            std::io::Error::new(e.kind(), format!("Patch {{{}}} decline failed", self))
        })
    }

    fn do_transition(
        &self,
        current_status: PatchStatus,
        target_status: PatchStatus,
    ) -> std::io::Result<()> {
        type Transition = (PatchStatus, PatchStatus);
        type TransitionAction = dyn Fn(&Patch) -> std::io::Result<()> + Sync;

        const PATCH_APPLY: &TransitionAction = &Patch::do_apply;
        const PATCH_REMOVE: &TransitionAction = &Patch::do_remove;
        const PATCH_ACTIVE: &TransitionAction = &Patch::do_active;
        const PATCH_DEACTIVE: &TransitionAction = &Patch::do_deactive;
        const PATCH_ACCEPT: &TransitionAction = &Patch::do_accept;
        const PATCH_DECLINE: &TransitionAction = &Patch::do_decline;

        lazy_static! {
            static ref PATCH_TRANSITION_MAP: HashMap<Transition, Vec<&'static TransitionAction>> =
                [
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
                ]
                .into_iter()
                .collect();
        }

        match PATCH_TRANSITION_MAP.get(&(current_status, target_status)) {
            Some(action_list) => {
                debug!(
                    "Switching patch {{{}}} status from {} to {}",
                    self, current_status, target_status
                );
                for action in action_list {
                    action(self)?;
                }
            }
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
        if current_status >= PatchStatus::Deactived {
            info!("Patch {{{}}} is already applied", self);
            return Ok(());
        }
        self.do_transition(current_status, PatchStatus::Actived)
    }

    pub fn remove(&self) -> std::io::Result<()> {
        self.do_transition(self.status()?, PatchStatus::NotApplied)
    }

    pub fn active(&self) -> std::io::Result<()> {
        let current_status = self.status()?;
        if current_status < PatchStatus::Deactived {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Patch {{{}}} is not applied", self),
            ));
        }
        self.do_transition(current_status, PatchStatus::Actived)
    }

    pub fn deactive(&self) -> std::io::Result<()> {
        let status = self.status()?;
        if status < PatchStatus::Deactived {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Patch {{{}}} is not applied", self),
            ));
        }
        self.do_transition(status, PatchStatus::Deactived)
    }

    pub fn accept(&self) -> std::io::Result<()> {
        let status = self.status()?;
        if status != PatchStatus::Actived {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Patch {{{}}} is not actived", self),
            ));
        }
        self.do_transition(status, PatchStatus::Accepted)
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
