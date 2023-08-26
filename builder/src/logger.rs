use std::path::Path;

use anyhow::{Context, Result};
use flexi_logger::{
    DeferredNow, Duplicate, FileSpec, LogSpecification, Logger as FlexiLogger, LoggerHandle,
    WriteMode,
};

use log::{LevelFilter, Record};
use once_cell::sync::OnceCell;

pub struct Logger;

static LOGGER: OnceCell<LoggerHandle> = OnceCell::new();

impl Logger {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> Result<(), std::io::Error> {
        write!(w, "{}", &record.args())
    }

    fn level_to_duplicate(level: LevelFilter) -> Duplicate {
        match level {
            LevelFilter::Off => Duplicate::None,
            LevelFilter::Error => Duplicate::Error,
            LevelFilter::Warn => Duplicate::Warn,
            LevelFilter::Info => Duplicate::Info,
            LevelFilter::Debug => Duplicate::Debug,
            LevelFilter::Trace => Duplicate::Trace,
        }
    }
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER.get().is_some()
    }

    pub fn initialize<P: AsRef<Path>>(
        log_dir: P,
        max_level: LevelFilter,
        stdout_level: LevelFilter,
    ) -> Result<()> {
        LOGGER.get_or_try_init(|| -> Result<LoggerHandle> {
            let log_spec = LogSpecification::builder().default(max_level).build();

            let file_spec = FileSpec::default()
                .directory(log_dir.as_ref())
                .basename("build")
                .use_timestamp(false);

            let logger = FlexiLogger::with(log_spec)
                .log_to_file(file_spec)
                .duplicate_to_stdout(Self::level_to_duplicate(stdout_level))
                .format(Self::format_log)
                .write_mode(WriteMode::Direct);

            logger.start().context("Failed to start logger")
        })?;

        Ok(())
    }
}
