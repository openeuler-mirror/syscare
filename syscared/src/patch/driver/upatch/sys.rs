use std::path::Path;

use anyhow::{bail, Result};
use log::{debug, Level};
use nix::libc::EEXIST;
use uuid::Uuid;

use syscare_common::process::Command;

const UPATCH_MANAGE_BIN: &str = "/usr/libexec/syscare/upatch-manage";

pub fn active_patch<'a, I>(
    uuid: &Uuid,
    target_elf: &Path,
    patch_file: &Path,
    pid_list: I,
) -> Result<()>
where
    I: IntoIterator<Item = &'a i32>,
{
    debug!(
        "Patching '{}' to {}",
        patch_file.display(),
        target_elf.display()
    );

    for pid in pid_list {
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
            0 => {}
            EEXIST => {}
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(exit_code)),
        }
    }

    Ok(())
}

pub fn deactive_patch<'a, I>(
    uuid: &Uuid,
    target_elf: &Path,
    patch_file: &Path,
    pid_list: I,
) -> Result<()>
where
    I: IntoIterator<Item = &'a i32>,
{
    debug!(
        "Unpatching '{}' from {}",
        patch_file.display(),
        target_elf.display()
    );

    for pid in pid_list {
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
            0 => {}
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(exit_code)),
        }
    }

    Ok(())
}
