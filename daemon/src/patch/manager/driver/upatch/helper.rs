use std::{
    ffi::{OsStr, OsString},
    os::unix::prelude::OsStringExt,
    path::Path,
};

use anyhow::Result;
use syscare_common::util::{fs, os_str::OsStrExt};

pub struct UPatchDriverHelper;

impl UPatchDriverHelper {
    fn parse_proc_fs_pid<P: AsRef<Path>>(path: P) -> Option<i32> {
        path.as_ref()
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::parse)
            .and_then(Result::ok)
    }

    pub fn find_target_elf_pid<P: AsRef<Path>>(target_elf: P) -> Result<Vec<i32>> {
        let pid_list = fs::list_dirs("/proc", fs::TraverseOptions { recursive: false })?
            .into_iter()
            .filter_map(Self::parse_proc_fs_pid)
            .filter(|pid| {
                fs::read(format!("/proc/{}/maps", pid))
                    .map(OsString::from_vec)
                    .map(|proc_map| proc_map.contains(target_elf.as_ref().as_os_str()))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        Ok(pid_list)
    }
}
