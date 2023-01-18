use std::process::{Command, Stdio};
use std::io::BufReader;
use std::ffi::OsStr;

use crate::log::{info, error};
use crate::util::lossy_lines::LossyLines;

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

impl ExternCommandExitStatus {
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct ExternCommand<'a> {
    path: &'a str,
    print_screen: bool,
}

impl ExternCommand<'_> {
    #[inline(always)]
    fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandExitStatus> {
        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("start process \"{}\" failed, {}", self.path, e.to_string())
                )
            })?;

        if let Some(stdout) = &mut child.stdout {
            for read_line in LossyLines::from(BufReader::new(stdout)) {
                let current_line = read_line?;
                if self.print_screen {
                    info!("{}", current_line);
                }
                stdout_buf.push_str(&current_line);
                stdout_buf.push('\n');
            }
            stdout_buf.pop();
        }

        if let Some(stderr) = &mut child.stderr {
            for read_line in LossyLines::from(BufReader::new(stderr)) {
                let current_line = read_line?;
                if self.print_screen {
                    error!("{}", current_line);
                }
                stderr_buf.push_str(&current_line);
                stderr_buf.push('\n');
            }
            stderr_buf.pop();
        }

        let exit_code = child.wait()?.code().expect("Get process exit code failed");
        Ok(ExternCommandExitStatus {
            exit_code,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    }
}

impl<'a> ExternCommand<'a> {
    pub const fn new(path: &'a str) -> Self {
        Self { path, print_screen: false }
    }

    pub fn set_print_screen(&mut self, value: bool) -> &Self {
        self.print_screen = value;
        self
    }

    pub fn execvp<I, S> (&self, arg_list: I) -> std::io::Result<ExternCommandExitStatus>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(self.path);
        command.args(arg_list);

        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}
