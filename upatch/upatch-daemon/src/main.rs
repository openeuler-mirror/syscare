use std::{fs, path::Path, process};

use anyhow::{ensure, Context, Result};
use daemonize::Daemonize;
use hijacker::Hijacker;
use jsonrpc_core::IoHandler;
use jsonrpc_ipc_server::{SecurityAttributes, Server, ServerBuilder};
use log::{error, info};

use syscare_common::os;

mod args;
mod hijacker;
mod logger;
mod rpc;

use args::Arguments;
use logger::Logger;
use rpc::{Skeleton, SkeletonImpl};

const DAEMON_NAME: &str = env!("CARGO_PKG_NAME");
const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
const DAEMON_UMASK: u32 = 0o027;

struct Daemon {
    uid: u32,
    args: Arguments,
}

impl Daemon {
    fn new() -> Self {
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
        self.prepare_directory(&self.args.log_dir)?;
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

    fn initialize_logger(&self) -> Result<()> {
        let max_level = self.args.log_level;
        let duplicate_stdout = !self.args.daemon;
        Logger::initialize(&self.args.log_dir, max_level, duplicate_stdout)?;

        Ok(())
    }

    fn initialize_skeleton(&self, hijacker: Hijacker) -> Result<IoHandler> {
        let mut io_handler = IoHandler::new();
        io_handler.extend_with(SkeletonImpl::new(hijacker)?.to_delegate());

        Ok(io_handler)
    }

    fn start_rpc_server(&self, io_handler: IoHandler) -> Result<Server> {
        let socket_file = self.args.socket_file.as_path();
        let builder = ServerBuilder::new(io_handler).set_client_buffer_size(1);
        let security_attr = SecurityAttributes::empty().allow_everyone_connect()?;
        let server = builder.set_security_attributes(security_attr).start(
            socket_file
                .to_str()
                .context("Failed to convert socket path to string")?,
        )?;

        Ok(server)
    }

    fn start_and_run(&self) -> Result<()> {
        self.check_root_permission()?;
        self.initialize_logger()?;

        info!("============================");
        info!("Syscare Builder Daemon - v{}", DAEMON_VERSION);
        info!("============================");
        info!("Preparing environment...");
        self.prepare_environment()?;

        info!("Start with {:#?}", self.args);
        self.daemonize()?;

        info!("Initializing hijacker...");
        let hijacker =
            Hijacker::new(&self.args.config_file).context("Failed to initialize hijacker")?;

        info!("Initializing skeleton...");
        let io_handler = self
            .initialize_skeleton(hijacker)
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
    process::exit(exit_code);
}
