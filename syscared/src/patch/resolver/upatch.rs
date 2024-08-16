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
    ffi::{CStr, OsString},
    path::Path,
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use object::{NativeFile, Object, ObjectSection};

use syscare_abi::{PatchEntity, PatchInfo};
use syscare_common::{concat_os, ffi::CStrExt, fs};

use super::PatchResolverImpl;
use crate::patch::entity::{Patch, UserPatch, UserPatchFunction};

mod ffi {
    use std::os::raw::{c_char, c_ulong};

    use object::{
        read::elf::{ElfSectionRelocationIterator, FileHeader},
        Pod, Relocation,
    };

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

    pub struct UpatchRelocationIterator<'data, 'file, Elf: FileHeader>(
        ElfSectionRelocationIterator<'data, 'file, Elf, &'data [u8]>,
    );

    impl<'data, 'file, Elf: FileHeader> UpatchRelocationIterator<'data, 'file, Elf> {
        pub fn new(relocations: ElfSectionRelocationIterator<'data, 'file, Elf>) -> Self {
            Self(relocations)
        }
    }

    impl<'data, 'file, Elf: FileHeader> Iterator for UpatchRelocationIterator<'data, 'file, Elf> {
        type Item = UpatchRelocation;

        fn next(&mut self) -> Option<Self::Item> {
            if let (Some(addr), Some(name)) = (self.0.next(), self.0.next()) {
                return Some(UpatchRelocation { _addr: addr, name });
            }
            None
        }
    }
}

const UPATCH_FUNCS_SECTION: &str = ".upatch.funcs";
const UPATCH_STRINGS_SECTION: &str = ".upatch.strings";

pub struct UpatchResolverImpl;

impl UpatchResolverImpl {
    #[inline]
    fn resolve_patch_elf(patch: &mut UserPatch) -> Result<()> {
        let patch_file =
            fs::MappedFile::open(&patch.patch_file).context("Failed to map patch file")?;
        let patch_elf = NativeFile::parse(patch_file.as_bytes()).context("Invalid patch format")?;

        // Read sections
        let function_section = patch_elf
            .section_by_name(UPATCH_FUNCS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", UPATCH_FUNCS_SECTION))?;
        let string_section = patch_elf
            .section_by_name(UPATCH_STRINGS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", UPATCH_STRINGS_SECTION))?;
        let function_data = function_section
            .data()
            .with_context(|| format!("Failed to read section '{}'", UPATCH_FUNCS_SECTION))?;
        let string_data = string_section
            .data()
            .with_context(|| format!("Failed to read section '{}'", UPATCH_FUNCS_SECTION))?;

        // Resolve patch functions
        let patch_functions = &mut patch.functions;
        let upatch_function_slice = object::slice_from_bytes::<ffi::UpatchFunction>(
            function_data,
            function_data.len() / ffi::UPATCH_FUNCTION_SIZE,
        )
        .map(|(f, _)| f)
        .map_err(|_| anyhow!("Invalid data format"))
        .context("Failed to resolve patch functions")?;

        for function in upatch_function_slice {
            patch_functions.push(UserPatchFunction {
                name: OsString::new(),
                old_addr: function.old_addr,
                old_size: function.old_size,
                new_addr: function.new_addr,
                new_size: function.new_size,
            });
        }

        // Relocate patch functions
        for relocation in ffi::UpatchRelocationIterator::new(function_section.relocations()) {
            let (name_reloc_offset, name_reloc) = relocation.name;

            let name_index = (name_reloc_offset as usize - ffi::UPATCH_FUNCTION_OFFSET)
                / ffi::UPATCH_FUNCTION_SIZE;
            let name_function = patch_functions
                .get_mut(name_index)
                .context("Failed to find patch function")?;
            let name_offset = name_reloc.addend() as usize;
            let name_string = CStr::from_bytes_with_next_nul(&string_data[name_offset..])
                .context("Failed to parse patch function name")?
                .to_os_string();

            name_function.name = name_string;
        }

        Ok(())
    }
}

impl PatchResolverImpl for UpatchResolverImpl {
    fn resolve_patch(
        &self,
        patch_root: &Path,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
    ) -> Result<Patch> {
        let mut patch = UserPatch {
            uuid: patch_entity.uuid,
            name: concat_os!(
                patch_info.target.short_name(),
                "/",
                patch_info.name(),
                "/",
                fs::file_name(&patch_entity.patch_target)
            ),
            info: patch_info.clone(),
            pkg_name: patch_info.target.full_name(),
            patch_file: patch_root.join(&patch_entity.patch_name),
            target_elf: patch_entity.patch_target.clone(),
            functions: Vec::new(),
            checksum: patch_entity.checksum.clone(),
        };
        Self::resolve_patch_elf(&mut patch).context("Failed to resolve patch")?;

        Ok(Patch::UserPatch(patch))
    }
}
