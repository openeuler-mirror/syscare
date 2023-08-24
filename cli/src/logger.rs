use anyhow::Result;
use flexi_logger::{DeferredNow, LogSpecification, Logger as FlexiLogger, LoggerHandle, WriteMode};
use lazy_static::lazy_static;
use log::{LevelFilter, Record};
use parking_lot::{Mutex, MutexGuard};

pub struct Logger;

lazy_static! {
    static ref LOGGER: Mutex<Option<LoggerHandle>> = Mutex::new(None);
}

impl Logger {
    fn format_log(
        w: &mut dyn std::io::Write,
        _now: &mut DeferredNow,
        record: &Record,
    ) -> Result<(), std::io::Error> {
        write!(w, "{}", &record.args())
    }
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER.lock().is_some()
    }

    pub fn initialize(max_level: LevelFilter) -> Result<()> {
        let mut logger: MutexGuard<Option<LoggerHandle>> = LOGGER.lock();

        if logger.is_none() {
            let log_spec = LogSpecification::builder().default(max_level).build();
            let log_handle = FlexiLogger::with(log_spec)
                .log_to_stdout()
                .format(Self::format_log)
                .write_mode(WriteMode::Direct)
                .start()?;

            let _ = logger.insert(log_handle);
        }

        Ok(())
    }
}
