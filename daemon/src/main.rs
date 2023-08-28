use std::{fs, path::Path, process::exit};

use anyhow::{ensure, Context, Result};
use daemonize::Daemonize;
use jsonrpc_core::IoHandler;
use jsonrpc_ipc_server::{Server, ServerBuilder};
use kmod::KernelModuleGuard;
use log::{error, info, LevelFilter};

use once_cell::sync::OnceCell;
use syscare_common::os;

mod args;
mod fast_reboot;
mod kmod;
mod logger;
mod patch;
mod rpc;

use args::*;
use logger::*;

use rpc::{
    skeleton::{FastRebootSkeleton, PatchSkeleton},
    skeleton_impl::{FastRebootSkeletonImpl, PatchSkeletonImpl},
};

const DAEMON_NAME: &str = env!("CARGO_PKG_NAME");
const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
const DAEMON_UMASK: u32 = 0o027;

struct Daemon {
    uid: u32,
    args: Arguments,
    guard: OnceCell<KernelModuleGuard>,
}

impl Daemon {
    fn new() -> Self {
        Self {
            uid: os::user::id(),
            args: Arguments::new(),
            guard: OnceCell::new(),
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
        let max_level = self.args.log_level;
        let stdout_level = match self.args.daemon {
            true => LevelFilter::Off,
            false => max_level,
        };
        Logger::initialize(&self.args.log_dir, max_level, stdout_level)?;

        Ok(())
    }

    fn prepare_directory<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let dir_path = path.as_ref();
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).with_context(|| {
                format!("Failed to create directory \"{}\"", dir_path.display())
            })?;
        }
        Ok(())
    }

    fn prepare_environment(&self) -> Result<()> {
        self.prepare_directory(&self.args.work_dir)?;
        self.prepare_directory(&self.args.data_dir)?;
        self.prepare_directory(&self.args.log_dir)?;

        self.guard
            .get_or_try_init(KernelModuleGuard::new)
            .context("Failed to initialize dependency")?;

        Ok(())
    }

    fn daemonize(&self) -> Result<()> {
        if !self.args.daemon {
            return Ok(());
        }

        Daemonize::new()
            .pid_file(&self.args.pid_file)
            .working_directory(&self.args.work_dir)
            .umask(DAEMON_UMASK)
            .start()
            .context("Daemonize failed")
    }

    fn initialize_skeletons(&self) -> Result<IoHandler> {
        let mut io_handler = IoHandler::new();

        let patch_skeleton = PatchSkeletonImpl::new(&self.args.data_dir)?;
        io_handler.extend_with(patch_skeleton.to_delegate());
        io_handler.extend_with(FastRebootSkeletonImpl.to_delegate());

        Ok(io_handler)
    }

    fn start_rpc_server(&self, io_handler: IoHandler) -> Result<Server> {
        let socket_path = &self
            .args
            .socket_file
            .to_str()
            .context("Failed to convert socket path to string")?;

        let server = ServerBuilder::new(io_handler)
            .set_client_buffer_size(1)
            .start(socket_path)?;

        Ok(server)
    }

    fn start_and_run(self) -> Result<()> {
        self.check_root_permission()?;
        self.initialize_logger()?;

        info!("============================");
        info!("Syscare Daemon - v{}", DAEMON_VERSION);
        info!("============================");
        info!("Preparing environment...");
        self.prepare_environment()?;

        info!("Start with {:#?}", self.args);
        self.daemonize()?;

        info!("Initializing skeletons...");
        let io_handler = self
            .initialize_skeletons()
            .context("Failed to initialize skeleton")?;

        info!("Starting remote procedure call server...");
        let server = self
            .start_rpc_server(io_handler)
            .context("Failed to create remote procedure call server")?;

        info!("Daemon is running...");
        server.wait();

        Ok(())
    }
}

pub fn main() {
    let exit_code = match Daemon::new().start_and_run() {
        Ok(_) => {
            info!("Daemon exited");
            0
        }
        Err(e) => {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {:?}", e)
                }
                true => {
                    error!("{:#}", e);
                    error!("Daemon exited unsuccessfully");
                }
            }
            -1
        }
    };
    exit(exit_code);
}
