// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * metadata-generator is licensed under Mulan PSL v2.
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
    collections::HashSet,
    env,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use flexi_logger::{LevelFilter, LogSpecification, Logger, WriteMode};
use log::{debug, info};

use syscare_abi::{
    PackageInfo, PackageType, PatchEntity, PatchFile, PatchInfo, PatchType, PATCH_INFO_MAGIC,
};
use syscare_common::{
    fs, os,
    util::{digest, serde},
};

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

const KERNEL_PATCH_EXTENSION: &str = "ko";
const METADATA_FILE_NAME: &str = "patch_info";

mod args;
use args::Arguments;

fn detect_patch_type(args: &Arguments) -> PatchType {
    for patch in &args.entity_patch {
        if patch.extension().unwrap_or_default() == KERNEL_PATCH_EXTENSION {
            return PatchType::KernelPatch;
        }
    }
    PatchType::UserPatch
}

fn parse_target_string(target: &str) -> Option<(&str, &str, &str, &str)> {
    const PKG_INFO_SPLITTER: char = '-';
    const PKG_NAME_SPLITTER: char = ':';
    const PKG_DEFAULT_TAG: &str = "(none)";

    let mut split = target.rsplitn(3, PKG_INFO_SPLITTER);

    let release = split.next()?;
    let version = split.next()?;
    let name_epoch = split.next()?;
    let (name, epoch) = name_epoch
        .split_once(PKG_NAME_SPLITTER)
        .map_or((name_epoch, PKG_DEFAULT_TAG), |(name, epoch)| (name, epoch));

    Some((name, epoch, version, release))
}

fn parse_target_package(args: &Arguments) -> Result<PackageInfo> {
    let (name, epoch, version, release) = self::parse_target_string(&args.target)
        .with_context(|| format!("Patch target package '{}' is invalid", args.target))?;
    let pkg_info = PackageInfo {
        name: name.to_string(),
        kind: PackageType::SourcePackage,
        arch: args.arch.clone(),
        epoch: epoch.to_string(),
        version: version.to_string(),
        release: release.to_string(),
        license: args.license.clone(),
        source_pkg: format!("{}-{}-{}.src.rpm", name, version, release),
    };

    debug!("* {:#?}", pkg_info);
    Ok(pkg_info)
}

fn parse_patch_entities(args: &Arguments) -> Result<Vec<PatchEntity>> {
    let mut entities = vec![];
    let mut target_set = HashSet::new();
    let mut patch_set = HashSet::new();

    let mut uuid_iter = args.uuid[1..].iter();
    let mut target_iter = args.entity_target.iter();
    let mut patch_iter = args.entity_patch.iter();
    while let (Some(uuid), Some(target), Some(patch)) =
        (uuid_iter.next(), target_iter.next(), patch_iter.next())
    {
        if !target_set.insert(target) {
            bail!("Found duplicated patch target {}", target.display());
        }
        if !patch_set.insert(patch) {
            bail!("Found duplicated patch binary {}", patch.display());
        }

        let entity = PatchEntity {
            uuid: *uuid,
            patch_name: patch
                .file_name()
                .map(|name| {
                    let mut name = PathBuf::from(name);
                    if name.extension().unwrap_or_default() == KERNEL_PATCH_EXTENSION {
                        name.set_extension("");
                    }
                    name.into_os_string()
                })
                .context("Failed to parse patch binary name")?,
            patch_target: target.to_path_buf(),
            checksum: digest::file(patch).context("Failed to calculate patch binary checksum")?,
        };

        debug!("* {:#?}", entity);
        entities.push(entity);
    }

    Ok(entities)
}

fn parse_patch_files(args: &Arguments) -> Result<Vec<PatchFile>> {
    let mut patches = vec![];
    let mut patch_set = HashSet::new();

    for patch in &args.patch_file {
        if !patch_set.insert(patch) {
            bail!("Found duplicated patch file {}", patch.display());
        }
        let name = patch
            .file_name()
            .context("Failed to parse patch file name")?;
        let file = PatchFile {
            name: name.to_os_string(),
            path: Path::new(&Component::CurDir).join(name),
            digest: digest::file(patch).context("Failed to calculate patch file checksum")?,
        };

        debug!("* {:#?}", file);
        patches.push(file);
    }

    Ok(patches)
}

fn generate_patch_metadata(args: &Arguments) -> Result<PatchInfo> {
    let patch_info = PatchInfo {
        uuid: args.uuid[0],
        name: args.name.clone(),
        version: args.version.clone(),
        release: args.release,
        arch: args.arch.clone(),
        kind: self::detect_patch_type(args),
        target: self::parse_target_package(args)?,
        entities: self::parse_patch_entities(args)?,
        description: args.description.clone(),
        patches: self::parse_patch_files(args)?,
    };
    Ok(patch_info)
}

fn write_patch_metadata(patch_info: &PatchInfo, output_dir: &Path) -> Result<()> {
    serde::serialize_with_magic(
        &patch_info,
        output_dir.join(METADATA_FILE_NAME),
        PATCH_INFO_MAGIC,
    )?;
    Ok(())
}

fn copy_patch_files(patch_files: &[PathBuf], output_dir: &Path) -> Result<()> {
    for src_path in patch_files {
        let dst_path = output_dir.join(
            src_path
                .file_name()
                .context("Failed to parse patch file name")?,
        );
        fs::copy(src_path, &dst_path)?;
    }
    Ok(())
}

fn main_process(args: &Arguments, output_dir: &Path) -> Result<()> {
    debug!("Generating patch metadata...");
    let patch_info =
        self::generate_patch_metadata(args).context("Failed to generate patch metadata")?;

    debug!("Writing patch metadata...");
    self::write_patch_metadata(&patch_info, output_dir)
        .context("Failed to write patch metadata")?;

    debug!("Copying patch files...");
    self::copy_patch_files(&args.entity_patch, output_dir).context("Failed to copy patch files")?;

    info!("---------------------------------------------");
    info!("Patch: {}", patch_info.uuid);
    info!("---------------------------------------------");
    info!("{}", patch_info);
    info!("---------------------------------------------");
    info!("");
    info!("Output: {}", output_dir.display());

    Ok(())
}

fn main() -> Result<()> {
    let args = Arguments::new()?;
    os::umask::set_umask(CLI_UMASK);

    let log_level = if args.verbose {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };
    let log_spec = LogSpecification::builder().default(log_level).build();
    let _ = Logger::with(log_spec)
        .log_to_stdout()
        .format(|w, _, record| write!(w, "{}", record.args()))
        .write_mode(WriteMode::Direct)
        .start()
        .context("Failed to initialize logger")?;

    debug!("===================================");
    debug!("{}", CLI_ABOUT);
    debug!("Version: {}", CLI_VERSION);
    debug!("===================================");
    debug!("{:#?}", args);
    debug!("");

    let output_dir = PathBuf::from(&args.output_dir).join(args.uuid[0].to_string());
    fs::create_dir_all(&output_dir).context("Failed to carete output directory")?;

    if let Err(e) = self::main_process(&args, &output_dir) {
        fs::remove_dir_all(&output_dir).ok();
        return Err(e);
    }

    debug!("Done");
    Ok(())
}
