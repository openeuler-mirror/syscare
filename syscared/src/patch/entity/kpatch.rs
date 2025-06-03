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
    collections::HashMap,
    ffi::{CStr, OsStr, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use object::{File, Object, ObjectSection};
use uuid::Uuid;

use syscare_abi::{PatchEntity, PatchInfo};
use syscare_common::{ffi::CStrExt, fs, os::kernel};

mod ffi {
    use std::os::raw::{c_char, c_long, c_ulong};

    use object::{Pod, Relocation, SectionRelocationIterator};

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
        pub _addr: (u64, Relocation),
        pub name: (u64, Relocation),
        pub object: (u64, Relocation),
    }

    pub struct KpatchRelocationIterator<'data, 'file>(pub SectionRelocationIterator<'data, 'file>);

    impl Iterator for KpatchRelocationIterator<'_, '_> {
        type Item = KpatchRelocation;

        fn next(&mut self) -> Option<Self::Item> {
            if let (Some(addr), Some(name), Some(object)) =
                (self.0.next(), self.0.next(), self.0.next())
            {
                return Some(KpatchRelocation {
                    _addr: addr,
                    name,
                    object,
                });
            }
            None
        }
    }
}

/// Kernel patch function definition
#[derive(Clone)]
pub struct KernelPatchFunction {
    pub name: OsString,
    pub object: OsString,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
}

impl std::fmt::Debug for KernelPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KernelPatchFunction")
            .field("name", &self.name)
            .field("object", &self.object)
            .field("old_addr", &format!("{:#x}", self.old_addr))
            .field("old_size", &format!("{:#x}", self.old_size))
            .field("new_addr", &format!("{:#x}", self.new_addr))
            .field("new_size", &format!("{:#x}", self.new_size))
            .finish()
    }
}

impl std::fmt::Display for KernelPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
             f,
             "name: {}, object: {}, old_addr: {:#x}, old_size: {:#x}, new_addr: {:#x}, new_size: {:#x}",
             self.name.to_string_lossy(),
             self.object.to_string_lossy(),
             self.old_addr,
             self.old_size,
             self.new_addr,
             self.new_size,
         )
    }
}

/// Kernel patch definition
#[derive(Debug)]
pub struct KernelPatch {
    pub uuid: Uuid,
    pub name: OsString,
    pub info: Arc<PatchInfo>,
    pub pkg_name: String,
    pub target_name: OsString,
    pub patch_file: PathBuf,
    pub status_file: PathBuf,
    pub module: kernel::ModuleInfo,
    pub functions: HashMap<OsString, Vec<KernelPatchFunction>>, // object name -> function list
    pub checksum: String,
}

impl KernelPatch {
    fn parse_functions(patch_file: &Path) -> Result<HashMap<OsString, Vec<KernelPatchFunction>>> {
        const KPATCH_FUNCS_SECTION: &str = ".kpatch.funcs";
        const KPATCH_STRINGS_SECTION: &str = ".kpatch.strings";

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
            .section_by_name(KPATCH_FUNCS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", KPATCH_FUNCS_SECTION))?;
        let string_section = file
            .section_by_name(KPATCH_STRINGS_SECTION)
            .with_context(|| format!("Cannot find section '{}'", KPATCH_STRINGS_SECTION))?;
        let function_data = function_section.data().map_err(|e| {
            anyhow!(
                "Failed to read section '{}', {}",
                KPATCH_FUNCS_SECTION,
                e.to_string().to_lowercase()
            )
        })?;
        let string_data = string_section.data().map_err(|e| {
            anyhow!(
                "Failed to read section '{}', {}",
                KPATCH_STRINGS_SECTION,
                e.to_string().to_lowercase()
            )
        })?;

        // Resolve patch functions
        let (slice, _) = object::slice_from_bytes::<ffi::KpatchFunction>(
            function_data,
            function_data.len() / ffi::KPATCH_FUNCTION_SIZE,
        )
        .map_err(|_| anyhow!("Invalid patch function layout"))?;

        let mut functions: Vec<_> = slice
            .iter()
            .map(|function| KernelPatchFunction {
                name: OsString::new(),
                object: OsString::new(),
                old_addr: function.old_addr,
                old_size: function.old_size,
                new_addr: function.new_addr,
                new_size: function.new_size,
            })
            .collect();

        // Relocate patch functions
        for relocation in ffi::KpatchRelocationIterator(function_section.relocations()) {
            let (name_offset, name_reloc) = relocation.name;
            let (object_offset, obj_reloc) = relocation.object;

            // Relocate patch function name
            let name_index =
                (name_offset as usize - ffi::KPATCH_FUNCTION_OFFSET) / ffi::KPATCH_FUNCTION_SIZE;
            let name_function = functions
                .get_mut(name_index)
                .with_context(|| format!("Invalid patch function index, index={}", name_index))?;
            let name_addend = name_reloc.addend() as usize;
            name_function.name = CStr::from_bytes_with_next_nul(&string_data[name_addend..])
                .map_err(|_| anyhow!("Invalid patch function name"))?
                .to_os_string();

            // Relocate patch function object
            let object_index =
                (object_offset as usize - ffi::KPATCH_OBJECT_OFFSET) / ffi::KPATCH_FUNCTION_SIZE;
            let object_function = functions
                .get_mut(object_index)
                .with_context(|| format!("Invalid patch object index, index={}", object_index))?;
            let object_addend = obj_reloc.addend() as usize;
            object_function.object = CStr::from_bytes_with_next_nul(&string_data[object_addend..])
                .map_err(|_| anyhow!("Invalid patch object name"))?
                .to_os_string();
        }

        // group functions by it's object
        let mut function_map: HashMap<_, Vec<_>> = HashMap::new();
        for function in functions {
            function_map
                .entry(function.object.clone())
                .or_default()
                .push(function);
        }

        Ok(function_map)
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
        const KPATCH_SYS_DIR: &str = "/sys/kernel/livepatch";
        const KPATCH_STATUS_FILE_NAME: &str = "enabled";

        let patch_file = patch_file.as_ref();
        let module = kernel::module_info(patch_file).map_err(|e| {
            anyhow!(
                "Failed to parse '{}' modinfo, {}",
                patch_file.display(),
                e.to_string().to_lowercase()
            )
        })?;

        let patch = Self {
            uuid: patch_entity.uuid,
            name: name.as_ref().to_os_string(),
            info: patch_info.clone(),
            pkg_name: patch_info.target.full_name(),
            target_name: patch_entity.patch_name.as_os_str().to_os_string(),
            patch_file: patch_file.to_path_buf(),
            status_file: PathBuf::from(KPATCH_SYS_DIR)
                .join(&module.name)
                .join(KPATCH_STATUS_FILE_NAME),
            module,
            functions: Self::parse_functions(patch_file)?,
            checksum: patch_entity.checksum.clone(),
        };
        Ok(patch)
    }
}

impl std::fmt::Display for KernelPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.to_string_lossy())
    }
}
