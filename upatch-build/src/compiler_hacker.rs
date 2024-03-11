use std::{path::Path, rc::Rc};

use anyhow::{Context, Result};
use indexmap::IndexSet;
use log::{error, info};

use super::compiler::Compiler;
use crate::rpc::{RpcRemote, UpatchProxy};

pub struct CompilerHacker<'a> {
    proxy: UpatchProxy,
    programs: IndexSet<&'a Path>,
    finished: Vec<&'a Path>,
}

impl<'a> CompilerHacker<'a> {
    pub fn new<I, P>(compilers: I, socket_file: P) -> Result<Self>
    where
        I: IntoIterator<Item = &'a Compiler>,
        P: AsRef<Path>,
    {
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
        instance.hack()?;

        Ok(instance)
    }
}

impl CompilerHacker<'_> {
    fn hack(&mut self) -> Result<()> {
        info!("Hacking compiler(s)");
        for exec_path in &self.programs {
            info!("- {}", exec_path.display());
            self.proxy
                .enable_hijack(exec_path)
                .with_context(|| format!("Failed to hack {}", exec_path.display()))?;

            self.finished.push(exec_path);
        }

        Ok(())
    }

    fn unhack(&mut self) {
        info!("Unhacking compiler(s)");
        while let Some(exec_path) = self.finished.pop() {
            info!("- {}", exec_path.display());
            let result: std::prelude::v1::Result<(), anyhow::Error> = self
                .proxy
                .disable_hijack(exec_path)
                .with_context(|| format!("Failed to unhack {}", exec_path.display()));

            if let Err(e) = result {
                error!("{:?}", e);
            }
        }
    }
}

impl Drop for CompilerHacker<'_> {
    fn drop(&mut self) {
        self.unhack()
    }
}
