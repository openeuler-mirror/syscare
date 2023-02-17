use std::ffi::{OsStr, OsString};
use std::collections::HashMap;

use std::process::{Command, Stdio};
use std::io::BufReader;

use log::{trace, debug};

use super::raw_line::RawLine;

pub struct ExternCommandArgs {
    args: Vec<OsString>,
}

impl ExternCommandArgs {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn arg<S>(mut self, arg: S) -> Self
    where
        S: AsRef<OsStr>
    {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>
    {
        for arg in args {
            self.args.push(arg.as_ref().to_os_string())
        }

        self
    }
}

impl IntoIterator for ExternCommandArgs {
    type Item = OsString;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}

pub struct ExternCommandEnvs {
    envs: HashMap<OsString, OsString>,
}

impl ExternCommandEnvs {
    pub fn new() -> Self {
        Self { envs: HashMap::new() }
    }

    pub fn env<K, V>(mut self, k: K, v: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs.insert(
            k.as_ref().to_os_string(),
            v.as_ref().to_os_string()
        );
        self
    }

    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in envs {
            self.envs.insert(
                k.as_ref().to_os_string(),
                v.as_ref().to_os_string()
            );
        }
        self
    }
}

impl IntoIterator for ExternCommandEnvs {
    type Item = (OsString, OsString);

    type IntoIter = std::collections::hash_map::IntoIter<OsString, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.envs.into_iter()
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
        let mut last_stdout = OsString::new();
        let mut last_stderr = OsString::new();

        debug!("executing {:?}", command);
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
        trace!("process '{}' ({}) started", process_name, process_id);

        let process_stdout = child_process.stdout.as_mut().expect("Pipe stdout failed");
        for read_line in RawLine::from(BufReader::new(process_stdout)) {
            last_stdout = read_line?;
            trace!("{}", last_stdout.to_string_lossy());
        }

        let process_stderr = child_process.stderr.as_mut().expect("Pipe stderr failed");
        for read_line in RawLine::from(BufReader::new(process_stderr)) {
            last_stderr = read_line?;
            trace!("{}", last_stderr.to_string_lossy());
        }

        let exit_code = child_process.wait()?.code().expect("Get process exit code failed");
        trace!("process '{}' ({}) exited, exit_code={}", process_name, process_id, exit_code);

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

    pub fn execvp(&self, args: ExternCommandArgs) -> std::io::Result<ExternCommandExitStatus> {
        let mut command = Command::new(self.path);
        command.args(args.into_iter());

        self.execute_command(&mut command)
    }

    pub fn execve(&self, args: ExternCommandArgs, vars: ExternCommandEnvs) -> std::io::Result<ExternCommandExitStatus>
    {
        let mut command = Command::new(self.path);
        command.args(args.into_iter());
        command.envs(vars.into_iter());

        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    exit_code: i32,
    stdout:    OsString,
    stderr:    OsString,
}

impl ExternCommandExitStatus {
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn stdout(&self) -> &OsStr {
        &self.stdout
    }

    pub fn stderr(&self) -> &OsStr {
        &self.stderr
    }
}
