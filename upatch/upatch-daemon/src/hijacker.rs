use std::{
    collections::HashMap,
    ffi::CString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Error, Result};

use super::ffi;

pub struct Hijacker {
    elf_map: HashMap<PathBuf, PathBuf>,
}

impl Hijacker {
    pub fn new(elf_map: HashMap<PathBuf, PathBuf>) -> Result<Self> {
        let ret_code = unsafe { ffi::upatch_hijacker_init() };
        ensure!(ret_code == 0, Self::create_error(ret_code));

        Ok(Self { elf_map })
    }

    pub fn hijack<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let target_elf = elf_path.as_ref();
        let elf_hijacker = self
            .elf_map
            .get(target_elf)
            .context("Target elf path is invalid")?
            .as_path();

        let prey_path = Self::create_cstring(target_elf)?;
        let hijacker_path = Self::create_cstring(elf_hijacker)?;
        let ret_code =
            unsafe { ffi::upatch_hijacker_register(prey_path.as_ptr(), hijacker_path.as_ptr()) };
        ensure!(ret_code == 0, Self::create_error(ret_code));

        Ok(())
    }

    pub fn release<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let target_elf = elf_path.as_ref();
        let elf_hijacker = self
            .elf_map
            .get(target_elf)
            .context("Target elf path is invalid")?
            .as_path();

        let prey_path = Self::create_cstring(target_elf)?;
        let hijacker_path = Self::create_cstring(elf_hijacker)?;
        let ret_code =
            unsafe { ffi::upatch_hijacker_unregister(prey_path.as_ptr(), hijacker_path.as_ptr()) };
        ensure!(ret_code == 0, Self::create_error(ret_code));

        Ok(())
    }
}

impl Hijacker {
    fn create_error(ret_code: i32) -> Error {
        anyhow!("Operation failure ({})", ret_code)
    }

    fn create_cstring<P: AsRef<Path>>(path: P) -> Result<CString> {
        CString::new(path.as_ref().as_os_str().as_bytes()).context("FFI failure")
    }
}

#[test]
fn test() {
    let hijacker = Hijacker::new(HashMap::from([(
        PathBuf::from("/usr/bin/gcc"),
        PathBuf::from("/usr/bin/gcc"),
    )]))
    .expect("Failed to create hijacker");

    hijacker
        .hijack("/usr/bin/gcc")
        .expect("Failed to hijack gcc");

    hijacker
        .release("/usr/bin/gcc")
        .expect("Failed to release gcc");
}
