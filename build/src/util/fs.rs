use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Write, BufWriter};

use sha2::Digest;
use sha2::Sha256;

pub fn stringtify_path<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().to_string()
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

pub fn create_dir<P: AsRef<Path>>(dir_path: P) -> std::io::Result<()> {
    if self::check_dir(dir_path.as_ref()).is_err() {
        std::fs::create_dir(dir_path.as_ref())?;
    }
    Ok(())
}

pub fn create_dir_all<P: AsRef<Path>>(dir_path: P) -> std::io::Result<()> {
    if self::check_dir(dir_path.as_ref()).is_err() {
        std::fs::create_dir_all(dir_path.as_ref())?;
    }
    Ok(())
}

pub fn list_all_dirs<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            if !current_path.is_dir() {
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

pub fn list_all_files<P: AsRef<Path>>(directory: P, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            let current_path_type = current_path.metadata()?.file_type();

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

pub fn list_all_files_ext<P: AsRef<Path>>(directory: P, file_ext: &str, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    let mut file_list = Vec::new();
    let mut dir_list = Vec::new();
    for dir_entry in std::fs::read_dir(search_path)? {
        if let Ok(entry) = dir_entry {
            let current_path = entry.path();
            let current_path_type = current_path.metadata()?.file_type();

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

pub fn find_directory<P: AsRef<Path>>(directory: P, dir_name: &str, fuzz: bool, recursive: bool) -> std::io::Result<PathBuf> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    for dir in self::list_all_dirs(search_path, recursive)? {
        if let Some(curr_dir_name) = dir.file_name().and_then(OsStr::to_str) {
            if curr_dir_name == dir_name {
                return Ok(dir);
            }
            if fuzz && curr_dir_name.contains(dir_name) {
                return Ok(dir);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Cannot find directory '{}' in '{}'", dir_name, search_path.display())
    ))
}

pub fn find_file<P: AsRef<Path>>(directory: P, file_name: &str, fuzz: bool, recursive: bool) -> std::io::Result<PathBuf> {
    let search_path = directory.as_ref();

    self::check_dir(search_path)?;

    for file in self::list_all_files(search_path, recursive)? {
        if let Some(curr_file_name) = file.file_name().and_then(OsStr::to_str) {
            if curr_file_name == file_name {
                return Ok(file);
            }
            if fuzz && curr_file_name.contains(file_name) {
                return Ok(file);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Cannot find file '{}' in '{}'", file_name, search_path.display())
    ))
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

pub fn copy_all_files<P: AsRef<Path>, Q: AsRef<Path>>(src_dir: P, dst_dir: Q) -> std::io::Result<()> {
    self::check_dir(&src_dir)?;
    self::check_dir(&dst_dir)?;

    for src_file in self::list_all_files(src_dir, true)? {
        if let Some(file_name) = src_file.file_name() {
            let mut dst_file = dst_dir.as_ref().to_path_buf();
            dst_file.push(file_name);

            std::fs::copy(src_file, dst_file)?;
        }
    }

    Ok(())
}

pub fn file_name<P: AsRef<Path>>(file_path: P) -> std::io::Result<String> {
    let file = file_path.as_ref();

    self::check_file(file)?;

    match file.file_name() {
        Some(file_name) => {
            Ok(self::stringtify_path(file_name))
        },
        None => {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Parse file name from '{}' failed", file.display())
            ))
        }
    }
}

pub fn file_ext<P: AsRef<Path>>(file_path: P) -> std::io::Result<String> {
    let file = file_path.as_ref();
    self::check_file(file)?;

    let file_ext = file.extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_string();

    Ok(file_ext)
}

pub fn realpath<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    path.as_ref().canonicalize()
}

pub fn read_file_to_string<P: AsRef<Path>>(file_path: P) -> std::io::Result<String> {
    self::check_file(file_path.as_ref())?;

    let str = std::io::read_to_string(
        std::fs::File::open(file_path)?
    )?;

    Ok(str.trim().to_owned())
}

pub fn write_string_to_file<P: AsRef<Path>>(file_path: P, str: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(file_path)?;

    write!(file, "{}", str)?;

    file.flush()
}

pub fn read_file_content<P: AsRef<Path>>(file_path: P) -> std::io::Result<VecDeque<String>> {
    self::check_file(file_path.as_ref())?;

    let file = std::fs::File::open(file_path)?;

    let mut file_content = VecDeque::new();
    for read_line in BufReader::new(file).lines() {
        file_content.push_back(read_line?)
    }

    Ok(file_content)
}

pub fn write_file_content<P, I>(file_path: P, file_content: I) -> std::io::Result<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = String>
{
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(file_path)?;

    let mut writer = BufWriter::new(file);
    for line in file_content {
        writeln!(writer, "{}", line)?;
    }

    writer.flush()
}

pub fn sha256_digest_file<P: AsRef<Path>>(file: P) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(std::fs::read(file)?);
    Ok(format!("{:#x}", hasher.finalize()))
}

pub fn sha256_digest_file_list<I, P>(file_list: I) -> std::io::Result<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>
{
    let mut hasher = Sha256::new();
    for file in file_list {
        hasher.update(std::fs::read(file)?);
    }
    Ok(format!("{:#x}", hasher.finalize()))
}
