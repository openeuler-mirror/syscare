use super::{Level, LevelFilter, Metadata, Record};

pub struct Logger {
    log_level: LevelFilter,
}

impl Logger {
    pub fn init_logger(log_level: LevelFilter) {
        log::set_logger(Box::leak(Box::new(Logger { log_level })))
            .map(|_| log::set_max_level(log_level))
            .expect("set logger failed");
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &Record) {
        let metadata = record.metadata();
        if !self.enabled(metadata) {
            return;
        }

        match metadata.level() {
            Level::Error | Level::Warn => {
                eprintln!("{}", record.args());
            },
            _=> {
                println!("{}", record.args());
            },
        };
    }

    fn flush(&self) { }
}
