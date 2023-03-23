use std::ffi::OsStr;
use std::path::{PathBuf, Path};

use crate::util::fs;

#[inline(always)]
fn find_disk<P: AsRef<Path>, S: AsRef<OsStr>>(directory: P, name: S) -> std::io::Result<PathBuf> {
    #[inline(always)]
    fn __find_disk(directory: &Path, name: &OsStr) -> std::io::Result<PathBuf> {
        let dev = fs::find_symlink(
            directory,
            name,
            fs::FindOptions { fuzz: false, recursive: false }
        )?;
        fs::canonicalize(dev)
    }

    __find_disk(directory.as_ref(), name.as_ref()).map_err(|_| std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Cannot find block device by label \"{}\"", name.as_ref().to_string_lossy())
    ))
}

pub fn find_by_id<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-id", name)
}

pub fn find_by_label<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-label", name)
}

pub fn find_by_uuid<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-uuid", name)
}

pub fn find_by_partuuid<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-partuuid", name)
}

pub fn find_by_path<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-path", name)
}
