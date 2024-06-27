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
    collections::{BTreeSet, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::package::rpm::{SPEC_SCRIPT_VALUE_NONE, SPEC_TAG_VALUE_NONE};

use super::tags::{RpmChangeLog, RpmDefAttr, RpmDefine, RpmPath};

pub struct RpmSpecFile {
    pub defines: BTreeSet<RpmDefine>,
    pub name: String,
    pub version: String,
    pub release: String,
    pub group: Option<String>,
    pub license: String,
    pub url: Option<String>,
    pub summary: String,
    pub build_requires: HashSet<String>,
    pub requires: HashSet<String>,
    pub conflicts: HashSet<String>,
    pub suggests: HashSet<String>,
    pub recommends: HashSet<String>,
    pub description: String,
    pub prep: String,
    pub build: String,
    pub install: String,
    pub check: Option<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    pub preun: Option<String>,
    pub postun: Option<String>,
    pub defattr: Option<RpmDefAttr>,
    pub files: BTreeSet<RpmPath>,
    pub source: BTreeSet<PathBuf>,
    pub patch: BTreeSet<PathBuf>,
    pub change_log: Option<RpmChangeLog>,
}

impl RpmSpecFile {
    pub fn new(
        name: String,
        version: String,
        release: String,
        license: String,
        summary: String,
        description: String,
    ) -> Self {
        Self {
            defines: BTreeSet::default(),
            name,
            version,
            release,
            group: Option::default(),
            license,
            url: Option::default(),
            summary,
            build_requires: HashSet::default(),
            requires: HashSet::default(),
            conflicts: HashSet::default(),
            suggests: HashSet::default(),
            recommends: HashSet::default(),
            description,
            prep: SPEC_SCRIPT_VALUE_NONE.to_string(),
            build: SPEC_SCRIPT_VALUE_NONE.to_string(),
            install: SPEC_SCRIPT_VALUE_NONE.to_string(),
            check: Option::default(),
            pre: Option::default(),
            post: Option::default(),
            preun: Option::default(),
            postun: Option::default(),
            defattr: Option::default(),
            files: BTreeSet::default(),
            source: BTreeSet::default(),
            patch: BTreeSet::default(),
            change_log: Option::default(),
        }
    }
}

impl RpmSpecFile {
    fn write_section<T>(f: &mut std::fmt::Formatter<'_>, name: &str, value: T) -> std::fmt::Result
    where
        T: Display,
    {
        writeln!(f)?;
        writeln!(f, "{}", name)?;
        writeln!(f, "{}", value)
    }

    fn write_opt_section<T>(
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        value: &Option<T>,
    ) -> std::fmt::Result
    where
        T: Display,
    {
        if let Some(v) = value {
            Self::write_section(f, name, v)?;
        }
        Ok(())
    }

    fn write_tag<T>(f: &mut std::fmt::Formatter<'_>, name: &str, value: T) -> std::fmt::Result
    where
        T: Display,
    {
        writeln!(f, "{}: {}", name, value)
    }

    fn write_opt_tag<T: Display>(
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        value: &Option<T>,
    ) -> std::fmt::Result {
        if let Some(v) = value {
            Self::write_tag(f, name, v)?;
        }
        Ok(())
    }

    fn write_tags<I, T>(f: &mut std::fmt::Formatter<'_>, name: &str, value: I) -> std::fmt::Result
    where
        I: IntoIterator<Item = T>,
        T: Display,
    {
        for item in value {
            writeln!(f, "{}: {}", name, item)?;
        }
        Ok(())
    }

    fn write_idx_tags<I, T>(
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        value: I,
    ) -> std::fmt::Result
    where
        I: IntoIterator<Item = T>,
        T: AsRef<Path>,
    {
        for (index, item) in value.into_iter().enumerate() {
            writeln!(f, "{}{}: {}", name, index, item.as_ref().display())?;
        }
        Ok(())
    }

    pub fn write_formatter(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for macro_item in &self.defines {
            writeln!(f, "{}", macro_item)?;
        }
        Self::write_tag(f, "Name", &self.name)?;
        Self::write_tag(f, "Version", &self.version)?;
        Self::write_tag(f, "Release", &self.release)?;
        Self::write_opt_tag(f, "Group", &self.group)?;
        Self::write_tag(f, "License", &self.license)?;
        Self::write_opt_tag(f, "URL", &self.url)?;
        Self::write_tag(f, "Summary", &self.summary)?;
        Self::write_tags(f, "BuildRequires", &self.build_requires)?;
        Self::write_tags(f, "Requires", &self.requires)?;
        Self::write_tags(f, "Conflict", &self.conflicts)?;
        Self::write_tags(f, "Suggest", &self.suggests)?;
        Self::write_tags(f, "Recommend", &self.recommends)?;
        Self::write_idx_tags(f, "Source", &self.source)?;
        Self::write_idx_tags(f, "Patch", &self.patch)?;
        Self::write_section(f, "%description", &self.description)?;
        Self::write_section(f, "%prep", &self.prep)?;
        Self::write_section(f, "%build", &self.build)?;
        Self::write_section(f, "%install", &self.install)?;
        Self::write_opt_section(f, "%check", &self.check)?;
        writeln!(f, "%files")?;
        if let Some(def_attr) = &self.defattr {
            writeln!(f, "{}", def_attr)?;
        }
        for path in &self.files {
            writeln!(f, "{}", path)?;
        }
        Self::write_opt_section(f, "%pre", &self.pre)?;
        Self::write_opt_section(f, "%post", &self.post)?;
        Self::write_opt_section(f, "%preun", &self.preun)?;
        Self::write_opt_section(f, "%postun", &self.postun)?;
        Self::write_opt_section(f, "%changelog", &self.change_log)?;

        Ok(())
    }
}

impl Default for RpmSpecFile {
    fn default() -> Self {
        Self {
            defines: BTreeSet::default(),
            name: SPEC_TAG_VALUE_NONE.to_string(),
            version: SPEC_TAG_VALUE_NONE.to_string(),
            release: SPEC_TAG_VALUE_NONE.to_string(),
            group: Option::default(),
            license: SPEC_TAG_VALUE_NONE.to_string(),
            url: Option::default(),
            summary: SPEC_TAG_VALUE_NONE.to_string(),
            build_requires: HashSet::default(),
            requires: HashSet::default(),
            conflicts: HashSet::default(),
            suggests: HashSet::default(),
            recommends: HashSet::default(),
            description: SPEC_TAG_VALUE_NONE.to_string(),
            prep: SPEC_SCRIPT_VALUE_NONE.to_string(),
            build: SPEC_SCRIPT_VALUE_NONE.to_string(),
            install: SPEC_SCRIPT_VALUE_NONE.to_string(),
            check: Option::default(),
            pre: Option::default(),
            post: Option::default(),
            preun: Option::default(),
            postun: Option::default(),
            defattr: Option::default(),
            files: BTreeSet::default(),
            source: BTreeSet::default(),
            patch: BTreeSet::default(),
            change_log: Option::default(),
        }
    }
}

impl std::fmt::Display for RpmSpecFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_formatter(f)
    }
}

#[test]
fn test() {
    println!(
        "RpmSpec::new()\n{}",
        RpmSpecFile::new(
            "spec_test".to_string(),
            "1.0.1".to_string(),
            "1".to_string(),
            "none".to_string(),
            "test spec".to_string(),
            "This is a spec test".to_string()
        )
    );
}
