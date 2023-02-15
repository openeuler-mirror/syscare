use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

pub fn serialize<P: AsRef<Path>, T: Serialize>(obj: T, path: P) -> std::io::Result<()> {
    std::fs::write(
        path,
        bincode::serialize(&obj).map_err(|_| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Serialize {} failed", std::any::type_name::<T>())
        ))?
    )
}

pub fn deserialize<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> std::io::Result<T> {
    bincode::deserialize_from(std::fs::File::open(&path)?).map_err(|_| std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Deserialize {} from \"{}\" failed", std::any::type_name::<T>(), path.as_ref().display())
    ))
}
