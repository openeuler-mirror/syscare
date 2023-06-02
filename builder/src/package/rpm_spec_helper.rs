use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::Path;

use common::util::fs;
use common::util::os_str::OsStrExt;

pub(super) const SPEC_FILE_EXT:   &str = "spec";
pub(super) const SOURCE_TAG_NAME: &str = "Source";
pub(super) const TAG_VALUE_NONE:  &str = "(none)";

#[derive(PartialEq, Eq)]
#[derive(PartialOrd, Ord)]
#[derive(Debug)]
pub struct RpmSpecTag {
    pub name:  String,
    pub id:    usize,
    pub value: OsString,
}

impl std::fmt::Display for RpmSpecTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}: {}",
            self.name,
            self.id,
            self.value.to_string_lossy()
        ))
    }
}

pub struct RpmSpecHelper;

impl RpmSpecHelper {
    fn parse_id_tag<S: AsRef<OsStr>>(line: S, tag_prefix: &str) -> Option<RpmSpecTag> {
        let line_str = line.as_ref().trim();
        if line_str.starts_with('#') || !line_str.starts_with(tag_prefix) {
            return None;
        }

        let mut split = line_str.split(':');
        if let (Some(tag_key), Some(tag_value)) = (split.next(), split.next()) {
            let parse_tag_id = tag_key.strip_prefix(tag_prefix)
                .and_then(|val| val.to_string_lossy().parse::<usize>().ok());

            if let Some(tag_id) = parse_tag_id {
                return Some(RpmSpecTag {
                    name:  tag_prefix.to_owned(),
                    id:    tag_id,
                    value: tag_value.trim().to_os_string()
                });
            }
        }

        None
    }

    fn create_new_source_tags<I, S>(start_tag_id: usize, file_list: I) -> Vec<RpmSpecTag>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut source_tag_list = Vec::new();
        let mut tag_id = start_tag_id + 1;

        for file_name in file_list {
            source_tag_list.push(RpmSpecTag {
                name:  SOURCE_TAG_NAME.to_owned(),
                id:    tag_id,
                value: file_name.as_ref().to_owned()
            });
            tag_id += 1;
        }

        source_tag_list
    }
}

impl RpmSpecHelper {
    pub fn add_files_to_spec<P, I, S>(spec_file: P, new_file_list: I) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        const PKG_SPEC_SECTION_DESC: &str = "%description";

        let mut spec_file_content = fs::read_to_string(&spec_file)?
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
        let last_tag_id = source_tags.into_iter().last().map(|tag| tag.id).unwrap_or_default();

        // Add 'Source' tag for new files
        for source_tag in Self::create_new_source_tags(last_tag_id, new_file_list).into_iter().rev() {
            lines_to_write.insert((line_num, source_tag.to_string()));
        }

        // Prepare file content
        for (line_index, line_value) in lines_to_write.into_iter().rev() {
            spec_file_content.insert(line_index, line_value);
        }

        // Write to file
        fs::write(
            spec_file,
            spec_file_content.into_iter()
                .flat_map(|mut s| {
                    s.push('\n');
                    s.into_bytes()
                }).collect::<Vec<_>>()
        )
    }
}
