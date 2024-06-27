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

use std::{env, process, rc::Rc};

use anyhow::{Context, Result};
use flexi_logger::{DeferredNow, LogSpecification, Logger, LoggerHandle, WriteMode};
use log::{debug, error, LevelFilter, Record};

mod args;
mod executor;
mod rpc;

use args::Arguments;
use executor::{build::BuildCommandExecutor, patch::PatchCommandExecutor, CommandExecutor};
use rpc::{RpcProxy, RpcRemote};
use syscare_common::{concat_os, os};

pub const CLI_NAME: &str = env!("CARGO_PKG_NAME");
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o077;

const PATH_ENV_NAME: &str = "PATH";
const PATH_ENV_VALUE: &str = "/usr/libexec/syscare";

const SOCKET_FILE_NAME: &str = "syscared.sock";
const PATCH_OP_LOCK_NAME: &str = "patch_op.lock";

struct SyscareCLI {
    args: Arguments,
    logger: LoggerHandle,
}

impl SyscareCLI {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> std::io::Result<()> {
        write!(w, "{}", &record.args())
    }

    fn new() -> Result<Self> {
        // Initialize arguments & prepare environments
        os::umask::set_umask(CLI_UMASK);
        if let Some(path_env) = env::var_os(PATH_ENV_NAME) {
            env::set_var(PATH_ENV_NAME, concat_os!(PATH_ENV_VALUE, ":", path_env));
        }

        let args = Arguments::new()?;

        // Initialize logger
        let log_level_max = if args.verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        };
        let log_spec = LogSpecification::builder().default(log_level_max).build();
        let logger = Logger::with(log_spec)
            .log_to_stdout()
            .format(Self::format_log)
            .write_mode(WriteMode::Direct)
            .start()
            .context("Failed to initialize logger")?;

        Ok(Self { args, logger })
    }
}

impl SyscareCLI {
    fn run(&self) -> Result<i32> {
        debug!("Start with {:#?}", self.args);

        debug!("Initializing remote procedure call client...");
        let socket_file = self.args.work_dir.join(SOCKET_FILE_NAME);
        let remote = Rc::new(RpcRemote::new(socket_file));

        debug!("Initializing remote procedure calls...");
        let patch_proxy = RpcProxy::new(remote);

        debug!("Initializing command executors...");
        let patch_lock_file = self.args.work_dir.join(PATCH_OP_LOCK_NAME);
        let executors: Vec<Box<dyn CommandExecutor>> = vec![
            Box::new(BuildCommandExecutor),
            Box::new(PatchCommandExecutor::new(patch_proxy, patch_lock_file)),
        ];

        let command = &self.args.command;
        debug!("Invoking command: {:#?}", command);
        for executor in &executors {
            if let Some(exit_code) = executor.invoke(command)? {
                debug!("Done");
                return Ok(exit_code);
            }
        }

        Ok(0)
    }
}

impl Drop for SyscareCLI {
    fn drop(&mut self) {
        self.logger.flush();
        self.logger.shutdown();
    }
}

fn main() {
    let cli = match SyscareCLI::new() {
        Ok(instance) => instance,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(-1);
        }
    };

    match cli.run() {
        Ok(exit_code) => {
            process::exit(exit_code);
        }
        Err(e) => {
            error!("Error: {:?}", e);

            drop(cli);
            process::exit(-1);
        }
    }
}
