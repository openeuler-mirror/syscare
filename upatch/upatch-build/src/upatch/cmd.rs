use std::process::{Command, ExitStatus, Stdio};
use std::io::Write;
use std::ffi::OsStr;
use std::fs::File;

use super::get_log_writer;

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    exit_status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl ExternCommandExitStatus {
    pub fn exit_code(&self) -> i32 {
        self.exit_status.code().expect("Get process exit code failed")
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
        let child_process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let mut binding = get_log_writer();
        let writer = binding.get_mut("log").unwrap();
        writeln!(writer, "Executing '{}' ({:?}):", &self, command)?;
        let process_stdout = String::from_utf8_lossy(&child_process.stdout).trim().to_string();
        let process_stderr = String::from_utf8_lossy(&child_process.stderr).trim().to_string();

        writeln!(writer, "{}", &process_stdout)?;
        writeln!(writer, "{}", &process_stderr)?;

        let exit_status = child_process.status;
        writeln!(writer, "Process ({}) exited, exit_code={}\n", &self, exit_status.code().expect("get code error"))?;

        Ok(ExternCommandExitStatus {
            exit_status,
            stdout: process_stdout,
            stderr: process_stderr,
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
