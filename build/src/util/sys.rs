use super::fs;

pub fn get_cpu_num() -> std::io::Result<usize> {
    const SYS_INFO_ONLINE_CPU_FILE_PATH: &str = "/sys/devices/system/cpu/online";
    const SYS_INFO_SPLITER: char = '-';

    fs::check_file(SYS_INFO_ONLINE_CPU_FILE_PATH)?;

    let cpu_online_info = fs::read_file_to_string(SYS_INFO_ONLINE_CPU_FILE_PATH)?;
    let max_cpu_id = cpu_online_info
        .split(SYS_INFO_SPLITER)
        .last()
        .unwrap_or_default()
        .parse::<usize>()
        .unwrap_or_default();

    Ok(max_cpu_id + 1) // cpu id start from 0
}
