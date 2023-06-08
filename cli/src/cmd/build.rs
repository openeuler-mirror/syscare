use std::process::Command;

use super::{CommandArguments, CommandExecutor};

const SYSCARE_BUILD_PATH: &str = "/usr/libexec/syscare/syscare-build";

pub struct BuildCommandExecutor;

impl BuildCommandExecutor {
    fn exec_build_cmd(args: &[String]) -> std::io::Result<i32> {
        Ok(Command::new(SYSCARE_BUILD_PATH)
            .args(args)
            .spawn()?
            .wait()?
            .code()
            .expect("get process exit code failed"))
    }
}

impl CommandExecutor for BuildCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::CommandLineArguments { args } = args {
            let build_result = Self::exec_build_cmd(args).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => {
                    std::io::Error::new(e.kind(), "Command \"syscare-build\" is not installed")
                }
                _ => std::io::Error::new(
                    e.kind(),
                    format!("Start process \"{}\" failed, {}", SYSCARE_BUILD_PATH, e),
                ),
            });
            return build_result;
        }

        Ok(0)
    }
}
