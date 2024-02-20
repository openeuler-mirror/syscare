use std::ffi::OsStr;

use lazy_static::lazy_static;

use nix::{
    sched::{sched_getaffinity, CpuSet},
    unistd::getpid,
};

use super::platform;

pub fn arch() -> &'static OsStr {
    platform::arch()
}

pub fn num() -> usize {
    lazy_static! {
        static ref CPU_NUM: usize = {
            let cpu_set = sched_getaffinity(getpid()).expect("Failed to get thread CPU affinity");
            let mut cpu_count = 0;
            for i in 0..CpuSet::count() {
                if cpu_set.is_set(i).expect("Failed to check cpu set") {
                    cpu_count += 1;
                }
            }
            cpu_count
        };
    }
    *CPU_NUM
}

#[test]
fn test() {
    println!("arch: {}", arch().to_string_lossy());
    println!("num: {}", num())
}
