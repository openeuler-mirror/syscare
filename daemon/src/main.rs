use std::process::exit;

mod daemon;
mod fast_reboot;
mod patch;
mod rpc;

pub fn main() {
    exit(daemon::run());
}
