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

use std::{
    ffi::OsStr,
    path::Path,
    process::{Command as StdCommand, Stdio},
};

use anyhow::{Context, Result};
use log::{trace, Level};

use super::{Child, ExitStatus, Output, StdioLevel};

pub struct Command {
    inner: StdCommand,
    stdio_level: StdioLevel,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            inner: StdCommand::new(program),
            stdio_level: StdioLevel::default(),
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg.as_ref());
        }
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, val);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (ref key, ref val) in vars {
            self.env(key, val);
        }
        self
    }

    pub fn env_clear(&mut self) -> &mut Self {
        self.inner.env_clear();
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.inner.stdin(cfg);
        self
    }

    pub fn stdout<T: Into<Option<Level>>>(&mut self, level: T) -> &mut Self {
        self.stdio_level.stdout = level.into();
        self
    }

    pub fn stderr(&mut self, level: Level) -> &mut Self {
        self.stdio_level.stderr = level.into();
        self
    }

    pub fn spawn(&mut self) -> Result<Child> {
        let name = Path::new(self.inner.get_program())
            .file_name()
            .context("Failed to get process name")?
            .to_string_lossy()
            .to_string();

        trace!("Executing {:?}", self.inner);
        let child = self
            .inner
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to start {}", name))?;

        Ok(Child {
            id: child.id(),
            name,
            stdio_level: self.stdio_level,
            inner: child,
        })
    }

    pub fn run(&mut self) -> Result<ExitStatus> {
        self.spawn()?.wait()
    }

    pub fn run_with_output(&mut self) -> Result<Output> {
        self.spawn()?.wait_with_output()
    }
}
