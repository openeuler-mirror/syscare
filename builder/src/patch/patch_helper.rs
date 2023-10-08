use std::{collections::HashSet, path::Path};

use anyhow::{bail, Result};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use syscare_abi::PatchFile;
use syscare_common::util::{digest, fs};

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
}
