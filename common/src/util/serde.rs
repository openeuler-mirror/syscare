use std::path::Path;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{digest, fs};

const DEFAULT_PACK_MAGIC: &str = "481898650EF59B7D5"; // SysCare!

#[derive(Serialize, Deserialize)]
struct PackedData {
    magic: String,
    payload: Vec<u8>,
    checksum: String,
}

impl PackedData {
    fn pack<T: Serialize, S: AsRef<str>>(magic: S, obj: &T) -> std::io::Result<Self> {
        let payload = serde_cbor::to_vec(obj).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Serialize data failed"))?;
        let checksum = digest::bytes(&payload);

        Ok(Self {
            magic: magic.as_ref().to_owned(),
            payload,
            checksum,
        })
    }

    fn unpack<'a, T: Deserialize<'a>, S: AsRef<str>>(&'a self, magic: S) -> std::io::Result<T> {
        if self.magic != magic.as_ref() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Data magic check failed",
            ));
        }

        if self.checksum != digest::bytes(&self.payload) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Data checksum failed",
            ));
        }

        serde_cbor::from_slice(&self.payload)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialize data failed"))
    }

    fn read_from<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        serde_cbor::from_reader(fs::open_file(path)?)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Invalid data format"))
    }

    fn write_to<P: AsRef<Path>>(self, path: P) -> std::io::Result<()> {
        serde_cbor::to_writer(fs::create_file(path)?, &self)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Serialize data failed"))
    }
}

pub fn serialize_with_magic<T, P, S>(obj: &T, path: P, magic: S) -> std::io::Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
    S: AsRef<str>,
{
    PackedData::pack(magic, obj)?.write_to(path)
}

pub fn deserialize_with_magic<T, P, S>(path: P, magic: S) -> std::io::Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
    S: AsRef<str>,
{
    PackedData::read_from(path)?.unpack(magic)
}

#[inline]
pub fn serialize<T, P>(obj: &T, path: P) -> std::io::Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    self::serialize_with_magic(obj, path, DEFAULT_PACK_MAGIC)
}

#[inline]
pub fn deserialize<T, P>(path: P) -> std::io::Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    self::deserialize_with_magic(path, DEFAULT_PACK_MAGIC)
}
