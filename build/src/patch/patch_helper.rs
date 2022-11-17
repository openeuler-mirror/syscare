use crate::constants::*;
use crate::util::fs;

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patches(directory: &str) -> std::io::Result<Vec<String>> {
        let patch_filter_fn = |path| {
            let path_str = fs::stringtify_path(path);
            match path_str.contains(PATCH_FILE_PREFIX) {
                true  => Some(path_str),
                false => None,
            }
        };
        let patch_list: Vec<String> = fs::list_all_files_ext(
            directory,
            PATCH_FILE_EXTENSION,
            false
        )?.into_iter().filter_map(patch_filter_fn).collect();

        Ok(patch_list)
    }
}
