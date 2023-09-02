use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{rpm::RpmSpecWriter, PackageFormat};

pub trait PackageSpecWriter {
    fn add_source_files(&self, spec_file: &Path, file_list: Vec<PathBuf>) -> Result<()>;
}

pub struct PackageSpecWriterFactory;

impl PackageSpecWriterFactory {
    pub fn get_writer(pkg_format: PackageFormat) -> Box<dyn PackageSpecWriter> {
        match pkg_format {
            PackageFormat::RpmPackage => Box::new(RpmSpecWriter),
        }
    }
}
