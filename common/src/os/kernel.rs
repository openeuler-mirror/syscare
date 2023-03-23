use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::util::os_str::OsStringExt;
use crate::util::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::platform;

const KEXEC:     ExternCommand = ExternCommand::new("kexec");
const SYSTEMCTL: ExternCommand = ExternCommand::new("systemctl");

pub fn version() -> &'static OsStr {
    platform::release()
}

pub fn load<P, Q>(kernel: P, initramfs: Q) -> std::io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let exit_status = KEXEC.execvp(
        ExternCommandArgs::new()
            .arg("--load")
            .arg(kernel.as_ref())
            .arg(OsString::from("--initrd=").concat(initramfs.as_ref()))
            .arg("--reuse-cmdline")
    )?;
    exit_status.check_exit_code()
}

pub fn systemd_exec() -> std::io::Result<()> {
    let exit_status = SYSTEMCTL.execvp(
        ExternCommandArgs::new().arg("kexec")
    )?;
    exit_status.check_exit_code()
}

pub fn direct_exec() -> std::io::Result<()> {
    let exit_status = KEXEC.execvp(
        ExternCommandArgs::new().arg("--exec")
    )?;
    exit_status.check_exit_code()
}
