use std::path::{Path, PathBuf};

use log::debug;

use crate::constants::*;
use crate::util::fs;

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patches<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Collecting patches from \"{}\"", directory.as_ref().display());

        let patch_list = fs::list_all_files_ext(
            directory,
            PATCH_FILE_EXTENSION,
            false
        )?.into_iter().collect();

        Ok(patch_list)
    }
}
