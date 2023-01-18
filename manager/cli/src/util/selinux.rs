use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use super::fs;

const SELINUX_ENFORCE_FILE: &str = "/sys/fs/selinux/enforce";
const SELINUX_PERMISSIVE:   u32  = 0;
const SELINUX_ENFORCING:    u32  = 1;

const SELINUX_SECURITY_CONTEXT:          &str  = "security.selinux\0";
const SELINUX_SECURITY_CONTEXT_SPLITTER: char  = ':';
const SELINUX_SECURITY_CONTEXT_TYPE_NUM: usize = 4;

pub fn set_enforce(value: u32) -> std::io::Result<()> {
    if (value != SELINUX_PERMISSIVE) || (value != SELINUX_ENFORCING) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("set enforce failed, value \"{}\" is invalid", value)
        ));
    }
    fs::write_string_to_file(SELINUX_ENFORCE_FILE, &value.to_string()).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("set enforce failed, {}", e.to_string())
        )
    })
}

pub fn get_enforce() -> std::io::Result<u32> {
    match fs::read_file_to_string(SELINUX_ENFORCE_FILE) {
        Ok(status) => {
            Ok(status.parse().unwrap_or(SELINUX_PERMISSIVE))
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(SELINUX_PERMISSIVE)
        },
        Err(e) => {
            Err(std::io::Error::new(
                e.kind(),
                format!("get enforce failed, {}", e.to_string())
            ))
        }
    }
}

enum SecurityContextType {
    User  = 0,
    Role  = 1,
    Type  = 2,
    Range = 3,
}

pub fn read_security_context<P>(file_path: P) -> std::io::Result<String>
where P: AsRef<Path>
{
    const BUF_SIZE: usize = libc::PATH_MAX as usize;

    let mut buf = [0u8; BUF_SIZE];

    let file  = fs::realpath(file_path)?;
    let path  = file.as_os_str().as_bytes().as_ptr() as *const libc::c_char;
    let name  = SELINUX_SECURITY_CONTEXT.as_ptr() as *const libc::c_char;
    let value = buf.as_mut_ptr() as *mut libc::c_void;

    let result = unsafe {
        let len = libc::getxattr(path, name, value, BUF_SIZE);
        if len <= 0 {
            return Err(std::io::Error::last_os_error());
        }
        String::from_utf8_lossy(&buf[0..len as usize]).to_string()
    };

    Ok(result)
}

pub fn write_security_context<P>(file_path: P, scontext: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    let file  = fs::realpath(file_path)?;
    let path  = file.as_os_str().as_bytes().as_ptr() as *const libc::c_char;
    let name  = SELINUX_SECURITY_CONTEXT.as_ptr() as *const libc::c_char;
    let value = scontext.as_ptr() as *const libc::c_void;
    let size  = scontext.len();

    unsafe {
        if libc::setxattr(path, name, value, size, 0) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}

fn get_security_context<P>(file_path: P, sc_type: SecurityContextType) -> std::io::Result<String>
where P: AsRef<Path>
{
    let scontext = self::read_security_context(file_path.as_ref())?;
    let sgroup = scontext.split(SELINUX_SECURITY_CONTEXT_SPLITTER).collect::<Vec<_>>();
    assert_eq!(sgroup.len(), SELINUX_SECURITY_CONTEXT_TYPE_NUM);

    Ok(sgroup[sc_type as usize].to_owned())
}

fn set_security_context<P>(file_path: P, sc_type: SecurityContextType, value: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    let old_scontext = self::read_security_context(&file_path)?;
    let mut sgroup = old_scontext.split(SELINUX_SECURITY_CONTEXT_SPLITTER).collect::<Vec<_>>();
    assert_eq!(sgroup.len(), SELINUX_SECURITY_CONTEXT_TYPE_NUM);

    sgroup[sc_type as usize] = value;

    let mut new_scontext = String::new();
    for part in sgroup {
        new_scontext.push_str(part);
        new_scontext.push(':');
    }
    new_scontext.pop();

    if old_scontext != new_scontext {
        self::write_security_context(&file_path, &new_scontext.trim())?;
    }

    Ok(())
}

pub fn get_security_context_user<P>(file_path: P) -> std::io::Result<String>
where P: AsRef<Path>
{
    self::get_security_context(file_path, SecurityContextType::User)
}

pub fn get_security_context_role<P>(file_path: P) -> std::io::Result<String>
where P: AsRef<Path>
{
    self::get_security_context(file_path, SecurityContextType::Role)
}

pub fn get_security_context_type<P>(file_path: P) -> std::io::Result<String>
where P: AsRef<Path>
{
    self::get_security_context(file_path, SecurityContextType::Type)
}

pub fn get_security_context_range<P>(file_path: P) -> std::io::Result<String>
where P: AsRef<Path>
{
    self::get_security_context(file_path, SecurityContextType::Range)
}

pub fn set_security_context_user<P>(file_path: P, value: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    self::set_security_context(file_path, SecurityContextType::User, value)
}

pub fn set_security_context_role<P>(file_path: P, value: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    self::set_security_context(file_path, SecurityContextType::Role, value)
}

pub fn set_security_context_type<P>(file_path: P, value: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    self::set_security_context(file_path, SecurityContextType::Type, value)
}

pub fn set_security_context_range<P>(file_path: P, value: &str) -> std::io::Result<()>
where P: AsRef<Path>
{
    self::set_security_context(file_path, SecurityContextType::Range, value)
}
