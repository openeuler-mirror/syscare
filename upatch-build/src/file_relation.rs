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
    dwarf::Dwarf,
    elf::{check_elf, read},
    pattern_path::glob,
};

const UPATCH_SYM_PREFIX: &str = ".upatch_";
const OBJECT_EXTENSION: &str = "o";

#[derive(Debug)]
pub struct FileRelation {
    binary_debug_map: IndexMap<PathBuf, PathBuf>, // Binary -> Debuginfo
    source_origin_map: IndexMap<PathBuf, PathBuf>, // Source file -> Original object
    binary_patched_map: IndexMap<PathBuf, IndexSet<PathBuf>>, // Binary -> Patched objects
    patched_original_map: IndexMap<PathBuf, PathBuf>, // Patched object -> Original object
}

impl FileRelation {
    pub fn new() -> Self {
        Self {
            binary_debug_map: IndexMap::new(),
            binary_patched_map: IndexMap::new(),
            source_origin_map: IndexMap::new(),
            patched_original_map: IndexMap::new(),
        }
    }

    pub fn collect_outputs<I, J, P, Q>(&mut self, binaries: I, debuginfos: J) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        J: IntoIterator<Item = Q>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut binaries = binaries.into_iter();
        let mut debuginfos = debuginfos.into_iter();

        while let (Some(binary), Some(debuginfo)) = (binaries.next(), debuginfos.next()) {
            let binary = Self::find_binary_file(binary)?;
            let debuginfo = debuginfo.as_ref().to_path_buf();

            self.binary_debug_map.insert(binary, debuginfo);
        }

        Ok(())
    }

    pub fn collect_original_build<P: AsRef<Path>>(&mut self, object_dir: P) -> Result<()> {
        for (binary, _) in &self.binary_debug_map {
            let upatch_ids = Self::parse_upatch_ids(binary)
                .with_context(|| format!("Failed to parse upatch id of {}", binary.display()))?;

            for upatch_id in upatch_ids {
                let original_object = Self::find_object_file(&object_dir, &upatch_id)
                    .with_context(|| {
                        format!("Failed to find object of {}", upatch_id.to_string_lossy())
                    })?;
                let source_file =
                    Dwarf::parse_source_file(&original_object).with_context(|| {
                        format!(
                            "Failed to parse source file of {}",
                            original_object.display()
                        )
                    })?;

                self.source_origin_map.insert(source_file, original_object);
            }
        }

        Ok(())
    }

    pub fn collect_patched_build<P: AsRef<Path>>(&mut self, object_dir: P) -> Result<()> {
        for (binary, _) in &self.binary_debug_map {
            let upatch_ids = Self::parse_upatch_ids(binary)
                .with_context(|| format!("Failed to parse upatch id of {}", binary.display()))?;

            let mut patched_objects = IndexSet::new();
            for upatch_id in upatch_ids {
                let patched_object =
                    Self::find_object_file(&object_dir, &upatch_id).with_context(|| {
                        format!("Failed to find object of {}", upatch_id.to_string_lossy())
                    })?;
                let source_file = Dwarf::parse_source_file(&patched_object).with_context(|| {
                    format!(
                        "Failed to parse source file of {}",
                        patched_object.display()
                    )
                })?;
                let original_object =
                    self.source_origin_map.get(&source_file).with_context(|| {
                        format!(
                            "Failed to find original object of {}",
                            patched_object.display()
                        )
                    })?;

                patched_objects.insert(patched_object.clone());
                self.patched_original_map
                    .insert(patched_object, original_object.to_path_buf());
            }

            self.binary_patched_map
                .insert(binary.to_path_buf(), patched_objects);
        }

        Ok(())
    }

    pub fn get_files(&self) -> impl IntoIterator<Item = (&Path, &Path)> {
        self.binary_debug_map
            .iter()
            .map(|(binary, debuginfo)| (binary.as_path(), debuginfo.as_path()))
    }

    pub fn get_patched_objects<P: AsRef<Path>>(&self, binary: P) -> Option<&IndexSet<PathBuf>> {
        self.binary_patched_map.get(binary.as_ref())
    }

    pub fn get_original_object<P: AsRef<Path>>(&self, object: P) -> Option<&Path> {
        self.patched_original_map
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

    fn find_object_file<P, S>(object_dir: P, upatch_id: S) -> Result<PathBuf>
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
