use anyhow::Result;
use clap::Parser;
use log::{debug, error, info, LevelFilter};

use crate::args::Arguments;

use super::logger::Logger;

#[derive(Debug)]
pub struct SyscareCLI {
    args: Arguments,
}

impl SyscareCLI {
    fn new() -> Self {
        Self {
            args: Arguments::parse(),
        }
    }

    fn initialize(&self) -> Result<()> {
        Logger::initialize(match self.args.verbose {
            false => LevelFilter::Info,
            true => LevelFilter::Debug,
        })?;

        Ok(())
    }

    fn start_and_run(self) -> Result<()> {
        self.initialize()?;

        debug!("Command {}", self.args);
        // let exit_code = cmd_executor.invoke(&cmd_arguments)?;
        debug!("Command {} done", self.args);

        Ok(())
    }
}

impl SyscareCLI {
    pub fn run() -> i32 {
        let (exit_code, err_msg) = match SyscareCLI::new().start_and_run() {
            Ok(_) => (0, None),
            Err(e) => (1, Some(e)),
        };

        if let Some(err) = err_msg {
            match Logger::is_inited() {
                false => {
                    eprintln!("Error: {:?}", err)
                }
                true => {
                    error!("{:#}", err);
                    info!("Process exited unsuccessfully, exit_code={}", exit_code);
                }
            }
            return exit_code;
        }

        info!("Process exited normally, exit_code={}", exit_code);
        exit_code
    }
}
