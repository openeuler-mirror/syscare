use std::ffi::{OsStr, OsString};
use std::path::Path;

use anyhow::Result;
use lazy_static::lazy_static;

lazy_static! {
    static ref KEXEC: ExternCommand = ExternCommand::new("kexec");
    static ref SYSTEMCTL: ExternCommand = ExternCommand::new("systemcl");
}

use super::platform;
use crate::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    os_str::OsStringExt,
};

pub fn version() -> &'static OsStr {
    platform::release()
}

pub fn load<P, Q>(kernel: P, initramfs: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let exit_status = KEXEC.execvp(
        ExternCommandArgs::new()
            .arg("--load")
            .arg(kernel.as_ref())
            .arg(OsString::from("--initrd=").concat(initramfs.as_ref()))
            .arg("--reuse-cmdline"),
    )?;
    exit_status.check_exit_code()
}

pub fn systemd_exec() -> Result<()> {
    SYSTEMCTL
        .execvp(ExternCommandArgs::new().arg("kexec"))?
        .check_exit_code()
}

pub fn direct_exec() -> Result<()> {
    KEXEC
        .execvp(ExternCommandArgs::new().arg("--exec"))?
        .check_exit_code()
}
