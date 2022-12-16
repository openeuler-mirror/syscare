use std::process::exit;
use upatch_build::upatch::UpatchBuild;

fn main() {
    let mut upatch = UpatchBuild::new();
    match upatch.run(){
        Ok(_) => {
            println!("SUCCESS!");
            exit(0);
        },
        Err(e) => {
            upatch.unhack_compiler();
            eprintln!("ERROR {}: {}", e.code(), e);
            exit(e.code());
        },
    }
}
