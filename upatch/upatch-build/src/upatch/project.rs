use std::process::Command;
use std::fs::File;
use std::io;

use super::Result;
use super::Error;
use super::{find_spec_file, list_all_dirs, stringtify};

const COMPILER_CMD_ENV: &str = "UPATCH_CMD";
const ASSEMBLER_DIR_ENV: &str = "UPATCH_OUTPUT";

pub struct Project {
    project_dir: String,
    build_command: String,
    rpmbuild: bool,
}

impl Project {
    pub fn new(project_dir: String,  build_command: String, rpmbuild: bool) -> Self {
        Self {
            project_dir,
            build_command,
            rpmbuild,
        }
    }

    pub fn build(&self, cmd: &str, output: &str, recursive: bool) -> Result<()> {
        let mut result;
        let mut build_cmd;
        match self.rpmbuild {
            true => {
                let spec = find_spec_file(&format!("{}/SPECS", &self.project_dir))?;
                build_cmd = Command::new("rpmbuild");
                let dir = &format!("_topdir {}", &self.project_dir);
                let arg = match recursive{
                    true => vec!["--define", &dir, "-bb", &spec, "--noprep"],
                    false => vec!["--define", &dir, "-bb", &spec],
                };

                result = build_cmd
                        .args(arg)
                        .env(COMPILER_CMD_ENV, cmd)
                        .env(ASSEMBLER_DIR_ENV, output);
            },
            false => {
                let command = self.build_command.split(" ").collect::<Vec<_>>();
                build_cmd = Command::new(command[0]);
                result = build_cmd.current_dir(&self.project_dir)
                    .env(COMPILER_CMD_ENV, cmd)
                    .env(ASSEMBLER_DIR_ENV, output);
                for i in 1..command.len() {
                    result = result.arg(command[i]);
                }
            }
        }
        let result = result.output()?;
        if !result.status.success(){
            return Err(Error::Project(format!("build project error {}: {}", result.status, String::from_utf8(result.stderr).unwrap_or_default())));
        }
        Ok(())
    }

    pub fn patch(&self, patch: String) -> Result<()> {
        let patch_dir = match self.rpmbuild {
            true => {
                let build = list_all_dirs(format!("{}/BUILD", &self.project_dir), false)?;
                if build.len() != 1 {
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("can't find build in rpmbuild")).into());
                }
                stringtify(build[0].clone())
            },
            false => self.project_dir.clone()
        };
        let mut build_cmd = Command::new("patch");
        let result = build_cmd.current_dir(&patch_dir).arg("-N").arg("-p1").stdin(File::open(&patch).unwrap()).output()?;
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