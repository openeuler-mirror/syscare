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
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use object::{NativeFile, Object, ObjectSection};

use syscare_abi::{PatchEntity, PatchInfo, PatchType};
use syscare_common::{
    concat_os,
    ffi::{CStrExt, OsStrExt},
    fs,
};

use super::PatchResolverImpl;
use crate::patch::entity::{KernelPatch, KernelPatchFunction, Patch};

const KPATCH_SUFFIX: &str = ".ko";
const KPATCH_SYS_DIR: &str = "/sys/kernel/livepatch";
const KPATCH_SYS_FILE_NAME: &str = "enabled";

mod ffi {
    use std::os::raw::{c_char, c_long, c_ulong};

    use object::{
        read::elf::{ElfSectionRelocationIterator, FileHeader},
        Pod, Relocation,
    };

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    /// Corresponds to `struct kpatch_patch_func` defined in `kpatch-patch.h`
    pub struct KpatchFunction {
        pub new_addr: c_ulong,
        pub new_size: c_ulong,
        pub old_addr: c_ulong,
        pub old_size: c_ulong,
        pub sympos: u64,
        pub name: *const c_char,
        pub obj_name: *const c_char,
        pub ref_name: *const c_char,
        pub ref_offset: c_long,
    }

    pub const KPATCH_FUNCTION_SIZE: usize = std::mem::size_of::<KpatchFunction>();
    pub const KPATCH_FUNCTION_OFFSET: usize = 40;
    pub const KPATCH_OBJECT_OFFSET: usize = 48;

    /*
     * SAFETY: This struct is
     * - #[repr(C)]
     * - have no invalid byte values
     * - have no padding
     */
    unsafe impl Pod for KpatchFunction {}

    pub struct KpatchRelocation {
        pub addr: (u64, Relocation),
        pub name: (u64, Relocation),
        pub object: (u64, Relocation),
    }

    pub struct KpatchRelocationIterator<'data, 'file, Elf: FileHeader>(
        ElfSectionRelocationIterator<'data, 'file, Elf, &'data [u8]>,
    );

    impl<'data, 'file, Elf: FileHeader> KpatchRelocationIterator<'data, 'file, Elf> {
        pub fn new(relocations: ElfSectionRelocationIterator<'data, 'file, Elf>) -> Self {
            Self(relocations)
        }
    }

    impl<'data, 'file, Elf: FileHeader> Iterator for KpatchRelocationIterator<'data, 'file, Elf> {
        type Item = KpatchRelocation;

        fn next(&mut self) -> Option<Self::Item> {
            if let (Some(addr), Some(name), Some(object)) =
                (self.0.next(), self.0.next(), self.0.next())
            {
                return Some(KpatchRelocation { addr, name, object });
            }
            None
        }
    }
}

const KPATCH_FUNCS_SECTION: &str = ".kpatch.funcs";
const KPATCH_STRINGS_SECTION: &str = ".kpatch.strings";

pub struct KpatchResolverImpl;

impl KpatchResolverImpl {
    #[inline]
    fn resolve_patch_file(patch: &mut KernelPatch) -> Result<()> {
        let patch_file =
            fs::MappedFile::open(&patch.patch_file).context("Failed to map patch file")?;
        let patch_elf = NativeFile::parse(patch_file.as_bytes()).context("Invalid patch format")?;

        // Read sections
        let function_section = patch_elf
            .section_by_name(KPATCH_FUNCS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", KPATCH_FUNCS_SECTION))?;
        let string_section = patch_elf
            .section_by_name(KPATCH_STRINGS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", KPATCH_STRINGS_SECTION))?;
        let function_data = function_section
            .data()
            .with_context(|| format!("Failed to read section '{}'", KPATCH_FUNCS_SECTION))?;
        let string_data = string_section
            .data()
            .with_context(|| format!("Failed to read section '{}'", KPATCH_FUNCS_SECTION))?;

        // Resolve patch functions
        let patch_functions = &mut patch.functions;
        let kpatch_function_slice = object::slice_from_bytes::<ffi::KpatchFunction>(
            function_data,
            function_data.len() / ffi::KPATCH_FUNCTION_SIZE,
        )
        .map(|(f, _)| f)
        .map_err(|_| anyhow!("Invalid data format"))
        .context("Failed to resolve patch functions")?;

        for function in kpatch_function_slice {
            patch_functions.push(KernelPatchFunction {
                name: OsString::new(),
                object: OsString::new(),
                old_addr: function.old_addr,
                old_size: function.old_size,
                new_addr: function.new_addr,
                new_size: function.new_size,
            });
        }

        // Relocate patch functions
        for relocation in ffi::KpatchRelocationIterator::new(function_section.relocations()) {
            let (name_reloc_offset, name_reloc) = relocation.name;
            let (object_reloc_offset, obj_reloc) = relocation.object;

            // Relocate patch function name
            let name_index = (name_reloc_offset as usize - ffi::KPATCH_FUNCTION_OFFSET)
                / ffi::KPATCH_FUNCTION_SIZE;
            let name_function = patch_functions
                .get_mut(name_index)
                .context("Failed to find patch function")?;
            let name_offset = name_reloc.addend() as usize;
            let name_string = CStr::from_bytes_with_next_nul(&string_data[name_offset..])
                .context("Failed to parse patch object name")?
                .to_os_string();

            name_function.name = name_string;

            // Relocate patch function object
            let object_index = (object_reloc_offset as usize - ffi::KPATCH_OBJECT_OFFSET)
                / ffi::KPATCH_FUNCTION_SIZE;
            let object_function = patch_functions
                .get_mut(object_index)
                .context("Failed to find patch function")?;
            let object_offset = obj_reloc.addend() as usize;
            let object_string = CStr::from_bytes_with_next_nul(&string_data[object_offset..])
                .context("Failed to parse patch function name")?
                .to_os_string();

            object_function.object = object_string;
        }

        Ok(())
    }
}

impl PatchResolverImpl for KpatchResolverImpl {
    fn resolve_patch(
        &self,
        patch_root: &Path,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
    ) -> Result<Patch> {
        let module_name = patch_entity.patch_name.replace(['-', '.'], "_");
        let patch_file = patch_root.join(concat_os!(&patch_entity.patch_name, KPATCH_SUFFIX));
        let sys_file = PathBuf::from(KPATCH_SYS_DIR)
            .join(&module_name)
            .join(KPATCH_SYS_FILE_NAME);

        let mut patch = KernelPatch {
            uuid: patch_entity.uuid,
            name: concat_os!(
                patch_info.target.short_name(),
                "/",
                patch_info.name(),
                "/",
                &patch_entity.patch_target
            ),
            kind: PatchType::KernelPatch,
            info: patch_info.clone(),
            pkg_name: patch_info.target.full_name(),
            module_name,
            patch_file,
            sys_file,
            functions: Vec::new(),
            checksum: patch_entity.checksum.clone(),
        };
        Self::resolve_patch_file(&mut patch).context("Failed to resolve patch")?;

        Ok(Patch::KernelPatch(patch))
    }
}
