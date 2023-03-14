use std::path::Path;

use serde::{Serialize, Deserialize, de::DeserializeOwned};

use super::fs;

const MODULE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod serde_unversioned {
    use super::*;

    pub fn serialize<P, T>(obj: T, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        T: Serialize,
    {
        let binary = bincode::serialize(&obj).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Serialize data failed")
            )
        })?;

        fs::write(path, binary)
    }

    pub fn deserialize<P, T>(path: P) -> std::io::Result<T>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let binary = fs::read(path)?;
        bincode::deserialize::<T>(&binary).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialize data failed")
            )
        })
    }
}

pub mod serde_versioned {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct VersionedData<T> {
        version: String,
        data: T
    }

    pub fn serialize<P, T>(obj: T, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        T: Serialize,
    {
        let vdata  = VersionedData {
            version: MODULE_VERSION.to_owned(),
            data:    obj
        };
        let binary = bincode::serialize(&vdata).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Serialize data failed")
            )
        })?;

        fs::write(path, binary)
    }

    pub fn deserialize<P, T>(path: P) -> std::io::Result<T>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let binary  = fs::read(path)?;
        let version = bincode::deserialize::<String>(&binary).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialize file version failed")
            )
        })?;
        if version != MODULE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Version \"{}\" mismatched", version)
            ));
        }

        let version_len = bincode::serialized_size(&version).unwrap() as usize;
        let data = bincode::deserialize::<T>(&binary[version_len..]).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialize data failed")
            )
        })?;

        Ok(data)
    }
}
