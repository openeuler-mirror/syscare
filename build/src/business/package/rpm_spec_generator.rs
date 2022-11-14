use std::ffi::OsStr;
use std::io::{Write, LineWriter};

use crate::business::patch::PatchInfo;
use crate::util::fs;

const PATCH_INSTALL_PATH:    &str = "/usr/lib/syscare/patches";
const PATCH_FILE_PERMISSION: &str = "644";

pub struct RpmSpecGenerator;

impl RpmSpecGenerator {
    #[inline(always)]
    fn get_patch_name(patch_info: &PatchInfo) -> &str {
        patch_info.get_patch_version().get_name()
    }

    #[inline(always)]
    fn parse_pkg_name(patch_info: &PatchInfo) -> String {
        let patch_name = Self::get_patch_name(patch_info);
        match patch_info.get_target_version() {
            Some(target) => format!("{}-patch-{}", target, patch_name),
            None => format!("patch-{}", patch_name),
        }
    }

    #[inline(always)]
    fn parse_build_requires(patch_info: &PatchInfo) -> Option<String> {
        patch_info.get_target_version().map(|version| {
            format!("{} = {}-{}", version.get_name(), version.get_version(), version.get_release())
        })
    }

    #[inline(always)]
    fn write_patch_info<W>(mut writer: W, patch_info: &PatchInfo, source_dir: &str) -> std::io::Result<()>
    where
        W: Write
    {
        let pkg_file_list = fs::list_all_files(source_dir, true)?;
        let pkg_install_path = format!("{}/{}", PATCH_INSTALL_PATH, Self::get_patch_name(patch_info));

        writeln!(writer, "Name:     {}", Self::parse_pkg_name(patch_info))?;
        writeln!(writer, "Version:  {}", patch_info.get_patch_version().get_version())?;
        writeln!(writer, "Release:  {}", patch_info.get_patch_version().get_release())?;
        writeln!(writer, "Group:    {}", patch_info.get_group())?;
        writeln!(writer, "License:  {}", patch_info.get_license())?;
        writeln!(writer, "Summary:  {}", patch_info.get_summary())?;
        if let Some(requirement) = Self::parse_build_requires(patch_info) {
            writeln!(writer, "Requires: {}", requirement)?;
        }
        writeln!(writer)?;

        writeln!(writer, "%description")?;
        writeln!(writer, "{}", patch_info)?;
        writeln!(writer)?;

        writeln!(writer, "%prep")?;
        writeln!(writer)?;

        writeln!(writer, "%build")?;
        writeln!(writer)?;

        writeln!(writer, "%install")?;
        writeln!(writer, "mkdir -p %{{buildroot}}{}", pkg_install_path)?;
        for file in &pkg_file_list {
            writeln!(writer, "install -m {} {} %{{buildroot}}{}", PATCH_FILE_PERMISSION, file.display(), pkg_install_path)?;
        }
        writeln!(writer)?;

        writeln!(writer, "%files")?;
        for file in &pkg_file_list {
            if let Some(file_name) = file.file_name().and_then(OsStr::to_str) {
                writeln!(writer, "{}/{}", pkg_install_path, file_name)?;
            }
        }
        writeln!(writer)?;

        writeln!(writer, "%changelog")?;
        writeln!(writer)?;

        writer.flush()
    }

    pub fn generate_from_patch_info(patch_info: &PatchInfo, source_dir: &str, output_dir: &str) -> std::io::Result<String> {
        fs::check_dir(source_dir)?;
        fs::check_dir(output_dir)?;

        let patch_name = Self::get_patch_name(patch_info);
        let pkg_spec_path = format!("{}/{}.spec", output_dir, patch_name);
        let writer = LineWriter::new(
            std::fs::File::create(&pkg_spec_path)?
        );

        Self::write_patch_info(writer, patch_info, source_dir)?;

        Ok(pkg_spec_path)
    }
}
