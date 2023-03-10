use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use lazy_static::lazy_static;
use log::{debug, error};

use crate::util::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::patch::Patch;
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;

#[derive(PartialEq)]
#[derive(Clone, Copy)]
enum UserPatchAction {
    Info,
    Install,
    Uninstall,
    Active,
    Deactive,
}

impl std::fmt::Display for UserPatchAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UserPatchAction::Info      => "info",
            UserPatchAction::Install   => "install",
            UserPatchAction::Uninstall => "uninstall",
            UserPatchAction::Active    => "active",
            UserPatchAction::Deactive  => "deactive",
        })
    }
}

struct UserPatch {
    elf:   PathBuf,
    patch: PathBuf,
}

impl std::fmt::Display for UserPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.elf.display()))
    }
}

impl UserPatch {
    fn status(&self) -> std::io::Result<PatchStatus> {
        let stdout = self.do_action(UserPatchAction::Info)?;
        let status = match stdout.to_str() {
            Some("Status: removed")   => PatchStatus::NotApplied,
            Some("Status: installed") => PatchStatus::Deactived,
            Some("Status: actived")   => PatchStatus::Actived,
            Some("Status: deactived") => PatchStatus::Deactived,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Patch status \"{}\" is invalid", stdout.to_string_lossy())
                ));
            }
        };

        Ok(status)
    }

    fn do_action(&self, action: UserPatchAction) -> std::io::Result<OsString> {
        const UPATCH_TOOL: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-tool");

        let mut args = ExternCommandArgs::new()
            .arg(action.to_string())
            .arg("--patch")
            .arg(&self.patch);

        if action == UserPatchAction::Install {
            args = args.arg("--binary").arg(&self.elf);
        }

        let exit_status = UPATCH_TOOL.execvp(args)?;
        exit_status.check_exit_code()?;

        Ok(exit_status.stdout().to_owned())
    }
}

pub struct UserPatchAdapter<'a> {
    patch: &'a Patch,
}

impl<'a> UserPatchAdapter<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self { patch }
    }

    fn get_user_patches(&self) -> Vec<UserPatch> {
        self.patch.target_elfs.iter().map(|(elf_name, elf_path)| {
            UserPatch {
                elf:   elf_path.to_path_buf(),
                patch: self.patch.root_dir.join(elf_name),
            }
        }).collect()
    }

    fn do_transaction(&self, action: UserPatchAction) -> std::io::Result<()> {
        struct TransactionRecord<'a> {
            upatch:      &'a UserPatch,
            old_status: PatchStatus,
        }

        #[inline(always)]
        fn __invoke_transaction(upatch: &UserPatch, action: UserPatchAction) -> std::io::Result<TransactionRecord> {
            let record = TransactionRecord { upatch, old_status: upatch.status()? };
            debug!("Applying changes to \"{}\"", upatch);
            upatch.do_action(action)?;
            debug!("Applied \"{}\"", upatch);
            Ok(record)
        }

        #[inline(always)]
        fn __rollback_transaction(records: Vec<TransactionRecord>) -> std::io::Result<()> {
            type RollbackTransition = (PatchStatus, PatchStatus);
            lazy_static! {
                static ref ROLLBACK_ACTION_MAP: HashMap<RollbackTransition, UserPatchAction> = [
                    ( (PatchStatus::NotApplied, PatchStatus::Deactived ), UserPatchAction::Install   ),
                    ( (PatchStatus::Deactived,  PatchStatus::Actived   ), UserPatchAction::Active    ),
                    ( (PatchStatus::Actived,    PatchStatus::Deactived ), UserPatchAction::Deactive  ),
                    ( (PatchStatus::Deactived,  PatchStatus::NotApplied), UserPatchAction::Uninstall ),
                ].into_iter().collect();
            }
            for record in records {
                let upatch = record.upatch;
                debug!("Rolling back changes to \"{}\"", upatch);
                if let Some(action) = ROLLBACK_ACTION_MAP.get(&(record.old_status, upatch.status()?)) {
                    upatch.do_action(*action)?;
                }
                debug!("Rolled back changes to \"{}\"", upatch);
            }
            Ok(())
        }

        let mut records = Vec::new();
        for upatch in &self.get_user_patches() {
            match __invoke_transaction(upatch, action) {
                Ok(record) => {
                    records.push(record);
                }
                Err(e) => {
                    if let Err(e) = __rollback_transaction(records) {
                        error!("{}", e);
                    }
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

impl PatchActionAdapter for UserPatchAdapter<'_> {
    fn check_compatibility(&self) -> std::io::Result<()> {
        self.patch.target.check_installed()
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        // Fetch all patches status
        let mut status_list = Vec::new();
        for patch in self.get_user_patches() {
            status_list.push(patch.status()?)
        }
        // Check if all patch status are same
        status_list.sort();
        status_list.dedup();
        if status_list.len() != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Patch {{{}}} status is not syncing", self.patch)
            ));
        }

        Ok(status_list.remove(0))
    }

    fn apply(&self) -> std::io::Result<()> {
        self.do_transaction(UserPatchAction::Install)
    }

    fn remove(&self) -> std::io::Result<()> {
        self.do_transaction(UserPatchAction::Uninstall)
    }

    fn active(&self) -> std::io::Result<()> {
        self.do_transaction(UserPatchAction::Active)
    }

    fn deactive(&self) -> std::io::Result<()> {
        self.do_transaction(UserPatchAction::Deactive)
    }
}
