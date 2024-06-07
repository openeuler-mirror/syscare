use std::{path::Path, rc::Rc};

use anyhow::{Context, Result};
use indexmap::IndexSet;
use log::{error, info};

use super::compiler::Compiler;
use crate::rpc::{RpcRemote, UpatchProxy};

const UPATCHD_SOCKET_NAME: &str = "upatchd.sock";

pub struct UpatchHelper<'a> {
    proxy: UpatchProxy,
    programs: IndexSet<&'a Path>,
    finished: Vec<&'a Path>,
}

impl<'a> UpatchHelper<'a> {
    pub fn new<I, P>(compilers: I, work_dir: P) -> Result<Self>
    where
        I: IntoIterator<Item = &'a Compiler>,
        P: AsRef<Path>,
    {
        let socket_file = work_dir.as_ref().join(UPATCHD_SOCKET_NAME);
        let remote = RpcRemote::new(socket_file);
        let proxy = UpatchProxy::new(Rc::new(remote));

        let mut programs = IndexSet::new();
        for compiler in compilers {
            programs.insert(compiler.path.as_path());
            programs.insert(compiler.assembler.as_path());
        }

        let mut instance = Self {
            proxy,
            programs,
            finished: vec![],
        };
        instance.enable()?;

        Ok(instance)
    }
}

impl UpatchHelper<'_> {
    fn enable(&mut self) -> Result<()> {
        info!("Hooking compiler(s)");
        for exec_path in &self.programs {
            info!("- {}", exec_path.display());
            self.proxy
                .hook_compiler(exec_path)
                .with_context(|| format!("Failed to hook compiler {}", exec_path.display()))?;

            self.finished.push(exec_path);
        }

        Ok(())
    }

    fn disable(&mut self) {
        info!("Unhooking compiler(s)");
        while let Some(exec_path) = self.finished.pop() {
            info!("- {}", exec_path.display());
            let result = self.proxy.unhook_compiler(exec_path).with_context(|| {
                format!("Failed to unhook compiler helper {}", exec_path.display())
            });

            if let Err(e) = result {
                error!("{:?}", e);
            }
        }
    }
}

impl Drop for UpatchHelper<'_> {
    fn drop(&mut self) {
        self.disable()
    }
}
