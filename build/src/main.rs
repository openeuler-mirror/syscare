use syscare_build::cli::PatchBuildCLI;

fn main() {
    if let Err(e) = PatchBuildCLI::new().run() {
        println!("Error: {}", e.to_string());
    }
}
