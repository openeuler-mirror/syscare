use std::io::{Write, LineWriter};
use std::path::{Path, PathBuf};

use crate::constants::*;
use crate::util::fs;

use crate::patch::PatchInfo;

pub struct RpmSpecGenerator;

impl RpmSpecGenerator {
    fn parse_pkg_root(patch_info: &PatchInfo) -> PathBuf {
        Path::new(PATCH_INSTALL_PATH).join(
            patch_info.target.short_name()
        )
    }

    fn parse_patch_name(patch_info: &PatchInfo) -> String {
        patch_info.name.to_owned()
    }

    fn parse_patch_root(patch_info: &PatchInfo) -> PathBuf {
        Path::new(PATCH_INSTALL_PATH)
            .join(patch_info.target.short_name())
            .join(&patch_info.name)
    }

    fn parse_pkg_name(patch_info: &PatchInfo) -> String {
        format!("{}-{}-{}",
            PKG_FLAG_PATCH_BINARY,
            patch_info.target.short_name(),
            patch_info.name)
    }

    fn parse_requires(patch_info: &PatchInfo) -> String {
        match &patch_info.target.epoch == PKG_FLAG_NONE {
            true => {
                format!("{} = {}-{}",
                    patch_info.target.name,
                    patch_info.target.version,
                    patch_info.target.release
                )
            },
            false => {
                format!("{} = {}:{}-{}",
                    patch_info.target.name,
                    patch_info.target.epoch,
                    patch_info.target.version,
                    patch_info.target.release
                )
            }
        }
    }

    fn parse_summary(patch_info: &PatchInfo) -> String {
        format!("Syscare patch '{}' for {}",
            patch_info.name,
            patch_info.target.short_name()
        )
    }

    fn write_spec_file<W, P>(mut writer: W, patch_info: &PatchInfo, source_dir: P) -> std::io::Result<()>
    where
        W: Write,
        P: AsRef<Path>
    {
        let pkg_file_list = fs::list_all_files(source_dir, true)?
            .into_iter()
            .map(fs::file_name)
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        writeln!(writer, "%global pkg_root              {}", Self::parse_pkg_root(patch_info).display())?;
        writeln!(writer, "%global patch_name            {}", Self::parse_patch_name(patch_info))?;
        writeln!(writer, "%global patch_root            {}", Self::parse_patch_root(patch_info).display())?;
        writeln!(writer, "%global patch_dir_permission  {}", PATCH_DIR_PERMISSION)?;
        writeln!(writer, "%global patch_file_permission {}", PATCH_FILE_PERMISSION)?;

        writeln!(writer, "Name:     {}", Self::parse_pkg_name(patch_info))?;
        writeln!(writer, "Version:  {}", patch_info.version)?;
        writeln!(writer, "Release:  {}", patch_info.release)?;
        writeln!(writer, "Group:    {}", PKG_SPEC_TAG_VALUE_GROUP)?;
        writeln!(writer, "License:  {}", patch_info.license)?;
        writeln!(writer, "Summary:  {}", Self::parse_summary(patch_info))?;
        writeln!(writer, "Requires: {}", Self::parse_requires(patch_info))?;
        writeln!(writer, "Requires: {}", PKG_SPEC_TAG_VALUE_REQUIRES)?;
        let mut file_index = 0usize;
        for file_name in &pkg_file_list {
            writeln!(writer, "Source{}: {}", file_index, file_name)?;
            file_index += 1;
        }
        writeln!(writer)?;

        writeln!(writer, "%description")?;
        writeln!(writer, "{}", patch_info.description)?;
        writeln!(writer)?;

        writeln!(writer, "%prep")?;
        writeln!(writer, "cp -a %{{_sourcedir}}/* %{{_builddir}}")?;
        writeln!(writer)?;

        writeln!(writer, "%build")?;
        writeln!(writer)?;

        writeln!(writer, "%install")?;
        writeln!(writer, "install -m %{{patch_dir_permission}} -d %{{buildroot}}%{{patch_root}}")?;
        for file_name in &pkg_file_list {
            writeln!(writer, "install -m %{{patch_file_permission}} %{{_builddir}}/{} %{{buildroot}}%{{patch_root}}", file_name)?;
        }
        writeln!(writer)?;

        writeln!(writer, "%files")?;
        writeln!(writer, "%{{patch_root}}")?;
        writeln!(writer)?;

        writeln!(writer, "%preun")?;
        writeln!(writer, "if [ \"$(syscare status %{{patch_name}})\" != \"NOT-APPLIED\" ]; then")?;
        writeln!(writer, "    echo \"error: cannot remove applied patch \'%{{patch_name}}\'\" >&2")?;
        writeln!(writer, "    exit 1")?;
        writeln!(writer, "fi")?;

        writeln!(writer, "%postun")?;
        writeln!(writer, "if [ \"$1\" != 0 ]; then")?;
        writeln!(writer, "    exit 0")?;
        writeln!(writer, "fi")?;
        writeln!(writer, "if [ -d \"%{{pkg_root}}\" ] && [ -z \"$(ls -A %{{pkg_root}})\" ]; then")?;
        writeln!(writer, "    rm -rf \"%{{pkg_root}}\"")?;
        writeln!(writer, "fi")?;
        writeln!(writer)?;

        writeln!(writer, "%changelog")?;
        writeln!(writer)?;

        writer.flush()
    }

    pub fn generate_from_patch_info<P: AsRef<Path>, Q: AsRef<Path>>(patch_info: &PatchInfo, source_dir: P, output_dir: Q) -> std::io::Result<PathBuf> {
        fs::check_dir(&source_dir)?;
        fs::check_dir(&output_dir)?;

        let spec_name = format!("{}.spec", Self::parse_pkg_name(patch_info));
        let pkg_spec_path = output_dir.as_ref().join(spec_name);
        let writer = LineWriter::new(
            std::fs::File::create(&pkg_spec_path)?
        );

        Self::write_spec_file(writer, patch_info, source_dir)?;

        Ok(pkg_spec_path)
    }
}
