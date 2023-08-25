use std::{path::Path, thread::Thread};

use anyhow::{Context, Result};
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification,
    Logger as FlexiLogger, LoggerHandle, Naming, WriteMode,
};

use log::{LevelFilter, Record};
use once_cell::sync::OnceCell;

use super::DAEMON_NAME;

pub struct Logger;

static LOGGER: OnceCell<LoggerHandle> = OnceCell::new();

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
        LOGGER.get().is_some()
    }

    pub fn initialize<P: AsRef<Path>>(
        log_dir: P,
        max_level: LevelFilter,
        duplicate_stdout: bool,
    ) -> Result<()> {
        LOGGER.get_or_try_init(|| -> Result<LoggerHandle> {
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

            logger.start().context("Failed to start logger")
        })?;

        Ok(())
    }
}
