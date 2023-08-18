use anyhow::{Context, Result};

mod kexec;
mod manager;
mod skeleton;

use log::info;
use manager::*;
pub use skeleton::FastRebootSkeleton;

use crate::rpc::{RpcFunction, RpcResult};

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
