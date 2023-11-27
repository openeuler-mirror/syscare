use std::cmp;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use log::*;
use which::which;

use crate::cmd::*;
use crate::dwarf::Dwarf;
use crate::rpc::*;

use super::Error;
use super::Result;

const UPATCHD_SOCKET: &str = "/var/run/upatchd.sock";

#[derive(Clone)]
pub struct Compiler {
    compiler: Vec<PathBuf>,
    assembler: Vec<PathBuf>,
    linker: Vec<PathBuf>,
    upatch_proxy: UpatchProxy,
}

impl Compiler {
    pub fn new() -> Self {
        let remote = RpcRemote::new(UPATCHD_SOCKET);
        Self {
            compiler: Vec::new(),
            assembler: Vec::new(),
            linker: Vec::new(),
            upatch_proxy: UpatchProxy::new(Rc::new(remote)),
        }
    }

    pub fn which(&self, name: &OsStr) -> Result<PathBuf> {
        match which(name) {
            Ok(result) => Ok(result),
            Err(e) => Err(Error::Compiler(format!("get {:?} failed: {}", name, e))),
        }
    }

    pub fn read_from_compiler<P: AsRef<Path>>(&self, compiler: P, name: &str) -> Result<OsString> {
        let args_list = ExternCommandArgs::new().arg(name);
        let output = ExternCommand::new(compiler.as_ref()).execv(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Compiler(format!(
                "get {} from compiler {:?} failed",
                name, &self.compiler
            )));
        }
        Ok(output.stdout().to_os_string())
    }

    pub fn analyze<I, P>(&mut self, compiler_file: I) -> Result<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for compiler in compiler_file {
            let compiler = compiler.as_ref().to_path_buf();
            let assembler =
                self.which(&self.read_from_compiler(&compiler, "-print-prog-name=as")?)?;
            let linker = self.which(&self.read_from_compiler(&compiler, "-print-prog-name=ld")?)?;

            if !self.compiler.contains(&compiler) {
                self.compiler.push(compiler);
            }

            if !self.assembler.contains(&assembler) {
                self.assembler.push(assembler);
            }

            if !self.linker.contains(&linker) {
                self.linker.push(linker);
            }
        }

        info!("Using compiler at: {:?}", &self.compiler);
        trace!("Using assembler at: {:?}", &self.assembler);
        trace!("Using linker at: {:?}", &self.linker);
        if self.assembler.is_empty() {
            return Err(Error::Compiler(
                "can't find assembler in compiler".to_string(),
            ));
        }
        if self.linker.is_empty() {
            return Err(Error::Compiler("can't find linker in compiler".to_string()));
        }
        Ok(())
    }

    pub fn hack(&self) -> Result<()> {
        self.hijacker_register(self.compiler.len() + self.assembler.len())
    }

    pub fn unhack(&self) -> Result<()> {
        self.hijacker_unregister(self.compiler.len() + self.assembler.len())
    }

    pub fn check_version<P, I, Q>(&self, cache_dir: P, debug_infoes: I) -> Result<()>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = Q>,
        Q: AsRef<Path>,
    {
        let cache_dir = cache_dir.as_ref();
        let test_path = Path::new(&cache_dir).join("test.c");
        let mut test_file = File::create(&test_path)?;
        test_file.write_all(b"int main() {return 0;}")?;
        let test_obj = Path::new(&cache_dir).join("test.o");
        let dwarf = Dwarf::new();
        let mut system_compiler_version = HashSet::new();

        for compiler in &self.compiler {
            let args_list = ExternCommandArgs::new()
                .args(["-gdwarf", "-ffunction-sections", "-fdata-sections", "-c"])
                .arg(&test_path)
                .arg("-o")
                .arg(&test_obj);
            let output = ExternCommand::new(compiler).execv(args_list)?;
            if !output.exit_status().success() {
                return Err(Error::Compiler(format!(
                    "compiler build test error {}: {}",
                    output.exit_code(),
                    output.stderr().to_string_lossy()
                )));
            };

            /* Dwraf DW_AT_producer
             * GNU standard version
             */
            for element in dwarf.file_in_obj(test_obj.clone())? {
                let compiler_version = element.get_compiler_version();
                let compiler_version_arr = compiler_version.split(' ').collect::<Vec<_>>();
                if compiler_version_arr.len() < 3 {
                    return Err(Error::Compiler(format!(
                        "read system compiler version failed: {}",
                        element.get_compiler_version()
                    )));
                }
                system_compiler_version.insert(compiler_version_arr[2].to_string());
            }
        }
        debug!("system compiler version: {:?}", &system_compiler_version);

        for debug_info in debug_infoes {
            for element in dwarf.file_in_obj(debug_info.as_ref())? {
                let compiler_version = element.get_compiler_version();
                let compiler_version_arr = compiler_version.split(' ').collect::<Vec<_>>();
                if compiler_version_arr.len() < 3 {
                    return Err(Error::Compiler(format!(
                        "read {:?} compiler version failed: {}",
                        debug_info.as_ref(),
                        &element.get_compiler_version()
                    )));
                }
                if !system_compiler_version.contains(compiler_version_arr[2]) {
                    return Err(Error::Compiler(format!("compiler version is different \n{:?}'s compiler version: {} \nsystem compiler version: {:?}", debug_info.as_ref(), &compiler_version_arr[2], &system_compiler_version)));
                }
            }
        }
        Ok(())
    }

    pub fn linker<P, Q>(&self, link_list: &Vec<P>, output_file: Q) -> Result<()>
    where
        P: AsRef<OsStr>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new()
            .args(["-r", "-o"])
            .arg(output_file.as_ref())
            .args(link_list);
        let output = ExternCommand::new(&self.linker[0]).execv(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Compiler(format!(
                "link object file error {}: {}",
                output.exit_code(),
                output.stderr().to_string_lossy()
            )));
        };
        Ok(())
    }
}

impl Compiler {
    fn hijacker_register(&self, num: usize) -> Result<()> {
        for i in 0..cmp::min(num, self.compiler.len()) {
            trace!("hack {}", self.compiler[i].display());
            if let Err(e) = self.upatch_proxy.enable_hijack(self.compiler[i].clone()) {
                trace!("hack {:?} error {}, try to rollback", self.compiler[i], e);
                self.hijacker_unregister(i)?;
                return Err(Error::Mod(format!(
                    "hack {} error {}",
                    self.compiler[i].display(),
                    e
                )));
            }
        }
        for i in self.compiler.len()..num {
            let i = i - self.compiler.len();
            trace!("hack {}", self.assembler[i].display());
            if let Err(e) = self.upatch_proxy.enable_hijack(self.assembler[i].clone()) {
                trace!(
                    "hack {} error {}, try to rollback",
                    self.assembler[i].display(),
                    e
                );
                self.hijacker_unregister(i)?;
                return Err(Error::Mod(format!(
                    "hack {} error {}",
                    self.assembler[i].display(),
                    e
                )));
            }
        }
        Ok(())
    }

    fn hijacker_unregister(&self, num: usize) -> Result<()> {
        for i in (self.compiler.len()..num).rev() {
            let i = i - self.compiler.len();
            trace!("unhack {}", self.assembler[i].display());
            if let Err(e) = self.upatch_proxy.disable_hijack(self.assembler[i].clone()) {
                trace!("unhack {} error {}", self.assembler[i].display(), e);
                return Err(Error::Mod(format!(
                    "unhack {} error {}",
                    self.assembler[i].display(),
                    e
                )));
            }
        }

        for i in (0..cmp::min(num, self.compiler.len())).rev() {
            trace!("unhack {}", self.compiler[i].display());
            if let Err(e) = self.upatch_proxy.disable_hijack(self.compiler[i].clone()) {
                trace!("unhack {} error {}", self.compiler[i].display(), e);
                return Err(Error::Mod(format!(
                    "unhack {} error {}",
                    self.compiler[i].display(),
                    e
                )));
            }
        }
        Ok(())
    }
}

pub struct CompilerHackGuard {
    compiler: Compiler,
}

impl CompilerHackGuard {
    pub fn new(compiler: Compiler) -> Result<Self> {
        compiler.hack()?;
        Ok(CompilerHackGuard { compiler })
    }
}
impl Drop for CompilerHackGuard {
    fn drop(&mut self) {
        if let Err(e) = self.compiler.unhack() {
            trace!("{}", e);
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
