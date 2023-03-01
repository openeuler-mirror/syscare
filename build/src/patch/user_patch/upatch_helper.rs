use std::path::{Path, PathBuf};

use log::debug;

use crate::package::RpmHelper;
use crate::util::fs;
use crate::util::os_str::OsStrSplit;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_debuginfo_file<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        const DEBUGINFO_FILE_EXT: &str  = "debug";

        debug!("Finding debuginfo from \"{}\"", directory.as_ref().display());
        fs::list_all_files_ext(
            directory,
            DEBUGINFO_FILE_EXT,
            true,
        )
    }

    pub fn query_pkg_file_list<P: AsRef<Path>>(pkg_path: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Reading package file list from \"{}\"", pkg_path.as_ref().display());

        let file_list_str = RpmHelper::query_package_info(pkg_path, "[%{FILENAMES} ]")?;
        let file_list = file_list_str
            .split(' ')
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        Ok(file_list)
    }
}
