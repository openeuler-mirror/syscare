use std::path::Path;

use sha2::Digest;
use sha2::Sha256;

use super::fs;

pub fn bytes<S: AsRef<[u8]>>(bytes: S) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    format!("{:#x}", hasher.finalize())
}

pub fn file<P: AsRef<Path>>(file: P) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(fs::read(file)?);

    Ok(format!("{:#x}", hasher.finalize()))
}

pub fn file_list<I, P>(file_list: I) -> std::io::Result<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut hasher = Sha256::new();
    for file in file_list {
        hasher.update(fs::read(file)?);
    }

    Ok(format!("{:#x}", hasher.finalize()))
}

pub fn dir<P: AsRef<Path>>(directory: P) -> std::io::Result<String> {
    file_list(fs::list_files(
        directory,
        fs::TraverseOptions { recursive: true },
    )?)
}
