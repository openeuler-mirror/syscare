use std::rc::Rc;

use anyhow::Result;
use function_name::named;

use crate::rpc::{RpcArguments, RpcRemote};

pub struct RebootProxy {
    remote: Rc<RpcRemote>,
}

impl RebootProxy {
    #[named]
    pub fn fast_reboot(&self, target: Option<String>, force: bool) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(target).arg(force))
    }
}

impl From<Rc<RpcRemote>> for RebootProxy {
    fn from(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }
}
