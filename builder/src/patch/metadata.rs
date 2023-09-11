use std::path::{Path, PathBuf};

use anyhow::Result;
use syscare_abi::{PatchInfo, PATCH_INFO_MAGIC};
use syscare_common::util::serde;

use crate::package::TarPackage;

const METADATA_PKG_NAME: &str = ".syscare.tar.gz";
const METADATA_DIR_NAME: &str = ".syscare";
const METADATA_FILE_NAME: &str = "patch_info";

pub struct PatchMetadata;

impl PatchMetadata {
    pub fn metadata_dir<P: AsRef<Path>>(root_dir: P) -> PathBuf {
        root_dir.as_ref().join(METADATA_DIR_NAME)
    }

    pub fn metadata_pkg<P: AsRef<Path>>(root_dir: P) -> PathBuf {
        root_dir.as_ref().join(METADATA_PKG_NAME)
    }

    pub fn metadata_file<P: AsRef<Path>>(root_dir: P) -> PathBuf {
        Self::metadata_dir(root_dir).join(METADATA_FILE_NAME)
    }

    pub fn metadata_file_name() -> &'static str {
        METADATA_FILE_NAME
    }

    pub fn compress_tar_pkg<P: AsRef<Path>>(root_dir: P) -> Result<()> {
        TarPackage::compress(Self::metadata_pkg(&root_dir), root_dir, METADATA_DIR_NAME)
    }

    pub fn decompress_tar_pkg<P: AsRef<Path>>(root_dir: P) -> Result<()> {
        TarPackage::decompress(Self::metadata_pkg(&root_dir), root_dir)
    }

    pub fn read_from_file<P: AsRef<Path>>(file_path: P) -> std::io::Result<PatchInfo> {
        serde::deserialize_with_magic::<PatchInfo, _, _>(file_path, PATCH_INFO_MAGIC)
    }

    pub fn write_to_file<P: AsRef<Path>>(
        patch_info: &PatchInfo,
        file_path: P,
    ) -> std::io::Result<()> {
        serde::serialize_with_magic(patch_info, file_path, PATCH_INFO_MAGIC)
    }
}
