use std::collections::BTreeSet;
use std::path::Path;

use crate::patch::PatchInfo;

use crate::constants::*;
use crate::util::fs;

use super::rpm_spec_parser::{RpmSpecParser, RpmSpecTag};

pub struct RpmSpecHelper;

impl RpmSpecHelper {
    fn create_new_source_tags(start_tag_id: usize, patch_info: &PatchInfo) -> Vec<RpmSpecTag> {
        let tag_name = PKG_SPEC_TAG_SOURCE;

        let mut source_tag_list = Vec::new();
        let mut tag_id = start_tag_id + 1;
        for patch_file in &patch_info.patches {
            // File path contains pid (in workdir) means some of patches are coming from source package
            if !patch_file.is_from_source_pkg() {
                source_tag_list.push(RpmSpecTag::new_id_tag(
                    tag_name.to_owned(),
                    tag_id,
                    patch_file.name.to_owned()
                ));
            }

            tag_id += 1;
        }

        // If the source package is not patched, generate files to record patch info
        if !patch_info.is_patched {
            source_tag_list.push(RpmSpecTag::new_id_tag(
                tag_name.to_owned(),
                tag_id,
                PATCH_INFO_FILE_NAME.to_owned()
            ));
        }

        source_tag_list
    }

    pub fn modify_spec_file_by_patches<P: AsRef<Path>>(spec_file: P, patch_info: &PatchInfo) -> std::io::Result<()> {
        let mut spec_file_content = fs::read_to_string(&spec_file)?
            .split('\n')
            .map(String::from)
            .collect::<Vec<_>>();

        // Parse whole file
        let mut source_tags = BTreeSet::new();
        let mut line_num = 0usize;
        for current_line in &spec_file_content {
            if let Some(_) = RpmSpecParser::parse_tag(&current_line, PKG_SPEC_TAG_BUILD_REQUIRES) {
                break;
            }
            // Add parsed source tag into the btree set
            if let Some(tag) = RpmSpecParser::parse_id_tag(&current_line, PKG_SPEC_TAG_SOURCE) {
                source_tags.insert(tag);
                line_num += 1;
                continue;
            }
            line_num += 1;
        }

        // Append 'Source' tag
        let mut lines_to_write = BTreeSet::new();
        let last_source_tag_id = match source_tags.into_iter().last() {
            Some(tag) => tag.get_id().unwrap(),
            None      => 0
        };

        for source_tag in Self::create_new_source_tags(last_source_tag_id, patch_info).into_iter().rev() {
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
