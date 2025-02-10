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
    collections::HashMap,
    ffi::OsString,
    io::BufReader,
    os::unix::{
        ffi::OsStringExt,
        io::{AsRawFd, RawFd},
    },
    process::{ChildStderr, ChildStdout},
    thread::JoinHandle,
};

use anyhow::{Context, Result};
use log::{error, log, Level};

use crate::io::{BufReadOsLines, OsLines, Select, SelectResult};

#[derive(Debug, Clone, Copy)]
pub struct StdioLevel {
    pub(super) stdout: Option<Level>,
    pub(super) stderr: Option<Level>,
}

impl Default for StdioLevel {
    fn default() -> Self {
        Self {
            stdout: None,
            stderr: Some(Level::Error),
        }
    }
}

pub enum StdioOutput {
    Stdout(OsString),
    Stderr(OsString),
}

enum StdioLines {
    Stdout(OsLines<BufReader<ChildStdout>>),
    Stderr(OsLines<BufReader<ChildStderr>>),
}

struct StdioReader {
    select: Select,
    stdio_map: HashMap<RawFd, StdioLines>,
    line_buf: Vec<StdioOutput>,
}

impl StdioReader {
    fn new(stdout: ChildStdout, stderr: ChildStderr) -> Self {
        let line_buf = Vec::new();
        let stdio_map = HashMap::from([
            (
                stdout.as_raw_fd(),
                StdioLines::Stdout(BufReader::new(stdout).os_lines()),
            ),
            (
                stderr.as_raw_fd(),
                StdioLines::Stderr(BufReader::new(stderr).os_lines()),
            ),
        ]);
        let select = Select::new(stdio_map.keys().copied());

        Self {
            select,
            stdio_map,
            line_buf,
        }
    }
}

impl Iterator for StdioReader {
    type Item = StdioOutput;

    fn next(&mut self) -> Option<Self::Item> {
        match self.select.select().context("Failed to select stdio") {
            Ok(result) => {
                let stdio_map = &mut self.stdio_map;
                let outputs = result.into_iter().filter_map(|income| match income {
                    SelectResult::Readable(fd) => {
                        stdio_map.get_mut(&fd).and_then(|stdio| match stdio {
                            StdioLines::Stdout(lines) => {
                                lines.next().and_then(Result::ok).map(StdioOutput::Stdout)
                            }
                            StdioLines::Stderr(lines) => {
                                lines.next().and_then(Result::ok).map(StdioOutput::Stderr)
                            }
                        })
                    }
                    _ => None,
                });
                self.line_buf.extend(outputs);
            }
            Err(e) => {
                error!("{:?}", e);
            }
        };

        self.line_buf.pop()
    }
}

pub struct Stdio {
    name: String,
    stdout: ChildStdout,
    stderr: ChildStderr,
    level: StdioLevel,
}

impl Stdio {
    pub fn new(name: String, stdout: ChildStdout, stderr: ChildStderr, level: StdioLevel) -> Self {
        Self {
            name,
            stdout,
            stderr,
            level,
        }
    }

    pub fn capture(self) -> Result<JoinHandle<(OsString, OsString)>> {
        let stdio_level = self.level;
        let stdio_reader = StdioReader::new(self.stdout, self.stderr);

        let thread_name = self.name.as_str();
        let thread = std::thread::Builder::new()
            .name(thread_name.to_string())
            .spawn(move || -> (OsString, OsString) {
                let mut stdout_buf = Vec::new();
                let mut stderr_buf = Vec::new();

                for output in stdio_reader {
                    match output {
                        StdioOutput::Stdout(str) => {
                            if let Some(level) = stdio_level.stdout {
                                log!(level, "{}", str.to_string_lossy());
                            }
                            stdout_buf.extend(str.into_vec());
                            stdout_buf.push(b'\n');
                        }
                        StdioOutput::Stderr(str) => {
                            if let Some(level) = stdio_level.stderr {
                                log!(level, "{}", str.to_string_lossy());
                            }
                            stderr_buf.extend(str.into_vec());
                            stderr_buf.push(b'\n');
                        }
                    }
                }
                if stdout_buf.ends_with(b"\n") {
                    stdout_buf.pop();
                }
                if stderr_buf.ends_with(b"\n") {
                    stderr_buf.pop();
                }

                (
                    OsString::from_vec(stdout_buf),
                    OsString::from_vec(stderr_buf),
                )
            })
            .with_context(|| format!("Failed to create thread {}", thread_name))?;

        Ok(thread)
    }
}
