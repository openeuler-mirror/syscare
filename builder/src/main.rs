use std::process::exit;

use syscare_build::cli::PatchBuildCLI;

fn main() {
    let exit_code = PatchBuildCLI::run();

    exit(exit_code);
}
