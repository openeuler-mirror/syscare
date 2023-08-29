use serde::{Deserialize, Serialize};

use super::PatchStatus;

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchStateRecord {
    pub name: String,
    pub status: PatchStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchListRecord {
    pub uuid: String,
    pub name: String,
    pub status: PatchStatus,
}
