use std::process::{Command, ExitStatus, Stdio};
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;

use log::*;

use super::LossyLines;

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    exit_status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl ExternCommandExitStatus {
    pub fn exit_code(&self) -> String {
        match self.exit_status.code() {
            Some(code) => code.to_string(),
            None => String::from("None"),
        }
    }

    pub fn exit_status(&self) -> ExitStatus {
        self.exit_status
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
    pub fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandExitStatus> {
        let mut last_stdout = String::new();
        let mut last_stderr = String::new();
        let mut child_process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        trace!("Executing '{}' ({:?}):", &self, command);
        let process_stdout = child_process.stdout.as_mut().expect("Pipe stdout failed");
        for read_line in LossyLines::from(BufReader::new(process_stdout)) {
            let out = read_line?;
            last_stdout.push_str(&format!("{}\n", &out));
            debug!("{}", out);
        }

        let process_stderr = child_process.stderr.as_mut().expect("Pipe stderr failed");
        for read_line in LossyLines::from(BufReader::new(process_stderr)) {
            let err = read_line?;
            last_stderr.push_str(&format!("{}\n", &err));
            trace!("{}", last_stderr);
        }

        let exit_status = child_process.wait()?;
        match exit_status.code() {
            Some(code) => trace!("Process ({}) exited, exit_code={}\n", &self, code),
            None => trace!("Process ({}) exited, exit_code=None\n", &self),
        }

        Ok(ExternCommandExitStatus {
            exit_status,
            stdout: last_stdout.trim().to_string(),
            stderr: last_stderr.trim().to_string(),
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

    pub fn execvp_file<I, S> (&self, arg_list: I, current_dir: &str, file: &str) -> std::io::Result<ExternCommandExitStatus>
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
    {
        let mut command = Command::new(self.path);
        command.args(arg_list);
        command.current_dir(current_dir);
        command.stdin(File::open(file).expect(&format!("open {} error", file)));

        self.execute_command(&mut command)
    }

    pub fn execve<I, E, S, K, V>(&self, arg_list: I, env_list: E, current_dir: &str) -> std::io::Result<ExternCommandExitStatus>
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
        command.current_dir(current_dir);

        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}
