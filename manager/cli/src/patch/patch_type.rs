use serde::{Serialize, Deserialize};

pub const PATCH_TYPE_USER_PATCH:   &str = "UserPatch";
pub const PATCH_TYPE_KERNEL_PATCH: &str = "KernelPatch";

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum PatchType {
    UserPatch,
    KernelPatch,
}

impl std::str::FromStr for PatchType {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            PATCH_TYPE_USER_PATCH   => Ok(PatchType::UserPatch),
            PATCH_TYPE_KERNEL_PATCH => Ok(PatchType::KernelPatch),
            _ => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("parse patch type failed")
                ))
            }
        }
    }
}

impl std::fmt::Display for PatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PatchType::UserPatch   => PATCH_TYPE_USER_PATCH,
            PatchType::KernelPatch => PATCH_TYPE_KERNEL_PATCH,
        })
    }
}
