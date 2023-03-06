use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::Level;

use crate::cmd::*;

use super::Result;
use super::Error;

const COMPILER_CMD_ENV: &str = "UPATCH_CMD";
const ASSEMBLER_DIR_ENV: &str = "UPATCH_AS_OUTPUT";
const BUILD_SHELL: &str = "build.sh";

pub struct Project {
    project_dir: PathBuf,
}

impl Project {
    pub fn new<P: AsRef<Path>>(project_dir: P) -> Self {
        Self {
            project_dir: project_dir.as_ref().to_path_buf()
        }
    }

    pub fn build<P: AsRef<Path>>(&self, cmd: &str, assembler_output: P, build_command: String) -> Result<()> {
        let assembler_output = assembler_output.as_ref();
        let command_shell_path = assembler_output.join(BUILD_SHELL);
        let mut command_shell = File::create(&command_shell_path)?;
        command_shell.write_all(build_command.as_ref())?;
        let args_list = ExternCommandArgs::new().arg(command_shell_path);
        let envs_list = ExternCommandEnvs::new().env(COMPILER_CMD_ENV, cmd).envs([
            (ASSEMBLER_DIR_ENV, assembler_output)
        ]);
        let output = ExternCommand::new("sh").execve(args_list, envs_list, &self.project_dir)?;
        if !output.exit_status().success() {
            return Err(Error::Project(format!("build project error {}: {:?}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    pub fn patch<P: AsRef<Path>>(&self, patch: P, level: Level) -> Result<()> {
        let patch = patch.as_ref();
        let args_list = ExternCommandArgs::new().args(["-N", "-p1"]);
        let output = ExternCommand::new("patch").execvp_stdio_level(args_list, &self.project_dir, File::open(&patch).expect(&format!("open {} error", patch.display())), level)?;
        if !output.exit_status().success() {
            return Err(Error::Project(format!("patch file {} error {}: {:?}", patch.display(), output.exit_code(), output.stderr())));
        };
        Ok(())
    }
}