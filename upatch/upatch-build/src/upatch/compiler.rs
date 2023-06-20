use std::collections::HashSet;
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use log::*;
use which::which;

use crate::cmd::*;
use crate::dwarf::Dwarf;
use crate::tool::realpath;

use super::Error;
use super::Result;
use super::UPATCH_DEV_NAME;

const UPATCH_REGISTER_COMPILER: u64 = 1074324737;
const UPATCH_UNREGISTER_COMPILER: u64 = 1074324738;
const UPATCH_REGISTER_ASSEMBLER: u64 = 1074324739;
const UPATCH_UNREGISTER_ASSEMBLER: u64 = 1074324740;

#[derive(Clone)]
pub struct Compiler {
    compiler: Vec<PathBuf>,
    assembler: Vec<PathBuf>,
    linker: Vec<PathBuf>,
    inode: HashSet<u64>,
    hack_request: Vec<u64>,
    unhack_request: Vec<u64>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            compiler: Vec::new(),
            assembler: Vec::new(),
            linker: Vec::new(),
            inode: HashSet::new(),
            hack_request: Vec::new(),
            unhack_request: Vec::new(),
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
            // soft link
            let compiler = realpath(compiler)?;
            let assembler =
                realpath(self.which(&self.read_from_compiler(&compiler, "-print-prog-name=as")?)?)?;
            let linker =
                realpath(self.which(&self.read_from_compiler(&compiler, "-print-prog-name=ld")?)?)?;

            // hard link
            if self.check_inode(&compiler)? {
                self.compiler.push(compiler);
            }

            if self.check_inode(&assembler)? {
                self.assembler.push(assembler);
            }

            if self.check_inode(&linker)? {
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

        self.hack_request = vec![UPATCH_REGISTER_COMPILER; self.compiler.len()];
        self.hack_request
            .append(&mut vec![UPATCH_REGISTER_ASSEMBLER; self.assembler.len()]);
        self.unhack_request = vec![UPATCH_UNREGISTER_COMPILER; self.compiler.len()];
        self.unhack_request
            .append(&mut vec![UPATCH_UNREGISTER_ASSEMBLER; self.assembler.len()]);
        Ok(())
    }

    pub fn hack(&self) -> Result<()> {
        let (ioctl_str, hack_array) = self.get_cstring()?;

        unsafe {
            let fd = libc::open(ioctl_str.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return Err(Error::Mod(format!("open {:?} error", ioctl_str)));
            }
            let result = self.ioctl_register(fd, hack_array.len(), &hack_array);
            let ret = libc::close(fd);
            if ret < 0 {
                return Err(Error::Mod(format!("close {:?} error", ioctl_str)));
            }
            result
        }
    }

    pub fn unhack(&self) -> Result<()> {
        let (ioctl_str, hack_array) = self.get_cstring()?;

        unsafe {
            let fd = libc::open(ioctl_str.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return Err(Error::Mod(format!("open {:?} error", ioctl_str)));
            }
            let result = self.ioctl_unregister(fd, hack_array.len(), &hack_array);
            let ret = libc::close(fd);
            if ret < 0 {
                return Err(Error::Mod(format!("close {:?} error", ioctl_str)));
            }
            result
        }
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
    fn get_cstring(&self) -> Result<(CString, Vec<CString>)> {
        let ioctl_str = CString::new(format!("/dev/{}", UPATCH_DEV_NAME)).unwrap();
        let mut hack_array = Vec::with_capacity(self.compiler.len() + self.assembler.len());
        for compiler in &self.compiler {
            hack_array.push(CString::new(compiler.as_os_str().as_bytes()).unwrap());
        }
        for assembler in &self.assembler {
            hack_array.push(CString::new(assembler.as_os_str().as_bytes()).unwrap());
        }
        Ok((ioctl_str, hack_array))
    }

    fn ioctl_register(&self, fd: i32, num: usize, hack_array: &[CString]) -> Result<()> {
        for i in 0..num {
            trace!("hack {:?}", hack_array[i]);
            let ret = unsafe { libc::ioctl(fd, self.hack_request[i], hack_array[i].as_ptr()) };
            if ret != 0 {
                trace!("hack {:?} error {}, try to rollback", hack_array[i], ret);
                self.ioctl_unregister(fd, i, hack_array)?;
                return Err(Error::Mod(format!(
                    "hack {:?} error {}",
                    hack_array[i], ret
                )));
            }
        }
        Ok(())
    }

    fn ioctl_unregister(&self, fd: i32, num: usize, hack_array: &[CString]) -> Result<()> {
        for i in (0..num).rev() {
            trace!("unhack {:?}", hack_array[i]);
            let ret = unsafe { libc::ioctl(fd, self.unhack_request[i], hack_array[i].as_ptr()) };
            if ret != 0 {
                trace!("unhack {:?} error {}", hack_array[i], ret);
                return Err(Error::Mod(format!(
                    "unhack {:?} error {}",
                    hack_array[i], ret
                )));
            }
        }
        Ok(())
    }

    fn check_inode<P: AsRef<Path>>(&mut self, path: P) -> Result<bool> {
        let inode = std::fs::metadata(path)?.ino(); // hard link
        Ok(self.inode.insert(inode))
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
