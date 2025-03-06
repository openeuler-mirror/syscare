// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{env, ffi::OsString, os::unix::process::CommandExt, process::Command};

use anyhow::{bail, Context, Result};
use args::SubCommand;
use flexi_logger::{LogSpecification, Logger, WriteMode};
use log::{debug, LevelFilter};

use syscare_common::{concat_os, ffi::OsStrExt, os};

mod args;
mod rpc;

use self::{
    args::Arguments,
    rpc::{PatchProxy, RpcClient},
};

pub const CLI_NAME: &str = env!("CARGO_PKG_NAME");
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
pub const CLI_UMASK: u32 = 0o022;

const PATH_ENV_NAME: &str = "PATH";
const PATH_ENV_VALUE: &str = "/usr/libexec/syscare";
const EXTERNAL_CMD_PREFIX: &str = "syscare-";

fn exec_external_cmd(mut args: Vec<OsString>) -> Result<()> {
    let program = concat_os!(EXTERNAL_CMD_PREFIX, args.remove(0).trim());

    let error = Command::new(&program).args(&args).exec();
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            bail!(
                "External command '{}' is not installed",
                program.to_string_lossy()
            );
        }
        _ => {
            bail!(
                "Failed to execute '{}', {}",
                program.to_string_lossy(),
                error
            );
        }
    }
}

fn main() -> Result<()> {
    // Parse arguments
    let args = Arguments::new()?;

    // Set up environments
    os::umask::set_umask(CLI_UMASK);
    if let Some(path_env) = env::var_os(PATH_ENV_NAME) {
        env::set_var(PATH_ENV_NAME, concat_os!(PATH_ENV_VALUE, ":", path_env));
    }

    // Initialize logger
    let max_log_level = if args.verbose {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };
    let log_spec = LogSpecification::builder().default(max_log_level).build();
    let _ = Logger::with(log_spec)
        .log_to_stdout()
        .format(|w, _, record| write!(w, "{}", record.args()))
        .write_mode(WriteMode::Direct)
        .start()
        .context("Failed to initialize logger")?;

    debug!("Start with {:#?}", args);
    if let SubCommand::External(cmd_args) = args.subcommand {
        self::exec_external_cmd(cmd_args)?;
        return Ok(());
    }

    debug!("Initializing rpc client...");
    let client = RpcClient::new(&args.work_dir).context("Failed to initialize rpc client")?;

    debug!("Invoking rpc call...");
    match &args.subcommand {
        SubCommand::Info { identifiers } => {
            PatchProxy::new(&client).show_patch_info(identifiers)?;
        }
        SubCommand::Target { identifiers } => {
            PatchProxy::new(&client).show_patch_target(identifiers)?;
        }
        SubCommand::Status { identifiers } => {
            PatchProxy::new(&client).show_patch_status(identifiers)?;
        }
        SubCommand::List => {
            PatchProxy::new(&client).show_patch_list()?;
        }
        SubCommand::Check { identifiers } => {
            PatchProxy::new(&client).check_patches(identifiers)?;
        }
        SubCommand::Apply { identifiers, force } => {
            PatchProxy::new(&client).apply_patches(identifiers, *force)?;
        }
        SubCommand::Remove { identifiers } => {
            PatchProxy::new(&client).remove_patches(identifiers)?;
        }
        SubCommand::Active { identifiers, force } => {
            PatchProxy::new(&client).active_patches(identifiers, *force)?;
        }
        SubCommand::Deactive { identifiers } => {
            PatchProxy::new(&client).deactive_patches(identifiers)?;
        }
        SubCommand::Accept { identifiers } => {
            PatchProxy::new(&client).accept_patches(identifiers)?;
        }
        SubCommand::Save => {
            PatchProxy::new(&client).save_patches()?;
        }
        SubCommand::Restore { accepted } => {
            PatchProxy::new(&client).restore_patches(*accepted)?;
        }
        SubCommand::Rescan => {
            PatchProxy::new(&client).rescan_all_patches()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
