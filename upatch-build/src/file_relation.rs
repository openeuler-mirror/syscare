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

use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};

use indexmap::{IndexMap, IndexSet};
use log::warn;
use object::{Object, ObjectKind, ObjectSymbol};

use syscare_common::{concat_os, ffi::OsStrExt as _, fs};

use crate::elf;

const UPATCH_ID_PREFIX: &str = ".upatch_";

const NON_EXIST_PATH: &str = "/dev/null";

/*
 * The task of this class is to find out:
 * 1. relationship between binary and debuginfo
 * 2. relationship between output binaries and objects
 * 3. relationship between original objects and patched objects
 */
#[derive(Debug)]
pub struct FileRelation {
    binary_debuginfo_map: IndexMap<PathBuf, PathBuf>, // Binary -> Debuginfo
    binary_relation_map: IndexMap<PathBuf, IndexMap<PathBuf, PathBuf>>, // Binary -> [ObjectRelation]
    original_object_map: IndexMap<PathBuf, PathBuf>, // Output object -> Original object
}

impl FileRelation {
    pub fn new() -> Self {
        Self {
            binary_debuginfo_map: IndexMap::new(),
            binary_relation_map: IndexMap::new(),
            original_object_map: IndexMap::new(),
        }
    }

    pub fn collect_debuginfo<P, I, J, Q, R>(
        &mut self,
        binary_dir: P,
        binaries: I,
        debuginfos: J,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = Q>,
        J: IntoIterator<Item = R>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
    {
        let mut binary_iter = binaries.into_iter();
        let mut debuginfo_iter = debuginfos.into_iter();

        while let (Some(binary), Some(debuginfo)) = (binary_iter.next(), debuginfo_iter.next()) {
            let binary_dir = binary_dir.as_ref();
            let binary_path = binary.as_ref().as_os_str();

            let mut binary_files = IndexSet::new();
            for match_result in fs::glob(binary_dir) {
                let matched_dir = match_result.with_context(|| {
                    format!("Cannot match binary directory {}", binary_dir.display())
                })?;
                let found_files =
                    fs::list_files(matched_dir, fs::TraverseOptions { recursive: true })?
                        .into_iter()
                        .filter(|file_path| file_path.ends_with(binary_path))
                        .filter(|file_path| {
                            matches!(
                                elf::elf_kind(file_path),
                                ObjectKind::Executable | ObjectKind::Dynamic
                            )
                        });
                binary_files.extend(found_files);
            }
            let binary_file = binary_files
                .pop()
                .with_context(|| format!("Cannot find any binary in {}", binary_dir.display()))?;
            ensure!(
                binary_files.is_empty(),
                "Binary {} matched to too many files",
                binary_path.to_string_lossy()
            );
            self.binary_debuginfo_map
                .insert(binary_file, debuginfo.as_ref().to_path_buf());
        }

        Ok(())
    }

    pub fn collect_original_build<P, Q>(&mut self, object_dir: P, collect_dir: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let id_object_map = Self::collect_objects(&object_dir, &collect_dir)?;
        for (_, (object_file, object_archive)) in id_object_map {
            self.original_object_map.insert(object_file, object_archive);
        }

        Ok(())
    }

    pub fn collect_patched_build<P, Q>(&mut self, object_dir: P, collect_dir: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let id_object_map = Self::collect_objects(&object_dir, &collect_dir)?;
        let binary_id_map = Self::collect_binaries(self.binary_debuginfo_map.keys())?;

        for (binary_file, upatch_ids) in binary_id_map {
            let mut object_relation = IndexMap::new();

            for upatch_id in upatch_ids {
                match id_object_map.get(&upatch_id) {
                    Some((object_file, patched_object)) => {
                        let original_object = self
                            .original_object_map
                            .get(object_file)
                            .map(|p| p.as_path())
                            .unwrap_or_else(|| Path::new(NON_EXIST_PATH));
                        object_relation
                            .insert(patched_object.to_path_buf(), original_object.to_path_buf());
                    }
                    None => {
                        warn!(
                            "Cannot find patched object of {} in target {}",
                            upatch_id.to_string_lossy(),
                            binary_file.display()
                        );
                    }
                }
            }
            object_relation.sort_keys();

            self.binary_relation_map
                .insert(binary_file, object_relation);
        }

        Ok(())
    }

    pub fn get_files(&self) -> impl IntoIterator<Item = (&Path, &Path)> {
        self.binary_debuginfo_map
            .iter()
            .map(|(binary, debuginfo)| (binary.as_path(), debuginfo.as_path()))
    }

    pub fn binary_objects<P: AsRef<Path>>(&self, binary: P) -> Option<&IndexMap<PathBuf, PathBuf>> {
        self.binary_relation_map.get(binary.as_ref())
    }
}

impl FileRelation {
    fn parse_upatch_ids<P: AsRef<Path>>(file_path: P) -> Result<IndexSet<OsString>> {
        let file_path = file_path.as_ref();
        let mmap = fs::mmap(file_path)
            .with_context(|| format!("Failed to mmap {}", file_path.display()))?;
        let file = object::File::parse(mmap.as_ref())
            .with_context(|| format!("Failed to parse {}", file_path.display()))?;

        let mut upatch_ids = IndexSet::new();
        for symbol in file.symbols() {
            let name_slice = symbol.name_bytes().with_context(|| {
                format!("Failed to parse symbol name, index={}", symbol.index().0)
            })?;
            if !name_slice.starts_with(UPATCH_ID_PREFIX.as_bytes()) {
                continue;
            }
            upatch_ids.insert(OsStr::from_bytes(name_slice).to_os_string());
        }

        Ok(upatch_ids)
    }

    fn collect_binaries<I, P>(binaries: I) -> Result<IndexMap<PathBuf, IndexSet<OsString>>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut binary_id_map = IndexMap::new();

        for binary in binaries {
            let binary_file = binary.as_ref();
            binary_id_map.insert(
                binary_file.to_path_buf(),
                Self::parse_upatch_ids(binary_file).with_context(|| {
                    format!("Failed to parse upatch id of {}", binary_file.display())
                })?,
            );
        }
        binary_id_map.sort_keys();

        Ok(binary_id_map)
    }

    fn collect_objects<P, Q>(
        object_dir: P,
        target_dir: Q,
    ) -> Result<IndexMap<OsString, (PathBuf, PathBuf)>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let object_dir = object_dir.as_ref();
        let target_dir = target_dir.as_ref();

        let mut file_id = 1usize;
        let mut object_info = Vec::new();
        for match_result in fs::glob(object_dir) {
            let matched_dir = match_result.with_context(|| {
                format!("Cannot match object directory {}", object_dir.display())
            })?;

            let file_list = fs::list_files(&matched_dir, fs::TraverseOptions { recursive: true })?;
            for object_file in file_list {
                let mmap = fs::mmap(&object_file)
                    .with_context(|| format!("Failed to mmap {}", object_file.display()))?;

                // Try to parse file as elf
                let file = match object::File::parse(mmap.as_ref()) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                // We only care about object file
                if !matches!(file.kind(), ObjectKind::Relocatable) {
                    continue;
                }

                // Copy object file to target directory
                let obj_name = object_file.file_name().unwrap_or_default();
                let archive_file =
                    target_dir.join(concat_os!(format!("{:04}-", file_id), obj_name));
                if fs::hard_link(&object_file, &archive_file).is_err() {
                    fs::copy(&object_file, &archive_file)?;
                }
                file_id += 1;

                // Parse upatch id of the object
                let upatch_ids = file
                    .symbols()
                    .filter_map(|symbol| {
                        let sym_name = symbol.name_bytes().unwrap_or_default();
                        if !sym_name.starts_with(UPATCH_ID_PREFIX.as_bytes()) {
                            return None;
                        }
                        Some(OsStr::from_bytes(sym_name).to_os_string())
                    })
                    .collect::<Vec<_>>();
                if upatch_ids.is_empty() {
                    warn!(
                        "Object {} does not contain upatch id",
                        object_file.display()
                    );
                    continue;
                }

                object_info.push((object_file, archive_file, upatch_ids));
            }
        }
        ensure!(
            !object_info.is_empty(),
            "Cannot find any object in {}",
            object_dir.display()
        );

        // We want subsequent objects to contain more identifiers.
        object_info.sort_by(|(_, _, lhs), (_, _, rhs)| rhs.len().cmp(&lhs.len()));

        let mut upatch_id_map = IndexMap::new();
        for (object, archive, ids) in object_info {
            for id in ids {
                let result = upatch_id_map.insert(id.clone(), (object.clone(), archive.clone()));
                if let Some((old_object, _)) = result {
                    warn!(
                        "{}: Object {} is replaced by {}",
                        id.to_string_lossy(),
                        old_object.display(),
                        object.display()
                    );
                }
            }
        }

        ensure!(
            !upatch_id_map.is_empty(),
            "Cannot find any upatch id in {}",
            object_dir.display()
        );

        Ok(upatch_id_map)
    }
}
