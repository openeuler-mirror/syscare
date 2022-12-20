use crate::util::fs;
use crate::patch::PatchInfo;

pub struct UserPatchHelper;

impl UserPatchHelper {
    pub fn find_debuginfo_file(directory: &str, patch_info: &PatchInfo) -> std::io::Result<String> {
        let target = patch_info.get_target();
        let file_name = format!("{}-{}-{}.{}.debug",
            patch_info.get_target_elf_name(), target.get_version(), target.get_release(), patch_info.get_arch()
        );

        let debuginfo_file_path = fs::find_file(
            directory,
            file_name.as_str(),
            false,
            true
        )?;

        Ok(fs::stringtify(debuginfo_file_path))
    }
}
