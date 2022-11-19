use crate::util::fs;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_debuginfo_file(directory: &str, elf_name: &str) -> std::io::Result<String> {
        let debuginfo_file_path = fs::find_file(
            directory,
            elf_name,
            true,
            true
        )?;

        Ok(fs::stringtify(debuginfo_file_path))
    }
}
