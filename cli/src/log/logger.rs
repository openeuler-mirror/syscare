use std::sync::Once;

use log::LevelFilter;

use log4rs::Config;
use log4rs::config::{Root, Appender};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::append::console::{ConsoleAppender, Target};

use super::LogLevelFilter;

pub struct Logger;

impl Logger {
    fn init_console_log(max_level: LevelFilter) -> Vec<Appender> {
        const STD_LOG_PATTERN: &str = "{m}{n}";
        const ERR_LOG_PATTERN: &str = "{l}: {m}{n}";
        const STDOUT_APPENDER: &str = "stdout";
        const STDERR_APPENDER: &str = "stderr";

        vec![
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Info, max_level)))
                .build(
                    STDOUT_APPENDER,
                    Box::new(ConsoleAppender::builder()
                        .target(Target::Stdout)
                        .encoder(Box::new(PatternEncoder::new(STD_LOG_PATTERN)))
                        .build())
                ),
            Appender::builder()
                .filter(Box::new(LogLevelFilter::new(LevelFilter::Error, LevelFilter::Warn)))
                .build(
                    STDERR_APPENDER,
                    Box::new(ConsoleAppender::builder()
                        .target(Target::Stderr)
                        .encoder(Box::new(PatternEncoder::new(ERR_LOG_PATTERN)))
                        .build())
                )
        ]
    }

    #[inline]
    fn do_init(max_level: LevelFilter) {
        let mut appenders = Vec::new();

        appenders.extend(Self::init_console_log(max_level));

        let root = Root::builder()
            .appenders(appenders.iter().map(Appender::name).collect::<Vec<_>>())
            .build(max_level);

        let log_config = Config::builder()
            .appenders(appenders)
            .build(root)
            .unwrap();

        log4rs::init_config(log_config).unwrap();
    }

    pub fn initialize(max_level: LevelFilter) {
        static LOGGER_INITIALIZE: Once = Once::new();

        LOGGER_INITIALIZE.call_once(|| {
            Self::do_init(max_level);
        });
    }
}
