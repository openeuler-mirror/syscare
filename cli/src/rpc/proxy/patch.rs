use std::rc::Rc;

use anyhow::Result;
use function_name::named;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use super::{args::RpcArguments, remote::RpcRemote};

pub struct PatchProxy {
    remote: Rc<RpcRemote>,
}

impl PatchProxy {
    pub fn new(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }

    #[named]
    pub fn check_patch(&self, identifier: &str) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn apply_patch(&self, identifier: &str, force: bool) -> Result<Vec<PatchStateRecord>> {
        self.remote.call_with_args(
            function_name!(),
            RpcArguments::new().arg(identifier).arg(force),
        )
    }

    #[named]
    pub fn remove_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn active_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn deactive_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn accept_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_list(&self) -> Result<Vec<PatchListRecord>> {
        self.remote.call(function_name!())
    }

    #[named]
    pub fn get_patch_status(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_info(&self, identifier: &str) -> Result<PatchInfo> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_target(&self, identifier: &str) -> Result<PackageInfo> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn save_patch_status(&self) -> Result<()> {
        self.remote.call(function_name!())
    }

    #[named]
    pub fn restore_patch_status(&self, accepted_only: bool) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(accepted_only))
    }
}
