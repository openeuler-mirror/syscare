use std::{ffi::CString, os::unix::prelude::OsStrExt, path::Path};

use anyhow::{Context, Result};

pub trait ToCString {
    fn to_cstring(&self) -> Result<CString>;
}

impl ToCString for Path {
    /// Converts a `Path` to an owned [`CString`].
    fn to_cstring(&self) -> Result<CString> {
        CString::new(self.as_os_str().as_bytes()).context("FFI failure")
    }
}

#[test]
fn test() -> Result<()> {
    use anyhow::ensure;

    let path = Path::new("/tmp");
    let cstring = path.to_cstring()?;

    let path_bytes = path.as_os_str().as_bytes();
    let cstring_bytes = cstring.as_bytes();

    ensure!(path_bytes == cstring_bytes);
    Ok(())
}
