use std::ffi::{OsString, OsStr};
use std::path::{Path, PathBuf};

use crate::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::patch::Patch;
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;

const UPATCH_TOOL: ExternCommand = ExternCommand::new("/usr/libexec/syscare/upatch-tool");
const UPATCH_ACTION_STATUS:    &str = "info";
const UPATCH_ACTION_INSTALL:   &str = "install";
const UPATCH_ACTION_UNINSTALL: &str = "uninstall";
const UPATCH_ACTION_ACTIVE:    &str = "active";
const UPATCH_ACTION_DEACTIVE:  &str = "deactive";
const UPATCH_STATUS_NOT_APPLY: &str = "Status: removed";
const UPATCH_STATUS_INSTALLED: &str = "Status: installed";
const UPATCH_STATUS_ACTIVED:   &str = "Status: actived";
const UPATCH_STATUS_DEACTIVED: &str = "Status: deactived";

pub struct UserPatchAdapter<'a> {
    patch: &'a Patch,
}

impl<'a> UserPatchAdapter<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self { patch }
    }

    fn get_patch_file<S: AsRef<OsStr>>(&self, elf_name: S) -> PathBuf {
        let mut patch_file_name = OsString::from(&self.patch.short_name());
        patch_file_name.push("-");
        patch_file_name.push(elf_name);

        self.patch.root_dir.join(patch_file_name)
    }

    fn do_action<P: AsRef<Path>>(&self, action: &str, patch: P) -> std::io::Result<OsString> {
        let exit_status = UPATCH_TOOL.execvp(
            ExternCommandArgs::new()
                    .arg(action)
                    .arg("--patch")
                    .arg(patch.as_ref())
        )?;

        Ok(exit_status.stdout().to_owned())
    }

    fn do_action_to_elf<P: AsRef<Path>, Q: AsRef<Path>>(&self, action: &str, patch: P, elf: Q) -> std::io::Result<OsString> {
        let exit_status = UPATCH_TOOL.execvp(
            ExternCommandArgs::new()
                    .arg(action)
                    .arg("--patch")
                    .arg(patch.as_ref())
                    .arg("--binary")
                    .arg(elf.as_ref())
        )?;

        Ok(exit_status.stdout().to_owned())
    }

    fn get_patch_status<S: AsRef<OsStr>>(&self, elf_name: S) -> std::io::Result<PatchStatus> {
        let patch  = self.get_patch_file(&elf_name);

        let stdout = self.do_action(UPATCH_ACTION_STATUS, patch)?;
        if stdout == UPATCH_STATUS_NOT_APPLY {
            Ok(PatchStatus::NotApplied)
        }
        else if stdout == UPATCH_STATUS_INSTALLED  {
            Ok(PatchStatus::Deactived)
        }
        else if stdout == UPATCH_STATUS_DEACTIVED  {
            Ok(PatchStatus::Deactived)
        }
        else if stdout == UPATCH_STATUS_ACTIVED  {
            Ok(PatchStatus::Actived)
        }
        else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Status \"{}\" is invalid", stdout.to_string_lossy())
            ))
        }
    }
}

impl PatchActionAdapter for UserPatchAdapter<'_> {
    fn check_compatibility(&self) -> std::io::Result<()> {
        self.patch.target.check_installed()
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        let mut status_list = Vec::new();
        for (elf_name, _) in &self.patch.target_elfs {
            status_list.push(self.get_patch_status(elf_name)?);
        }
        Ok(status_list[0])
    }

    fn apply(&self) -> std::io::Result<()> {
        for (elf_name, elf_path) in &self.patch.target_elfs {
            self.do_action_to_elf(
                UPATCH_ACTION_INSTALL,
                self.get_patch_file(&elf_name),
                elf_path
            )?;
        }
        Ok(())
    }

    fn remove(&self) -> std::io::Result<()> {
        for (elf_name, _) in &self.patch.target_elfs {
            self.do_action(
                UPATCH_ACTION_UNINSTALL,
                self.get_patch_file(&elf_name)
            )?;
        }
        Ok(())
    }

    fn active(&self) -> std::io::Result<()> {
        for (elf_name, _) in &self.patch.target_elfs {
            self.do_action(
                UPATCH_ACTION_ACTIVE,
                self.get_patch_file(&elf_name)
            )?;
        }
        Ok(())
    }

    fn deactive(&self) -> std::io::Result<()> {
        for (elf_name, _) in &self.patch.target_elfs {
            self.do_action(
                UPATCH_ACTION_DEACTIVE,
                self.get_patch_file(&elf_name)
            )?;
        }
        Ok(())
    }
}
