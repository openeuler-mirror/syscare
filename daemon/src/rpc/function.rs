pub use jsonrpc_core::Result as RpcResult;
use jsonrpc_core::{Error, ErrorCode};
pub use jsonrpc_derive::rpc;
use log::error;

const RPC_OP_ERROR: i64 = -1;

pub struct RpcFunction;

impl RpcFunction {
    pub fn call<F, T>(f: F) -> RpcResult<T>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        (f)().map_err(|e| {
            error!("{:?}", e);
            Error {
                code: ErrorCode::ServerError(RPC_OP_ERROR),
                message: format!("{:?}", e),
                data: None,
            }
        })
    }
}
