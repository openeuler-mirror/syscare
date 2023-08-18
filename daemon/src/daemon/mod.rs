use anyhow::{ensure, Context, Result};
use daemonize::Daemonize;
use jsonrpc_core::IoHandler;
use jsonrpc_ipc_server::{Server, ServerBuilder};
use log::{error, info};

use syscare_common::{os, util::fs};

mod args;
mod logger;

use args::*;
use logger::*;

use crate::{
    fast_reboot::{FastRebootSkeleton, FastRebootSkeletonImpl},
    patch::{PatchSkeleton, PatchSkeletonImpl},
};

const DAEMON_NAME: &str = env!("CARGO_PKG_NAME");
const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
const DAEMON_UMASK: u32 = 0o027;

struct Daemon {
    uid: u32,
    args: Arguments,
}

impl Daemon {
    pub fn new() -> Self {
        Self {
            uid: os::user::id(),
            args: Arguments::new(),
        }
    }

    fn check_root_permission(&self) -> Result<()> {
        const ROOT_UID: u32 = 0;

        ensure!(
            self.uid == ROOT_UID,
            "This command has to be run with superuser privileges (under the root user on most systems)."
        );

        Ok(())
    }

    fn initialize_logger(&self) -> Result<()> {
        Logger::initialize(&self.args.log_dir, self.args.log_level)?;

        if !self.args.daemon {
            Logger::duplicate_to_stdout()?;
        }

        Ok(())
    }

    fn prepare_environment(&self) {
        fs::create_dir_all(&self.args.work_dir).ok();
        fs::create_dir_all(&self.args.data_dir).ok();
        fs::create_dir_all(&self.args.log_dir).ok();
        fs::remove_file(&self.args.socket_file).ok();
        fs::remove_file(&self.args.pid_file).ok();
    }

    fn daemonize(&self) -> Result<()> {
        if !self.args.daemon {
            return Ok(());
        }

        Daemonize::new()
            .pid_file(&self.args.pid_file)
            .chown_pid_file(true)
            .working_directory(&self.args.work_dir)
            .umask(DAEMON_UMASK)
            .start()
            .context("Daemonize failed")
    }

    fn initialize_skeletons(&self) -> Result<IoHandler> {
        let mut io_handler = IoHandler::new();

        PatchSkeletonImpl::initialize(&self.args.data_dir)?;
        io_handler.extend_with(PatchSkeletonImpl.to_delegate());
        io_handler.extend_with(FastRebootSkeletonImpl.to_delegate());

        Ok(io_handler)
    }

    fn start_rpc_server(&self, io_handler: IoHandler) -> Result<Server> {
        let socket_file = self.args.socket_file.as_path();
        if socket_file.exists() {
            std::fs::remove_file(socket_file).ok();
        }

        let builder = ServerBuilder::new(io_handler).set_client_buffer_size(1);

        let server = builder.start(
            socket_file
                .to_str()
                .context("Failed to convert socket path to string")?,
        )?;

        Ok(server)
    }

    fn start_and_run(self) -> Result<()> {
        self.check_root_permission()?;
        self.initialize_logger()?;

        info!("============================");
        info!("Syscare Daemon - v{}", DAEMON_VERSION);
        info!("============================");
        info!("Start with {:#?}", self.args);

        info!("Preparing environment...");
        self.prepare_environment();
        self.daemonize()?;

        info!("Initializing skeletons...");
        let io_handler = self.initialize_skeletons()?;

        info!("Starting remote procedure call server...");
        let server = self
            .start_rpc_server(io_handler)
            .context("Failed to create remote procedure call server")?;

        info!("Daemon is running...");
        server.wait();

        info!("Daemon exited");
        Ok(())
    }
}

pub fn run() -> i32 {
    match Daemon::new().start_and_run() {
        Ok(_) => 0,
        Err(e) => {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {:?}", e)
                }
                true => {
                    error!("{:#}", e);
                    error!("Process exited unsuccessfully");
                }
            }
            -1
        }
    }
}
