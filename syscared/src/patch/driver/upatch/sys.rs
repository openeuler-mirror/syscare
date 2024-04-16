use std::path::Path;

use anyhow::{bail, Result};
use log::Level;
use nix::libc::EEXIST;
use uuid::Uuid;

use syscare_common::process::Command;

const UPATCH_MANAGE_BIN: &str = "/usr/libexec/syscare/upatch-manage";

pub fn active_patch(uuid: &Uuid, pid: i32, target_elf: &Path, patch_file: &Path) -> Result<()> {
    let exit_code = Command::new(UPATCH_MANAGE_BIN)
        .arg("patch")
        .arg("--uuid")
        .arg(uuid.to_string())
        .arg("--pid")
        .arg(pid.to_string())
        .arg("--binary")
        .arg(target_elf)
        .arg("--upatch")
        .arg(patch_file)
        .stdout(Level::Debug)
        .run_with_output()?
        .exit_code();

    match exit_code {
        0 => Ok(()),
        EEXIST => Ok(()),
        _ => bail!(std::io::Error::from_raw_os_error(exit_code)),
    }
}

pub fn deactive_patch(uuid: &Uuid, pid: i32, target_elf: &Path, patch_file: &Path) -> Result<()> {
    let exit_code = Command::new(UPATCH_MANAGE_BIN)
        .arg("unpatch")
        .arg("--uuid")
        .arg(uuid.to_string())
        .arg("--pid")
        .arg(pid.to_string())
        .arg("--binary")
        .arg(target_elf)
        .arg("--upatch")
        .arg(patch_file)
        .stdout(Level::Debug)
        .run_with_output()?
        .exit_code();

    match exit_code {
        0 => Ok(()),
        _ => bail!(std::io::Error::from_raw_os_error(exit_code)),
    }
}
