use crate::rpc::{rpc, RpcResult};

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

#[rpc(server)]
pub trait PatchSkeleton {
    #[rpc(name = "apply_patch")]
    fn apply_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "remove_patch")]
    fn remove_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "active_patch")]
    fn active_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "deactive_patch")]
    fn deactive_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "accept_patch")]
    fn accept_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "get_patch_list")]
    fn get_patch_list(&self) -> RpcResult<Vec<PatchListRecord>>;

    #[rpc(name = "get_patch_status")]
    fn get_patch_status(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "get_patch_info")]
    fn get_patch_info(&self, identifier: String) -> RpcResult<PatchInfo>;

    #[rpc(name = "get_patch_target")]
    fn get_patch_target(&self, identifier: String) -> RpcResult<PackageInfo>;

    #[rpc(name = "save_patch_status")]
    fn save_patch_status(&self) -> RpcResult<()>;

    #[rpc(name = "restore_patch_status")]
    fn restore_patch_status(&self, accepted_only: bool) -> RpcResult<()>;
}
