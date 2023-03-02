use serde::{Serialize, Deserialize};

pub const PATCH_STATUS_UNKNOWN:     &str = "UNKNOWN";
pub const PATCH_STATUS_NOT_APPLIED: &str = "NOT-APPLIED";
pub const PATCH_STATUS_DEACTIVED:   &str = "DEACTIVED";
pub const PATCH_STATUS_ACTIVED:     &str = "ACTIVED";

#[derive(Debug)]
#[derive(Hash)]
#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum PatchStatus {
    Unknown,
    NotApplied,
    Deactived,
    Actived,
}

impl From<u8> for PatchStatus {
    fn from(value: u8) -> Self {
        match value {
            1 => PatchStatus::NotApplied,
            2 => PatchStatus::Deactived,
            3 => PatchStatus::Actived,
            _ => PatchStatus::Unknown,
        }
    }
}

impl Default for PatchStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for PatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PatchStatus::Unknown    => PATCH_STATUS_UNKNOWN,
            PatchStatus::NotApplied => PATCH_STATUS_NOT_APPLIED,
            PatchStatus::Deactived  => PATCH_STATUS_DEACTIVED,
            PatchStatus::Actived    => PATCH_STATUS_ACTIVED,
        })
    }
}
