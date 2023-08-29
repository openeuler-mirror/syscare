use std::path::PathBuf;

use super::function::{rpc, RpcResult};

#[rpc(server)]
pub trait Skeleton {
    #[rpc(name = "enable_hijack")]
    fn enable_hijack(&self, exec_path: PathBuf) -> RpcResult<()>;

    #[rpc(name = "disable_hijack")]
    fn disable_hijack(&self, exec_path: PathBuf) -> RpcResult<()>;
}
