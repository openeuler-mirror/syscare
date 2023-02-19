use std::path::{Path, PathBuf};

use log::debug;

use crate::package::RpmHelper;
use crate::util::fs;
use crate::constants::*;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_debuginfo_file<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Finding debuginfo from \"{}\"", directory.as_ref().display());

        fs::list_all_files_ext(
            directory,
            PATCH_DEBUG_INFO_EXTENSION,
            true,
        )
    }

    pub fn query_pkg_file_list<P: AsRef<Path>>(pkg_path: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Reading package file list from \"{}\"", pkg_path.as_ref().display());

        let file_list_str = RpmHelper::query_package_info(pkg_path, "[%{FILENAMES} ]")?;
        let file_list = file_list_str.trim_end()
            .split(" ")
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        Ok(file_list)
    }
}
