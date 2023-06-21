use std::sync::atomic::{AtomicBool, Ordering};

use log::LevelFilter;

use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;

use common::log::{LogLevelFilter, SyslogAppender};

pub struct Logger;

static LOGGER_INIT_FLAG: AtomicBool = AtomicBool::new(false);

impl Logger {
    fn init_log_appenders(max_level: LevelFilter) -> Vec<Appender> {
        const STDOUT_APPENDER: &str = "stdout";
        const STDERR_APPENDER: &str = "stderr";
        const SYSLOG_APPENDER: &str = "syslog";

        const STD_LOG_PATTERN: &str = "{m}{n}";
        const ERR_LOG_PATTERN: &str = "{l}: {m}{n}";

        vec![
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Info, max_level)))
                .build(
                    STDOUT_APPENDER,
                    Box::new(
                        ConsoleAppender::builder()
                            .target(Target::Stdout)
                            .encoder(Box::new(PatternEncoder::new(STD_LOG_PATTERN)))
                            .build(),
                    ),
                ),
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(
                    LevelFilter::Error,
                    LevelFilter::Warn,
                )))
                .build(
                    STDERR_APPENDER,
                    Box::new(
                        ConsoleAppender::builder()
                            .target(Target::Stderr)
                            .encoder(Box::new(PatternEncoder::new(ERR_LOG_PATTERN)))
                            .build(),
                    ),
                ),
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(
                    LevelFilter::Error,
                    LevelFilter::Warn,
                )))
                .build(
                    SYSLOG_APPENDER,
                    Box::new(
                        SyslogAppender::builder()
                            .encoder(Box::new(PatternEncoder::new(STD_LOG_PATTERN)))
                            .build(),
                    ),
                ),
        ]
    }

    fn do_init(max_level: LevelFilter) -> std::io::Result<()> {
        let appenders = Self::init_log_appenders(max_level);

        let root = Root::builder()
            .appenders(appenders.iter().map(Appender::name).collect::<Vec<_>>())
            .build(max_level);

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

    pub fn initialize(max_level: LevelFilter) -> std::io::Result<()> {
        static INIT_ONCE: std::sync::Once = std::sync::Once::new();

        let mut result = Ok(());
        INIT_ONCE.call_once(|| {
            result = Self::do_init(max_level);
            LOGGER_INIT_FLAG.store(true, Ordering::SeqCst);
        });

        result
    }
}
