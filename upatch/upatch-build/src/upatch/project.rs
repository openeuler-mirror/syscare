use std::fs::File;
use std::io::Write;

use log::info;
use crate::cmd::ExternCommand;

use super::Result;
use super::Error;

const COMPILER_CMD_ENV: &str = "UPATCH_CMD";
const ASSEMBLER_DIR_ENV: &str = "UPATCH_OUTPUT";
const BUILD_SHELL: &str = "build.sh";

pub struct Project {
    project_dir: String,
}

impl Project {
    pub fn new(project_dir: String) -> Self {
        Self {
            project_dir,
        }
    }

    pub fn build(&self, cmd: &str, output: &str, build_command: String) -> Result<()> {
        let command_shell_str = format!("{}/{}", output, BUILD_SHELL);
        let mut command_shell = File::create(&command_shell_str)?;
        command_shell.write_all((&build_command).as_ref())?;
        let args_list = vec![&command_shell_str];
        let envs_list = vec![
            (COMPILER_CMD_ENV, cmd),
            (ASSEMBLER_DIR_ENV, output)
        ];
        let output = ExternCommand::new("sh").execve(args_list, envs_list, &self.project_dir)?;
        if !output.exit_status().success() {
            return Err(Error::Project(format!("build project error {}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    pub fn patch(&self, patch: String, verbose: bool) -> Result<()> {
        let args_list = vec!["-N", "-p1"];
        let output = ExternCommand::new("patch").execvp_file(args_list, &self.project_dir, &patch)?;
        match output.exit_status().success() {
            false => return Err(Error::Project(format!("patch file {} error {}: {}", patch,  output.exit_code(), output.stderr()))),
            true => match verbose {
                true => (),
                false => info!("{}", output.stdout()),
            }
        };
        Ok(())
    }
}