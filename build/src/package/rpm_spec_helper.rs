use std::collections::BTreeSet;

use crate::constants::*;
use crate::util::sys;
use crate::util::fs;

use crate::patch::PatchInfo;

use super::rpm_spec_parser::{RpmSpecParser, RpmSpecTag};

pub struct RpmSpecHelper;

impl RpmSpecHelper {
    fn create_new_release_tag(release_tag: (usize, RpmSpecTag), patch_info: &PatchInfo) -> Option<(usize, RpmSpecTag)> {
        let (line_num, orig_tag) = release_tag;

        let patch  = patch_info.get_patch();
        let target = patch_info.get_target();

        let tag_name  = orig_tag.get_name().to_string();
        let tag_value = format!("{}.{}.{}.{}.{}",
            target.get_release(),
            PKG_FLAG_PATCH_PKG,
            patch.get_name(),
            patch.get_version(),
            patch.get_release()
        );

        Some((line_num, RpmSpecTag::new_tag(tag_name, tag_value)))
    }

    fn create_new_source_tags(start_tag_id: usize, patch_info: &PatchInfo) -> Vec<RpmSpecTag> {
        let tag_name = PKG_SPEC_TAG_NAME_SOURCE;

        let mut source_tag_list = Vec::new();

        let mut tag_id = start_tag_id + 1;
        let mut is_patched_pkg = false;

        for patch_file in patch_info.get_file_list() {
            // File path contains pid (in workdir) means some of patches are come from source package
            match patch_file.get_path().contains(&sys::get_process_id().to_string()) {
                true  => {
                    // Exclude patches from patched source package
                    // and leave a flag to identify this
                    is_patched_pkg = true;
                },
                false => {
                    source_tag_list.push(RpmSpecTag::new_id_tag(
                        tag_name.to_owned(),
                        tag_id,
                        patch_file.get_name().to_owned()
                    ));
                }
            }

            tag_id += 1;
        }

        // If the package is patched, generate files to record
        // patch target name and patch version
        if !is_patched_pkg {
            source_tag_list.push(RpmSpecTag::new_id_tag(
                tag_name.to_owned(),
                tag_id,
                PKG_PATCH_VERSION_FILE_NAME.to_owned()
            ));
            tag_id += 1;

            source_tag_list.push(RpmSpecTag::new_id_tag(
                tag_name.to_owned(),
                tag_id,
                PKG_PATCH_TARGET_FILE_NAME.to_owned()
            ));
        }

        source_tag_list
    }

    pub fn modify_spec_file_by_patches(spec_file_path: &str, patch_info: &PatchInfo) -> std::io::Result<()> {
        let mut spec_file_content = fs::read_file_content(spec_file_path)?;
        let mut release_tag = None;
        let mut source_tags = BTreeSet::new();

        // Parse whole file
        let mut current_line_num = 0usize;
        for current_line in &spec_file_content {
            // Found build requires tag means there is no more data to parse
            if current_line.contains(PKG_SPEC_TAG_NAME_BUILD_REQUIRES) {
                break;
            }

            // If the release tag is not parsed, do parse
            if release_tag.is_none() {
                if let Some(tag) = RpmSpecParser::parse_tag(&current_line, PKG_SPEC_TAG_NAME_RELEASE) {
                    release_tag = Some((current_line_num, tag));
                    current_line_num += 1;
                    continue; // Since parsed release tag, the other tag would not be parsed
                }
            }

            // Add parsed source tag into the btree set
            if let Some(tag) = RpmSpecParser::parse_parse_id_tag(&current_line, PKG_SPEC_TAG_NAME_SOURCE) {
                source_tags.insert(tag);
                current_line_num += 1;
                continue;
            }

            current_line_num += 1;
        }

        // Check 'Release' tag existence
        if release_tag.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Parse rpm spec file '{}' failed, cannot find tag 'Release'", spec_file_path),
            ));
        }

        // Modify 'Release' tag
        if let Some((line_num, tag)) = Self::create_new_release_tag(release_tag.unwrap(), patch_info) {
            spec_file_content.remove(line_num);
            spec_file_content.insert(line_num, tag.to_string());
        }

        // Append 'Source' tag
        let mut lines_to_write = BTreeSet::new();
        let last_source_tag_id = match source_tags.into_iter().last() {
            Some(tag) => tag.get_id().unwrap_or_default(),
            None      => 0
        };

        for source_tag in Self::create_new_source_tags(last_source_tag_id, patch_info) {
            lines_to_write.insert((current_line_num, source_tag.to_string()));
        }

        // Prepare file content
        for (line_index, line_value) in lines_to_write.into_iter().rev() {
            spec_file_content.insert(line_index, line_value);
        }

        // Write to file
        fs::write_file_content(spec_file_path, spec_file_content)?;

        Ok(())
    }
}
