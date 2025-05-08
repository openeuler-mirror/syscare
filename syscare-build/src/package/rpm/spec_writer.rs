// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
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
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use syscare_common::{ffi::OsStrExt, fs};

use crate::package::PackageSpecWriter;

const SOURCE_TAG_NAME: &str = "Source";

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct RpmSpecTag {
    pub name: String,
    pub id: usize,
    pub value: OsString,
}

impl std::fmt::Display for RpmSpecTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}: {}",
            self.name,
            self.id,
            self.value.to_string_lossy()
        ))
    }
}

pub struct RpmSpecWriter;

impl RpmSpecWriter {
    fn parse_id_tag<S: AsRef<OsStr>>(line: S, tag_prefix: &str) -> Option<RpmSpecTag> {
        let line_str = line.as_ref().trim();
        if line_str.starts_with('#') || !line_str.starts_with(tag_prefix) {
            return None;
        }

        let mut split = line_str.split(':');
        if let (Some(tag_key), Some(tag_value)) = (split.next(), split.next()) {
            let parse_tag_id = tag_key
                .strip_prefix(tag_prefix)
                .and_then(|val| val.to_string_lossy().parse::<usize>().ok());

            if let Some(tag_id) = parse_tag_id {
                return Some(RpmSpecTag {
                    name: tag_prefix.to_owned(),
                    id: tag_id,
                    value: tag_value.trim().to_os_string(),
                });
            }
        }

        None
    }

    fn create_new_source_tags<I, P>(start_tag_id: usize, file_list: I) -> Vec<RpmSpecTag>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut source_tag_list = Vec::new();
        let mut tag_id = start_tag_id + 1;

        for file_path in file_list {
            source_tag_list.push(RpmSpecTag {
                name: SOURCE_TAG_NAME.to_owned(),
                id: tag_id,
                value: fs::file_name(file_path),
            });
            tag_id += 1;
        }

        source_tag_list
    }
}

impl PackageSpecWriter for RpmSpecWriter {
    fn add_source_files(&self, spec_file: &Path, file_list: Vec<PathBuf>) -> Result<()> {
        const PKG_SPEC_SECTION_DESC: &str = "%description";

        let mut spec_file_content = fs::read_to_string(spec_file)?
            .split('\n')
            .map(String::from)
            .collect::<Vec<_>>();

        // Parse whole file
        let mut source_tags = BTreeSet::new();
        let mut line_num = 0usize;
        for current_line in &spec_file_content {
            let line = current_line.trim();
            if line == PKG_SPEC_SECTION_DESC {
                break;
            }
            // Add parsed source tag into the btree set
            if let Some(tag) = Self::parse_id_tag(line, SOURCE_TAG_NAME) {
                source_tags.insert(tag);
                line_num += 1;
                continue;
            }
            line_num += 1;
        }

        // Find last 'Source' tag id
        let mut lines_to_write = BTreeSet::new();
        let last_tag_id = source_tags
            .into_iter()
            .next_back()
            .map(|tag| tag.id)
            .unwrap_or_default();

        // Add 'Source' tag for new files
        for source_tag in Self::create_new_source_tags(last_tag_id, file_list)
            .into_iter()
            .rev()
        {
            lines_to_write.insert((line_num, source_tag.to_string()));
        }

        // Prepare file content
        for (line_index, line_value) in lines_to_write.into_iter().rev() {
            spec_file_content.insert(line_index, line_value);
        }

        // Write to file
        fs::write(
            spec_file,
            spec_file_content
                .into_iter()
                .flat_map(|mut s| {
                    s.push('\n');
                    s.into_bytes()
                })
                .collect::<Vec<_>>(),
        )
        .context("Failed to write rpm spec file")
    }
}

#[test]
fn tests_spec_writer() {
    use std::fs::File;
    use std::io::{Read, Write};

    let mut specfile = File::create("/tmp/test.spec").unwrap();
    specfile.write_all(b"Source: kerneltest-1.tar.gz").unwrap();
    specfile.write_all(b"%description").unwrap();

    specfile.sync_all().unwrap();
    let filepath = PathBuf::from("/tmp/test.spec");

    let test = RpmSpecWriter;
    test.add_source_files(
        &filepath,
        vec![PathBuf::from("test1"), PathBuf::from("test2")],
    )
    .unwrap();

    let mut fileread = File::open(&filepath).unwrap();
    let mut content = String::new();
    fileread.read_to_string(&mut content).unwrap();
    assert!(content.contains("Source1: test1"));
    assert!(content.contains("Source2: test2"));

    if filepath.exists() {
        fs::remove_file(filepath).unwrap();
    }
}
