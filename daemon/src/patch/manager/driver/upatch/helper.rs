use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Result;
use lazy_static::lazy_static;
use syscare_common::util::{fs, os_str::OsStrExt};

use super::proc::ProcMappingReader;

pub struct UPatchDriverHelper;

impl UPatchDriverHelper {
    fn parse_proc_pid<P: AsRef<Path>>(path: P) -> Option<i32> {
        path.as_ref()
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::parse)
            .and_then(Result::ok)
    }

    fn proc_black_list_filter(pid: &i32) -> bool {
        lazy_static! {
            static ref PROC_BLACK_LIST: Vec<PathBuf> = vec![
                PathBuf::from("/usr/lib/systemd/systemd-journald"),
                PathBuf::from("/usr/lib/systemd/systemd-logind"),
                PathBuf::from("/usr/lib/systemd/systemd-udevd"),
                PathBuf::from("/usr/lib/systemd/systemd-hostnamed"),
                PathBuf::from("/usr/bin/udevadm"),
                PathBuf::from("/usr/sbin/auditd"),
                PathBuf::from("/usr/bin/syscare"),
                PathBuf::from("/usr/bin/syscared"),
                PathBuf::from("/usr/bin/upatchd"),
                PathBuf::from("/usr/libexec/syscare/as-hijacker"),
                PathBuf::from("/usr/libexec/syscare/cc-hijacker"),
                PathBuf::from("/usr/libexec/syscare/c++-hijacker"),
                PathBuf::from("/usr/libexec/syscare/gcc-hijacker"),
                PathBuf::from("/usr/libexec/syscare/g++-hijacker"),
                PathBuf::from("/usr/libexec/syscare/syscare-build"),
                PathBuf::from("/usr/libexec/syscare/upatch-build"),
                PathBuf::from("/usr/libexec/syscare/upatch-diff"),
                PathBuf::from("/usr/libexec/syscare/upatch-manage"),
            ];
        }
        fs::read_link(format!("/proc/{}/exe", pid))
            .map(|elf_path| !PROC_BLACK_LIST.contains(&elf_path))
            .unwrap_or(false)
    }

    pub fn find_target_elf_pid<P: AsRef<Path>>(target_elf: P) -> Result<Vec<i32>> {
        let pid_list = fs::list_dirs("/proc", fs::TraverseOptions { recursive: false })?
            .into_iter()
            .filter_map(Self::parse_proc_pid)
            .filter(Self::proc_black_list_filter)
            .filter(|pid| {
                if let Ok(reader) = ProcMappingReader::new(*pid) {
                    let elf_path = fs::canonicalize(target_elf.as_ref()).unwrap_or_default();
                    for mapping in reader {
                        let mapped_elf = mapping.path_name;
                        if mapped_elf == elf_path && !mapped_elf.contains("(deleted)") {
                            return true;
                        }
                    }
                }
                false
            })
            .collect::<Vec<_>>();
        Ok(pid_list)
    }
}
