use serde::{Deserialize, Serialize};

const PATCH_STATUS_UNKNOWN: &str = "UNKNOWN";
const PATCH_STATUS_NOT_APPLIED: &str = "NOT-APPLIED";
const PATCH_STATUS_DEACTIVED: &str = "DEACTIVED";
const PATCH_STATUS_ACTIVED: &str = "ACTIVED";
const PATCH_STATUS_ACCEPTED: &str = "ACCEPTED";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PatchStatus {
    Unknown,
    NotApplied,
    Deactived,
    Actived,
    Accepted,
}

impl Default for PatchStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for PatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PatchStatus::Unknown => PATCH_STATUS_UNKNOWN,
            PatchStatus::NotApplied => PATCH_STATUS_NOT_APPLIED,
            PatchStatus::Deactived => PATCH_STATUS_DEACTIVED,
            PatchStatus::Actived => PATCH_STATUS_ACTIVED,
            PatchStatus::Accepted => PATCH_STATUS_ACCEPTED,
        })
    }
}
