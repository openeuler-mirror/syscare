use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::fs::{Metadata, Permissions, ReadDir, File};

trait RewriteError {
    fn rewrite_err(self, err_msg: String) -> Self;
}

impl<T> RewriteError for std::io::Result<T> {
    #[inline]
    fn rewrite_err(self, err_msg: String) -> Self {
        self.map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("{}, {}", err_msg, e.to_string()).to_lowercase()
            )
        })
    }
}

/* std::fs functions */
#[inline]
pub fn read<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<u8>> {
    std::fs::read(path.as_ref()).rewrite_err(
        format!("cannot read \"{}\"",
            path.as_ref().display()
        )
    )
}

#[inline]
pub fn read_to_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    std::fs::read_to_string(path.as_ref()).rewrite_err(
        format!("cannot read \"{}\"",
            path.as_ref().display()
        )
    )
}

#[inline]
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> std::io::Result<()> {
    std::fs::write(path.as_ref(), contents).rewrite_err(
        format!("cannot write \"{}\"",
            path.as_ref().display()
        )
    )
}

#[inline]
pub fn remove_file<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    std::fs::remove_file(path.as_ref()).rewrite_err(
        format!("cannot remove \"{}\"",
            path.as_ref().display()
        )
    )
}

#[inline]
pub fn metadata<P: AsRef<Path>>(path: P) -> std::io::Result<Metadata> {
    std::fs::metadata(path.as_ref()).rewrite_err(
        format!("cannot access \"{}\"", path.as_ref().display())
    )
}

#[inline]
pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<Metadata> {
    std::fs::symlink_metadata(path.as_ref()).rewrite_err(
        format!("cannot access \"{}\"", path.as_ref().display())
    )
}

#[inline]
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> std::io::Result<()> {
    std::fs::rename(&from, &to).rewrite_err(
        format!("cannot rename \"{}\" to \"{}\"",
            from.as_ref().display(),
            to.as_ref().display()
        )
    )
}

#[inline]
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> std::io::Result<u64> {
    std::fs::copy(&from, &to).rewrite_err(
        format!("cannot rename \"{}\" to \"{}\"",
            from.as_ref().display(),
            to.as_ref().display()
        )
    )
}

#[inline]
pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> std::io::Result<()> {
    std::fs::hard_link(original.as_ref(), link.as_ref()).rewrite_err(
        format!("cannot link \"{}\" to \"{}\"",
            original.as_ref().display(),
            link.as_ref().display()
        )
    )
}

#[inline]
pub fn soft_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> std::io::Result<()> {
    // std::fs::soft_link() is deprecated, use std::os::unix::fs::symlink instead
    std::os::unix::fs::symlink(original.as_ref(), link.as_ref()).rewrite_err(
        format!("cannot link \"{}\" to \"{}\"",
            original.as_ref().display(),
            link.as_ref().display()
        )
    )
}

#[inline]
pub fn read_link<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    std::fs::read_link(path.as_ref()).rewrite_err(
        format!("cannot read symbol link \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path.as_ref()).rewrite_err(
        format!("cannot canonicalize \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn create_dir<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    std::fs::create_dir(path.as_ref()).rewrite_err(
        format!("cannot create directory \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    std::fs::create_dir_all(path.as_ref()).rewrite_err(
        format!("cannot create directory \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn remove_dir<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    std::fs::remove_dir(path.as_ref()).rewrite_err(
        format!("cannot remove directory \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    std::fs::remove_dir_all(path.as_ref()).rewrite_err(
        format!("cannot remove directory \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn read_dir<P: AsRef<Path>>(path: P) -> std::io::Result<ReadDir> {
    std::fs::read_dir(path.as_ref()).rewrite_err(
        format!("cannot read directory \"{}\"",
            path.as_ref().display(),
        )
    )
}

#[inline]
pub fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> std::io::Result<()> {
    std::fs::set_permissions(path.as_ref(), perm).rewrite_err(
        format!("cannot set permission to \"{}\"",
            path.as_ref().display(),
        )
    )
}

/* Extended functions */
pub fn create_file<P: AsRef<Path>>(path: P) -> std::io::Result<File> {
    std::fs::File::create(&path).rewrite_err(
        format!("cannot create file \"{}\"",
            path.as_ref().display(),
        )
    )
}

pub fn open_file<P: AsRef<Path>>(path: P) -> std::io::Result<File> {
    std::fs::File::open(&path).rewrite_err(
        format!("cannot open file \"{}\"",
            path.as_ref().display(),
        )
    )
}

pub fn file_name<P: AsRef<Path>>(path: P) -> OsString {
    path.as_ref()
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default()
}

pub fn file_ext<P: AsRef<Path>>(path: P) -> OsString {
    path.as_ref()
        .extension()
        .map(OsStr::to_os_string)
        .unwrap_or_default()
}

pub fn list_all_dirs<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut dir_list = Vec::new();

    for dir_entry in self::read_dir(directory)? {
        if let Ok(entry) = dir_entry {
            let path = entry.path();

            if !self::symlink_metadata(&path)?.file_type().is_dir() {
                continue;
            }
            dir_list.push(path);
        }
    }

    if recursive {
        for dir in dir_list.clone() {
            dir_list.append(&mut self::list_all_dirs(dir, recursive)?);
        }
    }

    Ok(dir_list)
}

pub fn list_all_files<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();

    for dir_entry in self::read_dir(directory)? {
        if let Ok(entry) = dir_entry {
            let path = entry.path();

            let path_type = self::symlink_metadata(&path)?.file_type();
            if path_type.is_symlink() {
                continue;
            }
            else if path_type.is_file() {
                file_list.push(path);
            }
            else if path_type.is_dir() {
                dir_list.push(path);
            }
        }
    }

    if recursive {
        for dir in dir_list.as_slice() {
            file_list.append(&mut self::list_all_files(dir, recursive)?);
        }
    }

    Ok(file_list)
}

pub fn list_all_files_ext<P: AsRef<Path>>(directory: P, file_ext: &str, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();

    for dir_entry in self::read_dir(directory)? {
        if let Ok(entry) = dir_entry {
            let path = entry.path();

            let path_type = self::symlink_metadata(&path)?.file_type();
            if path_type.is_symlink() {
                continue;
            }
            else if path_type.is_file() {
                if file_ext == path.extension().unwrap_or_default() {
                    file_list.push(path);
                }
            }
            else if path_type.is_dir() {
                dir_list.push(path);
            }
        }
    }

    if recursive {
        for dir in dir_list.as_slice() {
            file_list.append(
                &mut self::list_all_files_ext(dir, file_ext, recursive)?
            );
        }
    }

    Ok(file_list)
}

pub fn find_dir<P: AsRef<Path>>(directory: P, name: &str, fuzz: bool, recursive: bool) -> std::io::Result<PathBuf> {
    for path in self::list_all_dirs(&directory, recursive)? {
        if let Some(dir_name) = path.file_name() {
            if dir_name == name {
                return Ok(self::canonicalize(path)?);
            }
            // FIXME: OsStr::to_string_lossy() may loss non UTF-8 chars
            else if fuzz && dir_name.to_string_lossy().contains(name) {
                return Ok(self::canonicalize(path)?);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("cannot find file \"{}\" in \"{}\"", name, directory.as_ref().display())
    ))
}

pub fn find_file<P: AsRef<Path>>(directory: P, name: &str, fuzz: bool, recursive: bool) -> std::io::Result<PathBuf> {
    for path in self::list_all_files(&directory, recursive)? {
        if let Some(file_name) = path.file_name() {
            if file_name == name {
                return Ok(self::canonicalize(path)?);
            }
            // FIXME: OsStr::to_string_lossy() may loss non UTF-8 chars
            else if fuzz && file_name.to_string_lossy().contains(name) {
                return Ok(self::canonicalize(path)?);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("cannot find file \"{}\" in \"{}\"", name, directory.as_ref().display())
    ))
}

pub fn find_file_ext<P: AsRef<Path>>(directory: P, ext: &str, recursive: bool) -> std::io::Result<PathBuf> {
    for file in self::list_all_files_ext(&directory, ext, recursive)? {
        return Ok(self::canonicalize(file)?);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("cannot find '*.{}' file in \"{}\"", ext, directory.as_ref().display())
    ))
}

pub fn copy_dir_all<P: AsRef<Path>, Q: AsRef<Path>>(src_dir: P, dst_dir: Q) -> std::io::Result<()> {
    let dst_buf = dst_dir.as_ref().to_path_buf();

    for src_file in self::list_all_files(src_dir, true)? {
        let mut dst_file = dst_buf.clone();
        dst_file.push(src_file.file_name().unwrap_or_default());

        self::copy(src_file, dst_file)?;
    }

    Ok(())
}
