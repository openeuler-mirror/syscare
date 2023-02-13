use log::{LevelFilter, Record};
use log4rs::filter::{Filter, Response};

#[derive(Debug)]
pub struct LogLevelFilter {
    min_level: LevelFilter,
    max_level: LevelFilter,
}

impl LogLevelFilter {
    pub fn new(min_level: LevelFilter, max_level: LevelFilter) -> LogLevelFilter {
        LogLevelFilter { min_level, max_level }
    }
}

impl Filter for LogLevelFilter {
    fn filter(&self, record: &Record) -> Response {
        let log_level = record.level();
        if log_level < self.min_level || log_level > self.max_level {
            return Response::Reject;
        }
        Response::Neutral
    }
}
