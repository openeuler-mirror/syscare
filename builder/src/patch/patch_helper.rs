use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use syscare_abi::{PatchFile, PatchInfo};
use syscare_common::util::{digest, fs};

use super::{PatchMetadata, PATCH_FILE_EXT};

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patch_files<I, P>(patch_files: I) -> Result<Vec<PatchFile>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        let mut patch_list = Vec::new();
        for patch_file in patch_files {
            let file_path = fs::canonicalize(patch_file)?;
            let file_name = fs::file_name(&file_path);
            let file_digest = digest::file(&file_path)?;

            if !FILE_DIGESTS.lock().insert(file_digest.clone()) {
                bail!("Patch \"{}\" is duplicated", file_path.display());
            }
            patch_list.push(PatchFile {
                name: file_name,
                path: file_path,
                digest: file_digest,
            });
        }

        Ok(patch_list)
    }

    pub fn apply_patch_metadata<P: AsRef<Path>>(
        patch_info: &mut PatchInfo,
        pkg_source_dir: P,
    ) -> Result<()> {
        let metadata_dir = PatchMetadata::metadata_dir(&pkg_source_dir);
        let metadata_file = PatchMetadata::metadata_file(&pkg_source_dir);

        match PatchMetadata::read_from_file(metadata_file) {
            Ok(saved_patch_info) => {
                // Override target package
                patch_info.target = saved_patch_info.target;

                // Override patch release
                if patch_info.version == saved_patch_info.version {
                    patch_info.release = u32::max(patch_info.release, saved_patch_info.release + 1);
                }

                // Overide patch list
                let mut new_patches = Self::collect_patch_files(
                    fs::list_files_by_ext(
                        metadata_dir,
                        PATCH_FILE_EXT,
                        fs::TraverseOptions { recursive: false },
                    )
                    .context("Failed to find patch files")?,
                )
                .context("Failed to collect patch file from metadata directory")?;

                if new_patches.is_empty() {
                    bail!("Cannot find any patch file from metadata");
                }
                new_patches.append(&mut patch_info.patches);

                patch_info.patches = new_patches;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("Failed to read metadata"),
        }

        Ok(())
    }
}
