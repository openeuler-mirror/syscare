mod cmd;
mod dwarf;
mod elf;
mod log;
mod rpc;
mod tool;
mod upatch;

fn main() {
    std::process::exit(match upatch::UpatchBuild::run() {
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
    });
}
