use std::process::exit;

mod args;
mod cli;
mod logger;

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    exit(cli::SyscareCLI::run());
}
