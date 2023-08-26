use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use log::LevelFilter;

use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;

use syscare_common::log::LogLevelFilter;

use crate::workdir::WorkDir;

const LOG_PATTERN: &str = "{m}{n}";

static LOGGER_INIT_FLAG: AtomicBool = AtomicBool::new(false);

pub struct Logger;

impl Logger {
    fn init_console_log(max_level: LevelFilter) -> Vec<Appender> {
        const STDOUT_APPENDER_NAME: &str = "stdout";
        const STDERR_APPENDER_NAME: &str = "stderr";

        vec![
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Info, max_level)))
                .build(
                    STDOUT_APPENDER_NAME,
                    Box::new(
                        ConsoleAppender::builder()
                            .target(Target::Stdout)
                            .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                            .build(),
                    ),
                ),
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(
                    LevelFilter::Error,
                    LevelFilter::Warn,
                )))
                .build(
                    STDERR_APPENDER_NAME,
                    Box::new(
                        ConsoleAppender::builder()
                            .target(Target::Stderr)
                            .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                            .build(),
                    ),
                ),
        ]
    }

    fn init_file_log<P: AsRef<Path>>(path: P, max_level: LevelFilter) -> std::io::Result<Appender> {
        const FILE_APPENDER_NAME: &str = "log_file";

        Ok(Appender::builder()
            .filter(Box::new(LogLevelFilter::new(LevelFilter::Error, max_level)))
            .build(
                FILE_APPENDER_NAME,
                Box::new(
                    FileAppender::builder()
                        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                        .append(false)
                        .build(path)?,
                ),
            ))
    }

    fn do_init(work_dir: &WorkDir, max_level: LevelFilter) -> std::io::Result<()> {
        let mut appenders = Vec::new();

        appenders.extend(Self::init_console_log(max_level));
        appenders.push(Self::init_file_log(&work_dir.log_file, LevelFilter::Trace)?);

        let root = Root::builder()
            .appenders(appenders.iter().map(Appender::name).collect::<Vec<_>>())
            .build(LevelFilter::Trace);

        let log_config = Config::builder()
            .appenders(appenders)
            .build(root)
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to build log config")
            })?;

        log4rs::init_config(log_config).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to init log config")
        })?;

        Ok(())
    }
}

impl Logger {
    pub fn is_inited() -> bool {
        LOGGER_INIT_FLAG.load(Ordering::Acquire)
    }

    pub fn initialize(work_dir: &WorkDir, max_level: LevelFilter) -> std::io::Result<()> {
        static INIT_ONCE: std::sync::Once = std::sync::Once::new();

        let mut result = Ok(());
        INIT_ONCE.call_once(|| {
            result = Self::do_init(work_dir, max_level);
            LOGGER_INIT_FLAG.store(true, Ordering::SeqCst);
        });

        result
    }
}
