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
    io::Read,
    os::unix::{ffi::OsStringExt, io::AsRawFd},
    process::{ChildStderr, ChildStdout},
};

use log::{error, log, Level};
use nix::poll::{poll, PollFd, PollFlags};

const STREAM_BUFFER_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy)]
pub struct LogLevel {
    pub stdout: Option<Level>,
    pub stderr: Option<Level>,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self {
            stdout: None,
            stderr: Some(Level::Error),
        }
    }
}

struct Stream<R> {
    stream: R,
    buffer: Vec<u8>,
    offset: usize,
    log_level: Option<Level>,
    is_closed: bool,
}

impl<R: Read> Stream<R> {
    fn new(stream: R, log_level: Option<Level>) -> Self {
        Self {
            stream,
            buffer: Vec::with_capacity(STREAM_BUFFER_SIZE),
            offset: 0,
            log_level,
            is_closed: false,
        }
    }

    fn read_buf(&mut self) -> std::io::Result<usize> {
        if self.buffer.capacity().wrapping_sub(self.buffer.len()) < STREAM_BUFFER_SIZE {
            self.buffer.reserve(STREAM_BUFFER_SIZE);
        }

        let spare_cap = self.buffer.spare_capacity_mut();
        let spare_buf = unsafe {
            std::slice::from_raw_parts_mut(spare_cap.as_mut_ptr() as *mut u8, spare_cap.len())
        };

        let len = self.stream.read(spare_buf)?;
        unsafe {
            self.buffer.set_len(self.buffer.len() + len);
        }

        Ok(len)
    }

    fn print_logs(&mut self) {
        if let Some(level) = self.log_level {
            let start = self.offset;
            if start >= self.buffer.len() {
                return;
            }

            let slice = if !self.is_closed {
                let end = match self.buffer[start..].iter().rposition(|&b| b == b'\n') {
                    Some(pos) => start + pos,
                    None => return,
                };
                self.offset = end + 1; // skip '\n'
                &self.buffer[start..end]
            } else {
                self.offset = self.buffer.len();
                &self.buffer[start..]
            };
            if slice.is_empty() {
                return;
            }

            let lines = slice.split(|&b| b == b'\n').map(String::from_utf8_lossy);
            for line in lines {
                log!(level, "{}", line);
            }
        }
    }

    fn handle_revents(&mut self, revents: PollFlags) {
        if revents.contains(PollFlags::POLLIN) {
            match self.read_buf() {
                Ok(0) => self.is_closed = true, // EOF
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => {
                    error!("Failed to read stream, {}", e);
                    self.is_closed = true;
                }
            }
        }
        if revents.contains(PollFlags::POLLHUP) {
            self.is_closed = true;
        }

        self.print_logs();
    }
}

pub struct Outputs {
    fds: [PollFd; 2],
    stdout: Stream<ChildStdout>,
    stderr: Stream<ChildStderr>,
}

impl Outputs {
    pub fn new(stdout: ChildStdout, stderr: ChildStderr, log_level: LogLevel) -> Self {
        Self {
            fds: [
                PollFd::new(stdout.as_raw_fd(), PollFlags::POLLIN | PollFlags::POLLHUP),
                PollFd::new(stderr.as_raw_fd(), PollFlags::POLLIN | PollFlags::POLLHUP),
            ],
            stdout: Stream::new(stdout, log_level.stdout),
            stderr: Stream::new(stderr, log_level.stderr),
        }
    }

    pub fn redirect(mut self) -> (OsString, OsString) {
        const POLL_TIMEOUT: i32 = -1;

        loop {
            match poll(&mut self.fds, POLL_TIMEOUT) {
                Ok(events) => {
                    if events == 0 {
                        break;
                    }
                    for (i, fd) in self.fds.iter().enumerate() {
                        let revents = fd.revents().expect("Invalid poll event");
                        match i {
                            0 => self.stdout.handle_revents(revents),
                            1 => self.stderr.handle_revents(revents),
                            _ => unreachable!("Invalid poll fd"),
                        };
                    }
                }
                Err(e) => {
                    error!("Failed to poll events, {}", e);
                    break;
                }
            }
            if self.stdout.is_closed && self.stderr.is_closed {
                break;
            }
        }

        (
            OsString::from_vec(self.stdout.buffer),
            OsString::from_vec(self.stderr.buffer),
        )
    }
}
