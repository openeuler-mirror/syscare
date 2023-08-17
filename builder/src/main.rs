use std::process::exit;

pub mod cli;
pub mod package;
pub mod patch;
pub mod workdir;

use cli::PatchBuildCLI;

fn main() {
    exit(PatchBuildCLI::run());
}
