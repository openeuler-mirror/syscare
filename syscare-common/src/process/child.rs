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
    ffi::OsString, ops::Deref, os::unix::process::ExitStatusExt, process, thread::JoinHandle,
};

use anyhow::{anyhow, ensure, Context, Result};
use log::trace;

use super::output;

pub struct Child {
    pub(super) name: String,
    pub(super) child: process::Child,
    pub(super) log_level: output::LogLevel,
}

impl Child {
    fn redirect_outputs(&mut self) -> Result<JoinHandle<(OsString, OsString)>> {
        let stdout = self
            .child
            .stdout
            .take()
            .context("Failed to capture stdout")?;
        let stderr = self
            .child
            .stderr
            .take()
            .context("Failed to capture stderr")?;
        let outputs = output::Outputs::new(stdout, stderr, self.log_level);

        std::thread::Builder::new()
            .name(self.name.clone())
            .spawn(|| outputs.redirect())
            .with_context(|| format!("Failed to create thread {}", self.name))
    }
}

impl Child {
    pub fn kill(&mut self) -> Result<()> {
        let id = self.child.id();
        self.child
            .kill()
            .with_context(|| format!("Failed to kill process {} ({})", self.name, id))
    }

    pub fn wait(&mut self) -> Result<ExitStatus> {
        let id = self.child.id();
        let status = self
            .child
            .wait()
            .with_context(|| format!("Failed to wait process {} ({})", self.name, id))?;
        let exit_status = ExitStatus {
            id,
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
        let thread = self.redirect_outputs()?;
        let status = self.wait()?;
        let (stdout, stderr) = thread
            .join()
            .map_err(|_| anyhow!("Failed to join stdio thread"))?;

        let output = Output {
            status,
            stdout,
            stderr,
        };
        Ok(output)
    }
}

pub struct ExitStatus {
    id: u32,
    name: String,
    status: process::ExitStatus,
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
