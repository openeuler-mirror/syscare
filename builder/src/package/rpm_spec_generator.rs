use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::io::{Write, LineWriter};

use common::util::fs;

use crate::patch::PatchInfo;

use super::rpm_spec_helper::{SPEC_FILE_EXT, SOURCE_TAG_NAME, TAG_VALUE_NONE};

pub struct RpmSpecGenerator;

const SYSCARE_PKG_NAME:    &str = "syscare";
const PKG_GROUP:           &str = "Patch";
const PKG_INSTALL_DIR:     &str = "/usr/lib/syscare/patches";
const PKG_FILE_PERMISSION: &str = "664";
const PKG_DIR_PERMISSION:  &str = "775";

const RPM_SPEC_PKG_INFO: &str = r#"
Name:     %{pkg_name}
Version:  %{pkg_version}
Release:  %{pkg_release}
Group:    %{pkg_group}
License:  %{pkg_license}
Summary:  %{pkg_summary}
Requires: %{pkg_requires}
Requires: %{syscare_pkg_name}
"#;

const RPM_SPEC_BODY: &str = r#"
%description
%{pkg_description}

%prep
cp -a "%{_sourcedir}"/* "%{_builddir}"

%install
install -m "%{patch_dir_permission}" -d "%{buildroot}%{patch_root}"
for file in $(ls -A "%{_builddir}"); do
    install -m "%{patch_file_permission}" "%{_builddir}/$file" "%{buildroot}%{patch_root}"
done

%files
%{patch_root}
"#;

const RPM_SPEC_SRCIPTS: &str = r#"
%preun
syscare remove '%{patch_uuid}' || echo "Failed to remove patch '%{patch_target}/%{patch_name}'" >&2
"#;

impl RpmSpecGenerator {
    fn parse_patch_root(patch_info: &PatchInfo) -> PathBuf {
        Path::new(PKG_INSTALL_DIR).join(&patch_info.uuid)
    }

    fn parse_pkg_name(patch_info: &PatchInfo) -> String {
        format!("patch-{}-{}",
            patch_info.target.short_name(),
            patch_info.name)
    }

    fn parse_requires(patch_info: &PatchInfo) -> String {
        match &patch_info.target.epoch == TAG_VALUE_NONE {
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
        format!("Syscare patch \"{}\" for {}",
            patch_info.name,
            patch_info.target.short_name()
        )
    }

    fn write_spec_file<W, I, S>(mut writer: W, patch_info: &PatchInfo, file_names: I) -> std::io::Result<()>
    where
        W: Write,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>
    {
        #[inline(always)]
        fn write_new_line<W: Write>(w: &mut W) -> std::io::Result<()> {
            w.write_all(&[b'\n'])
        }

        #[inline(always)]
        fn write_line<W: Write, C: AsRef<OsStr>>(w: &mut W, content: C) -> std::io::Result<()> {
            w.write_all(content.as_ref().as_bytes())?;
            write_new_line(w)
        }

        #[inline(always)]
        fn write_line2<W: Write, S: AsRef<OsStr>, T: AsRef<OsStr>>(w: &mut W, s: S, t: T) -> std::io::Result<()> {
            w.write_all(s.as_ref().as_bytes())?;
            w.write_all(t.as_ref().as_bytes())?;
            write_new_line(w)
        }

        let w = &mut writer;
        write_line2(w, "%global syscare_pkg_name      ", SYSCARE_PKG_NAME)?;
        write_line2(w, "%global pkg_name              ", Self::parse_pkg_name(patch_info))?;
        write_line2(w, "%global pkg_version           ", &patch_info.version)?;
        write_line2(w, "%global pkg_release           ", patch_info.release.to_string())?;
        write_line2(w, "%global pkg_group             ", PKG_GROUP)?;
        write_line2(w, "%global pkg_license           ", &patch_info.license)?;
        write_line2(w, "%global pkg_summary           ", Self::parse_summary(patch_info))?;
        write_line2(w, "%global pkg_requires          ", Self::parse_requires(patch_info))?;
        write_line2(w, "%global pkg_description       ", &patch_info.description)?;
        write_line2(w, "%global patch_uuid            ", &patch_info.uuid)?;
        write_line2(w, "%global patch_name            ", &patch_info.name)?;
        write_line2(w, "%global patch_target          ", &patch_info.target.short_name())?;
        write_line2(w, "%global patch_root            ", Self::parse_patch_root(patch_info))?;
        write_line2(w, "%global patch_dir_permission  ", PKG_DIR_PERMISSION)?;
        write_line2(w, "%global patch_file_permission ", PKG_FILE_PERMISSION)?;

        write_line(w, RPM_SPEC_PKG_INFO)?;
        for file_name in file_names {
            write_line2(w, format!("{}: ", SOURCE_TAG_NAME), file_name)?;
        }
        write_line(w, RPM_SPEC_BODY)?;
        write_line(w, RPM_SPEC_SRCIPTS)?;

        writer.flush()
    }

    pub fn generate_spec_file<P: AsRef<Path>, Q: AsRef<Path>>(patch_info: &PatchInfo, source_dir: P, output_dir: Q) -> std::io::Result<PathBuf> {
        let spec_name = format!("{}.{}", Self::parse_pkg_name(patch_info), SPEC_FILE_EXT);
        let pkg_spec_path = output_dir.as_ref().join(spec_name);

        let writer = LineWriter::new(
            fs::create_file(&pkg_spec_path)?
        );

        let file_names = fs::list_files(source_dir, fs::TraverseOptions { recursive: true })?
            .into_iter()
            .map(fs::file_name)
            .collect::<Vec<_>>();

        Self::write_spec_file(writer, patch_info, file_names)?;

        Ok(pkg_spec_path)
    }
}
