use std::process;

mod cmd;
mod dwarf;
mod elf;
mod log;
mod rpc;
mod tool;
mod upatch;

use upatch::UpatchBuild;

fn main() {
    let exit_code = match UpatchBuild::start_and_run() {
        Ok(_) => {
            println!("SUCCESS!");
            0
        }
        Err(e) => {
            match log::Logger::is_inited() {
                true => log::error!("{}", e),
                false => eprintln!("Error: {}", e),
            };
            e.code()
        }
    };
    process::exit(exit_code);
}
