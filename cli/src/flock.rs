use std::{fs::File, path::Path};

use anyhow::{bail, Context, Result};

mod ffi {
    use std::{fs::File, os::unix::io::AsRawFd};

    pub fn flock_exclusive_unblock(file: &File) -> bool {
        let ret_code = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        ret_code == 0
    }

    pub fn flock_unlock(file: &File) -> bool {
        let ret_code = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        ret_code == 0
    }
}

pub struct ExclusiveFileLockGuard {
    file: File,
}

impl ExclusiveFileLockGuard {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_path = path.as_ref();
        let file = match file_path.exists() {
            false => File::create(file_path)
                .with_context(|| format!("Failed to create file \"{}\"", file_path.display()))?,
            true => File::open(file_path)
                .with_context(|| format!("Failed to open file \"{}\"", file_path.display()))?,
        };
        let instance = Self { file };
        instance.lock()?;

        Ok(instance)
    }

    fn lock(&self) -> Result<()> {
        if !ffi::flock_exclusive_unblock(&self.file) {
            bail!("Failed to acquire exclusive lock")
        }
        Ok(())
    }

    fn release(&self) -> Result<()> {
        if !ffi::flock_unlock(&self.file) {
            bail!("Failed to unlock exclusive lock")
        }
        Ok(())
    }
}

impl Drop for ExclusiveFileLockGuard {
    fn drop(&mut self) {
        self.release().ok();
    }
}
