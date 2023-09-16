use std::{process::exit, rc::Rc};

use anyhow::Result;
use log::{debug, error, LevelFilter};

mod args;
mod executor;
mod flock;
mod logger;
mod rpc;

use args::Arguments;
use executor::{
    build::BuildCommandExecutor, patch::PatchCommandExecutor, reboot::RebootCommandExecutor,
    CommandExecutor,
};
use flock::ExclusiveFileLockGuard;
use logger::Logger;
use rpc::{PatchProxy, RebootProxy, RpcRemote};

const CLI_LOCK_FILE_PATH: &str = "/var/run/syscare.lock";

struct SyscareCLI {
    args: Arguments,
}

impl SyscareCLI {
    fn start_and_run() -> Result<()> {
        let instance = Self {
            args: Arguments::new()?,
        };
        Logger::initialize(match instance.args.verbose {
            true => LevelFilter::Debug,
            false => LevelFilter::Info,
        })?;
        debug!("Start with {:#?}", instance.args);

        debug!("Acquiring exclusive file lock...");
        let _guard = ExclusiveFileLockGuard::new(CLI_LOCK_FILE_PATH)?;

        debug!("Initializing remote procedure call client...");
        let remote = Rc::new(RpcRemote::new(&instance.args.socket_file));

        debug!("Initializing remote procedure calls...");
        let patch_proxy = PatchProxy::new(remote.clone());
        let reboot_proxy = RebootProxy::new(remote);

        debug!("Initializing command executors...");
        let executors = vec![
            Box::new(BuildCommandExecutor) as Box<dyn CommandExecutor>,
            Box::new(PatchCommandExecutor::new(patch_proxy)) as Box<dyn CommandExecutor>,
            Box::new(RebootCommandExecutor::new(reboot_proxy)) as Box<dyn CommandExecutor>,
        ];

        let command = instance.args.command;
        debug!("Invoking command: {:#?}", command);
        for executor in &executors {
            executor.invoke(&command)?;
        }
        debug!("Done");

        Ok(())
    }
}

fn main() {
    let exit_code = match SyscareCLI::start_and_run() {
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
            1
        }
    };
    exit(exit_code);
}
