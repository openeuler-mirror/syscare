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
            match Logger::is_inited() {
                true => error!("{}", e),
                false => eprintln!("ERROR: {}", e),
            };
            e.code()
        },
    });
}
