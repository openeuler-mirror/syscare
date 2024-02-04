use std::{
    ffi::{CStr, OsStr, OsString},
    os::unix::{ffi::OsStringExt, prelude::OsStrExt},
    path::{Path, PathBuf},
};

pub trait CStrExt: AsRef<CStr> {
    fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes(self.as_ref().to_bytes())
    }

    fn as_path(&self) -> &Path {
        Path::new(self.as_os_str())
    }

    fn to_os_string(&self) -> OsString {
        OsString::from_vec(self.as_ref().to_bytes().to_vec())
    }

    fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_os_string())
    }
}

impl<T: AsRef<CStr>> CStrExt for T {}

#[test]
fn test_cstr() {
    use std::ffi::CString;

    let path = Path::new("/tmp/test");
    let cstring = CString::new("/tmp/test").unwrap();

    assert_eq!(path.as_os_str().as_bytes(), cstring.to_bytes());
    assert_ne!(path.as_os_str().as_bytes(), cstring.to_bytes_with_nul());

    println!("Testing trait CStrExt::as_os_str...");
    assert_eq!(path.as_os_str(), cstring.as_os_str());

    println!("Testing trait CStrExt::as_path...");
    assert_eq!(path, cstring.as_path());

    println!("Testing trait CStrExt::to_os_string...");
    assert_eq!(path.as_os_str().to_os_string(), cstring.to_os_string());

    println!("Testing trait CStrExt::to_path_buf...");
    assert_eq!(path.to_path_buf(), cstring.to_path_buf());
}
