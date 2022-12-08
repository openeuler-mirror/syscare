use lazy_static::*;

use super::fs;

lazy_static! {
    static ref CPU_NUM:      usize  = SysInitializer::init_cpu_num();
    static ref PROCESS_PATH: String = SysInitializer::init_process_path();
    static ref PROCESS_NAME: String = SysInitializer::init_process_name();
}

struct SysInitializer;

impl SysInitializer {
    pub fn init_cpu_num() -> usize {
        let cpu_online_info = fs::read_file_to_string("/sys/devices/system/cpu/online")
            .expect("Read cpu number failed");

        let max_cpu_id = cpu_online_info
            .split('-')
            .last()
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or_default() + 1; // cpu id start from 0

        max_cpu_id
    }

    pub fn init_process_path() -> String {
        fs::stringtify(
            std::fs::read_link("/proc/self/exe")
                .expect("Read process path failed")
        )
    }

    pub fn init_process_name() -> String {
        fs::file_name(
            std::fs::read_link("/proc/self/exe")
                .expect("Read process name failed")
        ).expect("Parse process name failed")
    }
}

pub fn get_cpu_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn get_cpu_num() -> usize {
    *CPU_NUM
}

pub fn get_process_id() -> u32 {
    std::process::id()
}

pub fn get_process_path() -> &'static str {
    PROCESS_PATH.as_str()
}

pub fn get_process_name() -> &'static str {
    PROCESS_NAME.as_str()
}
