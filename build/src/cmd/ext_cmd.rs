use std::process::{Command, Stdio};
use std::io::BufReader;
use std::ffi::OsStr;

use crate::log::debug;

use super::lossy_lines::LossyLines;

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
}

impl ExternCommand<'_> {
    #[inline(always)]
    fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandExitStatus> {
        let mut last_stdout = String::new();
        let mut last_stderr = String::new();

        debug!("Executing {:?}", command);
        let mut child_process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("Start process '{}' failed: {}", self.path, e.to_string())
                )
            })?;

        let process_name = self.path;
        let process_id = child_process.id();
        debug!("Process '{}' ({}) started", process_name, process_id);

        let process_stdout = child_process.stdout.as_mut().expect("Pipe stdout failed");
        for read_line in LossyLines::from(BufReader::new(process_stdout)) {
            last_stdout = read_line?;
            debug!("{}", last_stdout);
        }

        let process_stderr = child_process.stderr.as_mut().expect("Pipe stderr failed");
        for read_line in LossyLines::from(BufReader::new(process_stderr)) {
            last_stderr = read_line?;
            debug!("{}", last_stderr);
        }

        let exit_code = child_process.wait()?.code().expect("Get process exit code failed");
        debug!("Process '{}' ({}) exited, exit_code={}\n", process_name, process_id, exit_code);

        Ok(ExternCommandExitStatus {
            exit_code,
            stdout: last_stdout,
            stderr: last_stderr,
        })
    }
}

impl<'a> ExternCommand<'a> {
    pub const fn new(path: &'a str) -> Self {
        Self { path }
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

    pub fn execve<I, E, S, K, V>(&self, arg_list: I, env_list: E) -> std::io::Result<ExternCommandExitStatus>
    where
        I: IntoIterator<Item = S>,
        E: IntoIterator<Item = (K, V)>,
        S: AsRef<OsStr>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let mut command = Command::new(self.path);
        command.args(arg_list);
        command.envs(env_list);

        debug!("Executing {:?}", command);
        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}
