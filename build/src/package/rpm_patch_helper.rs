use std::collections::BTreeSet;

use crate::patch::PatchFile;
use crate::util::fs;

use super::rpm_spec_parser::{RpmSpecParser, RpmSpecTag};

pub struct RpmPatchHelper;

impl RpmPatchHelper {
    pub fn modify_patch_list(patch_list: &[PatchFile]) -> Vec<PatchFile> {
        const PATCH_FILE_PREFIX: &str = "syscare-patch";
        const PATCH_EXT: &str = "patch";

        let mut new_patch_list = Vec::with_capacity(patch_list.len());
    
        let mut patch_index = 1usize;
        for patch_file in patch_list {
            // The patch file may come form extracted source rpm, which is already renamed.
            // Thus, we have to figure out whether the patch source and rename it.
            let orig_file_name = patch_file.get_name();
            if !orig_file_name.contains(PATCH_EXT) {
                continue;
            }

            let new_file_name = match orig_file_name.strip_prefix(PATCH_FILE_PREFIX) {
                Some(patch_name) => format!("{}-{:04}-{}", PATCH_FILE_PREFIX, patch_index, patch_name),
                None             => format!("{}-{:04}-{}", PATCH_FILE_PREFIX, patch_index, orig_file_name),
            };

            let mut new_patch_file = patch_file.clone();
            new_patch_file.set_name(new_file_name);
            new_patch_list.push(new_patch_file);

            patch_index += 1;
        }

        new_patch_list
    }

    pub fn modify_spec_file_by_patches(spec_file_path: &str, patch_list: &[PatchFile]) -> std::io::Result<()> {
        const RELEASE_TAG_NAME:        &str = "Release:";
        const RELEASE_TAG_MACRO:       &str = "%{?syscare_patch_release}";
        const SOURCE_TAG_PREFIX:       &str = "Source";
        const BUILD_REQUIRES_TAG_NAME: &str = "BuildRequires:";

        #[inline(always)]
        fn create_new_release_tag(release_tag: (usize, RpmSpecTag)) -> Option<(usize, RpmSpecTag)> {
            let (line_num, original_tag) = release_tag;

            let tag_name = original_tag.get_name();
            let tag_value = original_tag.get_value();
            if tag_value.contains(RELEASE_TAG_MACRO) {
                return None; // Already has syscare patch macro
            }

            Some((
                line_num,
                RpmSpecTag::new_tag(
                    tag_name.to_owned(),
                    format!("{}{}", tag_value, RELEASE_TAG_MACRO) // Append a patch info macro to release tag
                )
            ))
        }

        #[inline(always)]
        fn create_new_source_tags(start_tag_id: usize, patch_list: &[PatchFile]) -> Vec<RpmSpecTag> {
            let mut source_tag_list = Vec::new();

            let mut tag_id = start_tag_id + 1;

            for patch_file in patch_list {
                let tag_name  = SOURCE_TAG_PREFIX.to_owned();
                let tag_value = patch_file.get_name().to_owned();

                source_tag_list.push(RpmSpecTag::new_id_tag(tag_name, tag_id, tag_value));
                tag_id += 1;
            }

            source_tag_list
        }

        // Read spec file
        let mut spec_file_content = fs::read_file_content(spec_file_path)?;

        // Parse whole file
        let mut release_tag = None;
        let mut source_tags = BTreeSet::new();

        let mut current_line_num = 0usize;
        for current_line in &spec_file_content {
            // Found build requires tag means there is no more data to parse
            if current_line.contains(BUILD_REQUIRES_TAG_NAME) {
                break;
            }

            // If the release tag is not parsed, do parse
            if release_tag.is_none() {
                if let Some(tag) = RpmSpecParser::parse_tag(&current_line, RELEASE_TAG_NAME) {
                    release_tag = Some((current_line_num, tag));

                    current_line_num += 1;
                    continue; // Since parsed release tag, the other tag would not be parsed
                }
            }
            // Add parsed source tag into the btree set
            if let Some(tag) = RpmSpecParser::parse_parse_id_tag(&current_line, SOURCE_TAG_PREFIX) {
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
        if let Some((line_num, tag)) = create_new_release_tag(release_tag.unwrap()) {
            // lines_to_write.insert((line_num, tag.to_string()));
            spec_file_content.remove(line_num);
            spec_file_content.insert(line_num, tag.to_string());
        }

        // Add 'Source' tag
        let mut lines_to_write = BTreeSet::new();
        let last_source_tag_id = match source_tags.into_iter().last() {
            Some(tag) => {
                tag.get_id().unwrap_or_default()
            },
            None => 0
        };
        for source_tag in create_new_source_tags(last_source_tag_id, patch_list) {
            lines_to_write.insert((current_line_num, source_tag.to_string()));
            current_line_num += 1;
        }

        for (line_index, line_value) in lines_to_write.into_iter().rev() {
            spec_file_content.insert(line_index, line_value);
        }

        // Write to file
        fs::write_file_contect(spec_file_path, spec_file_content)?;

        Ok(())
    }
}
