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

use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;

use memmap2::Mmap;

use super::super::{Endian, OperateRead, ReadInteger, SymbolRead};

#[derive(Debug)]
pub struct SymbolHeader<'a> {
    mmap: &'a Mmap,
    endian: Endian,
    strtab: usize,
    offset: usize,
}

impl<'a> SymbolHeader<'a> {
    pub fn from(mmap: &'a Mmap, endian: Endian, strtab: usize, offset: usize) -> Self {
        Self {
            mmap,
            endian,
            strtab,
            offset,
        }
    }

    pub fn get_st_name(&self) -> &'a OsStr {
        let name_offset = self.get_st_name_offset() as usize;
        self.read_to_os_string(name_offset)
    }
}

impl SymbolRead for SymbolHeader<'_> {}

impl<'a> SymbolHeader<'a> {
    fn read_to_os_string(&self, offset: usize) -> &'a OsStr {
        let mut end = offset;
        loop {
            let data = &self.mmap[self.strtab + end];
            if data.eq(&0) {
                break;
            }
            end += 1;
        }
        OsStr::from_bytes(&self.mmap[self.strtab + offset..self.strtab + end])
    }
}

impl OperateRead for SymbolHeader<'_> {
    fn get<T: ReadInteger<T>>(&self, start: usize) -> T {
        self.endian.read_integer::<T>(
            &self.mmap[self.offset + start..(self.offset + start + std::mem::size_of::<T>())],
        )
    }
}

#[derive(Debug)]
pub struct SymbolHeaderTable<'a> {
    mmap: &'a Mmap,
    endian: Endian,
    strtab: usize,
    offset: usize,
    size: usize,
    num: usize,
    count: usize,
}

impl<'a> SymbolHeaderTable<'a> {
    pub fn from(
        mmap: &'a Mmap,
        endian: Endian,
        strtab: usize,
        offset: usize,
        size: usize,
        num: usize,
    ) -> Self {
        Self {
            mmap,
            endian,
            strtab,
            offset,
            size,
            num,
            count: 0,
        }
    }

    pub fn reset(&mut self, n: usize) {
        if n < self.num {
            self.count = n;
        } else {
            self.count = 0;
        }
    }
}

impl<'a> Iterator for SymbolHeaderTable<'a> {
    type Item = SymbolHeader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.num {
            let offset = self.count * self.size + self.offset;
            self.count += 1;
            Some(SymbolHeader::from(
                self.mmap,
                self.endian,
                self.strtab,
                offset,
            ))
        } else {
            self.count = 0;
            None
        }
    }
}
