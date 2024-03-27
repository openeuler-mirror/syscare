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
    ffi::OsString,
    ops::Deref,
    os::unix::process::ExitStatusExt,
    process::{Child as StdChild, ExitStatus as StdExitStatus},
    thread::JoinHandle,
};

use anyhow::{ensure, Context, Result};
use log::trace;

use super::{Stdio, StdioLevel};

pub struct Child {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) stdio_level: StdioLevel,
    pub(super) inner: StdChild,
}

impl Child {
    fn capture_stdio(&mut self) -> Result<JoinHandle<(OsString, OsString)>> {
        Stdio::new(
            self.name.clone(),
            self.inner
                .stdout
                .take()
                .context("Failed to capture stdout")?,
            self.inner
                .stderr
                .take()
                .context("Failed to capture stderr")?,
            self.stdio_level,
        )
        .capture()
    }
}

impl Child {
    pub fn kill(&mut self) -> Result<()> {
        self.inner
            .kill()
            .with_context(|| format!("Failed to kill process {} ({})", self.name, self.id))
    }

    pub fn wait(&mut self) -> Result<ExitStatus> {
        let status = self
            .inner
            .wait()
            .with_context(|| format!("Failed to wait process {} ({})", self.name, self.id))?;

        let exit_status = ExitStatus {
            id: self.id,
            name: self.name.clone(),
            status,
        };
        trace!(
            "Process {} ({}) exited, exit_code={}",
            exit_status.name,
            exit_status.id,
            exit_status.exit_code()
        );

        Ok(exit_status)
    }

    pub fn wait_with_output(&mut self) -> Result<Output> {
        let stdio_thread = self.capture_stdio()?;
        let status = self.wait()?;
        let (stdout, stderr) = stdio_thread.join().expect("Failed to join stdio thread");

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

pub struct ExitStatus {
    id: u32,
    name: String,
    status: StdExitStatus,
}

impl ExitStatus {
    pub fn exit_code(&self) -> i32 {
        const SIGNAL_SHIFT: i32 = 1 << 7;

        if let Some(exit_code) = self.status.code() {
            return exit_code;
        }
        if let Some(signal) = self.status.signal() {
            return signal + SIGNAL_SHIFT;
        }
        if let Some(signal) = self.status.stopped_signal() {
            return signal + SIGNAL_SHIFT;
        }

        self.status.into_raw()
    }

    pub fn exit_ok(&self) -> Result<()> {
        let exit_code = self
            .status
            .code()
            .with_context(|| format!("Process {} ({}) was terminated", self.name, self.id))?;

        ensure!(
            exit_code == 0,
            "Process {} ({}) exited unsuccessfully, exit_code={}",
            self.name,
            self.id,
            exit_code
        );

        Ok(())
    }

    pub fn success(&self) -> bool {
        self.status.success()
    }
}

pub struct Output {
    pub status: ExitStatus,
    pub stdout: OsString,
    pub stderr: OsString,
}

impl Deref for Output {
    type Target = ExitStatus;

    fn deref(&self) -> &Self::Target {
        &self.status
    }
}
