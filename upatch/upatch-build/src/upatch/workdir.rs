use std::fs::{self, File};
use std::{path::Path, env};

use super::Result;

pub struct WorkDir {
    cache_dir: String,
    source_dir: String,
    patch_dir: String,
    output_dir: String,
    log_file: String,
}

impl WorkDir {
    pub fn new() -> Self {
        Self {
            cache_dir: String::new(),
            source_dir: String::new(),
            patch_dir: String::new(),
            output_dir: String::new(),
            log_file: String::new(),
        }
    }

    pub fn create_dir(&mut self, work_dir: String) -> Result<()> {
        #![allow(deprecated)]
        if work_dir.is_empty(){
            // home_dir() don't support BSD system
            self.cache_dir.push_str(&format!("{}/{}", env::home_dir().unwrap().to_str().unwrap(), ".upatch"));
        }
        else{
            self.cache_dir.push_str(&work_dir);
        }

        self.source_dir.push_str(&format!("{}/{}", &self.cache_dir, "source"));
        self.patch_dir.push_str(&format!("{}/{}", &self.cache_dir, "patch"));
        self.output_dir.push_str(&format!("{}/{}", &self.cache_dir, "output"));
        self.log_file.push_str(&format!("{}/{}", &self.cache_dir, "build.log"));

        if Path::new(&self.cache_dir).is_dir() {
            fs::remove_dir_all(self.cache_dir.clone())?;
        }

        fs::create_dir_all(self.cache_dir.clone())?;
        fs::create_dir(self.source_dir.clone())?;
        fs::create_dir(self.patch_dir.clone())?;
        fs::create_dir(self.output_dir.clone())?;
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

    pub fn log_file(&self) -> &str {
        &self.log_file
    }
}
