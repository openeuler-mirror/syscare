use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use super::fs;

pub fn serialize<P: AsRef<Path>, T: Serialize>(obj: T, path: P) -> std::io::Result<()> {
    fs::write(
        path,
        bincode::serialize(&obj).map_err(|_| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("serialize \"{}\" failed", std::any::type_name::<T>())
        ))?
    )
}

pub fn deserialize<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> std::io::Result<T> {
    bincode::deserialize_from(fs::open_file(&path)?).map_err(|_| std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("deserialize \"{}\" failed", std::any::type_name::<T>())
    ))
}
