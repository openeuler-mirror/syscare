use std::ffi::CStr;

pub fn get_uid() -> u32 {
    unsafe { libc::getuid() as u32 }
}

pub fn get_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn get_kernel_version() -> std::io::Result<String> {
    let version_str = unsafe {
        let mut buf = std::mem::zeroed();
        if libc::uname(&mut buf) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        CStr::from_ptr(buf.release.as_ptr()).to_string_lossy().to_string()
    };
    Ok(version_str)
}
