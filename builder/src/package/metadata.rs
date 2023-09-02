use std::path::{Path, PathBuf};

use anyhow::Result;

use super::tar::TarPackage;

const METADATA_FILE_NAME: &str = ".syscare.tar.gz";
const METADATA_DIR_NAME: &str = ".syscare";

pub struct PackageMetadata;

impl PackageMetadata {
    pub fn metadata_dir<P: AsRef<Path>>(pkg_source_dir: P) -> PathBuf {
        pkg_source_dir.as_ref().join(METADATA_DIR_NAME)
    }

    pub fn metadata_file<P: AsRef<Path>>(pkg_source_dir: P) -> PathBuf {
        Self::metadata_dir(pkg_source_dir).join(METADATA_FILE_NAME)
    }

    pub fn compress<P: AsRef<Path>>(pkg_source_dir: P) -> Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_FILE_NAME);

        TarPackage::compress(metadata_file, metadata_source_dir, METADATA_DIR_NAME)
    }

    pub fn decompress<P: AsRef<Path>>(pkg_source_dir: P) -> Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_FILE_NAME);

        TarPackage::decompress(metadata_file, metadata_source_dir)
    }
}
