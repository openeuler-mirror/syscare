use anyhow::{Context, Result};

use crate::fast_reboot::{KExecManager, RebootOption};
use log::info;

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::FastRebootSkeleton,
};

pub struct FastRebootSkeletonImpl;

impl FastRebootSkeleton for FastRebootSkeletonImpl {
    fn fast_reboot(&self, kernel_version: Option<String>, force: bool) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            info!("Rebooting system...");

            KExecManager::load_kernel(kernel_version)
                .and_then(|_| {
                    KExecManager::execute(match force {
                        true => RebootOption::Forced,
                        false => RebootOption::Normal,
                    })
                })
                .context("Failed to reboot system")
        })
    }
}
