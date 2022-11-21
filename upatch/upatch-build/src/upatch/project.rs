use std::process::Command;
use std::fs::File;
use std::io::Write;

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
        let result = Command::new("sh")
            .arg(&command_shell_str)
            .current_dir(&self.project_dir)
            .env(COMPILER_CMD_ENV, cmd)
            .env(ASSEMBLER_DIR_ENV, output)
            .output()?;
        if !result.status.success(){
            return Err(Error::Project(format!("build project error {}: {}", result.status, String::from_utf8(result.stderr).unwrap_or_default())));
        }

        Ok(())
    }

    pub fn patch(&self, patch: String) -> Result<()> {
        let mut build_cmd = Command::new("patch");
        let result = build_cmd.current_dir(&self.project_dir).arg("-N").arg("-p1").stdin(File::open(&patch).unwrap()).output()?;
        match result.status.success() {
            true =>{
                println!("{}", String::from_utf8(result.stdout).unwrap().trim());
                Ok(())
            },
            false => {
                Err(Error::Project(format!("patch file {} error: {}", patch, String::from_utf8(result.stderr).unwrap().trim())))
            }
        }
    }
}