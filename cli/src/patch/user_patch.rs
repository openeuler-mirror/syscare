use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lazy_static::lazy_static;
use log::{debug, error};

use common::util::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::patch_action::PatchActionAdapter;
use super::patch_info::PatchInfo;
use super::patch_status::PatchStatus;

#[derive(PartialEq, Clone, Copy)]
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
            UserPatchAction::Info => "info",
            UserPatchAction::Install => "install",
            UserPatchAction::Uninstall => "uninstall",
            UserPatchAction::Active => "active",
            UserPatchAction::Deactive => "deactive",
        })
    }
}

struct ElfPatch {
    elf_file: PathBuf,
    patch_file: PathBuf,
}

impl std::fmt::Display for ElfPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.elf_file.display()))
    }
}

impl ElfPatch {
    fn status(&self) -> std::io::Result<PatchStatus> {
        let stdout = self.do_action(UserPatchAction::Info)?;
        let status = match stdout.to_str() {
            Some("Status: removed") => PatchStatus::NotApplied,
            Some("Status: installed") => PatchStatus::Deactived,
            Some("Status: actived") => PatchStatus::Actived,
            Some("Status: deactived") => PatchStatus::Deactived,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Patch status \"{}\" is invalid", stdout.to_string_lossy()),
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
            .arg(&self.patch_file);

        if action == UserPatchAction::Install {
            args = args.arg("--binary").arg(&self.elf_file);
        }

        let exit_status = UPATCH_TOOL.execvp(args)?;
        exit_status.check_exit_code()?;

        Ok(exit_status.stdout().to_owned())
    }
}

pub struct UserPatchAdapter {
    patch_info: Rc<PatchInfo>,
    elf_patchs: Vec<ElfPatch>,
}

impl UserPatchAdapter {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_info: Rc<PatchInfo>) -> Self {
        let elf_patchs = patch_info
            .entities
            .iter()
            .map(|entity| ElfPatch {
                elf_file: entity.patch_target.to_path_buf(),
                patch_file: patch_root.as_ref().join(&entity.patch_name),
            })
            .collect();

        Self {
            patch_info,
            elf_patchs,
        }
    }

    fn do_transaction(&self, action: UserPatchAction) -> std::io::Result<()> {
        struct TransactionRecord<'a> {
            elf_patch: &'a ElfPatch,
            old_status: PatchStatus,
        }

        #[inline(always)]
        fn __invoke_transaction(
            elf_patch: &ElfPatch,
            action: UserPatchAction,
        ) -> std::io::Result<TransactionRecord> {
            let record = TransactionRecord {
                elf_patch,
                old_status: elf_patch.status()?,
            };
            debug!("Applying changes to \"{}\"", elf_patch);
            elf_patch.do_action(action)?;
            debug!("Applied chages to \"{}\"", elf_patch);
            Ok(record)
        }

        #[inline(always)]
        fn __rollback_transaction(records: Vec<TransactionRecord>) -> std::io::Result<()> {
            type RollbackTransition = (PatchStatus, PatchStatus);
            lazy_static! {
                static ref ROLLBACK_ACTION_MAP: HashMap<RollbackTransition, UserPatchAction> = [
                    (
                        (PatchStatus::NotApplied, PatchStatus::Deactived),
                        UserPatchAction::Install
                    ),
                    (
                        (PatchStatus::Deactived, PatchStatus::Actived),
                        UserPatchAction::Active
                    ),
                    (
                        (PatchStatus::Actived, PatchStatus::Deactived),
                        UserPatchAction::Deactive
                    ),
                    (
                        (PatchStatus::Deactived, PatchStatus::NotApplied),
                        UserPatchAction::Uninstall
                    ),
                ]
                .into_iter()
                .collect();
            }
            for record in records {
                let elf_patch = record.elf_patch;
                let old_status = record.old_status;
                let current_status = elf_patch.status()?;
                debug!(
                    "Rolling back \"{}\" from {} to {}",
                    elf_patch, current_status, old_status
                );
                if let Some(action) = ROLLBACK_ACTION_MAP.get(&(current_status, old_status)) {
                    elf_patch.do_action(*action)?;
                }
                debug!("Rolled back \"{}\"", elf_patch);
            }
            Ok(())
        }

        let mut records = Vec::new();
        for elf_patch in &self.elf_patchs {
            match __invoke_transaction(elf_patch, action) {
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

impl PatchActionAdapter for UserPatchAdapter {
    fn check(&self) -> std::io::Result<()> {
        self.patch_info.target.check_installed()
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        // Fetch all patches status
        let mut status_set = HashSet::new();
        for patch in &self.elf_patchs {
            status_set.insert(patch.status()?);
        }

        if status_set.len() != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Patch {{{}}} status is not syncing", self.patch_info),
            ));
        }

        Ok(status_set.iter().next().cloned().unwrap_or_default())
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
