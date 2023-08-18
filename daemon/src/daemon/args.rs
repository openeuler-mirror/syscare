use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use log::LevelFilter;

use syscare_common::util::fs;

use super::{DAEMON_NAME, DAEMON_VERSION};

const DEFAULT_DATA_ROOT: &str = "/usr/lib/syscare";
const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_PID_FILE: &str = "/var/run/syscare.pid";
const DEFAULT_LOG_DIR: &str = "/var/log/syscare";
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_SOCKET_FILE: &str = "/var/run/syscare.sock";

#[derive(Debug, Clone, Parser)]
#[clap(bin_name=DAEMON_NAME, version=DAEMON_VERSION)]
pub struct Arguments {
    /// Run as a daemon
    #[clap(short, long)]
    pub daemon: bool,

    /// Daemon working directory
    #[clap(long, default_value=DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    /// Daemon data directory
    #[clap(long, default_value=DEFAULT_DATA_ROOT)]
    pub data_dir: PathBuf,

    /// Path for daemon pid file
    #[clap(long, default_value=DEFAULT_PID_FILE)]
    pub pid_file: PathBuf,

    /// Path for daemon unix socket
    #[clap(long, default_value=DEFAULT_SOCKET_FILE)]
    pub socket_file: PathBuf,

    /// Path for daemon log file
    #[clap(long, default_value=DEFAULT_LOG_DIR)]
    pub log_dir: PathBuf,

    /// Set the logging level ("trace"|"debug"|"info"|"warn"|"error")
    #[clap(short, long, default_value=DEFAULT_LOG_LEVEL)]
    pub log_level: LevelFilter,
}

impl Arguments {
    pub fn new() -> Self {
        Arguments::parse()
            .normalize_pathes()
            .expect("Failed to parse arguments")
    }

    fn normalize_pathes(mut self) -> Result<Self> {
        self.work_dir = fs::normalize(self.work_dir)?;
        self.data_dir = fs::normalize(self.data_dir)?;
        self.pid_file = fs::normalize(&self.pid_file)?;
        self.socket_file = fs::normalize(&self.socket_file)?;
        self.log_dir = fs::normalize(&self.log_dir)?;

        Ok(self)
    }
}
