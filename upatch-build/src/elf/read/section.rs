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

use anyhow::{ensure, Result};
use memmap2::Mmap;

use super::super::{Endian, OperateRead, ReadInteger, SectionRead};

#[derive(Debug)]
pub struct SectionHeader<'a> {
    mmap: &'a Mmap,
    endian: Endian,
    offset: usize,
}

impl<'a> SectionHeader<'a> {
    pub fn from(mmap: &'a Mmap, endian: Endian, offset: usize) -> Self {
        Self {
            mmap,
            endian,
            offset,
        }
    }
}

impl SectionRead for SectionHeader<'_> {}

impl OperateRead for SectionHeader<'_> {
    fn get<T: ReadInteger<T>>(&self, start: usize) -> T {
        self.endian.read_integer::<T>(
            &self.mmap[self.offset + start..(self.offset + start + std::mem::size_of::<T>())],
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SectionHeaderTable<'a> {
    mmap: &'a Mmap,
    endian: Endian,
    offset: usize,
    size: usize,
    num: usize,
    count: usize,
}

impl<'a> SectionHeaderTable<'a> {
    pub fn from(mmap: &'a Mmap, endian: Endian, offset: usize, size: usize, num: usize) -> Self {
        Self {
            mmap,
            endian,
            offset,
            size,
            num,
            count: 0,
        }
    }

    pub fn get(&self, index: usize) -> Result<SectionHeader<'a>> {
        ensure!(
            index < self.num,
            "The index is {}, but the len is {}",
            index,
            self.num
        );

        Ok(SectionHeader::from(
            self.mmap,
            self.endian,
            index * self.size + self.offset,
        ))
    }
}

impl<'a> Iterator for SectionHeaderTable<'a> {
    type Item = SectionHeader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.count < self.num {
            true => {
                let offset = self.count * self.size + self.offset;
                self.count += 1;
                Some(SectionHeader::from(self.mmap, self.endian, offset))
            }
            false => {
                self.count = 0;
                None
            }
        }
    }
}
