use super::function::{rpc, RpcResult};

#[rpc(server)]
pub trait FastRebootSkeleton {
    #[rpc(name = "fast_reboot")]
    fn fast_reboot(&self, kernel_version: Option<String>, force: bool) -> RpcResult<()>;
}
