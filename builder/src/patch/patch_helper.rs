use std::{collections::HashSet, path::Path, sync::Mutex};

use lazy_static::lazy_static;
use log::{log, Level};
use syscare_abi::{PackageInfo, PatchEntity, PatchFile, PatchInfo, PatchType};
use syscare_common::util::{digest, fs};
use uuid::Uuid;

use crate::args::Arguments;

pub const PATCH_FILE_EXT: &str = "patch";
pub const PATCH_INFO_FILE_NAME: &str = "patch_info";

pub struct PatchHelper;

impl PatchHelper {
    fn is_patch_file_digest_exists<S: AsRef<str>>(digest: S) -> bool {
        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }
        FILE_DIGESTS
            .lock()
            .expect("Lock failed")
            .insert(digest.as_ref().to_owned())
    }

    fn parse_patch_file<P: AsRef<Path>>(path: P) -> std::io::Result<PatchFile> {
        let file_path = fs::canonicalize(path)?;
        let file_name = fs::file_name(&file_path);
        let file_digest = digest::file(&file_path)?;

        if !Self::is_patch_file_digest_exists(&file_digest) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch \"{}\" is duplicated", file_path.display()),
            ));
        }

        Ok(PatchFile {
            name: file_name,
            path: file_path,
            digest: file_digest,
        })
    }

    pub fn parse_patch_entity<P: AsRef<Path>, Q: AsRef<Path>>(
        patch_file: P,
        elf_file: Q,
    ) -> std::io::Result<PatchEntity>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        Ok(PatchEntity {
            uuid: Uuid::new_v4().to_string(),
            patch_name: fs::file_name(patch_file.as_ref()),
            patch_target: elf_file.as_ref().to_owned(),
            checksum: digest::file(patch_file.as_ref())?,
        })
    }

    pub fn parse_patch_info(args: &Arguments, target_package: &PackageInfo) -> PatchInfo {
        const KERNEL_PKG_NAME: &str = "kernel";

        let patch_type = match target_package.name == KERNEL_PKG_NAME {
            true => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };

        PatchInfo {
            uuid: Uuid::new_v4().to_string(),
            name: args.patch_name.to_owned(),
            kind: patch_type,
            version: args.patch_version.to_owned(),
            release: args.patch_release.to_owned(),
            arch: args.patch_arch.to_owned(),
            target: target_package.to_owned(),
            entities: Vec::new(),
            description: args.patch_description.to_owned(),
            patches: args
                .patches
                .iter()
                .flat_map(Self::parse_patch_file)
                .collect(),
        }
    }

    pub fn print_patch_info(patch_info: &PatchInfo, level: Level) {
        const PATCH_FLAG_NONE: &str = "(none)";

        let patch_elfs = match patch_info.entities.is_empty() {
            true => PATCH_FLAG_NONE.to_owned(),
            false => patch_info
                .entities
                .iter()
                .map(|entity| {
                    format!(
                        "{}, ",
                        fs::file_name(&entity.patch_target).to_string_lossy()
                    )
                })
                .collect::<String>()
                .trim_end_matches(", ")
                .to_string(),
        };

        log!(level, "uuid:        {}", patch_info.uuid);
        log!(level, "name:        {}", patch_info.name);
        log!(level, "version:     {}", patch_info.version);
        log!(level, "release:     {}", patch_info.release);
        log!(level, "arch:        {}", patch_info.arch);
        log!(level, "type:        {}", patch_info.kind);
        log!(level, "target:      {}", patch_info.target.short_name());
        log!(level, "target_elf:  {}", patch_elfs);
        log!(level, "license:     {}", patch_info.target.license);
        log!(level, "description: {}", patch_info.description);
        log!(level, "patch:");
        let mut patch_id = 1usize;
        for patch_file in &patch_info.patches {
            log!(level, "{}. {}", patch_id, patch_file.name.to_string_lossy());
            patch_id += 1;
        }
    }
}
