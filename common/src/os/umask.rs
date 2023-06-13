use std::fs::Permissions;
use std::os::unix::prelude::PermissionsExt;

pub fn set_umask(mask: u32) -> u32 {
    unsafe { libc::umask(mask) }
}

pub fn set_from_permission(mask: Permissions) -> Permissions {
    Permissions::from_mode(set_umask(mask.mode()))
}
