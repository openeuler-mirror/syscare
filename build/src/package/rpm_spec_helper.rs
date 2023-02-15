use std::collections::BTreeSet;
use std::path::Path;

use crate::patch::PatchInfo;

use crate::constants::*;
use crate::util::fs;

use super::rpm_spec_parser::{RpmSpecParser, RpmSpecTag};

pub struct RpmSpecHelper;

impl RpmSpecHelper {
    fn create_new_release_tag(orig_release_tag: RpmSpecTag, patch_info: &PatchInfo) -> RpmSpecTag {
        let target = patch_info.get_target();

        let tag_name  = orig_release_tag.get_name().to_string();
        let tag_value = format!("{}.{}.{}.{}",
            target.get_release(),
            patch_info.get_name(),
            patch_info.get_version(),
            patch_info.get_release()
        );

        RpmSpecTag::new_tag(tag_name, tag_value)
    }

    fn create_new_source_tags(start_tag_id: usize, patch_info: &PatchInfo) -> Vec<RpmSpecTag> {
        let tag_name = PKG_SPEC_TAG_NAME_SOURCE;

        let mut source_tag_list = Vec::new();
        let mut tag_id = start_tag_id + 1;
        let is_patched_pkg = patch_info.is_incremental();

        for patch_file in patch_info.get_patches() {
            // File path contains pid (in workdir) means some of patches are coming from source package
            if !patch_file.is_from_source_pkg() {
                source_tag_list.push(RpmSpecTag::new_id_tag(
                    tag_name.to_owned(),
                    tag_id,
                    patch_file.get_name().to_owned()
                ));
            }

            tag_id += 1;
        }

        // If the package is patched, generate files to record patch info
        if !is_patched_pkg {
            source_tag_list.push(RpmSpecTag::new_id_tag(
                tag_name.to_owned(),
                tag_id,
                PATCH_INFO_FILE_NAME.to_owned()
            ));
        }

        source_tag_list
    }

    pub fn modify_spec_file_by_patches<P: AsRef<Path>>(spec_file: P, patch_info: &PatchInfo) -> std::io::Result<()> {
        let mut spec_file_content = fs::read_file_content(&spec_file)?;
        let mut orig_release_tag = None;
        let mut source_tags = BTreeSet::new();

        // Parse whole file
        let mut current_line_num = 0usize;
        for current_line in &spec_file_content {
            if let Some(_) = RpmSpecParser::parse_tag(&current_line, PKG_SPEC_TAG_NAME_BUILD_REQUIRES) {
                break;
            }

            // If the release tag is not parsed, do parse
            if orig_release_tag.is_none() {
                if let Some(tag) = RpmSpecParser::parse_tag(&current_line, PKG_SPEC_TAG_NAME_RELEASE) {
                    orig_release_tag = Some((current_line_num, tag));
                    current_line_num += 1;
                    continue; // Since parsed release tag, the other tag would not be parsed
                }
            }

            // Add parsed source tag into the btree set
            if let Some(tag) = RpmSpecParser::parse_id_tag(&current_line, PKG_SPEC_TAG_NAME_SOURCE) {
                source_tags.insert(tag);
                current_line_num += 1;
                continue;
            }

            current_line_num += 1;
        }

        // Modify 'Release' tag
        match orig_release_tag {
            Some((line_num, orig_release_tag)) => {
                let tag_value = Self::create_new_release_tag(orig_release_tag, patch_info).to_string();
                spec_file_content[line_num] = tag_value.replace('-', "_"); // release tag don't allow '-'
            },
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Parse rpm spec file '{}' failed, cannot find tag 'Release'",
                        spec_file.as_ref().display()
                    ),
                ));
            }
        }

        // Append 'Source' tag
        let mut lines_to_write = BTreeSet::new();
        let last_source_tag_id = match source_tags.into_iter().last() {
            Some(tag) => tag.get_id().unwrap(),
            None      => 0
        };

        for source_tag in Self::create_new_source_tags(last_source_tag_id, patch_info).into_iter().rev() {
            lines_to_write.insert((current_line_num, source_tag.to_string()));
        }

        // Prepare file content
        for (line_index, line_value) in lines_to_write.into_iter().rev() {
            spec_file_content.insert(line_index, line_value);
        }

        // Write to file
        fs::write_file_content(&spec_file, spec_file_content)?;

        Ok(())
    }
}
