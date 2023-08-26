use anyhow::{anyhow, Context, Result};

use std::process::{Command, exit};

use super::CommandExecutor;
use crate::args::CliCommand;

const SYSCARE_BUILD_NAME: &str = "syscare-build";
const SYSCARE_BUILD_PATH: &str = "/usr/libexec/syscare/syscare-build";

pub struct BuildCommandExecutor;

impl BuildCommandExecutor {
    fn exec_patch_build_cmd(args: &[String]) -> std::io::Result<i32> {
        Ok(Command::new(SYSCARE_BUILD_PATH)
            .args(args)
            .spawn()?
            .wait()?
            .code()
            .expect("Failed to get process exit code"))
    }
}

impl CommandExecutor for BuildCommandExecutor {
    fn invoke(&self, command: &CliCommand) -> Result<()> {
        if let CliCommand::Build { args } = command {
            let exit_code = Self::exec_patch_build_cmd(args)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        anyhow!("Command \"{}\" is not installed", SYSCARE_BUILD_NAME)
                    }
                    _ => e.into(),
                })
                .with_context(|| format!("Failed to start \"{}\" process", SYSCARE_BUILD_NAME))?;

            exit(exit_code);
        }

        Ok(())
    }
}
