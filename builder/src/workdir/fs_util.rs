use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir_path = path.as_ref();
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .with_context(|| format!("Failed to create directory \"{}\"", dir_path.display()))?;
    }
    Ok(())
}

pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir_path = path.as_ref();
    if dir_path.exists() {
        fs::remove_dir_all(dir_path)
            .with_context(|| format!("Failed to remove directory \"{}\"", dir_path.display()))?;
    }
    Ok(())
}
