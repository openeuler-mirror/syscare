use std::{fs::File, path::Path};

use anyhow::{Context, Result};

mod ffi {
    use std::{fs::File, os::unix::io::AsRawFd};

    pub fn flock_exclusive(file: &File) -> std::io::Result<()> {
        let ret_code = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret_code != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn flock_unlock(file: &File) -> std::io::Result<()> {
        let ret_code = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        if ret_code != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

pub struct ExclusiveFileLockGuard {
    file: File,
}

impl ExclusiveFileLockGuard {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_path = path.as_ref();
        let file = match file_path.exists() {
            false => File::create(file_path).context("Failed to create lock file")?,
            true => File::open(file_path).context("Failed to open lock file")?,
        };
        let instance = Self { file };
        instance.acquire()?;

        Ok(instance)
    }

    fn acquire(&self) -> Result<()> {
        ffi::flock_exclusive(&self.file).context("Failed to acquire exclusive file lock")
    }

    fn release(&self) -> Result<()> {
        ffi::flock_unlock(&self.file).context("Failed to release exclusive file lock")
    }
}

impl Drop for ExclusiveFileLockGuard {
    fn drop(&mut self) {
        self.release().ok();
    }
}
