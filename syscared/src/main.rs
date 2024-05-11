// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{fs::Permissions, os::unix::fs::PermissionsExt, panic, process, sync::Arc};

use anyhow::{ensure, Context, Result};
use daemonize::Daemonize;
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, Naming,
    WriteMode,
};
use jsonrpc_core::IoHandler;
use jsonrpc_ipc_server::{Server, ServerBuilder};
use log::{debug, error, info, warn, LevelFilter, Record};
use nix::unistd::{chown, Gid, Uid};
use parking_lot::RwLock;
use patch::manager::PatchManager;
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals, low_level::signal_name};

use syscare_common::{fs, os};

mod args;
mod config;
mod fast_reboot;
mod patch;
mod rpc;

use args::Arguments;
use config::Config;
use patch::monitor::PatchMonitor;
use rpc::{
    skeleton::{FastRebootSkeleton, PatchSkeleton},
    skeleton_impl::{FastRebootSkeletonImpl, PatchSkeletonImpl},
};

const DAEMON_NAME: &str = env!("CARGO_PKG_NAME");
const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
const DAEMON_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const DAEMON_UMASK: u32 = 0o077;

const CONFIG_FILE_NAME: &str = "syscared.yaml";
const PID_FILE_NAME: &str = "syscared.pid";
const SOCKET_FILE_NAME: &str = "syscared.sock";

const CONFIG_DIR_PERM: u32 = 0o700;
const DATA_DIR_PERM: u32 = 0o700;
const WORK_DIR_PERM: u32 = 0o755;
const LOG_DIR_PERM: u32 = 0o700;
const SOCKET_FILE_PERM: u32 = 0o660;
const SOCKET_FILE_PERM_STRICT: u32 = 0o600;

const MAIN_THREAD_NAME: &str = "main";
const UNNAMED_THREAD_NAME: &str = "<unnamed>";
const LOG_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";

struct Daemon {
    args: Arguments,
    config: Config,
}

impl Daemon {
    fn format_log(
        w: &mut dyn std::io::Write,
        now: &mut DeferredNow,
        record: &Record,
    ) -> std::io::Result<()> {
        thread_local! {
            static THREAD_NAME: String = std::thread::current().name().and_then(|name| {
                if name == MAIN_THREAD_NAME {
                    return os::process::name().to_str();
                }
                Some(name)
            })
            .unwrap_or(UNNAMED_THREAD_NAME)
            .to_string();
        }

        THREAD_NAME.with(|thread_name| {
            write!(
                w,
                "[{}] [{}] [{}] {}",
                now.format(LOG_FORMAT),
                record.level(),
                thread_name,
                &record.args()
            )
        })
    }

    fn new() -> Result<Self> {
        // Check root permission
        ensure!(
            os::user::id() == 0,
            "This command has to be run with superuser privileges (under the root user on most systems)."
        );

        // Initialize arguments & prepare environments
        os::umask::set_umask(DAEMON_UMASK);

        let args = Arguments::new()?;
        fs::create_dir_all(&args.config_dir)?;
        fs::create_dir_all(&args.data_dir)?;
        fs::create_dir_all(&args.work_dir)?;
        fs::create_dir_all(&args.log_dir)?;
        fs::set_permissions(&args.config_dir, Permissions::from_mode(CONFIG_DIR_PERM))?;
        fs::set_permissions(&args.data_dir, Permissions::from_mode(DATA_DIR_PERM))?;
        fs::set_permissions(&args.work_dir, Permissions::from_mode(WORK_DIR_PERM))?;
        fs::set_permissions(&args.log_dir, Permissions::from_mode(LOG_DIR_PERM))?;

        std::env::set_current_dir(&args.work_dir).with_context(|| {
            format!(
                "Failed to change current directory to {}",
                args.work_dir.display()
            )
        })?;

        // Initialize logger
        let max_level = args.log_level;
        let stdout_level = match args.daemon {
            true => LevelFilter::Off,
            false => max_level,
        };
        let log_spec = LogSpecification::builder().default(max_level).build();
        let file_spec = FileSpec::default()
            .directory(&args.log_dir)
            .use_timestamp(false);
        Logger::with(log_spec)
            .log_to_file(file_spec)
            .format(Self::format_log)
            .duplicate_to_stdout(Duplicate::from(stdout_level))
            .rotate(
                Criterion::Age(Age::Day),
                Naming::Timestamps,
                Cleanup::KeepCompressedFiles(30),
            )
            .write_mode(WriteMode::Direct)
            .start()
            .context("Failed to initialize logger")?;

        // Initialize config
        debug!("Initializing configuation...");
        let config_file = args.config_dir.join(CONFIG_FILE_NAME);
        let config = match Config::parse(&config_file) {
            Ok(config) => config,
            Err(e) => {
                warn!("{:?}", e);
                info!("Using default configuration...");
                let config = Config::default();
                config.write(&config_file)?;

                config
            }
        };

        // Print panic to log incase it really happens
        panic::set_hook(Box::new(|info| error!("{}", info)));

        Ok(Self { args, config })
    }
}

impl Daemon {
    fn daemonize(&self) -> Result<()> {
        if !self.args.daemon {
            return Ok(());
        }

        let pid_file = self.args.work_dir.join(PID_FILE_NAME);
        Daemonize::new()
            .umask(DAEMON_UMASK)
            .working_directory(&self.args.work_dir)
            .pid_file(pid_file)
            .start()
            .context("Daemonize failed")
    }

    fn initialize_skeletons(&self, patch_manager: Arc<RwLock<PatchManager>>) -> Result<IoHandler> {
        let mut io_handler = IoHandler::new();

        io_handler.extend_with(PatchSkeletonImpl::new(patch_manager).to_delegate());
        io_handler.extend_with(FastRebootSkeletonImpl.to_delegate());

        Ok(io_handler)
    }

    fn start_rpc_server(&self, io_handler: IoHandler) -> Result<Server> {
        let socket_file = self.args.work_dir.join(SOCKET_FILE_NAME);
        let builder = ServerBuilder::new(io_handler).set_client_buffer_size(1);
        let server = builder.start(
            socket_file
                .to_str()
                .context("Failed to convert socket path to string")?,
        )?;

        let socket_owner = Uid::from_raw(self.config.daemon.socket.uid);
        let socket_group = Gid::from_raw(self.config.daemon.socket.gid);
        chown(&socket_file, Some(socket_owner), Some(socket_group))?;

        fs::set_permissions(
            &socket_file,
            match socket_owner.as_raw() == socket_group.as_raw() {
                true => Permissions::from_mode(SOCKET_FILE_PERM_STRICT),
                false => Permissions::from_mode(SOCKET_FILE_PERM),
            },
        )?;

        Ok(server)
    }

    fn run(&self) -> Result<()> {
        info!("================================");
        info!("Syscare Daemon - {}", DAEMON_VERSION);
        info!("================================");
        info!("Start with {:#?}", self.args);
        self.daemonize()?;

        info!("Initializing patch manager...");
        let patch_root = &self.args.data_dir;
        let patch_manager = Arc::new(RwLock::new(
            PatchManager::new(patch_root).context("Failed to initialize patch manager")?,
        ));

        info!("Initializing patch monitor...");
        let _patch_monitor = PatchMonitor::new(patch_root, patch_manager.clone())
            .context("Failed to initialize patch monitor")?;

        info!("Initializing skeletons...");
        let io_handler = self
            .initialize_skeletons(patch_manager)
            .context("Failed to initialize skeleton")?;

        info!("Starting remote procedure call server...");
        let server = self
            .start_rpc_server(io_handler)
            .context("Failed to create remote procedure call server")?;

        info!("Daemon is running...");
        let mut signals =
            Signals::new(TERM_SIGNALS).context("Failed to initialize signal handler")?;
        if let Some(signal) = signals.forever().next() {
            info!(
                "Received {} signal",
                signal_name(signal).unwrap_or("UNKNOWN")
            );
        }

        info!("Shutting down...");
        server.close();

        Ok(())
    }
}

fn main() {
    let daemon = match Daemon::new() {
        Ok(instance) => instance,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(-1);
        }
    };

    if let Err(e) = daemon.run() {
        error!("Error: {:?}", e);
        error!("Daemon exited unsuccessfully");

        drop(daemon);
        process::exit(-1);
    }

    info!("Daemon exited");
}
