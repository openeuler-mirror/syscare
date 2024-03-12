use std::{path::Path, rc::Rc};

use anyhow::{Context, Result};
use indexmap::IndexSet;
use log::{error, info};

use super::compiler::Compiler;
use crate::rpc::{RpcRemote, UpatchProxy};

const UPATCHD_SOCKET_NAME: &str = "upatchd.sock";

pub struct Hijacker<'a> {
    proxy: UpatchProxy,
    programs: IndexSet<&'a Path>,
    finished: Vec<&'a Path>,
}

impl<'a> Hijacker<'a> {
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
        instance.hijack()?;

        Ok(instance)
    }
}

impl Hijacker<'_> {
    fn hijack(&mut self) -> Result<()> {
        info!("Hijacking compiler(s)");
        for exec_path in &self.programs {
            info!("- {}", exec_path.display());
            self.proxy
                .enable_hijack(exec_path)
                .with_context(|| format!("Failed to hijack {}", exec_path.display()))?;

            self.finished.push(exec_path);
        }

        Ok(())
    }

    fn unhack(&mut self) {
        info!("Releasing compiler(s)");
        while let Some(exec_path) = self.finished.pop() {
            info!("- {}", exec_path.display());
            let result = self
                .proxy
                .disable_hijack(exec_path)
                .with_context(|| format!("Failed to release {}", exec_path.display()));

            if let Err(e) = result {
                error!("{:?}", e);
            }
        }
    }
}

impl Drop for Hijacker<'_> {
    fn drop(&mut self) {
        self.unhack()
    }
}
