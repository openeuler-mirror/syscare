use std::ffi::{OsString, OsStr};
use std::path::{Path, PathBuf};
use std::os::unix::ffi::OsStrExt;

use crate::tool::os_str::OsStrContains;

pub fn stringtify<P: AsRef<Path>>(path: P) -> String {
    format!("{}", path.as_ref().display())
}

pub fn check_exist<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let path_ref = path.as_ref();
    if !path_ref.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path '{}' is not exist", path_ref.display())
        ));
    }
    Ok(())
}

pub fn check_dir<P: AsRef<Path>>(dir_path: P) -> std::io::Result<()> {
    let path = dir_path.as_ref();

    self::check_exist(path)?;
    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path '{}' is not a directory", path.display())
        ));
    }

    Ok(())
}

pub fn check_file<P: AsRef<Path>>(file_path: P) -> std::io::Result<()> {
    let path = file_path.as_ref();

    self::check_exist(path)?;
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path '{}' is not a file", path.display())
        ));
    }

    Ok(())
}

pub fn file_name<P: AsRef<Path>>(file_path: P) -> std::io::Result<OsString> {
    let file = file_path.as_ref();

    self::check_exist(file)?;

    match file.file_name() {
        Some(file_name) => {
            Ok(file_name.to_os_string())
        },
        None => {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Parse file name from '{}' failed", file.display())
            ))
        }
    }
}

pub fn list_all_files<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            let current_path_type = current_path.symlink_metadata()?.file_type();

            if current_path_type.is_symlink() {
                continue;
            }
            if current_path_type.is_file() {
                file_list.push(self::realpath(current_path.as_path())?);
            }
            if current_path_type.is_dir() {
                dir_list.push(self::realpath(current_path.as_path())?);
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

pub fn list_all_dirs<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            let current_path_type = current_path.symlink_metadata()?.file_type();

            if current_path_type.is_symlink() {
                continue;
            }
            if !current_path_type.is_dir() {
                continue;
            }
            dir_list.push(self::realpath(current_path.as_path())?);
        }
    }

    if recursive {
        for dir in dir_list.clone() {
            dir_list.append(&mut self::list_all_dirs(dir, recursive)?);
        }
    }

    Ok(dir_list)
}

pub fn list_all_files_ext<P: AsRef<Path>>(directory: P, file_ext: &str, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            let current_path_type = current_path.symlink_metadata()?.file_type();

            if current_path_type.is_symlink() {
                continue;
            }
            if current_path_type.is_file() {
                let current_path_ext = current_path.extension().unwrap_or_default();
                if current_path_ext == file_ext {
                    file_list.push(self::realpath(current_path.as_path())?);
                }
            }
            if current_path_type.is_dir() {
                dir_list.push(self::realpath(current_path.as_path())?);
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

pub fn find_file_ext<P: AsRef<Path>>(directory: P, file_ext: &str, recursive: bool) -> std::io::Result<PathBuf> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    for file in self::list_all_files_ext(search_path, file_ext, recursive)? {
        if let Some(currrent_file_ext) = file.extension().and_then(OsStr::to_str) {
            if currrent_file_ext == file_ext {
                return Ok(file);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Cannot find '*.{}' file in '{}'", file_ext, search_path.display())
    ))
}

pub fn realpath<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    path.as_ref().canonicalize()
}

pub fn find_files<P: AsRef<Path>, Q: AsRef<Path>>(directory: P, file_name: Q, fuzz: bool, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();
    let file_name = file_name.as_ref();
    let mut file_list = Vec::new();

    self::check_dir(search_path)?;

    for file in self::list_all_files(search_path, recursive)? {
        if let Ok(curr_file_name) = self::file_name(file.as_path()) {
            if curr_file_name == file_name {
                file_list.push(file);
            }
            else if fuzz && curr_file_name.contains(file_name.as_os_str().as_bytes()) {
                file_list.push(file);
            }
        }
    }
    Ok(file_list)
}

pub fn search_tool<P: AsRef<Path>>(tool: P) -> std::io::Result<PathBuf> {
    let current_tool = tool.as_ref();
    match self::check_file(&current_tool) {
        Err(e) => return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("can't find supporting tools {}: {}", current_tool.display(), e)
        )),
        Ok(()) => realpath(current_tool),
    }
}

pub fn real_arg<P: AsRef<Path>>(name: P) -> std::io::Result<PathBuf> {
    let path = name.as_ref();
    match realpath(path) {
        Ok(result) => Ok(result),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is InvalidInput, {}", path.display(), e),
        )),
    }
}