use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use log::LevelFilter;

use log4rs::Config;
use log4rs::config::{Root, Appender};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;

use crate::cli::CliWorkDir;

use super::LogLevelFilter;

const LOG_PATTERN: &str = "{m}{n}";

pub struct Logger;

static LOGGER_INIT_FLAG: AtomicBool = AtomicBool::new(false);

impl Logger {
    fn init_console_log(max_level: LevelFilter) -> Vec<Appender> {
        const STDOUT_APPENDER_NAME: &str = "stdout";
        const STDERR_APPENDER_NAME: &str = "stderr";

        vec![
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Info, max_level)))
                .build(
                    STDOUT_APPENDER_NAME,
                    Box::new(ConsoleAppender::builder()
                        .target(Target::Stdout)
                        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                        .build())
                ),
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Error, LevelFilter::Warn)))
                .build(
                    STDERR_APPENDER_NAME,
                    Box::new(ConsoleAppender::builder()
                        .target(Target::Stderr)
                        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                        .build())
                )
        ]
    }

    fn init_file_log<P: AsRef<Path>>(path: P, max_level: LevelFilter) -> std::io::Result<Appender> {
        const FILE_APPENDER_NAME: &str = "log_file";

        Ok(Appender::builder()
            .filter(Box::new(LogLevelFilter::new(LevelFilter::Error, max_level)))
            .build(
                FILE_APPENDER_NAME,
                Box::new(FileAppender::builder()
                    .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
                    .append(false)
                    .build(path)?)
            )
        )
    }

    pub fn is_inited() -> bool {
        LOGGER_INIT_FLAG.load(Ordering::Relaxed)
    }

    pub fn initialize(work_dir: &CliWorkDir, max_level: LevelFilter) -> std::io::Result<()> {
        let mut appenders = Vec::new();

        appenders.extend(Self::init_console_log(max_level));
        appenders.push(Self::init_file_log(work_dir.log_file_path(), LevelFilter::Trace)?);

        let root = Root::builder()
            .appenders(appenders.iter().map(Appender::name).collect::<Vec<_>>())
            .build(LevelFilter::Trace);

        let log_config = Config::builder()
            .appenders(appenders)
            .build(root)
            .unwrap();

        log4rs::init_config(log_config).unwrap();
        LOGGER_INIT_FLAG.store(true, Ordering::Relaxed);

        Ok(())
    }
}
