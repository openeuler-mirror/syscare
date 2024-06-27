use std::{
    os::unix::io::{AsRawFd, RawFd},
    time::Duration,
};

use anyhow::Result;
use nix::sys::{
    select::{select, FdSet},
    time::TimeVal,
};

pub enum SelectResult {
    Readable(RawFd),
    Writable(RawFd),
    Error(RawFd),
}

pub struct Select {
    fd_set: FdSet,
    readfds: FdSet,
    writefds: FdSet,
    errorfds: FdSet,
    timeout: Option<TimeVal>,
}

impl Select {
    pub fn new<I, F>(fds: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: AsRawFd,
    {
        Self::with_timeout(fds, None)
    }

    pub fn with_timeout<I, F>(fds: I, duration: Option<Duration>) -> Self
    where
        I: IntoIterator<Item = F>,
        F: AsRawFd,
    {
        let mut fd_set = FdSet::new();
        for fd in fds {
            fd_set.insert(fd.as_raw_fd());
        }
        let readfds = FdSet::new();
        let writefds = FdSet::new();
        let errorfds = FdSet::new();
        let timeout = duration.map(|t| TimeVal::new(t.as_secs() as i64, t.subsec_micros() as i64));

        Self {
            fd_set,
            readfds,
            writefds,
            errorfds,
            timeout,
        }
    }

    pub fn select(&mut self) -> Result<impl IntoIterator<Item = SelectResult> + '_> {
        self.readfds = self.fd_set;
        self.writefds = self.fd_set;
        self.errorfds = self.fd_set;

        select(
            None,
            &mut self.readfds,
            &mut self.writefds,
            &mut self.errorfds,
            &mut self.timeout,
        )?;

        let rd_fds = self.readfds.fds(None).map(SelectResult::Readable);
        let wr_fds = self.writefds.fds(None).map(SelectResult::Writable);
        let err_fds = self.errorfds.fds(None).map(SelectResult::Error);

        Ok(rd_fds.chain(wr_fds).chain(err_fds))
    }
}
