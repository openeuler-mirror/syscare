use std::path::Path;

use serde::{Serialize, Deserialize, de::DeserializeOwned};

use super::fs;

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

    pub fn serialize<P, T>(obj: T, path: P, version: &str) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        T: Serialize,
    {
        let vdata  = VersionedData {
            version: version.to_owned(),
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

    pub fn deserialize<P, T>(path: P, version: &str) -> std::io::Result<T>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let binary  = fs::read(path)?;
        let data_version = bincode::deserialize::<String>(&binary).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialize data version failed")
            )
        })?;
        if data_version != version {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Data version \"{}\" does not match expected version \"{}\"", data_version, version)
            ));
        }

        let version_len = bincode::serialized_size(&data_version).unwrap() as usize;
        let data = bincode::deserialize::<T>(&binary[version_len..]).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialize data failed")
            )
        })?;

        Ok(data)
    }
}
