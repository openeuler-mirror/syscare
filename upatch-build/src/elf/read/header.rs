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

use memmap2::Mmap;

use super::super::{Endian, HeaderRead, OperateRead, ReadInteger};

#[derive(Debug)]
pub struct Header<'a> {
    mmap: &'a Mmap,
    endian: Endian,
}

impl<'a> Header<'a> {
    pub fn from(mmap: &'a Mmap, endian: Endian) -> Self {
        Self { mmap, endian }
    }
}

impl HeaderRead for Header<'_> {}

impl OperateRead for Header<'_> {
    fn get<T: ReadInteger<T>>(&self, start: usize) -> T {
        self.endian
            .read_integer::<T>(&self.mmap[start..(start + std::mem::size_of::<T>())])
    }
}
