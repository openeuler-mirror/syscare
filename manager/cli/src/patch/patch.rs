use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::debug;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::util::fs;

use super::patch_type::PatchType;
use super::patch_info::PatchInfo;
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;
use super::user_patch::UserPatchAdapter;
use super::kernel_patch::KernelPatchAdapter;

const PATCH_INFO_FILE_NAME: &str = "patch_info";

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Patch {
    root:   PathBuf,
    info:   PatchInfo,
    status: PatchStatus,
}

impl Patch {
    pub fn parse_from<P: AsRef<Path>>(patch_root: P) -> std::io::Result<Self> {
        let file_path = patch_root.as_ref().join(PATCH_INFO_FILE_NAME);

        Ok(Self {
            root:   patch_root.as_ref().to_path_buf(),
            info:   fs::read_file_to_string(file_path)?.parse::<PatchInfo>()?,
            status: PatchStatus::NotApplied,
        })
    }

    pub fn get_root(&self) -> &Path {
        &self.root
    }

    pub fn get_info(&self) -> &PatchInfo {
        &self.info
    }

    pub fn get_status(&self) -> PatchStatus {
        self.status
    }

    pub fn get_simple_name(&self) -> &str {
        self.get_info().get_name()
    }

    pub fn get_target(&self) -> &str {
        self.get_info().get_target()
    }

    pub fn get_arch(&self) -> &str {
        self.get_info().get_arch()
    }

    pub fn get_full_name(&self) -> String {
        format!("{}/{}", self.get_target(), self.get_simple_name())
    }
}

impl Patch {
    fn get_adapter(&self) -> Box<dyn PatchActionAdapter + '_> {
        match self.get_info().get_type() {
            PatchType::UserPatch   => Box::new(UserPatchAdapter::new(self)),
            PatchType::KernelPatch => Box::new(KernelPatchAdapter::new(self)),
        }
    }

    fn check_compatibility(&self) -> std::io::Result<()> {
        self.get_adapter().check_compatibility().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("check patch \"{}\" failed, {}", self, e.to_string())
            )
        })?;

        debug!("patch \"{}\" is compatible", self);
        Ok(())
    }

    fn set_status(&mut self, status: PatchStatus) {
        let current_status = self.status;
        let target_tatus   = status;

        if current_status == target_tatus {
            return;
        }

        self.status = target_tatus;
        debug!("patch \"{}\" status changed from \"{}\" to \"{}\"", self, current_status, target_tatus);
    }

    fn do_apply(&mut self) -> std::io::Result<()> {
        self.get_adapter().apply().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("patch \"{}\" apply failed, {}", self, e.to_string())
            )
        })?;

        self.set_status(PatchStatus::Deactived);
        Ok(())
    }

    fn do_remove(&mut self) -> std::io::Result<()> {
        self.get_adapter().remove().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("patch \"{}\" remove failed, {}", self, e.to_string())
            )
        })?;

        self.set_status(PatchStatus::NotApplied);
        Ok(())
    }

    fn do_active(&mut self) -> std::io::Result<()> {
        self.get_adapter().active().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("patch \"{}\" active failed, {}", self, e.to_string())
            )
        })?;

        self.set_status(PatchStatus::Actived);
        Ok(())
    }

    fn do_deactive(&mut self) -> std::io::Result<()> {
        self.get_adapter().deactive().map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("patch \"{}\" deactive failed, {}", self, e.to_string())
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

        Ok(())
    }

    pub fn apply(&mut self) -> std::io::Result<()> {
        debug!("applying patch \"{}\"", self);

        match self.get_status() {
            PatchStatus::NotApplied => {
                self.check_compatibility()?;
                self.do_apply()?;
                self.do_active()?;
            },
            _ => {
                debug!("patch \"{}\" is already applied", self);
            },
        }

        Ok(())
    }

    pub fn remove(&mut self) -> std::io::Result<()> {
        debug!("removing patch \"{}\"", self);

        match self.get_status() {
            PatchStatus::NotApplied => {
                debug!("patch \"{}\" is already removed", self);
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
        debug!("activing patch \"{}\"", self);

        match self.get_status() {
            PatchStatus::NotApplied => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("patch \"{}\" is not applied", self)
                ));
            },
            PatchStatus::Deactived => {
                self.do_active()?;
            },
            PatchStatus::Actived => {
                debug!("patch \"{}\" is already actived", self);
            },
        }

        Ok(())
    }

    pub fn deactive(&mut self) -> std::io::Result<()> {
        debug!("deactiving patch \"{}\"", self);

        match self.get_status() {
            PatchStatus::NotApplied => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("patch \"{}\" is not applied", self)
                ));
            },
            PatchStatus::Deactived => {
                debug!("patch \"{}\" is already deactived", self);
            },
            PatchStatus::Actived => {
                self.do_deactive()?;
            },
        }

        Ok(())
    }

    pub fn restore(&mut self, status: PatchStatus) -> std::io::Result<()> {
        type PatchTransition   = (PatchStatus, PatchStatus);
        type PatchTransitionFn = dyn Fn(&mut Patch) -> std::io::Result<()> + Sync;

        const PATCH_APPLY_FN:    &PatchTransitionFn = &Patch::apply;
        const PATCH_REMOVE_FN:   &PatchTransitionFn = &Patch::remove;
        const PATCH_ACTIVE_FN:   &PatchTransitionFn = &Patch::active;
        const PATCH_DEACTIVE_FN: &PatchTransitionFn = &Patch::deactive;

        lazy_static! {
            static ref PATCH_TRANSITION_MAP: HashMap<PatchTransition, &'static PatchTransitionFn> = [
                ( (PatchStatus::NotApplied, PatchStatus::Deactived ), PATCH_APPLY_FN    ),
                ( (PatchStatus::NotApplied, PatchStatus::Actived   ), PATCH_ACTIVE_FN   ),
                ( (PatchStatus::Deactived,  PatchStatus::NotApplied), PATCH_REMOVE_FN   ),
                ( (PatchStatus::Deactived,  PatchStatus::Actived   ), PATCH_ACTIVE_FN   ),
                ( (PatchStatus::Actived,    PatchStatus::NotApplied), PATCH_REMOVE_FN   ),
                ( (PatchStatus::Actived,    PatchStatus::Deactived ), PATCH_DEACTIVE_FN ),
            ].into_iter().collect();
        }

        let transition = (self.get_status(), status);
        match PATCH_TRANSITION_MAP.get(&transition) {
            Some(action) => action(self)?,
            None         => debug!("patch \"{}\" status not change", self),
        }

        Ok(())
    }
}

impl AsRef<Patch> for Patch {
    fn as_ref(&self) -> &Patch {
        self
    }
}

impl std::fmt::Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_full_name())
    }
}
