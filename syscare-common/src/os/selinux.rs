// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::Path,
};

use nix::errno::Errno;

use crate::fs;

const SELINUX_SYS_FILE: &str = "/sys/fs/selinux/enforce";
const SELINUX_STATUS_PERMISSIVE: &str = "0";
const SELINUX_STATUS_ENFORCING: &str = "1";

const SECURITY_CONTEXT_XATTR_NAME: &str = "security.selinux";
const SECURITY_CONTEXT_SPLITER: u8 = b':';
const SECURITY_CONTEXT_SPLITER_COUNT: usize = 3;
const SECURITY_CONTEXT_ATTR_COUNT: usize = SECURITY_CONTEXT_SPLITER_COUNT + 1;

const SECURITY_CONTEXT_USER_INDEX: usize = 0;
const SECURITY_CONTEXT_ROLE_INDEX: usize = 1;
const SECURITY_CONTEXT_TYPE_INDEX: usize = 2;
const SECURITY_CONTEXT_LEVEL_INDEX: usize = 3;

pub type SELinuxStatus = Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Disabled,
    Permissive,
    Enforcing,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn get_status() -> Status {
    let value = fs::read_to_string(SELINUX_SYS_FILE).unwrap_or_default();
    match value.as_str() {
        SELINUX_STATUS_PERMISSIVE => Status::Permissive,
        SELINUX_STATUS_ENFORCING => Status::Enforcing,
        _ => Status::Disabled,
    }
}

pub fn set_status(value: Status) -> std::io::Result<()> {
    let contents = match value {
        Status::Permissive => SELINUX_STATUS_PERMISSIVE,
        Status::Enforcing => SELINUX_STATUS_ENFORCING,
        _ => return Err(std::io::Error::from(Errno::EINVAL)),
    };
    fs::write(SELINUX_SYS_FILE, contents)?;

    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecurityContext(OsString);

impl SecurityContext {
    fn get_attribute(&self, index: usize) -> &OsStr {
        self.0
            .as_bytes()
            .splitn(SECURITY_CONTEXT_ATTR_COUNT, |&b| {
                b == SECURITY_CONTEXT_SPLITER
            })
            .nth(index)
            .map(OsStr::from_bytes)
            .expect("Unexpected security context format")
    }

    fn set_attribute<S: AsRef<OsStr>>(&mut self, index: usize, value: S) -> std::io::Result<()> {
        let value = value.as_ref().as_bytes();

        if value.is_empty() {
            return Err(std::io::Error::from(Errno::EINVAL));
        }
        if (index != SECURITY_CONTEXT_LEVEL_INDEX) && (value.contains(&SECURITY_CONTEXT_SPLITER)) {
            return Err(std::io::Error::from(Errno::EINVAL));
        }
        let attrs = self.0.as_bytes().splitn(SECURITY_CONTEXT_ATTR_COUNT, |&b| {
            b == SECURITY_CONTEXT_SPLITER
        });

        let mut new_context = Vec::new();
        for (i, attr) in attrs.enumerate() {
            new_context.extend_from_slice(if i != index { attr } else { value });
            new_context.push(SECURITY_CONTEXT_SPLITER);
        }
        new_context.pop();

        self.0 = OsString::from_vec(new_context);
        Ok(())
    }
}

impl SecurityContext {
    pub fn parse<S: AsRef<OsStr>>(value: S) -> std::io::Result<Self> {
        let context = value.as_ref();
        if context.is_empty() {
            return Err(std::io::Error::from(Errno::EINVAL));
        }

        let spliter_count = context
            .as_bytes()
            .iter()
            .filter(|&b| *b == SECURITY_CONTEXT_SPLITER)
            .count();
        if spliter_count < SECURITY_CONTEXT_SPLITER_COUNT {
            return Err(std::io::Error::from(Errno::EINVAL));
        }

        Ok(Self(context.to_os_string()))
    }

    pub fn get_user(&self) -> &OsStr {
        self.get_attribute(SECURITY_CONTEXT_USER_INDEX)
    }

    pub fn get_role(&self) -> &OsStr {
        self.get_attribute(SECURITY_CONTEXT_ROLE_INDEX)
    }

    pub fn get_type(&self) -> &OsStr {
        self.get_attribute(SECURITY_CONTEXT_TYPE_INDEX)
    }

    pub fn get_level(&self) -> &OsStr {
        self.get_attribute(SECURITY_CONTEXT_LEVEL_INDEX)
    }

    pub fn set_user<S: AsRef<OsStr>>(&mut self, value: S) -> std::io::Result<()> {
        self.set_attribute(SECURITY_CONTEXT_USER_INDEX, value)
    }

    pub fn set_role<S: AsRef<OsStr>>(&mut self, value: S) -> std::io::Result<()> {
        self.set_attribute(SECURITY_CONTEXT_ROLE_INDEX, value)
    }

    pub fn set_type<S: AsRef<OsStr>>(&mut self, value: S) -> std::io::Result<()> {
        self.set_attribute(SECURITY_CONTEXT_TYPE_INDEX, value)
    }

    pub fn set_level<S: AsRef<OsStr>>(&mut self, value: S) -> std::io::Result<()> {
        self.set_attribute(SECURITY_CONTEXT_LEVEL_INDEX, value)
    }

    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }
}

impl Debug for SecurityContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl AsRef<OsStr> for SecurityContext {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

pub fn get_security_context<P>(file_path: P) -> std::io::Result<SecurityContext>
where
    P: AsRef<Path>,
{
    SecurityContext::parse(fs::getxattr(file_path, SECURITY_CONTEXT_XATTR_NAME)?)
}

pub fn set_security_context<P>(file_path: P, value: &SecurityContext) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    fs::setxattr(file_path, SECURITY_CONTEXT_XATTR_NAME, value)
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn test_selinux_status() {
        let status = self::get_status();
        println!("SELinux status: {}", status);

        let sys_file = Path::new(self::SELINUX_SYS_FILE);
        if sys_file.exists() {
            assert!(self::get_status() == Status::Disabled);
        } else {
            assert!(self::get_status() == Status::Disabled);
        }
        assert!(self::set_status(Status::Disabled).is_err());
    }

    #[test]
    fn test_security_context_parse() {
        const TEST_CASES: &[&str] = &[
            "system_u:object_r:bin_t:s0",
            "user_u:role_r:type_t:s0:c1,c2",
            "user.dom:role-1:type_x:s0:c1.c2",
            "a:b:c:d.e-f,g",
            "a_:b_:c_:d_",
        ];
        for str in TEST_CASES {
            let result = SecurityContext::parse(str).is_ok();
            assert!(result, "Failed to parse security context '{}'", str);
        }
    }

    #[test]
    fn test_security_context_get() {
        const TEST_CASES: &[(&str, [&str; 4])] = &[
            (
                "system_u:object_r:bin_t:s0",
                ["system_u", "object_r", "bin_t", "s0"],
            ),
            (
                "user_u:role_r:type_t:s0:c1,c2",
                ["user_u", "role_r", "type_t", "s0:c1,c2"],
            ),
            (
                "user.dom:role-1:type_x:s0:c1.c2",
                ["user.dom", "role-1", "type_x", "s0:c1.c2"],
            ),
            ("a:b:c:d.e-f,g", ["a", "b", "c", "d.e-f,g"]),
            ("a_:b_:c_:d_", ["a_", "b_", "c_", "d_"]),
        ];

        for (case, attrs) in TEST_CASES {
            let context = SecurityContext::parse(case).expect("Failed to parse security context");
            assert_eq!(context.get_user(), attrs[0]);
            assert_eq!(context.get_role(), attrs[1]);
            assert_eq!(context.get_type(), attrs[2]);
            assert_eq!(context.get_level(), attrs[3]);
        }
    }

    #[test]
    fn test_security_context_set() {
        const DEFAULT_CONTEXT: &str = "unconfined_u:object_r:default_t:s0";
        const TEST_CASES: &[(&str, [&str; 4])] = &[
            (
                "system_u:object_r:bin_t:s0",
                ["system_u", "object_r", "bin_t", "s0"],
            ),
            (
                "user_u:role_r:type_t:s0:c1,c2",
                ["user_u", "role_r", "type_t", "s0:c1,c2"],
            ),
            (
                "user.dom:role-1:type_x:s0:c1.c2",
                ["user.dom", "role-1", "type_x", "s0:c1.c2"],
            ),
            ("a:b:c:d.e-f,g", ["a", "b", "c", "d.e-f,g"]),
            ("a_:b_:c_:d_", ["a_", "b_", "c_", "d_"]),
        ];

        for (result, attrs) in TEST_CASES {
            let mut context =
                SecurityContext::parse(DEFAULT_CONTEXT).expect("Failed to parse security context");
            assert!(context.set_user(attrs[0]).is_ok());
            assert!(context.set_role(attrs[1]).is_ok());
            assert!(context.set_type(attrs[2]).is_ok());
            assert!(context.set_level(attrs[3]).is_ok());
            println!("{:?}", context);

            assert_eq!(context.as_os_str(), *result);
        }
    }

    #[test]
    fn test_get_set_security_context() {
        const TEST_FILE: &str = "selinux_test";
        const TEST_CONTEXT: &str = "unconfined_u:object_r:default_t:s0";

        let file_path = std::env::temp_dir().join(TEST_FILE);

        fs::remove_file(&file_path).ok();
        fs::write(&file_path, TEST_FILE).expect("Failed to write test file");

        let set_context = SecurityContext::parse(TEST_CONTEXT).expect("Invalid security context");
        self::set_security_context(&file_path, &set_context)
            .expect("Failed to set security context");
        println!("set context: {:#?}", set_context);

        let get_context =
            self::get_security_context(&file_path).expect("Failed to get security context");
        println!("get context: {:#?}", get_context);

        assert_eq!(set_context, get_context);
        fs::remove_file(&file_path).expect("Failed to remove test file");
    }
}
