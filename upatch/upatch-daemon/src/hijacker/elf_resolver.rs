use std::{fs, path::PathBuf};

use anyhow::Result;
use object::{NativeFile, Object, ObjectSymbol};
use syscare_common::{
    os,
    util::{
        ext_cmd::{ExternCommand, ExternCommandArgs},
        os_str::OsStrExt,
    },
};

pub struct ElfResolver<'a> {
    elf: NativeFile<'a, &'a [u8]>,
}

impl<'a> ElfResolver<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Ok(Self {
            elf: NativeFile::parse(data)?,
        })
    }
}

impl ElfResolver<'_> {
    pub fn dependencies(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let args_list = ExternCommandArgs::new().arg(os::process::path());
        let output = ExternCommand::new("ldd").execvp(args_list)?;
        let lines = output.stdout().lines().filter_map(|s| s.ok());

        for line in lines {
            let words = line.split_whitespace().collect::<Vec<_>>();
            if let Some(path) = words.get(2) {
                if let Ok(path) = fs::canonicalize(path) {
                    paths.push(path);
                }
            }
        }

        Ok(paths)
    }

    pub fn find_symbol_addr(&self, symbol_name: &str) -> Result<Option<u64>> {
        let symbols = self.elf.dynamic_symbols();
        for sym in symbols {
            if let Ok(sym_name) = sym.name() {
                if sym_name == symbol_name {
                    return Ok(Some(sym.address()));
                }
            }
        }

        Ok(None)
    }
}
