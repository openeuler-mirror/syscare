// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{fs::File, io::Write, os::unix::io::AsRawFd, path::Path};

use anyhow::{anyhow, Result};
use nix::{ioctl_none, ioctl_write_ptr, libc::PATH_MAX};
use syscare_common::{ffi::OsStrExt, fs};

const KMOD_IOCTL_MAGIC: u16 = 0xE5;

ioctl_write_ptr!(
    ioctl_enable_hijacker,
    KMOD_IOCTL_MAGIC,
    0x1,
    UpatchEnableRequest
);
ioctl_none!(ioctl_disable_hijacker, KMOD_IOCTL_MAGIC, 0x2);
ioctl_write_ptr!(
    ioctl_register_hijacker,
    KMOD_IOCTL_MAGIC,
    0x3,
    UpatchRegisterRequest
);
ioctl_write_ptr!(
    ioctl_unregister_hijacker,
    KMOD_IOCTL_MAGIC,
    0x4,
    UpatchRegisterRequest
);

#[repr(C)]
pub struct UpatchEnableRequest {
    path: [u8; PATH_MAX as usize],
    offset: u64,
}

pub struct UpatchRegisterRequest {
    exec_path: [u8; PATH_MAX as usize],
    jump_path: [u8; PATH_MAX as usize],
}

pub struct HijackerIoctl {
    dev: File,
}

impl HijackerIoctl {
    pub fn new<P: AsRef<Path>>(dev_path: P) -> Result<Self> {
        Ok(Self {
            dev: fs::open_file(dev_path)?,
        })
    }

    pub fn enable_hijacker<P: AsRef<Path>>(&self, lib_path: P, offset: u64) -> Result<()> {
        let mut msg = UpatchEnableRequest {
            path: [0; PATH_MAX as usize],
            offset: 0,
        };

        msg.path
            .as_mut()
            .write_all(lib_path.as_ref().to_cstring()?.to_bytes_with_nul())?;
        msg.offset = offset;

        unsafe {
            ioctl_enable_hijacker(self.dev.as_raw_fd(), &msg)
                .map_err(|e| anyhow!("Ioctl error, ret={}", e))?
        };

        Ok(())
    }

    pub fn disable_hijacker(&self) -> Result<()> {
        unsafe {
            ioctl_disable_hijacker(self.dev.as_raw_fd())
                .map_err(|e| anyhow!("Ioctl error, ret={}", e))?
        };

        Ok(())
    }

    pub fn register_hijacker<P, Q>(&self, exec_path: P, jump_path: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut msg = UpatchRegisterRequest {
            exec_path: [0; PATH_MAX as usize],
            jump_path: [0; PATH_MAX as usize],
        };

        msg.exec_path
            .as_mut()
            .write_all(exec_path.as_ref().to_cstring()?.to_bytes_with_nul())?;
        msg.jump_path
            .as_mut()
            .write_all(jump_path.as_ref().to_cstring()?.to_bytes_with_nul())?;

        unsafe {
            ioctl_register_hijacker(self.dev.as_raw_fd(), &msg)
                .map_err(|e| anyhow!("Ioctl error, {}", e.desc()))?
        };

        Ok(())
    }

    pub fn unregister_hijacker<P, Q>(&self, exec_path: P, jump_path: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut msg = UpatchRegisterRequest {
            exec_path: [0; PATH_MAX as usize],
            jump_path: [0; PATH_MAX as usize],
        };

        msg.exec_path
            .as_mut()
            .write_all(exec_path.as_ref().to_cstring()?.to_bytes_with_nul())?;
        msg.jump_path
            .as_mut()
            .write_all(jump_path.as_ref().to_cstring()?.to_bytes_with_nul())?;

        unsafe {
            ioctl_unregister_hijacker(self.dev.as_raw_fd(), &msg)
                .map_err(|e| anyhow!("Ioctl error, {}", e.desc()))?
        };

        Ok(())
    }
}
