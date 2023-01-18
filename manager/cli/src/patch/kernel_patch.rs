use std::path::PathBuf;

use log::debug;

use crate::util::sys;
use crate::util::fs;
use crate::util::selinux;

use crate::ext_cmd::ExternCommand;

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
            self.patch.get_simple_name(),
            KPATCH_PATCH_SUFFIX
        );
        self.patch.get_root().join(patch_name)
    }

    fn set_patch_security_context(&self) -> std::io::Result<()> {
        if selinux::get_enforce()? == 0 {
            debug!("SELinux is permissive");
            return Ok(());
        }
        debug!("SELinux is enforcing");

        let patch      = self.patch;
        let patch_file = self.get_patch_file();
        let sec_type   = selinux::get_security_context_type(patch_file.as_path())?;
        if sec_type != KPATCH_PATCH_SEC_TYPE {
            debug!("set patch \"{}\" security context type to \"{}\"",
                patch, KPATCH_PATCH_SEC_TYPE
            );
            selinux::set_security_context_type(&patch_file, KPATCH_PATCH_SEC_TYPE)?;
        }

        Ok(())
    }

    fn get_sys_interface(&self) -> String {
        let patch_name = self.patch.get_simple_name().replace('-', "_");
        fs::stringtify(
            PathBuf::from(KPATCH_MGNT_DIR)
                .join(patch_name)
                .join(KPATCH_MGNT_FILE)
        )
    }

    fn read_patch_status(&self) -> std::io::Result<PatchStatus> {
        let sys_file_path = self.get_sys_interface();

        let read_result = fs::read_file_to_string(&sys_file_path);
        match read_result {
            Ok(s) => {
                let patch_status = match s.as_str() {
                    KPATCH_STATUS_DISABLED => PatchStatus::Deactived,
                    KPATCH_STATUS_ENABLED  => PatchStatus::Actived,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("status \"{}\" is invalid", s)
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
        let sys_file_path = self.get_sys_interface();
        let status_str = match status {
            PatchStatus::NotApplied | PatchStatus::Deactived => KPATCH_STATUS_DISABLED,
            PatchStatus::Actived => KPATCH_STATUS_ENABLED,
        };

        fs::write_string_to_file(&sys_file_path, status_str)
    }
}

impl PatchActionAdapter for KernelPatchAdapter<'_> {
    fn check_compatibility(&self) -> std::io::Result<()> {
        let patch_target = self.patch.get_target();
        let patch_arch   = self.patch.get_arch();

        let kernel_version = sys::get_kernel_version()?;
        let patch_kernel   = format!("{}.{}", patch_target, patch_arch);
        let current_kernel = format!("kernel-{}", kernel_version);
        debug!("current kernel: \"{}\"", current_kernel);
        debug!("target kernel:  \"{}\"", patch_kernel);

        if patch_kernel != current_kernel {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("current kernel \"{}\" is incompatible", kernel_version)
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
        let exit_status = INSMOD.execvp([patch_file])?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                exit_status.stderr(),
            ));
        }

        Ok(())
    }

    fn remove(&self) -> std::io::Result<()> {
        let patch_file  = self.get_patch_file();
        let exit_status = RMMOD.execvp([patch_file])?;

        if exit_status.exit_code() != 0 {
            debug!("{}", exit_status.stderr());
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                exit_status.stderr(),
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
