use std::ffi::OsString;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use log::debug;

use common::os;
use common::util::ext_cmd::{ExternCommand, ExternCommandArgs};
use common::util::fs;
use common::util::os_str::OsStringExt;

use super::patch_action::PatchActionAdapter;
use super::patch_info::PatchInfo;
use super::patch_status::PatchStatus;

pub struct KernelPatchAdapter {
    patch_info: Rc<PatchInfo>,
    patch_file: PathBuf,
    sys_file: PathBuf,
}

const INSMOD: ExternCommand = ExternCommand::new("insmod");
const RMMOD: ExternCommand = ExternCommand::new("rmmod");

const KPATCH_PATCH_PREFIX: &str = "syscare";
const KPATCH_PATCH_SUFFIX: &str = "ko";
const KPATCH_PATCH_SEC_TYPE: &str = "modules_object_t";

const KPATCH_MGNT_DIR: &str = "/sys/kernel/livepatch";
const KPATCH_MGNT_FILE: &str = "enabled";

const KPATCH_STATUS_DISABLED: &str = "0";
const KPATCH_STATUS_ENABLED: &str = "1";

impl KernelPatchAdapter {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_info: Rc<PatchInfo>) -> Self {
        let patch_name = format!("{}-{}", KPATCH_PATCH_PREFIX, patch_info.uuid); // Use uuid to avoid name collision
        let patch_file = patch_root
            .as_ref()
            .join(format!("{}.{}", patch_name, KPATCH_PATCH_SUFFIX));
        let sys_file = PathBuf::from(KPATCH_MGNT_DIR)
            .join(patch_name.replace('-', "_"))
            .join(KPATCH_MGNT_FILE);

        Self {
            patch_info,
            patch_file,
            sys_file,
        }
    }

    fn set_patch_security_context(&self) -> std::io::Result<()> {
        if os::selinux::get_enforce()? != os::selinux::SELinuxStatus::Enforcing {
            debug!("SELinux is disabled");
            return Ok(());
        }
        debug!("SELinux is enforcing");

        let sec_type = os::selinux::get_security_context_type(&self.patch_file)?;
        if sec_type != KPATCH_PATCH_SEC_TYPE {
            debug!(
                "Setting patch {{{}}} security context type to \"{}\"",
                self.patch_info, KPATCH_PATCH_SEC_TYPE
            );
            os::selinux::set_security_context_type(&self.patch_file, KPATCH_PATCH_SEC_TYPE)?;
        }

        Ok(())
    }

    fn read_patch_status(&self) -> std::io::Result<PatchStatus> {
        let read_result = fs::read_to_string(&self.sys_file);
        match read_result {
            Ok(s) => {
                let status = s.trim();
                debug!("Read file \"{}\": {}", self.sys_file.display(), status);

                let patch_status = match status {
                    KPATCH_STATUS_DISABLED => PatchStatus::Deactived,
                    KPATCH_STATUS_ENABLED => PatchStatus::Actived,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Patch status \"{}\" is invalid", status),
                        ));
                    }
                };

                Ok(patch_status)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PatchStatus::NotApplied),
            Err(e) => Err(e),
        }
    }

    fn write_patch_status(&self, status: PatchStatus) -> std::io::Result<()> {
        let status_str = match status {
            PatchStatus::NotApplied | PatchStatus::Deactived => KPATCH_STATUS_DISABLED,
            PatchStatus::Actived => KPATCH_STATUS_ENABLED,
            _ => unreachable!("Patch status is unknown"),
        };
        debug!("Write file \"{}\": {}", self.sys_file.display(), status_str);

        fs::write(&self.sys_file, status_str)
    }
}

impl PatchActionAdapter for KernelPatchAdapter {
    fn check(&self) -> std::io::Result<()> {
        let kernel_version = os::kernel::version();

        let current_kernel = OsString::from("kernel-").concat(kernel_version);
        let patch_target = self.patch_info.target.full_name();
        debug!("Current kernel: \"{}\"", current_kernel.to_string_lossy());
        debug!("Patch target:   \"{}\"", patch_target);

        if patch_target.as_bytes() != current_kernel.as_bytes() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Current kernel \"{}\" is incompatible with patch target \"{}\"",
                    kernel_version.to_string_lossy(),
                    patch_target
                ),
            ));
        }

        Ok(())
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        self.read_patch_status()
    }

    fn apply(&self) -> std::io::Result<()> {
        self.set_patch_security_context()?;

        let exit_status = INSMOD.execvp(ExternCommandArgs::new().arg(&self.patch_file))?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr().to_string_lossy());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "Process \"{}\" exited unsuccessfully, exit_code={}",
                    INSMOD,
                    exit_status.exit_code()
                ),
            ));
        }

        Ok(())
    }

    fn remove(&self) -> std::io::Result<()> {
        let exit_status = RMMOD.execvp(ExternCommandArgs::new().arg(&self.patch_file))?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr().to_string_lossy());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "Process \"{}\" exited unsuccessfully, exit_code={}",
                    RMMOD,
                    exit_status.exit_code()
                ),
            ));
        }

        Ok(())
    }

    fn active(&self) -> std::io::Result<()> {
        self.write_patch_status(PatchStatus::Actived)
    }

    fn deactive(&self) -> std::io::Result<()> {
        self.write_patch_status(PatchStatus::Deactived)
    }
}
