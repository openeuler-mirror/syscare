use std::path::Path;

use anyhow::{anyhow, bail, Result};
use log::error;

mod cstring;
mod ebpf;
mod ffi;
mod kmod;

use cstring::*;
use ebpf::EbpfProgramGuard;
use kmod::*;

enum HijackDependency {
    KernelModule(KernelModuleGuard),
    EbpfProgram(EbpfProgramGuard),
}

impl HijackDependency {
    fn new() -> Result<Self> {
        match KernelModuleGuard::new() {
            Ok(kmod) => {
                return Ok(HijackDependency::KernelModule(kmod));
            }
            Err(e) => {
                error!("{:?}", e);
            }
        };
        match EbpfProgramGuard::new() {
            Ok(ebpf) => {
                return Ok(HijackDependency::EbpfProgram(ebpf));
            }
            Err(e) => {
                error!("{:?}", e);
            }
        };
        bail!("Both of hijacker kmod and ebpf were initialize failed");
    }
}

pub struct HijackLibrary {
    _dependency: HijackDependency,
}

impl HijackLibrary {
    fn call_ffi(ret_code: i32) -> Result<()> {
        match ret_code == 0 {
            true => Ok(()),
            false => Err(anyhow!("Operation failed ({})", ret_code)),
        }
    }

    fn hijacker_init() -> Result<()> {
        Self::call_ffi(unsafe { ffi::upatch_hijacker_init() })
    }

    fn hijacker_destroy() {
        unsafe {
            ffi::upatch_hijacker_cleanup();
        }
    }
}

impl HijackLibrary {
    pub fn new() -> Result<Self> {
        let _dependency = HijackDependency::new()?;
        Self::hijacker_init()?;

        Ok(Self { _dependency })
    }

    pub fn hijacker_register<P, Q>(&self, target: P, hijacker: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let target_path = target.as_ref().to_cstring()?;
        let hijacker_path = hijacker.as_ref().to_cstring()?;

        Self::call_ffi(unsafe {
            ffi::upatch_hijacker_register(target_path.as_ptr(), hijacker_path.as_ptr())
        })
    }

    pub fn hijacker_unregister<P, Q>(&self, target: P, hijacker: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let target_path = target.as_ref().to_cstring()?;
        let hijacker_path = hijacker.as_ref().to_cstring()?;

        Self::call_ffi(unsafe {
            ffi::upatch_hijacker_unregister(target_path.as_ptr(), hijacker_path.as_ptr())
        })
    }
}

impl Drop for HijackLibrary {
    fn drop(&mut self) {
        Self::hijacker_destroy();
    }
}
