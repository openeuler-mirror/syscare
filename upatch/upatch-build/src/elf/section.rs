// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use memoffset::offset_of;

use super::OperateRead;

pub const SHT_SYMTAB: u32 = 2;

pub trait SectionRead: OperateRead {
    fn get_sh_type(&self) -> u32 {
        self.get(offset_of!(SectionHeader64, sh_type))
    }

    fn get_sh_offset(&self) -> u64 {
        self.get(offset_of!(SectionHeader64, sh_offset))
    }

    fn get_sh_link(&self) -> u32 {
        self.get(offset_of!(SectionHeader64, sh_link))
    }

    fn get_sh_size(&self) -> u64 {
        self.get(offset_of!(SectionHeader64, sh_size))
    }
}

#[repr(C)]
pub struct SectionHeader64 {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u64,
    pub sh_addr: u64,
    pub sh_offset: u64,
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}
