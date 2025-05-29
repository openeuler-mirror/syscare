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

use nix::errno::Errno;
use sha2::{Digest, Sha256};

use crate::fs;

pub fn bytes<T: AsRef<[u8]>>(bytes: T) -> String {
    format!("{:#x}", Sha256::digest(bytes))
}

pub fn file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let file_path = path.as_ref();
    if !file_path.is_file() {
        return Err(std::io::Error::from(Errno::EINVAL));
    }
    Ok(self::bytes(&*fs::mmap(file_path)?))
}

pub fn file_list<I, P>(file_list: I) -> std::io::Result<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut hasher = Sha256::new();

    for file in file_list {
        let file_path = file.as_ref();
        if file_path.is_file() {
            hasher.update(&*fs::mmap(file)?);
        }
    }

    Ok(format!("{:#x}", hasher.finalize()))
}

pub fn dir<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let dir_path = path.as_ref();
    if !dir_path.is_dir() {
        return Err(std::io::Error::from(Errno::EINVAL));
    }

    let mut file_list = fs::list_files(path, fs::TraverseOptions { recursive: true })?;
    file_list.sort_unstable();

    self::file_list(file_list)
}

pub fn path<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    if path.as_ref().is_file() {
        self::file(path)
    } else {
        self::dir(path)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::Write,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn unique_name(prefix: &str) -> std::io::Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(format!("{}_{}", prefix, timestamp.as_nanos()))
    }

    fn create_temp_file(content: &[u8]) -> std::io::Result<PathBuf> {
        let temp_file = std::env::temp_dir().join(self::unique_name("digest_test")?);

        let mut file = File::create(&temp_file)?;
        file.write_all(content)?;

        Ok(temp_file)
    }

    fn create_temp_dir() -> std::io::Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(unique_name("test_dir")?);

        fs::create_dir(&test_dir)?;
        File::create(test_dir.join("file1.txt"))?.write_all(b"file1")?;
        File::create(test_dir.join("file2.bin"))?.write_all(b"file2")?;
        fs::create_dir(test_dir.join("subdir"))?;
        File::create(test_dir.join("subdir/file3.txt"))?.write_all(b"file3")?;

        Ok(test_dir)
    }

    #[test]
    fn test_bytes() -> std::io::Result<()> {
        assert_eq!(
            self::bytes(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        Ok(())
    }

    #[test]
    fn test_file() -> std::io::Result<()> {
        let file_path = self::create_temp_file(b"hello")?;
        let hash = self::file(&file_path)?;

        std::fs::remove_file(&file_path).ok();
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );

        Ok(())
    }

    #[test]
    fn test_file_not_exists() -> std::io::Result<()> {
        let result = self::file("/non_exist_file");

        assert!(result.is_err());
        if let Err(e) = &result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }

        Ok(())
    }

    #[test]
    fn test_file_is_dir() -> std::io::Result<()> {
        let result = self::file(std::env::temp_dir());

        assert!(result.is_err());
        if let Err(e) = &result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }

        Ok(())
    }

    #[test]
    fn test_file_list() -> std::io::Result<()> {
        let files = vec![
            self::create_temp_file(b"file1")?,
            self::create_temp_file(b"file2")?,
            self::create_temp_file(b"file3")?,
        ];

        let hash = self::file_list(&files)?;
        for file in files {
            std::fs::remove_file(file)?;
        }
        assert_eq!(
            hash,
            "d944e85974a48cfc20a944738d9617ad5ffde6e1219cf4c362dc058a47419848"
        );

        Ok(())
    }

    #[test]
    fn test_dir() -> std::io::Result<()> {
        let test_dir = self::create_temp_dir()?;
        let hash = self::dir(&test_dir)?;
        std::fs::remove_dir_all(test_dir)?;

        assert_eq!(
            hash,
            "d944e85974a48cfc20a944738d9617ad5ffde6e1219cf4c362dc058a47419848"
        );
        Ok(())
    }

    #[test]
    fn test_dir_not_exists() -> std::io::Result<()> {
        let result = self::dir("/non_exist_file");

        assert!(result.is_err());
        if let Err(e) = &result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }

        Ok(())
    }

    #[test]
    fn test_dir_is_file() -> std::io::Result<()> {
        let file_path = self::create_temp_file(b"hello")?;
        let result = self::dir(&file_path);
        std::fs::remove_file(file_path)?;

        assert!(result.is_err());
        if let Err(e) = &result {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        }

        Ok(())
    }

    #[test]
    fn test_path() -> std::io::Result<()> {
        let file_path = self::create_temp_file(b"hello")?;
        let dir_path = self::create_temp_dir()?;

        let file_hash = self::path(&file_path)?;
        let dir_hash = self::path(&dir_path)?;
        std::fs::remove_file(&file_path)?;
        std::fs::remove_dir_all(&dir_path)?;

        assert_eq!(
            file_hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(
            dir_hash,
            "d944e85974a48cfc20a944738d9617ad5ffde6e1219cf4c362dc058a47419848"
        );
        Ok(())
    }
}
