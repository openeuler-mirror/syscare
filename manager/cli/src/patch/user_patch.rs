use std::path::{Path, PathBuf};
use std::io::Read;

use log::debug;

use crate::util::fs;
use crate::ext_cmd::ExternCommand;

use super::patch::Patch;
use super::patch_status::PatchStatus;
use super::patch_action::PatchActionAdapter;

const RPM:         ExternCommand = ExternCommand::new("rpm");
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
    patch: &'a Patch
}

impl<'a> UserPatchAdapter<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self { patch }
    }

    fn is_elf_file<P: AsRef<Path>>(file_path: P) -> bool {
        const ELF_MAGIC: [u8; 4] = [127, 69, 76, 70];

        let has_elf_magic = || -> std::io::Result<bool> {
            let mut buf = [0; 4];

            let file_path_ref = file_path.as_ref();
            if !file_path_ref.is_file() {
                return Ok(false);
            }

            let mut file = std::fs::File::open(file_path_ref)?;
            file.read_exact(&mut buf)?;

            Ok(buf == ELF_MAGIC)
        };

        match has_elf_magic() {
            Ok(result) => result,
            Err(_)     => false,
        }
    }

    fn get_elf_file(&self) -> std::io::Result<String> {
        let patch_info = self.patch.get_info();
        let pkg_name   = patch_info.get_target();
        let elf_name   = patch_info.get_elf_name();

        let exit_status = RPM.execvp(["-ql", pkg_name])?;
        if exit_status.exit_code() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                exit_status.stdout()
            ));
        }

        let file_list = exit_status.stdout().split('\n');
        for file_path in file_list {
            if file_path.ends_with(elf_name) && Self::is_elf_file(file_path) {
                return Ok(file_path.to_owned());
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("cannot find target elf \"{}\"", elf_name)
        ))
    }

    fn get_patch_file(&self) -> PathBuf {
        self.patch.get_root().join(self.patch.get_simple_name())
    }

    fn exec_upatch_tool(&self, action: &str) -> std::io::Result<String> {
        let patch_file = fs::stringtify(self.get_patch_file());
        let exit_status = match action {
            UPATCH_ACTION_INSTALL => {
                let elf_file = self.get_elf_file()?;
                UPATCH_TOOL.execvp([action, "-p", &patch_file, "-b", &elf_file])?
            },
            _ => {
                UPATCH_TOOL.execvp([action, "-p", &patch_file])?
            }
        };

        if exit_status.exit_code() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                exit_status.stderr()
            ));
        }

        Ok(exit_status.stdout().trim().to_owned())
    }
}

impl PatchActionAdapter for UserPatchAdapter<'_> {
    fn check_compatibility(&self) -> std::io::Result<()> {
        let patch_target = self.patch.get_target();
        let patch_arch   = self.patch.get_arch();

        let target_name = format!("{}.{}", patch_target, patch_arch);
        debug!("target_name:  \"{}\"", target_name);

        let exit_status = RPM.execvp(["-q", &target_name])?;
        if exit_status.exit_code() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                exit_status.stdout()
            ));
        }
        debug!("package_name: \"{}\"", exit_status.stdout());

        Ok(())
    }

    fn status(&self) -> std::io::Result<PatchStatus> {
        let stdout = self.exec_upatch_tool(UPATCH_ACTION_STATUS)?;
        match stdout.as_str() {
            UPATCH_STATUS_NOT_APPLY => Ok(PatchStatus::NotApplied),
            UPATCH_STATUS_INSTALLED => Ok(PatchStatus::Deactived),
            UPATCH_STATUS_DEACTIVED => Ok(PatchStatus::Deactived),
            UPATCH_STATUS_ACTIVED   => Ok(PatchStatus::Actived),
            _ => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("status \"{}\" is invalid", stdout)
                ))
            }
        }
    }

    fn apply(&self) -> std::io::Result<()> {
        self.exec_upatch_tool(UPATCH_ACTION_INSTALL)?;

        Ok(())
    }

    fn remove(&self) -> std::io::Result<()> {
        self.exec_upatch_tool(UPATCH_ACTION_UNINSTALL)?;

        Ok(())
    }

    fn active(&self) -> std::io::Result<()> {
        self.exec_upatch_tool(UPATCH_ACTION_ACTIVE)?;

        Ok(())
    }

    fn deactive(&self) -> std::io::Result<()> {
        self.exec_upatch_tool(UPATCH_ACTION_DEACTIVE)?;

        Ok(())
    }
}
