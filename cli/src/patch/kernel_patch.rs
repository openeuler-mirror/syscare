use std::ffi::OsString;
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;

use log::debug;

use crate::os::{SELinux, SELinuxStatus};


use common::util::os_str::OsStringExt;
use common::util::{sys, fs};
use common::util::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::patch::Patch;
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;

pub struct KernelPatchAdapter<'a> {
    patch: &'a Patch
}

const INSMOD: ExternCommand = ExternCommand::new("insmod");
const RMMOD:  ExternCommand = ExternCommand::new("rmmod");

const KPATCH_PATCH_SUFFIX:   &str = "ko";
const KPATCH_PATCH_SEC_TYPE: &str = "modules_object_t";

const KPATCH_MGNT_DIR:  &str = "/sys/kernel/livepatch";
const KPATCH_MGNT_FILE: &str = "enabled";

const KPATCH_STATUS_DISABLED: &str = "0";
const KPATCH_STATUS_ENABLED:  &str = "1";

impl<'a> KernelPatchAdapter<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self { patch }
    }

    fn get_patch_file(&self) -> PathBuf {
        let patch_name = format!("{}.{}",
            self.patch.name,
            KPATCH_PATCH_SUFFIX
        );
        self.patch.root_dir.join(patch_name)
    }

    fn set_patch_security_context(&self) -> std::io::Result<()> {
        if SELinux::get_enforce()? != SELinuxStatus::Enforcing {
            debug!("SELinux is disabled");
            return Ok(());
        }
        debug!("SELinux is enforcing");

        let patch      = self.patch;
        let patch_file = self.get_patch_file();
        let sec_type   = SELinux::get_security_context_type(patch_file.as_path())?;
        if sec_type != KPATCH_PATCH_SEC_TYPE {
            debug!("Setting patch {{{}}} security context type to \"{}\"",
                patch, KPATCH_PATCH_SEC_TYPE
            );
            SELinux::set_security_context_type(&patch_file, KPATCH_PATCH_SEC_TYPE)?;
        }

        Ok(())
    }

    fn patch_sys_interface(&self) -> PathBuf {
        let patch_name = self.patch.short_name().replace('-', "_");

        PathBuf::from(KPATCH_MGNT_DIR)
            .join(patch_name)
            .join(KPATCH_MGNT_FILE)
    }

    fn read_patch_status(&self) -> std::io::Result<PatchStatus> {
        let sys_file_path = self.patch_sys_interface();

        let read_result = fs::read_to_string(&sys_file_path);
        match read_result {
            Ok(s) => {
                let status = s.trim();
                debug!("Read file \"{}\": {}", sys_file_path.display(), status);

                let patch_status = match status {
                    KPATCH_STATUS_DISABLED => PatchStatus::Deactived,
                    KPATCH_STATUS_ENABLED  => PatchStatus::Actived,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Patch status \"{}\" is invalid", status)
                        ));
                    }
                };

                Ok(patch_status)
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(PatchStatus::NotApplied)
            },
            Err(e) => Err(e)
        }
    }

    fn write_patch_status(&self, status: PatchStatus) -> std::io::Result<()> {
        let sys_file_path = self.patch_sys_interface();
        let status_str = match status {
            PatchStatus::NotApplied | PatchStatus::Deactived => KPATCH_STATUS_DISABLED,
            PatchStatus::Actived => KPATCH_STATUS_ENABLED,
            _ => unreachable!("Patch status is unknown"),
        };
        debug!("Write file \"{}\": {}", sys_file_path.display(), status_str);

        fs::write(&sys_file_path, status_str)
    }
}

impl PatchActionAdapter for KernelPatchAdapter<'_> {
    fn check_compatibility(&self) -> std::io::Result<()> {
        let kernel_version = sys::kernel_version();

        let current_kernel = OsString::from("kernel-").concat(&kernel_version);
        let patch_target   = self.patch.target.full_name();
        debug!("Current kernel: \"{}\"", current_kernel.to_string_lossy());
        debug!("Patch target:   \"{}\"", patch_target);

        if patch_target.as_bytes() != current_kernel.as_bytes() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Current kernel \"{}\" is incompatible with patch target \"{}\"",
                    kernel_version.to_string_lossy(),
                    patch_target
                )
            ));
        }

        Ok(())
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        self.read_patch_status()
    }

    fn apply(&self) -> std::io::Result<()> {
        self.set_patch_security_context()?;

        let patch_file = self.get_patch_file();
        let exit_status = INSMOD.execvp(
            ExternCommandArgs::new().arg(patch_file)
        )?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr().to_string_lossy());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process \"{}\" exited unsuccessfully, exit_code={}",
                    INSMOD,
                    exit_status.exit_code()
                ),
            ));
        }

        Ok(())
    }

    fn remove(&self) -> std::io::Result<()> {
        let patch_file  = self.get_patch_file();
        let exit_status = RMMOD.execvp(
            ExternCommandArgs::new().arg(patch_file)
        )?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr().to_string_lossy());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process \"{}\" exited unsuccessfully, exit_code={}",
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
