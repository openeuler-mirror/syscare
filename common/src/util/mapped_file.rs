use std::{fs::File, ops::Deref, os::unix::io::AsRawFd, path::Path};

use anyhow::Result;
use memmap2::{Mmap, MmapOptions};

use super::fs;

pub struct MappedFile {
    _file: File,
    mmap: Mmap,
}

impl MappedFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::open_file(path)?;
        let mmap = unsafe { MmapOptions::new().map(file.as_raw_fd())? };

        Ok(Self { _file: file, mmap })
    }
}

impl Deref for MappedFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mmap
    }
}
