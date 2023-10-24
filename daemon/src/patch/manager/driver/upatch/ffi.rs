use std::ffi::{c_char, c_int, CString};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{Context, Result};
use syscare_abi::PatchStatus;

pub trait ToCString {
    fn to_cstring(&self) -> Result<CString>;
}

impl ToCString for Path {
    /// Converts a `Path` to an owned [`CString`].
    fn to_cstring(&self) -> Result<CString> {
        CString::new(self.as_os_str().as_bytes()).context("FFI failure")
    }
}

impl ToCString for str {
    /// Converts a `str` to an owned [`CString`].
    fn to_cstring(&self) -> Result<CString> {
        CString::new(self.as_bytes()).context("FFI failure")
    }
}

#[repr(C)]
pub enum UpatchStatus {
    NotApplied = 1,
    Deactived = 2,
    Active = 3,
    Invalid = 4,
}

impl From<UpatchStatus> for PatchStatus {
    fn from(status: UpatchStatus) -> Self {
        match status {
            UpatchStatus::NotApplied => PatchStatus::NotApplied,
            UpatchStatus::Deactived => PatchStatus::Deactived,
            UpatchStatus::Active => PatchStatus::Actived,
            UpatchStatus::Invalid => PatchStatus::Unknown,
        }
    }
}

#[link(name="upatch-tool-lib", kind="static")]
extern "C" {
    pub fn upatch_load(
        uuid: *const c_char,
        target_elf: *const c_char,
        patch_file: *const c_char,
    ) -> c_int;
    pub fn upatch_remove(uuid: *const c_char) -> c_int;
    pub fn upatch_active(uuid: *const c_char) -> c_int;
    pub fn upatch_deactive(uuid: *const c_char) -> c_int;
    pub fn upatch_status(uuid: *const c_char) -> UpatchStatus;
}
