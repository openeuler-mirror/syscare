use std::path::Path;

use anyhow::Result;

use crate::BuildParameters;

use super::{rpm::RpmPackageBuilder, PackageBuildRoot, PackageFormat};

pub trait PackageBuilder {
    fn build_prepare(&self, spec_file: &Path) -> Result<()>;
    fn build_source_package(
        &self,
        build_params: &BuildParameters,
        spec_file: &Path,
        output_dir: &Path,
    ) -> Result<()>;
    fn build_binary_package(&self, spec_file: &Path, output_dir: &Path) -> Result<()>;
}

pub struct PackageBuilderFactory;

impl PackageBuilderFactory {
    pub fn get_builder(
        pkg_format: PackageFormat,
        build_root: &PackageBuildRoot,
    ) -> Box<dyn PackageBuilder + '_> {
        match pkg_format {
            PackageFormat::RpmPackage => Box::new(RpmPackageBuilder::new(build_root)),
        }
    }
}
