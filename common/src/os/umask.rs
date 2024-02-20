use nix::sys::stat::{umask, Mode};

pub fn set_umask(mode: u32) -> u32 {
    umask(Mode::from_bits_truncate(mode)).bits()
}

#[test]
fn test() {
    use std::{fs, fs::File, os::unix::fs::PermissionsExt};

    const FILE_PATH: &str = "/tmp/umask_test";
    const UMASK1: u32 = 0o077; // 10600
    const UMASK2: u32 = 0o022; // 10644

    fs::remove_file(FILE_PATH).ok();

    println!("Testing umask {:03o}...", UMASK1);
    set_umask(UMASK1);
    let file1 = File::create(FILE_PATH).expect("Failed to create file");
    let perm1 = file1
        .metadata()
        .map(|s| s.permissions())
        .expect("Failed to read file permission");

    println!("umask: {:03o}, perm: {:05o}", UMASK1, perm1.mode());

    drop(file1);
    fs::remove_file(FILE_PATH).ok();

    println!("Testing umask {:03o}...", UMASK2);
    set_umask(UMASK2);
    let file2 = File::create(FILE_PATH).expect("Failed to create file");
    let perm2 = file2
        .metadata()
        .map(|s| s.permissions())
        .expect("Failed to read file permission");

    println!("umask: {:03o}, perm: {:05o}", UMASK2, perm2.mode());

    drop(file2);
    fs::remove_file(FILE_PATH).ok();
}
