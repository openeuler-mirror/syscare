// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * metadata-viewer is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::env;

use anyhow::{Context, Result};
use flexi_logger::{LevelFilter, LogSpecification, Logger, WriteMode};
use log::{debug, info};

use syscare_abi::{PatchInfo, PATCH_INFO_MAGIC};
use syscare_common::util::serde;

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

mod args;
use args::Arguments;

fn main() -> Result<()> {
    let args = Arguments::new()?;

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

    for file in &args.files {
        let patch_info = serde::deserialize_with_magic::<PatchInfo, _, _>(file, PATCH_INFO_MAGIC)?;

        info!("=============================================");
        info!("Patch: {}", patch_info.uuid);
        info!("=============================================");
        info!("{}", patch_info);
        info!("---------------------------------------------");
        info!("");

        info!("---------------------------------------------");
        info!("Patch target");
        info!("---------------------------------------------");
        info!("{}", patch_info.target);
        info!("---------------------------------------------");
        info!("");

        info!("---------------------------------------------");
        info!("Patch entity");
        info!("---------------------------------------------");
        for entity in &patch_info.entities {
            info!("{}", entity);
            info!("---------------------------------------------");
        }
        info!("");

        info!("---------------------------------------------");
        info!("Patch file");
        info!("---------------------------------------------");
        for patch in &patch_info.patches {
            info!("{}", patch);
            info!("---------------------------------------------");
        }
        info!("");
    }

    Ok(())
}
