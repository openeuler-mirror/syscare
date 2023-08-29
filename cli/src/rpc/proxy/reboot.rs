use std::rc::Rc;

use anyhow::Result;
use function_name::named;

use super::{args::RpcArguments, remote::RpcRemote};

pub struct RebootProxy {
    remote: Rc<RpcRemote>,
}

impl RebootProxy {
    pub fn new(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }

    #[named]
    pub fn fast_reboot(&self, target: Option<String>, force: bool) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(target).arg(force))
    }
}
