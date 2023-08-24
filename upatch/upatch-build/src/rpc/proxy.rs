use std::{path::PathBuf, rc::Rc};

use anyhow::Result;
use function_name::named;

use super::{args::RpcArguments, remote::RpcRemote};

pub struct UpatchProxy {
    remote: Rc<RpcRemote>,
}

impl UpatchProxy {
    pub fn new(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }

    #[named]
    pub fn enable_hijack(&self, exec_path: PathBuf) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(exec_path))
    }

    #[named]
    pub fn disable_hijack(&self, exec_path: PathBuf) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(exec_path))
    }
}
