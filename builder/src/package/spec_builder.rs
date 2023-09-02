use std::path::{Path, PathBuf};

use anyhow::Result;
use syscare_abi::PatchInfo;

use super::{rpm::RpmSpecBuilder, PackageFormat};

pub trait PackageSpecBuilder {
    fn build(
        &self,
        patch_info: &PatchInfo,
        source_dir: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf>;
}

pub struct PackageSpecBuilderFactory;

impl PackageSpecBuilderFactory {
    pub fn get_builder(pkg_format: PackageFormat) -> Box<dyn PackageSpecBuilder> {
        match pkg_format {
            PackageFormat::RpmPackage => Box::new(RpmSpecBuilder),
        }
    }
}
