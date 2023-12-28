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

#[derive(Clone)]
pub struct Compiler {
    compiler: Vec<PathBuf>,
    assembler: Vec<PathBuf>,
    linker: Vec<PathBuf>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            compiler: Vec::new(),
            assembler: Vec::new(),
            linker: Vec::new(),
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
                .args(&["-gdwarf", "-ffunction-sections", "-fdata-sections", "-c"])
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
            .args(&["-r", "-o"])
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

pub struct CompilerHackGuard<'a> {
    inner: &'a Compiler,
    upatch_proxy: UpatchProxy,
}

impl<'a> CompilerHackGuard<'a> {
    pub fn new<P: AsRef<Path>>(compiler: &'a Compiler, socket_file: P) -> Result<Self> {
        let remote = RpcRemote::new(socket_file);
        let instance = Self {
            inner: compiler,
            upatch_proxy: UpatchProxy::new(Rc::new(remote)),
        };
        instance.hack()?;

        Ok(instance)
    }
}

impl CompilerHackGuard<'_> {
    pub fn hack(&self) -> Result<()> {
        let mut hack_list = Vec::new();
        hack_list.extend(&self.inner.compiler);
        hack_list.extend(&self.inner.assembler);

        let mut finished_list = Vec::new();
        let mut need_rollback = false;

        for exec_path in hack_list {
            trace!("Hacking \"{}\"...", exec_path.display());
            if let Err(e) = self.upatch_proxy.enable_hijack(exec_path.to_owned()) {
                error!("Failed to hack \"{}\", {}", exec_path.display(), e);
                need_rollback = true;
                break;
            }
            finished_list.push(exec_path);
        }

        if need_rollback {
            trace!("Rolling back...");
            for exec_path in finished_list {
                trace!("Unhacking \"{}\"...", exec_path.display());
                self.upatch_proxy.disable_hijack(exec_path.to_owned()).ok();
            }
            return Err(Error::Mod(String::from("Failed to hack compilers")));
        }

        Ok(())
    }

    pub fn unhack(&self) -> Result<()> {
        let mut hack_list = Vec::new();
        hack_list.extend(&self.inner.compiler);
        hack_list.extend(&self.inner.assembler);
        hack_list.reverse();

        for exec_path in hack_list {
            trace!("Unhacking \"{}\"...", exec_path.display());
            if let Err(e) = self.upatch_proxy.disable_hijack(exec_path.to_owned()) {
                error!("Failed to unhack \"{}\", {}", exec_path.display(), e);
                return Err(Error::Mod(String::from("Failed to unhack compilers")));
            }
        }
        Ok(())
    }
}

impl Drop for CompilerHackGuard<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.unhack() {
            trace!("{}", e);
        }
    }
}
