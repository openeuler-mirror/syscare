use std::process::exit;

use log::error;

use syscare::cli::*;

fn main() {
    match SyscareCLI::new().run() {
        Ok(exit_code) => {
            exit(exit_code);
        },
        Err(e) => {
            error!("{}: {}", CLI_NAME, e);
            exit(-1);
        }
    }
}
