use jsonrpc::{serde::Serialize, serde_json::value::RawValue};
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct RpcArguments {
    args: Vec<Box<RawValue>>,
}

impl RpcArguments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg<T: Serialize>(mut self, arg: T) -> Self {
        self.args.push(jsonrpc::arg(arg));
        self
    }
}

impl Deref for RpcArguments {
    type Target = [Box<RawValue>];

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}
