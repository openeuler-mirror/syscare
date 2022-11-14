use crate::util::fs;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_source_directory(directory: &str, pkg_name: &str) -> std::io::Result<String> {
        let source_dir = fs::find_directory(
            directory,
            pkg_name,
            true,
            true
        )?;

        Ok(fs::stringtify_path(source_dir))
    }
}
