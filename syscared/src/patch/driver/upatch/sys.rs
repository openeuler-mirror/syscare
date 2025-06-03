use std::path::Path;

use anyhow::{bail, Result};
use log::{debug, Level};
use uuid::Uuid;

use syscare_common::process::Command;

const UPATCH_MANAGE_BIN: &str = "upatch-manage";

pub fn active_patch<P, Q>(uuid: &Uuid, pid: i32, target_elf: P, patch_file: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let target_elf = target_elf.as_ref();
    let patch_file = patch_file.as_ref();

    debug!(
        "Upatch: Patching '{}' to '{}' (pid: {})...",
        patch_file.display(),
        target_elf.display(),
        pid,
    );
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
        .stdout(Level::Error)
        .run_with_output()?
        .exit_code();

    match exit_code {
        0 => Ok(()),
        _ => bail!(std::io::Error::from_raw_os_error(exit_code)),
    }
}

pub fn deactive_patch<P, Q>(uuid: &Uuid, pid: i32, target_elf: P, patch_file: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let target_elf = target_elf.as_ref();
    let patch_file = patch_file.as_ref();

    debug!(
        "Upatch: Unpatching '{}' from '{}' (pid: {})...",
        patch_file.display(),
        target_elf.display(),
        pid,
    );
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
        .stdout(Level::Error)
        .run_with_output()?
        .exit_code();

    match exit_code {
        0 => Ok(()),
        _ => bail!(std::io::Error::from_raw_os_error(exit_code)),
    }
}
