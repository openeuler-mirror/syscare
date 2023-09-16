use clap::{clap_app, crate_description, crate_name, crate_version, AppSettings, ArgMatches};

const DEFAULT_SOCKET_FILE: &str = "/var/run/syscared.sock";

pub struct ArgMatcher;

impl ArgMatcher {
    pub fn get_matched_args() -> ArgMatches<'static> {
        clap_app!(syscare_cli =>
            (name: crate_name!())
            (version: crate_version!())
            (about: crate_description!())
            (global_settings: &[
                AppSettings::DeriveDisplayOrder,
                AppSettings::UnifiedHelpMessage,
                AppSettings::VersionlessSubcommands,
                AppSettings::DisableHelpSubcommand,
            ])
            (settings: &[
                AppSettings::SubcommandRequiredElseHelp,
            ])
            (@arg socket_file: -s --socket_file value_name("SOCKET_FILE") +takes_value default_value(DEFAULT_SOCKET_FILE) "Path for daemon unix socket")
            (@arg verbose: -v --verbose "Provide more detailed info")
            (@subcommand build =>
                (about: "Build a patch")
                (settings: &[
                    AppSettings::DisableHelpFlags,
                    AppSettings::AllowLeadingHyphen,
                ])
                (@arg args: +multiple)
            )
            (@subcommand info =>
                (about: "Show patch info")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand target =>
                (about: "Show patch target")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand status =>
                (about: "Show patch status")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand list =>
                (about: "List all patches")
            )
            (@subcommand apply =>
                (about: "Apply a patch")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand remove =>
                (about: "Remove a patch")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand active =>
                (about: "Active a patch")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand deactive =>
                (about: "Deactive a patch")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand accept =>
                (about: "Accept a patch")
                (@arg identifier: value_name("IDENTIFIER") +takes_value +required "Patch identifier")
            )
            (@subcommand save =>
                (about: "Save all patch status")
            )
            (@subcommand restore =>
                (about: "Restore all patch status")
                (@arg accepted: --accepted "Accepted patch only")
            )
            (@subcommand reboot =>
                (about: "Reboot the system")
                (@arg target: -t --target value_name("TARGET") "Target kernel name")
                (@arg force: -f --force "Skip all checks, force reboot")
            )
        ).get_matches()
    }
}
