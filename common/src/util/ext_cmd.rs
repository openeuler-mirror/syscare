use std::collections::HashMap;
use std::ffi::{OsStr, OsString};

use std::io::{BufReader, Read};
use std::os::unix::prelude::{OsStrExt, OsStringExt};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;

use log::trace;

use super::raw_line::RawLines;

pub struct ExternCommandArgs {
    args: Vec<OsString>,
}

impl ExternCommandArgs {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn arg<S>(mut self, arg: S) -> Self
    where
        S: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
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

impl Default for ExternCommandEnvs {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExternCommandEnvs {
    envs: HashMap<OsString, OsString>,
}

impl ExternCommandEnvs {
    pub fn new() -> Self {
        Self {
            envs: HashMap::new(),
        }
    }

    pub fn env<K, V>(mut self, k: K, v: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs
            .insert(k.as_ref().to_os_string(), v.as_ref().to_os_string());
        self
    }

    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in envs {
            self.envs
                .insert(k.as_ref().to_os_string(), v.as_ref().to_os_string());
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

impl Default for ExternCommandArgs {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExternCommand<'a> {
    path: &'a str,
}

impl ExternCommand<'_> {
    #[inline(always)]
    fn create_stdio_thread<R>(stdio: R) -> JoinHandle<std::io::Result<OsString>>
    where
        R: Read + Send + Sync + 'static,
    {
        std::thread::spawn(move || -> std::io::Result<OsString> {
            let mut output = Vec::new();

            for line in RawLines::from(BufReader::new(stdio)).flatten() {
                trace!("{}", line.to_string_lossy());

                output.extend(line.into_vec());
                output.push(b'\n');
            }
            output.pop();

            Ok(OsStr::from_bytes(&output).into())
        })
    }

    #[inline(always)]
    fn execute_command(&self, mut command: Command) -> std::io::Result<ExternCommandExitStatus> {
        trace!("Executing {:?}", command);

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "Start process \"{}\" failed, {}",
                        self.path,
                        e.to_string().to_lowercase()
                    ),
                )
            })?;

        let child_name = self.path.to_owned();
        let child_pid = child.id();
        trace!("Process \"{}\" ({}) started", child_name, child_pid);

        let stdout_thread = child
            .stdout
            .take()
            .map(Self::create_stdio_thread)
            .expect("Pipe stdout failed");
        let stderr_thread = child
            .stderr
            .take()
            .map(Self::create_stdio_thread)
            .expect("Pipe stderr failed");

        let child_retval = child.wait()?.code().expect("Get process exit code failed");
        let child_stdout = stdout_thread.join().expect("Join stdout thread failed")?;
        let child_stderr = stderr_thread.join().expect("Join stderr thread failed")?;
        trace!(
            "Process \"{}\" ({}) exited, exit_code={}",
            child_name,
            child_pid,
            child_retval
        );

        Ok(ExternCommandExitStatus {
            cmd_name: child_name,
            exit_code: child_retval,
            stdout: child_stdout,
            stderr: child_stderr,
        })
    }
}

impl<'a> ExternCommand<'a> {
    pub const fn new(path: &'a str) -> Self {
        Self { path }
    }

    pub fn execvp(&self, args: ExternCommandArgs) -> std::io::Result<ExternCommandExitStatus> {
        let mut command = Command::new(self.path);
        command.args(args);

        self.execute_command(command)
    }

    pub fn execve(
        &self,
        args: ExternCommandArgs,
        vars: ExternCommandEnvs,
    ) -> std::io::Result<ExternCommandExitStatus> {
        let mut command = Command::new(self.path);
        command.args(args);
        command.envs(vars);

        self.execute_command(command)
    }
}

impl std::fmt::Display for ExternCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path))
    }
}

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    cmd_name: String,
    exit_code: i32,
    stdout: OsString,
    stderr: OsString,
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

    pub fn check_exit_code(&self) -> std::io::Result<()> {
        if self.exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "Process \"{}\" exited unsuccessfully, exit_code={}",
                    self.cmd_name, self.exit_code
                ),
            ));
        }

        Ok(())
    }
}
