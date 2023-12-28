use clap::{clap_app, crate_description, crate_name, crate_version, AppSettings, ArgMatches};

const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_CONFIG_DIR: &str = "/etc/syscare";
const DEFAULT_LOG_DIR: &str = "/var/log/syscare";
const DEFAULT_LOG_LEVEL: &str = "info";

pub struct ArgMatcher;

impl ArgMatcher {
    pub fn get_matched_args() -> ArgMatches<'static> {
        clap_app!(syscare_cli =>
            (name: crate_name!())
            (version: crate_version!())
            (about: crate_description!())
            (set_term_width: 120)
            (global_settings: &[
                AppSettings::ColorNever,
                AppSettings::DeriveDisplayOrder,
                AppSettings::UnifiedHelpMessage,
            ])
            (@arg daemon: short("d") long("daemon") "Run as a daemon")
            (@arg config_dir: long("config-dir") +takes_value value_name("CONFIG_DIR") default_value(DEFAULT_CONFIG_DIR) "Daemon config directory")
            (@arg work_dir: long("work-dir") +takes_value value_name("WORK_DIR") default_value(DEFAULT_WORK_DIR) "Daemon working directory")
            (@arg log_dir: long("log-dir") +takes_value value_name("LOG_DIR") default_value(DEFAULT_LOG_DIR) "Daemon logging directory")
            (@arg log_level: short("l") long("log-level") +takes_value value_name("LOG_LEVEL") default_value(DEFAULT_LOG_LEVEL) "Set the logging level (\"trace\"|\"debug\"|\"info\"|\"warn\"|\"error\")")
        ).get_matches()
    }
}
