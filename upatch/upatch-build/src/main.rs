use log::error;
use upatch_build::upatch::UpatchBuild;
use upatch_build::log::Logger;

fn main() {
    let mut upatch = UpatchBuild::new();
    std::process::exit(match upatch.run() {
        Ok(_) => {
            println!("SUCCESS!");
            0
        }
        Err(e) => {
            if let Err(e) = upatch.unhack_compiler() {
                eprintln!("unhack failed after upatch build error: {}", e);
            }
            match Logger::is_inited() {
                true => error!("{}", e),
                false => eprintln!("ERROR: {}", e),
            };
            e.code()
        },
    });
}
