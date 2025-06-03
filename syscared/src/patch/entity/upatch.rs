// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
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
    ffi::{CStr, OsStr, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use object::{File, Object, ObjectSection};
use uuid::Uuid;

use syscare_abi::{PatchEntity, PatchInfo};
use syscare_common::{ffi::CStrExt, fs};

mod ffi {
    use std::os::raw::{c_char, c_ulong};

    use object::{Pod, Relocation, SectionRelocationIterator};

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    /// Corresponds to `struct upatch_path_func` defined in `upatch-patch.h`
    pub struct UpatchFunction {
        pub new_addr: c_ulong,
        pub new_size: c_ulong,
        pub old_addr: c_ulong,
        pub old_size: c_ulong,
        pub sympos: c_ulong,
        pub name: *const c_char,
    }

    /*
     * SAFETY: This struct is
     * - #[repr(C)]
     * - have no invalid byte values
     * - have no padding
     */
    unsafe impl Pod for UpatchFunction {}

    pub const UPATCH_FUNCTION_SIZE: usize = std::mem::size_of::<UpatchFunction>();
    pub const UPATCH_FUNCTION_OFFSET: usize = 40;

    pub struct UpatchRelocation {
        pub _addr: (u64, Relocation),
        pub name: (u64, Relocation),
    }

    pub struct UpatchRelocationIterator<'data, 'file>(pub SectionRelocationIterator<'data, 'file>);

    impl Iterator for UpatchRelocationIterator<'_, '_> {
        type Item = UpatchRelocation;

        fn next(&mut self) -> Option<Self::Item> {
            if let (Some(addr), Some(name)) = (self.0.next(), self.0.next()) {
                return Some(UpatchRelocation { _addr: addr, name });
            }
            None
        }
    }
}

/// User patch function definition
#[derive(Clone)]
pub struct UserPatchFunction {
    pub name: OsString,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
}

impl std::fmt::Debug for UserPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserPatchFunction")
            .field("name", &self.name)
            .field("old_addr", &format!("0x{:x}", self.old_addr))
            .field("old_size", &format!("0x{:x}", self.old_size))
            .field("new_addr", &format!("0x{:x}", self.new_addr))
            .field("new_size", &format!("0x{:x}", self.new_size))
            .finish()
    }
}

impl std::fmt::Display for UserPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, old_addr: 0x{:x}, old_size: 0x{:x}, new_addr: 0x{:x}, new_size: 0x{:x}",
            self.name.to_string_lossy(),
            self.old_addr,
            self.old_size,
            self.new_addr,
            self.new_size,
        )
    }
}

/// User patch definition
#[derive(Debug)]
pub struct UserPatch {
    pub uuid: Uuid,
    pub name: OsString,
    pub info: Arc<PatchInfo>,
    pub pkg_name: String,
    pub target_elf: PathBuf,
    pub patch_file: PathBuf,
    pub functions: Vec<UserPatchFunction>,
    pub checksum: String,
}

impl UserPatch {
    fn parse_functions(patch_file: &Path) -> Result<Vec<UserPatchFunction>> {
        const UPATCH_FUNCS_SECTION: &str = ".upatch.funcs";
        const UPATCH_STRINGS_SECTION: &str = ".upatch.strings";

        let mmap = fs::mmap(patch_file).map_err(|e| {
            anyhow!(
                "Failed to mmap '{}', {}",
                patch_file.display(),
                e.to_string().to_lowercase()
            )
        })?;
        let file = File::parse(mmap.as_ref()).map_err(|e| {
            anyhow!(
                "Failed to parse '{}', {}",
                patch_file.display(),
                e.to_string().to_lowercase()
            )
        })?;

        // Read sections
        let function_section = file
            .section_by_name(UPATCH_FUNCS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", UPATCH_FUNCS_SECTION))?;
        let string_section = file
            .section_by_name(UPATCH_STRINGS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", UPATCH_STRINGS_SECTION))?;
        let function_data = function_section.data().map_err(|e| {
            anyhow!(
                "Failed to read section '{}', {}",
                UPATCH_FUNCS_SECTION,
                e.to_string().to_lowercase()
            )
        })?;
        let string_data = string_section.data().map_err(|e| {
            anyhow!(
                "Failed to read section '{}', {}",
                UPATCH_STRINGS_SECTION,
                e.to_string().to_lowercase()
            )
        })?;

        // Resolve patch functions
        let (slice, _) = object::slice_from_bytes::<ffi::UpatchFunction>(
            function_data,
            function_data.len() / ffi::UPATCH_FUNCTION_SIZE,
        )
        .map_err(|_| anyhow!("Invalid patch function layout"))?;

        let mut functions: Vec<_> = slice
            .iter()
            .map(|function| UserPatchFunction {
                name: OsString::new(),
                old_addr: function.old_addr,
                old_size: function.old_size,
                new_addr: function.new_addr,
                new_size: function.new_size,
            })
            .collect();

        // Relocate patch functions
        for relocation in ffi::UpatchRelocationIterator(function_section.relocations()) {
            let (value, reloc) = relocation.name;

            let index = (value as usize - ffi::UPATCH_FUNCTION_OFFSET) / ffi::UPATCH_FUNCTION_SIZE;
            let function = functions
                .get_mut(index)
                .with_context(|| format!("Invalid patch function index, index={}", index))?;
            let addend = reloc.addend() as usize;

            function.name = CStr::from_bytes_with_next_nul(&string_data[addend..])
                .map_err(|_| anyhow!("Invalid patch function name"))?
                .to_os_string();
        }

        Ok(functions)
    }

    pub fn parse<S, P>(
        name: S,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
        patch_file: P,
    ) -> Result<Self>
    where
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let patch_file = patch_file.as_ref();

        let patch = Self {
            uuid: patch_entity.uuid,
            name: name.as_ref().to_os_string(),
            info: patch_info.clone(),
            pkg_name: patch_info.target.full_name(),
            target_elf: patch_entity.patch_target.clone(),
            patch_file: patch_file.to_path_buf(),
            functions: Self::parse_functions(patch_file)?,
            checksum: patch_entity.checksum.clone(),
        };
        Ok(patch)
    }
}

impl std::fmt::Display for UserPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.to_string_lossy())
    }
}
