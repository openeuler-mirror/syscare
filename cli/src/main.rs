use std::process::exit;
use std::rc::Rc;

use anyhow::Result;
use clap::Parser;
use log::{debug, error};

mod args;
mod executor;
mod logger;
mod rpc;

use args::CliArguments;
use executor::{
    build::BuildCommandExecutor, patch::PatchCommandExecutor, reboot::RebootCommandExecutor,
    CommandExecutor,
};
use logger::Logger;
use rpc::{PatchProxy, RebootProxy, RpcRemote};

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

struct SyscareCLI {
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
        let patch_proxy = PatchProxy::new(remote.clone());
        let reboot_proxy = RebootProxy::new(remote);

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

fn main() {
    let exit_code = match SyscareCLI::new().start_and_run() {
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
    };
    exit(exit_code);
}
