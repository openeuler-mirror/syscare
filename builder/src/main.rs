use std::process::exit;

use syscare_build::cli::PatchBuildCLI;

fn main() {
    exit(PatchBuildCLI::run());
}
