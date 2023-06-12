use std::path::Path;

use flexbuffers::FlexbufferSerializer;
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
        let mut serializer = FlexbufferSerializer::new();
        obj.serialize(&mut serializer)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Serialize data failed"))?;

        let payload = serializer.take_buffer();
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

        flexbuffers::from_slice::<T>(&self.payload)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialize data failed"))
    }

    fn read_from<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        flexbuffers::from_slice(&fs::read(path)?)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Invalid data format"))
    }

    fn write_to<P: AsRef<Path>>(self, path: P) -> std::io::Result<()> {
        let mut serializer = FlexbufferSerializer::new();
        self.serialize(&mut serializer)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Serialize data failed"))?;

        fs::write(path, serializer.take_buffer())
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
