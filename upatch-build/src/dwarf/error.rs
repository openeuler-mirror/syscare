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

use std::result;
use std::{
    fmt::{self, Debug},
    io,
};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Gimli(gimli::Error),
    Object(object::read::Error),
    Io,
}

impl From<gimli::Error> for Error {
    fn from(err: gimli::Error) -> Self {
        Error::Gimli(err)
    }
}

impl From<io::Error> for Error {
    fn from(_: io::Error) -> Self {
        Error::Io
    }
}

impl From<object::read::Error> for Error {
    fn from(err: object::read::Error) -> Self {
        Error::Object(err)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        Debug::fmt(self, f)
    }
}
