use std::fs::{self, File};

use crate::tool::*;
use super::Result;

pub struct WorkDir {
    cache_dir: String,
    source_dir: String,
    patch_dir: String,
    output_dir: String,
    binary_dir: String,
    log_file: String,
}

impl WorkDir {
    pub fn new() -> Self {
        Self {
            cache_dir: String::new(),
            source_dir: String::new(),
            patch_dir: String::new(),
            output_dir: String::new(),
            binary_dir: String::new(),
            log_file: String::new(),
        }
    }

    pub fn create_dir(&mut self, work_dir: String) -> Result<()> {
        self.cache_dir = work_dir.clone();
        self.source_dir.push_str(&format!("{}/{}", &self.cache_dir, "source"));
        self.patch_dir.push_str(&format!("{}/{}", &self.cache_dir, "patch"));
        self.output_dir.push_str(&format!("{}/{}", &self.cache_dir, "output"));
        self.binary_dir.push_str(&format!("{}/{}", &self.cache_dir, "binary"));
        self.log_file.push_str(&format!("{}/{}", &self.cache_dir, "build.log"));

        if let Ok(()) = self::check_dir(&self.cache_dir) {
            fs::remove_dir_all(self.cache_dir.clone())?;
        }

        fs::create_dir_all(self.cache_dir.clone())?;
        fs::create_dir(self.source_dir.clone())?;
        fs::create_dir(self.patch_dir.clone())?;
        fs::create_dir(self.output_dir.clone())?;
        fs::create_dir(self.binary_dir.clone())?;
        File::create(&self.log_file)?;
        Ok(())
    }


    pub fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    pub fn source_dir(&self) -> &str {
        &self.source_dir
    }

    pub fn patch_dir(&self) -> &str {
        &self.patch_dir
    }

    pub fn output_dir(&self) -> &str {
        &self.output_dir
    }

    pub fn binary_dir(&self) -> &str {
        &self.binary_dir
    }

    pub fn log_file(&self) -> &str {
        &self.log_file
    }
}
