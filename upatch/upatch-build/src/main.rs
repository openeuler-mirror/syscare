use upatch_build::upatch::UpatchBuild;

fn main() {
    let mut upatch = UpatchBuild::new();
    std::process::exit(match upatch.run() {
        Ok(_) => {
            println!("SUCCESS!");
            0
        }
        Err(e) => {
            upatch.unhack_compiler();
            eprintln!("ERROR: {}", e);
            e.code()
        },
    });
}
