use std::io;
use std::process::Command;
use std::fs::File;

const COMPILER_CMD_ENV: &str = "UPATCH_CMD";
const ASSEMBLER_DIR_ENV: &str = "UPATCH_OUTPUT";

pub struct Project {
    project_dir: String,
    build_file: String,
}

impl Project {
    pub fn new(project_dir: String,  build_file: String) -> Self {
        Self {
            project_dir,
            build_file,
        }
    }

    pub fn build(&self, cmd: &str, output: &str) -> io::Result<()> {
        let mut build_cmd = Command::new("sh");
        let result = build_cmd.current_dir(&self.project_dir)
                .arg(&self.build_file)
                .env(COMPILER_CMD_ENV, cmd)
                .env(ASSEMBLER_DIR_ENV, output)
                .output()?;
        if !result.status.success(){
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("build project error {}: {}", result.status, String::from_utf8(result.stderr).unwrap_or_default())));
        }
        Ok(())
    }

    pub fn patch(&self, patch: String) -> io::Result<()> {
        let mut build_cmd = Command::new("patch");
        let result = build_cmd.current_dir(&self.project_dir).arg("-N").arg("-p1").stdin(File::open(&patch).unwrap()).output()?;
        match result.status.success() {
            true =>{
                println!("{}", String::from_utf8(result.stdout).unwrap().trim());
                Ok(())
            },
            false => {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("patch file {} error: {}", patch, String::from_utf8(result.stderr).unwrap().trim())))
            }
        }
    }
}