use std::path::{Path, PathBuf};

use log::debug;
use common::util::fs;

pub const PATCH_FILE_EXT:        &str = "patch";
pub const PATCH_INFO_FILE_NAME:  &str = "patch_info";

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patches<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Collecting patches from \"{}\"", directory.as_ref().display());
        let patch_list = fs::list_files_by_ext(
            directory,
            PATCH_FILE_EXT,
            fs::TraverseOptions { recursive: false }
        )?.into_iter().collect();

        Ok(patch_list)
    }
}
