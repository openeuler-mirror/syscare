// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};

use std::os::unix::prelude::OsStringExt;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use log::trace;

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
pub struct ExternCommand {
    path: OsString,
}

impl ExternCommand {
    #[inline(always)]
    fn execute(&self, mut command: Command) -> Result<ExternCommandExitStatus> {
        trace!("Executing {:?}", command);

        let child_name = self.path.to_os_string();
        let child_display = child_name.as_os_str().to_string_lossy();

        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to start process \"{}\"", child_display))?;

        let child_pid = child.id();
        trace!("Process \"{}\" ({}) started", child_display, child_pid);

        let child_output = child
            .wait_with_output()
            .with_context(|| format!("Failed to wait child process {}", child_pid))?;

        let child_retval = child_output
            .status
            .code()
            .with_context(|| format!("Failed to get process {} exit code", child_pid))?;
        let child_stdout = OsString::from_vec(child_output.stdout);
        let child_stderr = OsString::from_vec(child_output.stderr);
        trace!(
            "Process \"{}\" ({}) exited, exit_code={}",
            child_display,
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

impl ExternCommand {
    pub fn new<S: AsRef<OsStr>>(path: S) -> Self {
        Self {
            path: path.as_ref().to_os_string(),
        }
    }

    pub fn execvp(&self, args: ExternCommandArgs) -> Result<ExternCommandExitStatus> {
        let mut command = Command::new(&self.path);
        command.args(args);

        self.execute(command)
    }

    pub fn execve(
        &self,
        args: ExternCommandArgs,
        vars: ExternCommandEnvs,
    ) -> Result<ExternCommandExitStatus> {
        let mut command = Command::new(&self.path);
        command.args(args);
        command.envs(vars);

        self.execute(command)
    }
}

impl std::fmt::Display for ExternCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path.to_string_lossy()))
    }
}

#[derive(Debug)]
pub struct ExternCommandExitStatus {
    cmd_name: OsString,
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

    pub fn check_exit_code(&self) -> Result<()> {
        if self.exit_code == 0 {
            return Ok(());
        }
        bail!(
            "Process \"{}\" exited unsuccessfully, exit_code={}",
            self.cmd_name.as_os_str().to_string_lossy(),
            self.exit_code
        );
    }
}
