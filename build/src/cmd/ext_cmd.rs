use std::ffi::OsStr;
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead};

use crate::util::fs;

#[derive(Debug)]
pub struct ExternCommandOutput {
    stdout: String,
    stderr: String,
}

impl ExternCommandOutput {
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
    redirect: bool,
}

impl<'a> ExternCommand<'a> {
    #[inline(always)]
    fn execute_command(&self, command: &mut Command) -> std::io::Result<ExternCommandOutput> {
        let mut child_process = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        let stdout_handle_fn = |line: String| {
            stdout.push_str(&line);
            if self.redirect {
                println!("{}", line)
            }
        };

        let stderr_handle_fn = |line: String| {
            stderr.push_str(&line);
            if self.redirect {
                eprintln!("{}", line)
            }
        };

        BufReader::new(child_process.stdout.as_mut().expect("Pipe stdout failed"))
            .lines()
            .filter_map(Result::ok)
            .for_each(stdout_handle_fn);

        BufReader::new(child_process.stderr.as_mut().expect("Pipe stderr failed"))
            .lines()
            .filter_map(Result::ok)
            .for_each(stderr_handle_fn);

        let exit_code = child_process.wait()?.code().expect("Get process exit code failed");
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("process '{}' exited unsuccessfully: exit_code={}, message='{}'", self.path, exit_code, stderr)
            ));
        }
        Ok(ExternCommandOutput { stdout, stderr })
    }
}

impl<'a> ExternCommand<'a> {
    pub const fn new(path: &'a str) -> Self {
        Self { path, redirect: false }
    }

    pub fn redirect_output(&mut self) -> &mut Self {
        self.redirect = true;
        self
    }

    pub fn execvp<I, S> (&self, arg_list: I) -> std::io::Result<ExternCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        fs::check_file(self.path)?;

        let mut command = Command::new(self.path);
        command.args(arg_list);

        self.execute_command(&mut command)
    }

    pub fn execve<I, E, S, K, V>(&self, arg_list: I, env_list: E) -> std::io::Result<ExternCommandOutput>
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
