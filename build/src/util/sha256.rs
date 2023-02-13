use std::path::Path;

use sha2::Digest;
use sha2::Sha256;

pub fn file_digest<P: AsRef<Path>>(file: P) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(std::fs::read(file)?);

    Ok(format!("{:#x}", hasher.finalize()))
}

pub fn file_list_digest<I, P>(file_list: I) -> std::io::Result<String>
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
