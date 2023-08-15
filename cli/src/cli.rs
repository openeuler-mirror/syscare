use std::rc::Rc;

use anyhow::Result;
use clap::Parser;
use log::{debug, error};

use crate::executor::build::BuildCommandExecutor;
use crate::executor::patch::PatchCommandExecutor;
use crate::executor::reboot::RebootCommandExecutor;

use super::args::CliArguments;
use super::executor::CommandExecutor;
use super::logger::Logger;
use super::proxy::{PatchProxy, RebootProxy};
use super::rpc::RpcRemote;

pub struct SyscareCLI {
    args: CliArguments,
}

impl SyscareCLI {
    fn new() -> Self {
        Self {
            args: CliArguments::parse(),
        }
    }

    fn start_and_run(self) -> Result<()> {
        Logger::initialize(match self.args.verbose {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        })?;
        debug!("{:#?}", self.args);

        debug!("Initializing remote procedure call client...");
        let remote = Rc::new(RpcRemote::new(&self.args.socket_file));

        debug!("Initializing remote procedure calls...");
        let patch_proxy = PatchProxy::from(remote.clone());
        let reboot_proxy = RebootProxy::from(remote.clone());

        debug!("Initializing command executors...");
        let executors = vec![
            Box::new(BuildCommandExecutor) as Box<dyn CommandExecutor>,
            Box::new(PatchCommandExecutor::new(patch_proxy)) as Box<dyn CommandExecutor>,
            Box::new(RebootCommandExecutor::new(reboot_proxy)) as Box<dyn CommandExecutor>,
        ];

        let command = self.args.command;
        debug!("Invoking command: {:#?}", command);
        for executor in &executors {
            executor.invoke(&command)?;
        }
        debug!("Done");

        Ok(())
    }
}

impl SyscareCLI {
    pub fn run() -> i32 {
        match SyscareCLI::new().start_and_run() {
            Ok(_) => 0,
            Err(e) => {
                match Logger::is_inited() {
                    false => {
                        eprintln!("Error: {:?}", e)
                    }
                    true => {
                        error!("Error: {:?}", e);
                    }
                }
                -1
            }
        }
    }
}
