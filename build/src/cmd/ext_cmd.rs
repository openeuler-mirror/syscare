use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write, LineWriter};
use std::sync::{Mutex, MutexGuard};
use std::ffi::OsStr;

use lazy_static::*;

use crate::util::fs;

const EXTERN_CMD_LOG_PREFIX: &str = "syscare-build-process";

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
    fn get_log_writer<'a>(&self) -> MutexGuard<'a, impl Write> {
        lazy_static! {
            static ref LOG_FILE_PATH: String = format!("./{}-{}.log", EXTERN_CMD_LOG_PREFIX, std::process::id());
            static ref LOG_WRITER: Mutex<LineWriter<File>> = Mutex::new(LineWriter::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(LOG_FILE_PATH.as_str())
                    .expect("Cannot access log file")
            ));
        }

        LOG_WRITER.lock().expect("Lock posioned")
    }

    #[inline(always)]
    fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandExitStatus> {
        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut child_process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let process_name = self.path;
        let process_id = child_process.id();
        let mut writer = self.get_log_writer();
        writeln!(writer, "Executing '{}' ({}):", process_name, process_id)?;

        let process_stdout = child_process.stdout.as_mut().expect("Pipe stdout failed");
        for read_line in BufReader::new(process_stdout).lines() {
            let line = read_line?;

            writeln!(writer, "{}", line)?;
            stdout.push_str(&line);
        }

        let process_stderr = child_process.stderr.as_mut().expect("Pipe stderr failed");
        for read_line in BufReader::new(process_stderr).lines() {
            let line = read_line?;

            writeln!(writer, "{}", line)?;
            stderr.push_str(&line);
        }

        let exit_code = child_process.wait()?.code().expect("Get process exit code failed");
        writeln!(writer, "Process '{}' ({}) exited, exit_code={}", process_name, process_id, exit_code)?;
        writeln!(writer)?;

        let exit_status = ExternCommandExitStatus { exit_code, stdout, stderr };
        Ok(exit_status)
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
        fs::check_file(self.path)?;

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
        fs::check_file(self.path)?;

        let mut command = Command::new(self.path);
        command.args(arg_list);
        command.envs(env_list);

        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}
