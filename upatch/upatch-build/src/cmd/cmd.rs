use std::process::{Command, ExitStatus, Stdio};
use std::ffi::{OsStr, OsString};
use std::io::BufReader;
use std::path::Path;
use std::collections::HashMap;

use log::*;

use super::LossyLines;

#[derive(Clone)]
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
    path: &'a OsStr,
}

impl ExternCommand<'_> {
    #[inline(always)]
    pub fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandExitStatus> {
        let mut last_stdout = String::new();
        let mut last_stderr = String::new();
        let mut child_process = match command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(child_process) => child_process,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound{
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("can't find command: {:?}", self.path)));
                }
                return Err(e);
            }
        };

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
    pub fn new<S: AsRef<OsStr> + ?Sized>(path: &'a S) -> Self {
        Self { path: path.as_ref() }
    }

    pub fn execvp(&self, args: ExternCommandArgs) -> std::io::Result<ExternCommandExitStatus> {
        let mut command = Command::new(self.path);
        command.args(args.into_iter());

        self.execute_command(&mut command)
    }

    pub fn execvp_stdio<P, T>(&self, args: ExternCommandArgs, current_dir: P, stdio: T) -> std::io::Result<ExternCommandExitStatus>
    where
        P: AsRef<Path>,
        T: Into<Stdio>,
    {
        let mut command = Command::new(self.path);
        command.args(args.into_iter());
        command.current_dir(current_dir);
        command.stdin(stdio);

        self.execute_command(&mut command)
    }

    pub fn execve<P: AsRef<Path>>(&self, args: ExternCommandArgs, envs: ExternCommandEnvs, current_dir: P) -> std::io::Result<ExternCommandExitStatus> {
        let mut command = Command::new(self.path);
        command.args(args.into_iter());
        command.envs(envs.into_iter());
        command.current_dir(current_dir);

        self.execute_command(&mut command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.path))
    }
}
