use crate::business::patch::{PatchFile, Version};
use crate::util::fs;
use crate::business::cmd::ExternCommand;

use super::rpm_buildroot::RpmBuildRoot;

const RPM_BUILD: ExternCommand = ExternCommand::new("/usr/bin/rpmbuild");

pub struct RpmBuilder {
    build_root: RpmBuildRoot
}

impl RpmBuilder {
    pub fn new(build_root: &str) -> Self {
        Self { build_root: RpmBuildRoot::new(&build_root) }
    }

    pub fn copy_patch_file_to_source(&self, patch_list: &[PatchFile]) -> std::io::Result<()> {
        for patch_file in patch_list {
            let src_path = patch_file.get_path();
            fs::check_file(src_path)?;

            let dst_path = format!("{}/{}", self.build_root.get_source_path(), patch_file.get_name());
            std::fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }
    
    pub fn build_source_package(&self, spec_file_path: &str, patch_version: &Version, output_dir: &str) -> std::io::Result<()> {
        const PATCHED_FLAG:           &str = "patched";
        const RELEASE_TAG_MACRO_NAME: &str = "syscare_patch_release";

        fs::check_file(spec_file_path)?;
        fs::check_dir(output_dir)?;

        let patch_release = format!(".{}.{}.{}.{}", PATCHED_FLAG,
            patch_version.get_name(), patch_version.get_version(), patch_version.get_release()
        );

        // Build source rpm
        RPM_BUILD.execvp([
            "--define", &format!("_topdir {}", self.build_root),
            "--define", &format!("{} {}", RELEASE_TAG_MACRO_NAME, patch_release),
            "-bs", spec_file_path
        ])?;

        // Copy source rpm to output directory
        fs::copy_all_files(self.build_root.get_srpm_path(), output_dir)?;

        Ok(())
    }

    pub fn build_binary_package(&self, spec_file_path: &str, output_dir: &str) -> std::io::Result<()> {
        fs::check_file(spec_file_path)?;
        fs::check_dir(output_dir)?;

        RPM_BUILD.execvp([
            "--define", &format!("_topdir {}", self.build_root),
            "-bb", spec_file_path
        ])?;
        fs::copy_all_files(self.build_root.get_rpm_path(), output_dir)?;

        Ok(())
    }
}

impl From<RpmBuildRoot> for RpmBuilder {
    fn from(build_root: RpmBuildRoot) -> Self {
        Self { build_root }
    }
}
