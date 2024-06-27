// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};

use indexmap::{IndexMap, IndexSet};
use syscare_common::{ffi::OsStrExt, fs};

use super::{
    elf::{check_elf, read},
    pattern_path::glob,
};

const UPATCH_SYM_PREFIX: &str = ".upatch_";
const OBJECT_EXTENSION: &str = "o";

/*
 * The task of this class is to find out:
 * 1. relationship between binary and debuginfo
 * 2. relationship between output binaries and objects
 * 3. relationship between original objects and patched objects
 */

#[derive(Debug)]
pub struct FileRelation {
    debuginfo_map: IndexMap<PathBuf, PathBuf>, // Binary -> Debuginfo
    symlink_map: IndexMap<PathBuf, PathBuf>,   // Symlink object -> Orignal object
    patch_objects_map: IndexMap<PathBuf, IndexSet<PathBuf>>, // Binary -> Patched objects
    original_object_map: IndexMap<PathBuf, PathBuf>, // Patched object -> Original object
}

impl FileRelation {
    pub fn new() -> Self {
        Self {
            debuginfo_map: IndexMap::new(),
            symlink_map: IndexMap::new(),
            patch_objects_map: IndexMap::new(),
            original_object_map: IndexMap::new(),
        }
    }

    pub fn collect_debuginfo<I, J, P, Q>(&mut self, binaries: I, debuginfos: J) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        J: IntoIterator<Item = Q>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut binary_iter = binaries.into_iter();
        let mut debuginfo_iter = debuginfos.into_iter();

        while let (Some(binary), Some(debuginfo)) = (binary_iter.next(), debuginfo_iter.next()) {
            let binary_file = Self::find_binary_file(binary)?;
            let debuginfo_file = debuginfo.as_ref().to_path_buf();

            self.debuginfo_map.insert(binary_file, debuginfo_file);
        }

        Ok(())
    }

    pub fn collect_original_build<P, Q>(&mut self, object_dir: P, expected_dir: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let symlinks = fs::list_symlinks(&object_dir, fs::TraverseOptions { recursive: true })?;
        for symlink in symlinks {
            let object = fs::read_link(&symlink)?;
            if !object.starts_with(expected_dir.as_ref().as_os_str()) {
                continue;
            }
            self.symlink_map.insert(symlink, object);
        }
        ensure!(
            !self.symlink_map.is_empty(),
            "Cannot find any valid objects in {}",
            object_dir.as_ref().display()
        );

        Ok(())
    }

    pub fn collect_patched_build<P, Q>(&mut self, object_dir: P, expected_dir: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut symlink_map = IndexMap::new();
        let symlinks = fs::list_symlinks(&object_dir, fs::TraverseOptions { recursive: true })?;
        for symlink in symlinks {
            let object = fs::read_link(&symlink)?;
            if !object.starts_with(expected_dir.as_ref().as_os_str()) {
                continue;
            }
            symlink_map.insert(object, symlink);
        }
        ensure!(
            !self.symlink_map.is_empty(),
            "Cannot find any valid objects in {}",
            object_dir.as_ref().display()
        );

        for (binary, _) in &self.debuginfo_map {
            let mut objects = IndexSet::new();

            let upatch_ids = Self::parse_upatch_ids(binary)
                .with_context(|| format!("Failed to parse upatch id of {}", binary.display()))?;
            for upatch_id in upatch_ids {
                let patched_object = Self::get_object_file(&expected_dir, &upatch_id)
                    .with_context(|| {
                        format!("Failed to get object of {}", upatch_id.to_string_lossy())
                    })?;
                let original_object = symlink_map
                    .get(&patched_object)
                    .and_then(|path| self.symlink_map.get(path))
                    .with_context(|| {
                        format!(
                            "failed to find original object of {}",
                            patched_object.display()
                        )
                    })
                    .cloned()?;

                // Update object relations
                self.original_object_map
                    .insert(patched_object.to_owned(), original_object);
                objects.insert(patched_object);
            }
            self.patch_objects_map.insert(binary.to_owned(), objects);
        }
        self.symlink_map.clear(); // clear useless records

        Ok(())
    }

    pub fn get_files(&self) -> impl IntoIterator<Item = (&Path, &Path)> {
        self.debuginfo_map
            .iter()
            .map(|(binary, debuginfo)| (binary.as_path(), debuginfo.as_path()))
    }

    pub fn get_patched_objects<P: AsRef<Path>>(&self, binary: P) -> Option<&IndexSet<PathBuf>> {
        self.patch_objects_map.get(binary.as_ref())
    }

    pub fn get_original_object<P: AsRef<Path>>(&self, object: P) -> Option<&Path> {
        self.original_object_map
            .get(object.as_ref())
            .map(|p| p.as_path())
    }
}

impl FileRelation {
    fn find_binary_file<P: AsRef<Path>>(binary: P) -> Result<PathBuf> {
        let binary_file = binary.as_ref();
        let matched_file = glob(binary_file)?
            .into_iter()
            .filter(|path| {
                path.is_file()
                    && fs::open_file(path)
                        .map(|file| check_elf(&file).is_ok())
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>(); // for rpm's "BUILDROOT/*/path"

        match matched_file.len() {
            1 => Ok(matched_file[0].clone()),
            0 => bail!("Path does not match to any binary"),
            _ => bail!("Path matches to too many binaries"),
        }
    }

    fn get_object_file<P, S>(object_dir: P, upatch_id: S) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        let mut file_path = object_dir.as_ref().join(upatch_id.as_ref());
        file_path.set_extension(OBJECT_EXTENSION);

        ensure!(
            file_path.is_file(),
            "Cannot access object {}",
            file_path.display()
        );
        Ok(file_path)
    }

    /*
     * To find out the relationship between the object and the binary file,
     * we add a marker symbol to the object that matches its file name, named ."upatch_xxx."
     * Once the binary is linked, all of the object's marker symbols will be linked into the binary.
     * Thus, we can find out which object is associated w/ the binary by looking up the marker symbols.
     */
    fn parse_upatch_ids<P: AsRef<Path>>(binary: P) -> Result<IndexSet<OsString>> {
        let object_path = binary.as_ref();
        let object_elf = read::Elf::parse(object_path).context("Failed to parse elf")?;
        let object_ids = object_elf
            .symbols()
            .context("Failed to read symbols")?
            .filter_map(|symbol| symbol.get_st_name().strip_prefix(UPATCH_SYM_PREFIX))
            .map(|upatch_id| upatch_id.to_os_string())
            .collect::<IndexSet<_>>();

        Ok(object_ids)
    }
}
