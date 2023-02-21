use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::io::{Write, LineWriter};

use crate::constants::*;
use crate::util::fs;

use crate::patch::PatchInfo;

pub struct RpmSpecGenerator;

const RPM_SPEC_PKG_INFO: &str = r#"
Name:     %{pkg_name}
Version:  %{pkg_version}
Release:  %{pkg_release}
Group:    %{pkg_group}
License:  %{pkg_license}
Summary:  %{pkg_summary}
Requires: %{pkg_requires}
Requires: %{manager_pkg_name}
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
if [ "$(syscare status %{patch_name})" != "NOT-APPLIED" ]; then
    echo "error: cannot remove applied patch '%{patch_name}'" >&2
    exit 1
fi

%postun
if [ "$1" != 0 ]; then
    exit 0
fi
if [ -d "%{pkg_root}" ] && [ -z "$(ls -A %{pkg_root})" ]; then
    rm -rf "%{pkg_root}"
fi
"#;

impl RpmSpecGenerator {
    fn parse_pkg_root(patch_info: &PatchInfo) -> PathBuf {
        Path::new(PATCH_INSTALL_PATH).join(
            patch_info.target.short_name()
        )
    }

    fn parse_patch_root(patch_info: &PatchInfo) -> PathBuf {
        Path::new(PATCH_INSTALL_PATH)
            .join(patch_info.target.short_name())
            .join(&patch_info.name)
    }

    fn parse_pkg_name(patch_info: &PatchInfo) -> String {
        format!("{}-{}-{}",
            PKG_PATCH_PKG_PREFIX,
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
        format!("Syscare patch \"{}\" for {}",
            patch_info.name,
            patch_info.target.short_name()
        )
    }

    fn generate_spec_file<W, I, S>(mut writer: W, patch_info: &PatchInfo, file_names: I) -> std::io::Result<()>
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
        write_line2(w, "%global manager_pkg_name      ", PKG_PATCH_PKG_REQUIRE)?;
        write_line2(w, "%global pkg_root              ", Self::parse_pkg_root(patch_info))?;
        write_line2(w, "%global pkg_name              ", Self::parse_pkg_name(patch_info))?;
        write_line2(w, "%global pkg_version           ", patch_info.version.to_string())?;
        write_line2(w, "%global pkg_release           ", &patch_info.release)?;
        write_line2(w, "%global pkg_group             ", PKG_PATCH_PKG_GROUP)?;
        write_line2(w, "%global pkg_license           ", &patch_info.license)?;
        write_line2(w, "%global pkg_summary           ", Self::parse_summary(patch_info))?;
        write_line2(w, "%global pkg_requires          ", Self::parse_requires(patch_info))?;
        write_line2(w, "%global pkg_description       ", &patch_info.description)?;
        write_line2(w, "%global patch_name            ", &patch_info.name)?;
        write_line2(w, "%global patch_root            ", Self::parse_patch_root(patch_info))?;
        write_line2(w, "%global patch_dir_permission  ", PATCH_DIR_PERMISSION)?;
        write_line2(w, "%global patch_file_permission ", PATCH_FILE_PERMISSION)?;

        write_line(w, RPM_SPEC_PKG_INFO)?;
        for file_name in file_names {
            write_line2(w, "Source: ", file_name)?;
        }
        write_line(w, RPM_SPEC_BODY)?;
        write_line(w, RPM_SPEC_SRCIPTS)?;

        writer.flush()
    }

    pub fn generate_from_patch_info<P: AsRef<Path>, Q: AsRef<Path>>(patch_info: &PatchInfo, source_dir: P, output_dir: Q) -> std::io::Result<PathBuf> {
        let spec_name = format!("{}.{}", Self::parse_pkg_name(patch_info), PKG_SPEC_EXTENSION);
        let pkg_spec_path = output_dir.as_ref().join(spec_name);

        let writer = LineWriter::new(
            fs::create_file(&pkg_spec_path)?
        );

        let file_names = fs::list_all_files(source_dir, true)?
            .into_iter()
            .map(fs::file_name)
            .collect::<Vec<_>>();

        Self::generate_spec_file(writer, patch_info, file_names)?;

        Ok(pkg_spec_path)
    }
}
