use std::ffi::OsStr;

use lazy_static::lazy_static;

use super::{platform, process};

pub fn arch() -> &'static OsStr {
    platform::arch()
}

pub fn num() -> usize {
    lazy_static! {
        static ref CPU_NUM: usize = unsafe {
            let mut cpu_set = std::mem::MaybeUninit::zeroed().assume_init();
            let ret = libc::sched_getaffinity(
                process::id(),
                std::mem::size_of::<libc::cpu_set_t>(),
                &mut cpu_set,
            );
            assert_eq!(ret, 0);

            libc::CPU_COUNT(&cpu_set) as usize
        };
    }
    *CPU_NUM
}

#[derive(Debug)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

pub fn load() -> (f64, f64, f64) {
    let mut loadavg = [0f64; 3];
    let ret = unsafe { libc::getloadavg(loadavg.as_mut_ptr(), loadavg.len() as i32) };
    assert_eq!(ret, 3);

    (loadavg[0], loadavg[1], loadavg[2])
}
