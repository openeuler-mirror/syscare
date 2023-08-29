use std::ffi::OsString;

use anyhow::{anyhow, bail, Result};

use syscare_abi::PatchStatus;
use syscare_common::util::{
    digest,
    ext_cmd::{ExternCommand, ExternCommandArgs},
};

use super::{Patch, PatchDriver, UserPatchExt};

const UPATCH_TOOL: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-tool");

#[derive(PartialEq, Debug)]
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

pub struct UserPatchDriver;

impl UserPatchDriver {
    fn do_action(patch: &Patch, action: UserPatchAction) -> Result<OsString> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let mut args = ExternCommandArgs::new()
            .arg(action.to_string())
            .arg("--patch")
            .arg(patch_ext.patch_file.as_path());

        if action == UserPatchAction::Install {
            args = args.arg("--binary").arg(patch_ext.target_elf.as_path());
        }

        let exit_status = UPATCH_TOOL
            .execvp(args)
            .map_err(|e| anyhow!("Upatch: {}", e))?;

        exit_status
            .check_exit_code()
            .map_err(|_| anyhow!("Upatch: {}", exit_status.stderr().to_string_lossy()))?;

        Ok(exit_status.stdout().to_owned())
    }
}

impl PatchDriver for UserPatchDriver {
    fn check(&self, patch: &Patch) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();

        let real_checksum = digest::file(patch_file).map_err(|e| anyhow!("Upatch: {}", e))?;

        if !patch.checksum.eq(&real_checksum) {
            bail!(
                "Upatch: Patch file \"{}\" checksum failed",
                patch_file.display()
            );
        }

        Ok(())
    }

    fn status(&self, patch: &Patch) -> Result<PatchStatus> {
        let stdout = Self::do_action(patch, UserPatchAction::Info)?;
        let status = match stdout.to_str() {
            Some("removed") => PatchStatus::NotApplied,
            Some("installed") => PatchStatus::Deactived,
            Some("actived") => PatchStatus::Actived,
            Some("deactived") => PatchStatus::Deactived,
            _ => {
                bail!(
                    "Upatch: Patch status \"{}\" is invalid",
                    stdout.to_string_lossy()
                );
            }
        };

        Ok(status)
    }

    fn apply(&self, patch: &Patch) -> Result<()> {
        Self::do_action(patch, UserPatchAction::Install)?;

        Ok(())
    }

    fn remove(&self, patch: &Patch) -> Result<()> {
        Self::do_action(patch, UserPatchAction::Uninstall)?;

        Ok(())
    }

    fn active(&self, patch: &Patch) -> Result<()> {
        Self::do_action(patch, UserPatchAction::Active)?;

        Ok(())
    }

    fn deactive(&self, patch: &Patch) -> Result<()> {
        Self::do_action(patch, UserPatchAction::Deactive)?;

        Ok(())
    }
}
