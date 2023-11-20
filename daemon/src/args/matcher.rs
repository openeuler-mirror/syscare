use clap::{clap_app, crate_description, crate_name, crate_version, AppSettings, ArgMatches};

const DEFAULT_PID_FILE: &str = "/var/run/syscared.pid";
const DEFAULT_SOCKET_FILE: &str = "/var/run/syscared.sock";
const DEFAULT_DATA_DIR: &str = "/usr/lib/syscare";
const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_LOG_DIR: &str = "/var/log/syscare";
const DEFAULT_LOG_LEVEL: &str = "info";

pub struct ArgMatcher;

impl ArgMatcher {
    pub fn get_matched_args() -> ArgMatches<'static> {
        clap_app!(syscare_cli =>
            (name: crate_name!())
            (version: crate_version!())
            (about: crate_description!())
            (global_settings: &[ AppSettings::DeriveDisplayOrder, AppSettings::UnifiedHelpMessage ])
            (@arg daemon: short("d") long("daemon") "Run as a daemon")
            (@arg pid_file: long("pid-file") +takes_value value_name("PID_FILE") default_value(DEFAULT_PID_FILE) "Path for daemon pid file")
            (@arg socket_file: long("socket-file") +takes_value value_name("SOCKET_FILE") default_value(DEFAULT_SOCKET_FILE) "Path for daemon unix socket")
            (@arg data_dir: long("data-dir") +takes_value value_name("DATA_DIR") default_value(DEFAULT_DATA_DIR) "Daemon data directory")
            (@arg work_dir: long("work-dir") +takes_value value_name("WORK_DIR") default_value(DEFAULT_WORK_DIR) "Daemon working directory")
            (@arg log_dir: long("log-dir") +takes_value value_name("LOG_DIR") default_value(DEFAULT_LOG_DIR) "Daemon log directory")
            (@arg log_level: short("l") long("log-level") +takes_value value_name("LOG_LEVEL") default_value(DEFAULT_LOG_LEVEL) "Set the logging level (\"trace\"|\"debug\"|\"info\"|\"warn\"|\"error\")")
        ).get_matches()
    }
}