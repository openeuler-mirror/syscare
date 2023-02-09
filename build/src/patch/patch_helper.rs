use std::path::{Path, PathBuf};

use crate::constants::*;
use crate::util::fs;

use super::patch_info::PatchFile;

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patches<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        let patch_filter_fn = |file_path: PathBuf| {
            let file_name = fs::file_name(&file_path).unwrap();
            match PatchFile::validate_naming_rule(file_name.as_str()) {
                true  => Some(file_path),
                false => None,
            }
        };
        let patch_list = fs::list_all_files_ext(
            directory,
            PATCH_FILE_EXTENSION,
            false
        )?.into_iter().filter_map(patch_filter_fn).collect();

        Ok(patch_list)
    }
}
