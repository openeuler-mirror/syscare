use std::process::exit;
use upatch_build::upatch::UpatchBuild;
use upatch_build::arg::Arg;

fn main() {
    let mut args = Arg::new();
    args.read();

    let mut upatch = UpatchBuild::new(args);
    // TODO build UpatchError
    match upatch.run(){
        Ok(()) => {
            println!("SUCCESS!");
            exit(0);
        },
        Err(e) => {
            eprintln!("ERROR: {}", e);
            exit(-1);
        },
    }
}
