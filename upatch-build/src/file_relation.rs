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
use log::warn;
use syscare_common::{ffi::OsStrExt, fs};

use super::{
    dwarf::Dwarf,
    elf::{check_elf, read},
    pattern_path::glob,
};

const UPATCH_SYM_PREFIX: &str = ".upatch_";
const OBJECT_EXTENSION: &str = "o";

#[derive(Debug)]
pub struct ObjectRelation {
    pub source_file: PathBuf,
    pub original_object: PathBuf,
    pub patched_object: PathBuf,
}

impl std::fmt::Display for ObjectRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Source: {}, original: {}, patched: {}",
            self.source_file.display(),
            self.original_object.display(),
            self.patched_object.to_string_lossy()
        )
    }
}

#[derive(Debug)]
pub struct BinaryRelation {
    pub path: PathBuf,
    pub debuginfo: PathBuf,
    pub objects: Vec<ObjectRelation>,
}

impl BinaryRelation {
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

    fn find_object_file<P, S>(object_dir: P, object_id: S) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        let mut file_path = object_dir.as_ref().join(object_id.as_ref());
        file_path.set_extension(OBJECT_EXTENSION);

        ensure!(
            file_path.is_file(),
            "Cannot access object {}",
            file_path.display()
        );
        Ok(file_path)
    }

    fn parse_object_map<P: AsRef<Path>>(object_dir: P) -> Result<IndexMap<PathBuf, PathBuf>> {
        let object_dir = object_dir.as_ref();

        let objects = fs::list_files_by_ext(
            object_dir,
            OBJECT_EXTENSION,
            fs::TraverseOptions { recursive: false },
        )?;
        ensure!(
            !objects.is_empty(),
            "Cannot find any object from {}",
            object_dir.display()
        );

        let mut object_map = IndexMap::new();
        for object in objects {
            if let Ok(source_file) = Dwarf::parse_source_file(&object) {
                object_map.insert(source_file, object);
            }
        }

        Ok(object_map)
    }

    /*
     * To find out the relationship between the object and the binary file,
     * we add a marker symbol to the object that matches its file name, named ."upatch_xxx."
     * Once the binary is linked, all of the object's marker symbols will be linked into the binary.
     * Thus, we can find out which object is associated w/ the binary by looking up the marker symbols.
     */
    fn parse_object_ids<P: AsRef<Path>>(object: P) -> Result<IndexSet<OsString>> {
        let object_path = object.as_ref();
        let object_elf = read::Elf::parse(object_path).context("Failed to parse elf")?;
        let object_ids = object_elf
            .symbols()
            .context("Failed to read symbols")?
            .filter_map(|symbol| symbol.get_st_name().strip_prefix(UPATCH_SYM_PREFIX))
            .map(|upatch_id| upatch_id.to_os_string())
            .collect::<IndexSet<_>>();

        Ok(object_ids)
    }

    fn parse_object_relations<P, Q, R>(
        binary: P,
        original_dir: Q,
        patched_dir: R,
    ) -> Result<Vec<ObjectRelation>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
    {
        let binary_path = binary.as_ref();
        let object_dir = original_dir.as_ref();

        let original_objects = Self::parse_object_map(object_dir)
            .with_context(|| format!("Failed to parse object map from {}", object_dir.display()))?;
        let object_ids = Self::parse_object_ids(binary_path).with_context(|| {
            format!("Failed to parse object ids from {}", binary_path.display())
        })?;

        let mut relations = Vec::new();
        for object_id in object_ids {
            let patched_object =
                Self::find_object_file(&patched_dir, &object_id).with_context(|| {
                    format!(
                        "Failed to find patched object of {}{}",
                        UPATCH_SYM_PREFIX,
                        object_id.to_string_lossy()
                    )
                })?;

            match Dwarf::parse_source_file(&patched_object).with_context(|| {
                format!("Failed to find source file of {}", patched_object.display())
            }) {
                Ok(source_file) => {
                    let original_object = original_objects
                        .get(&source_file)
                        .cloned()
                        .with_context(|| {
                            format!(
                                "Failed to find original object of {}",
                                patched_object.display()
                            )
                        })?;

                    relations.push(ObjectRelation {
                        source_file,
                        original_object,
                        patched_object,
                    });
                }
                Err(e) => {
                    warn!("{:?}", e);
                }
            }
        }

        Ok(relations)
    }
}

impl BinaryRelation {
    pub fn parse<I, J, P, Q, R, S>(
        binaries: I,
        debuginfos: J,
        original_dir: P,
        patched_dir: Q,
    ) -> Result<Vec<Self>>
    where
        I: IntoIterator<Item = R>,
        J: IntoIterator<Item = S>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
        S: AsRef<Path>,
    {
        let mut relations = Vec::new();

        let mut binaries = binaries.into_iter();
        let mut debuginfos = debuginfos.into_iter();
        while let (Some(binary), Some(debuginfo)) = (binaries.next(), debuginfos.next()) {
            let binary = Self::find_binary_file(binary)?;
            let objects = Self::parse_object_relations(&binary, &original_dir, &patched_dir)?;
            let debuginfo = debuginfo.as_ref().to_path_buf();

            relations.push(BinaryRelation {
                path: binary,
                debuginfo,
                objects,
            });
        }

        Ok(relations)
    }
}

impl std::fmt::Display for BinaryRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Binary: {}, debuginfo: {}",
            self.path.display(),
            self.debuginfo.display(),
        )?;
        for obj in &self.objects {
            writeln!(f, "{}", obj)?;
        }

        Ok(())
    }
}
