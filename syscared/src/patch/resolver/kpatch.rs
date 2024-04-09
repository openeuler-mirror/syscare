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
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use object::{NativeFile, Object, ObjectSection};

use syscare_abi::{PatchEntity, PatchInfo, PatchType};
use syscare_common::{concat_os, ffi::OsStrExt, fs};

use super::PatchResolverImpl;
use crate::patch::entity::{KernelPatch, KernelPatchSymbol, Patch};

const KPATCH_SUFFIX: &str = ".ko";
const KPATCH_SYS_DIR: &str = "/sys/kernel/livepatch";
const KPATCH_SYS_FILE_NAME: &str = "enabled";

mod ffi {
    use std::os::raw::{c_char, c_long, c_ulong};

    use object::Pod;

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

    pub const KPATCH_FUNC_SIZE: usize = std::mem::size_of::<KpatchFunction>();
    pub const KPATCH_FUNC_NAME_OFFSET: usize = 40;
    pub const KPATCH_OBJECT_NAME_OFFSET: usize = 48;

    /*
     * SAFETY: This struct is
     * - #[repr(C)]
     * - have no invalid byte values
     * - have no padding
     */
    unsafe impl Pod for KpatchFunction {}

    pub enum KpatchRelocation {
        NewAddr = 0,
        Name = 1,
        ObjName = 2,
    }

    impl From<usize> for KpatchRelocation {
        fn from(value: usize) -> Self {
            match value {
                0 => KpatchRelocation::NewAddr,
                1 => KpatchRelocation::Name,
                2 => KpatchRelocation::ObjName,
                _ => unreachable!(),
            }
        }
    }

    pub const KPATCH_FUNC_RELA_TYPE_NUM: usize = 3;
}

use ffi::*;

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
        let patch_symbols = &mut patch.symbols;
        let patch_functions = object::slice_from_bytes::<KpatchFunction>(
            function_data,
            function_data.len() / KPATCH_FUNC_SIZE,
        )
        .map(|(f, _)| f)
        .map_err(|_| anyhow!("Invalid data format"))
        .context("Failed to resolve patch functions")?;

        for function in patch_functions {
            patch_symbols.push(KernelPatchSymbol {
                name: OsString::new(),
                target: OsString::new(),
                old_addr: function.old_addr,
                old_size: function.old_size,
                new_addr: function.new_addr,
                new_size: function.new_size,
            });
        }

        // Relocate patch functions
        for (index, (offset, relocation)) in function_section.relocations().enumerate() {
            match KpatchRelocation::from(index % KPATCH_FUNC_RELA_TYPE_NUM) {
                KpatchRelocation::Name => {
                    let symbol_index =
                        (offset as usize - KPATCH_FUNC_NAME_OFFSET) / KPATCH_FUNC_SIZE;
                    let patch_symbol = patch_symbols
                        .get_mut(symbol_index)
                        .context("Failed to find patch symbol")?;

                    let name_offset = relocation.addend() as usize;
                    let mut name_bytes = &string_data[name_offset..];
                    let string_end = name_bytes
                        .iter()
                        .position(|b| b == &b'\0')
                        .context("Failed to find termination char")?;
                    name_bytes = &name_bytes[..string_end];

                    patch_symbol.name = OsString::from_vec(name_bytes.to_vec());
                }
                KpatchRelocation::ObjName => {
                    let symbol_index =
                        (offset as usize - KPATCH_OBJECT_NAME_OFFSET) / KPATCH_FUNC_SIZE;
                    let patch_symbol = patch_symbols
                        .get_mut(symbol_index)
                        .context("Failed to find patch symbol")?;

                    let name_offset = relocation.addend() as usize;
                    let mut name_bytes = &string_data[name_offset..];
                    let string_end = name_bytes
                        .iter()
                        .position(|b| b == &b'\0')
                        .context("Failed to find termination char")?;
                    name_bytes = &name_bytes[..string_end];

                    patch_symbol.target = OsString::from_vec(name_bytes.to_vec());
                }
                _ => {}
            };
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
            symbols: Vec::new(),
            checksum: patch_entity.checksum.clone(),
        };
        Self::resolve_patch_file(&mut patch).context("Failed to resolve patch elf")?;

        Ok(Patch::KernelPatch(patch))
    }
}
