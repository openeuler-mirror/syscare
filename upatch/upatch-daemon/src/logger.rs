use std::{path::Path, thread::Thread};

use anyhow::{Context, Result};
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification,
    Logger as FlexiLogger, LoggerHandle, Naming, WriteMode,
};
use lazy_static::lazy_static;
use log::{LevelFilter, Record};
use parking_lot::{Mutex, MutexGuard};

use super::DAEMON_NAME;

pub struct Logger;

lazy_static! {
    static ref LOGGER: Mutex<Option<LoggerHandle>> = Mutex::new(None);
}

impl Logger {
    fn thread_name(thread: &Thread) -> &str {
        const MAIN_THREAD_NAME: &str = "main";
        const UNNAMED_THREAD_NAME: &str = "<unnamed>";

        match thread.name() {
            Some(MAIN_THREAD_NAME) => DAEMON_NAME,
            Some(thread_name) => thread_name,
            None => UNNAMED_THREAD_NAME,
        }
    }

    fn format_log(
        w: &mut dyn std::io::Write,
        now: &mut DeferredNow,
        record: &Record,
    ) -> Result<(), std::io::Error> {
        write!(
            w,
            "[{}] [{}] [{}] {}",
            now.format("%Y-%m-%d %H:%M:%S%.6f"),
            record.level(),
            Self::thread_name(&std::thread::current()),
            &record.args()
        )
    }
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER.lock().is_some()
    }

    pub fn initialize<P: AsRef<Path>>(
        log_dir: P,
        max_level: LevelFilter,
        duplicate_stdout: bool,
    ) -> Result<()> {
        let mut log_handle: MutexGuard<Option<LoggerHandle>> = LOGGER.lock();

        if log_handle.is_none() {
            let log_spec = LogSpecification::builder().default(max_level).build();

            let file_spec = FileSpec::default()
                .directory(log_dir.as_ref())
                .use_timestamp(false);

            let mut logger = FlexiLogger::with(log_spec)
                .log_to_file(file_spec)
                .format(Self::format_log)
                .rotate(
                    Criterion::Age(Age::Day),
                    Naming::Timestamps,
                    Cleanup::KeepCompressedFiles(30),
                )
                .write_mode(WriteMode::Direct);

            if duplicate_stdout {
                logger = logger.duplicate_to_stdout(Duplicate::All);
            }
            let _ = log_handle.insert(logger.start().context("Failed to start logger")?);
        }

        Ok(())
    }
}
