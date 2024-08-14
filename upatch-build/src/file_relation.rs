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
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};

use indexmap::{IndexMap, IndexSet};
use log::warn;
use object::ObjectKind;
use syscare_common::{concat_os, ffi::OsStrExt, fs};

use crate::elf;

const BUILD_ROOT_PREFIX: &str = "upatch-build.";
const UPATCH_ID_PREFIX: &str = ".upatch_";

const NON_EXIST_PATH: &str = "/dev/null";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRelation {
    pub original_object: PathBuf,
    pub patched_object: PathBuf,
}

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
            let binary_path = binary_dir.as_ref().join(&binary);
            let binary_files =
                fs::list_files(&binary_dir, fs::TraverseOptions { recursive: true })?
                    .into_iter()
                    .filter(|file_path| {
                        file_path.ends_with(binary.as_ref().as_os_str())
                            && matches!(
                                elf::parse_file_kind(file_path).unwrap_or(ObjectKind::Unknown),
                                ObjectKind::Executable | ObjectKind::Dynamic
                            )
                    })
                    .collect::<IndexSet<_>>();

            ensure!(
                !binary_files.is_empty(),
                "Path {} does not match to any file",
                binary_path.display()
            );
            ensure!(
                binary_files.len() == 1,
                "Path {} matches to too many file",
                binary_path.display()
            );
            self.binary_debuginfo_map
                .insert(binary_files[0].clone(), debuginfo.as_ref().to_path_buf());
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
                let (object_file, patched_object) =
                    id_object_map.get(&upatch_id).with_context(|| {
                        format!(
                            "Cannot find patched object of {}",
                            upatch_id.to_string_lossy()
                        )
                    })?;
                let original_object = self
                    .original_object_map
                    .get(object_file)
                    .map(|p| p.as_path())
                    .unwrap_or_else(|| Path::new(NON_EXIST_PATH));

                object_relation.insert(patched_object.to_path_buf(), original_object.to_path_buf());
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
        let elf_file = elf::read::Elf::parse(file_path).context("Failed to parse elf")?;
        let symbols = elf_file.symbols().context("Failed to read elf symbols")?;

        let mut upatch_ids = IndexSet::new();
        for symbol in symbols {
            let symbol_name = symbol.get_st_name();
            if symbol_name.starts_with(UPATCH_ID_PREFIX) {
                upatch_ids.insert(symbol_name.to_os_string());
            }
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
        search_dir: P,
        target_dir: Q,
    ) -> Result<IndexMap<OsString, (PathBuf, PathBuf)>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut object_files = IndexSet::new();

        let matched_dirs = fs::glob(&search_dir)
            .with_context(|| format!("Cannot match path {}", search_dir.as_ref().display()))?;
        for matched_dir in matched_dirs {
            object_files.extend(elf::find_elf_files(&matched_dir, |obj_path, obj_kind| {
                !obj_path.contains(BUILD_ROOT_PREFIX) && (obj_kind == ObjectKind::Relocatable)
            })?);
        }
        ensure!(
            !object_files.is_empty(),
            "Cannot find any valid objects in {}, please add compile flag '-save-temps' manually",
            search_dir.as_ref().display()
        );

        let mut object_relations = Vec::with_capacity(object_files.len());
        for (file_id, object_file) in object_files.into_iter().enumerate() {
            let object_archive = target_dir.as_ref().join(concat_os!(
                format!("{:04}-", file_id),
                object_file.file_name().with_context(|| {
                    format!("Failed to parse file name of {}", object_file.display())
                })?
            ));
            fs::copy(&object_file, &object_archive)?;

            let upatch_ids = Self::parse_upatch_ids(&object_file).with_context(|| {
                format!("Failed to parse upatch id of {}", object_file.display())
            })?;
            object_relations.push((object_file, object_archive, upatch_ids));
        }

        let mut id_object_map = IndexMap::with_capacity(object_relations.len());
        for (object_file, object_archive, object_ids) in &object_relations {
            if object_relations.iter().all(|(obj, _, ids)| {
                if (obj != object_file) && !ids.is_empty() && ids.is_subset(object_ids) {
                    warn!("Skipped object {}", object_archive.display());
                    return false;
                }
                true
            }) {
                id_object_map.extend(object_ids.iter().map(|id| {
                    (
                        id.to_os_string(),
                        (object_file.clone(), object_archive.clone()),
                    )
                }));
            }
        }

        Ok(id_object_map)
    }
}
