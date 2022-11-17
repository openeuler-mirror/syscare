use super::fs;

const PROC_SELF_PATH:     &str = "/proc/self";
const PROC_SELF_EXE_PATH: &str = "/proc/self/exe";

lazy_static::lazy_static! {
    static ref PROCESS_ID:   u32    = SysInitializer::init_process_id();
    static ref PROCESS_PATH: String = SysInitializer::init_process_path();
    static ref PROCESS_NAME: String = SysInitializer::init_process_name();
    static ref CPU_NUM:      usize  = SysInitializer::init_cpu_num();
}

struct SysInitializer;

impl SysInitializer {
    pub fn init_process_id() -> u32 {
        fs::stringtify_path(
            std::fs::read_link(PROC_SELF_PATH).expect("Get process id failed")
        ).parse::<u32>().expect("Parse process id failed")
    }

    pub fn init_cpu_num() -> usize {
        const SYS_INFO_SPLITER: char = '-';
        const SYS_INFO_ONLINE_CPU_FILE_PATH: &str = "/sys/devices/system/cpu/online";

        let cpu_online_info = fs::read_file_to_string(SYS_INFO_ONLINE_CPU_FILE_PATH).expect("Read cpu number failed");
        let max_cpu_id = cpu_online_info
            .split(SYS_INFO_SPLITER)
            .last()
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or_default();

        // cpu id start from 0
        max_cpu_id + 1
    }

    pub fn init_process_path() -> String {
        fs::stringtify_path(
            std::fs::read_link(PROC_SELF_EXE_PATH).expect("Read process id failed")
        )
    }

    pub fn init_process_name() -> String {
        fs::stringtify_path(
            std::fs::read_link(PROC_SELF_EXE_PATH).expect("Read process name failed")
                .file_name().expect("Parse process name failed")
        )
    }
}

pub fn get_process_id() -> u32 {
    *PROCESS_ID
}

pub fn get_process_path() -> &'static str {
    PROCESS_PATH.as_str()
}

pub fn get_process_name() -> &'static str {
    PROCESS_NAME.as_str()
}

pub fn get_cpu_num() -> usize {
    *CPU_NUM
}
