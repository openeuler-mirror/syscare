// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::Path;

use log::debug;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_cbor::{de, ser};

use crate::fs;

use super::digest;

const DEFAULT_PACK_MAGIC: &str = "481898650EF59B7D5"; // SysCare!

#[derive(Serialize, Deserialize)]
struct PackedData {
    magic: String,
    payload: Vec<u8>,
    checksum: String,
}

impl PackedData {
    fn pack<T: Serialize, S: AsRef<str>>(magic: S, obj: &T) -> std::io::Result<Self> {
        let payload = serde_cbor::to_vec(obj).map_err(|e| {
            debug!("Packing data failed, {}", e.to_string());
            std::io::Error::new(std::io::ErrorKind::Other, "Packing data failed")
        })?;
        let checksum = digest::bytes(&payload);

        Ok(Self {
            magic: magic.as_ref().to_owned(),
            payload,
            checksum,
        })
    }

    fn unpack<'a, T: for<'de> Deserialize<'de>, S: AsRef<str>>(
        &'a self,
        magic: S,
    ) -> std::io::Result<T> {
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

        de::from_slice(&self.payload).map_err(|e| {
            debug!("Unpacking data failed, {}", e.to_string());
            std::io::Error::new(std::io::ErrorKind::Other, "Unpacking data failed")
        })
    }

    fn read_from<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        de::from_reader::<Self, _>(fs::open_file(path)?).map_err(|e| {
            debug!("Deserialize packed data failed, {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Invalid data format")
        })
    }

    fn write_to<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        ser::to_writer(&mut fs::create_file(path)?, &self).map_err(|e| {
            debug!("Serialize packed data failed, {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Write data failed")
        })
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
