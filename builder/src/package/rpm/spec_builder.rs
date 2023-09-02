use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;

use syscare_abi::PatchInfo;
use syscare_common::util::fs;

use crate::package::spec_builder::PackageSpecBuilder;

use super::{
    spec_file::RpmSpecFile,
    tags::{RpmChangeLog, RpmDefAttr, RpmDefine, RpmPath},
    SPEC_FILE_EXT, SPEC_TAG_VALUE_NONE,
};

pub struct RpmSpecBuilder;

const PKG_DEFINE_PATCH_UUID: &str = "patch_uuid";
const PKG_DEFINE_PATCH_NAME: &str = "patch_name";
const PKG_DEFINE_PATCH_ROOT: &str = "patch_root";

const PKG_GROUP: &str = "Patch";
const PKG_REQUIRE: &str = "syscare";
const PKG_AUTHOR: &str = "syscare";
const PKG_CHANGE_LOG: &str = "Automatic generated patch";

const PKG_INSTALL_DIR: &str = "/usr/lib/syscare/patches";
const PKG_USER_NAME: &str = "root";
const PKG_GROUP_NAME: &str = "root";
const PKG_FILE_MODE: u32 = 0o640;
const PKG_DIR_MODE: u32 = 0o750;

const PKG_SCRIPT_PREP: &str = r#"cp -a "%{_sourcedir}"/* "%{_builddir}""#;
const PKG_SCRIPT_INSTALL: &str = r#"install -d "%{buildroot}%{patch_root}"
for file in $(ls -A "%{_builddir}"); do
    install "%{_builddir}/$file" "%{buildroot}%{patch_root}"
done"#;
const PKG_SCRIPT_PREUN: &str =
    r#"syscare remove '%{patch_uuid}' || echo "Failed to remove patch '%{patch_name}'" >&2"#;

impl RpmSpecBuilder {
    fn parse_requires(patch_info: &PatchInfo) -> String {
        match patch_info.target.epoch.as_str() {
            SPEC_TAG_VALUE_NONE => {
                format!(
                    "{} = {}-{}",
                    patch_info.target.name, patch_info.target.version, patch_info.target.release
                )
            }
            _ => {
                format!(
                    "{} = {}:{}-{}",
                    patch_info.target.name,
                    patch_info.target.epoch,
                    patch_info.target.version,
                    patch_info.target.release
                )
            }
        }
    }

    fn parse_summary(patch_info: &PatchInfo) -> String {
        format!(
            "Syscare patch \"{}\" for {}",
            patch_info.name,
            patch_info.target.short_name()
        )
    }

    fn create_pkg_spec<I, P>(patch_info: &PatchInfo, pkg_file_list: I) -> RpmSpecFile
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let pkg_name = format!(
            "patch-{}-{}",
            patch_info.target.short_name(),
            patch_info.name
        );
        let pkg_version = format!("{}-{}", patch_info.version, patch_info.release);
        let pkg_root = Path::new(PKG_INSTALL_DIR).join(&patch_info.uuid);

        let mut spec = RpmSpecFile::new(
            pkg_name,
            patch_info.version.clone(),
            patch_info.release.to_string(),
            patch_info.target.license.clone(),
            Self::parse_summary(patch_info),
            patch_info.description.clone(),
        );
        spec.defines.insert(RpmDefine {
            name: PKG_DEFINE_PATCH_UUID.to_owned(),
            value: patch_info.uuid.clone(),
        });
        spec.defines.insert(RpmDefine {
            name: PKG_DEFINE_PATCH_NAME.to_owned(),
            value: format!("{}/{}", patch_info.target.short_name(), patch_info.name),
        });
        spec.defines.insert(RpmDefine {
            name: PKG_DEFINE_PATCH_ROOT.to_owned(),
            value: pkg_root.to_string_lossy().to_string(),
        });
        spec.group = Some(PKG_GROUP.to_owned());
        spec.requires.insert(Self::parse_requires(patch_info));
        spec.requires.insert(PKG_REQUIRE.to_string());
        spec.prep = PKG_SCRIPT_PREP.to_owned();
        spec.install = PKG_SCRIPT_INSTALL.to_owned();
        spec.preun = Some(PKG_SCRIPT_PREUN.to_owned());
        spec.defattr = Some(RpmDefAttr {
            file_mode: PKG_FILE_MODE,
            user: PKG_USER_NAME.to_owned(),
            group: PKG_GROUP_NAME.to_owned(),
            dir_mode: PKG_DIR_MODE,
        });
        spec.change_log = Some(RpmChangeLog {
            date: Local::now(),
            author: PKG_AUTHOR.to_owned(),
            version: pkg_version,
            records: vec![PKG_CHANGE_LOG.to_owned()],
        });
        spec.files.insert(RpmPath::Directory(pkg_root.clone()));
        for pkg_file in pkg_file_list {
            let orig_file_path = pkg_file.as_ref();
            let new_file_path = pkg_root.join(fs::file_name(orig_file_path));

            if orig_file_path.is_dir() {
                spec.files.insert(RpmPath::Directory(new_file_path));
                continue;
            }
            if orig_file_path.is_file() {
                spec.files.insert(RpmPath::File(new_file_path));
                continue;
            }
        }

        spec
    }
}

impl PackageSpecBuilder for RpmSpecBuilder {
    fn build(
        &self,
        patch_info: &PatchInfo,
        source_dir: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let pkg_spec_file = Self::create_pkg_spec(
            patch_info,
            fs::list_files(source_dir, fs::TraverseOptions { recursive: true })
                .context("Failed to list packge files")?,
        );
        let spec_file_path = output_dir.join(format!("{}.{}", pkg_spec_file.name, SPEC_FILE_EXT));

        let mut writer =
            BufWriter::new(fs::create_file(&spec_file_path).context("Failed to create spec file")?);
        write!(writer, "{}", pkg_spec_file).context("Failed to write spec file")?;
        writer.flush()?;

        Ok(spec_file_path)
    }
}
