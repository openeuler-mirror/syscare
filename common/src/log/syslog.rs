use std::ffi::CString;
use std::os::unix::prelude::OsStrExt;

use lazy_static::lazy_static;
use log4rs::append::Append;
use log4rs::encode::Encode;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::encode::writer::simple::SimpleWriter;

use crate::os;

struct Syslog;

impl Syslog {
    #[inline]
    fn init() {
        lazy_static! {
            static ref SYSLOG_IDENT: CString = CString::new(os::process::name().as_bytes()).unwrap();
        }
        unsafe {
            libc::openlog(
                SYSLOG_IDENT.as_ptr(),
                libc::LOG_PID | libc::LOG_NDELAY,
                libc::LOG_USER
            )
        }
    }

    pub fn write(log_level: log::Level, mut buff: Vec<u8>) {
        static SYSLOG_INIT: std::sync::Once = std::sync::Once::new();
        SYSLOG_INIT.call_once(|| Self::init());

        // Ensure buffer does not contain unexpected terminator
        for idx in 0..buff.len() {
            if buff[idx] == b'\0' {
                buff[idx] = b' ';
            }
        }
        // Write log
        unsafe {
            libc::syslog(
                match log_level {
                    log::Level::Error => libc::LOG_ERR,
                    log::Level::Warn  => libc::LOG_WARNING,
                    log::Level::Info  => libc::LOG_NOTICE,
                    log::Level::Debug => libc::LOG_INFO,
                    log::Level::Trace => libc::LOG_DEBUG,
                },
                CString::from_vec_unchecked(buff).as_ptr()
            )
        }
    }


}

pub struct SyslogAppenderBuilder {
    encoder: Option<Box<dyn Encode>>,
}

impl SyslogAppenderBuilder {
    pub fn encoder(mut self, encoder: Box<dyn Encode>) -> Self {
        self.encoder = Some(encoder);
        self
    }

    pub fn build(self) -> SyslogAppender {
        SyslogAppender {
            encoder: self.encoder.unwrap_or_else(|| Box::new(PatternEncoder::default())),
        }
    }
}

impl Default for SyslogAppenderBuilder {
    fn default() -> Self {
        Self { encoder: None }
    }
}

#[derive(Debug)]
pub struct SyslogAppender {
    encoder: Box<dyn Encode>,
}

impl SyslogAppender {
    pub fn new(encoder: Box<dyn Encode>) -> Self {
        SyslogAppender { encoder }
    }

    pub fn builder() -> SyslogAppenderBuilder {
        SyslogAppenderBuilder::default()
    }
}

impl Append for SyslogAppender {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        const BUFF_SIZE: usize = 128; // Usually 128 is enough for formatted log

        let mut buff   = Vec::with_capacity(BUFF_SIZE);
        let mut writer = SimpleWriter(&mut buff);
        self.encoder.encode(&mut writer, record)?;

        Syslog::write(record.level(), buff);
        Ok(())
    }

    fn flush(&self) { }
}
