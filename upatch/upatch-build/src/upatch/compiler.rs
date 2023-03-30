use std::ffi::{CString, OsStr, OsString};
use std::path::{Path, PathBuf};
use std::os::unix::ffi::OsStrExt;

use log::*;
use which::which;

use crate::dwarf::Dwarf;
use crate::cmd::*;

use super::Result;
use super::Error;
use super::UPATCH_DEV_NAME;

const UPATCH_REGISTER_COMPILER: u64 = 1074324737;
const UPATCH_UNREGISTER_COMPILER: u64 = 1074324738;
const UPATCH_REGISTER_ASSEMBLER: u64 = 1074324739;
const UPATCH_UNREGISTER_ASSEMBLER: u64 = 1074324740;
const UPATCH_REGISTER_LINK: u64 = 1074324741;
const UPATCH_UNREGISTER_LINK: u64 = 1074324742;
const UPATCH_HACK_NUM: usize = 3;

#[derive(Clone)]
pub struct Compiler {
    compiler: PathBuf,
    assembler: PathBuf,
    linker: PathBuf,
    collect2: PathBuf,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            compiler: PathBuf::new(),
            assembler: PathBuf::new(),
            linker: PathBuf::new(),
            collect2: PathBuf::new(),
        }
    }

    pub fn readlink(&self, name: &OsStr) -> Result<PathBuf> {
        match which(name) {
            Ok(result) => Ok(result),
            Err(e) => Err(Error::Compiler(format!("get {:?} failed: {}", name, e))),
        }
    }

    pub fn read_from_compiler(&self, name: &str) -> Result<OsString> {
        let args_list = ExternCommandArgs::new().arg(&name);
        let output = ExternCommand::new(&self.compiler).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Compiler(format!("get {} from compiler {:?} failed", name, &self.compiler)));
        }
        Ok(output.stdout().to_os_string())
    }

    pub fn analyze<P: AsRef<Path>>(&mut self, compiler_file: P) -> Result<()> {
        self.compiler = compiler_file.as_ref().to_path_buf();
        info!("Using compiler at: {:?}", &self.compiler);

        self.assembler = self.readlink(&self.read_from_compiler("-print-prog-name=as")?)?;
        self.linker = self.readlink(&self.read_from_compiler("-print-prog-name=ld")?)?;
        self.collect2 = self.readlink(&self.read_from_compiler("-print-prog-name=collect2")?)?;
        Ok(())
    }

    pub fn hack(&self) -> Result<()> {
        self.__hack(true)
    }

    pub fn unhack(&self) -> Result<()> {
        self.__hack(false)
    }

    pub fn check_version<P: AsRef<Path>>(&self, cache_dir: P, debug_info: P) -> Result<()> {
        let cache_dir = cache_dir.as_ref();
        let debug_info = debug_info.as_ref();
        let test_obj = Path::new(&cache_dir).join("test.o");
        let output = std::process::Command::new("echo").arg("int main() {return 0;}").stdout(std::process::Stdio::piped()).spawn().expect("exec echo failed");

        let args_list = ExternCommandArgs::new().args(["-gdwarf", "-ffunction-sections", "-fdata-sections", "-x", "c", "-", "-o"]).arg(&test_obj);
        let output = ExternCommand::new(&self.compiler).execvp_stdio(args_list, cache_dir, output.stdout.expect("get echo stdout failed"))?;
        if !output.exit_status().success() {
            return Err(Error::Compiler(format!("compiler build test error {}: {:?}", output.exit_code(), output.stderr())))
        };

        let dwarf = Dwarf::new();
        let mut gcc_version = String::new();
        for element in dwarf.file_in_obj(&debug_info)? {
            gcc_version.push_str(&element.get_compiler_version());
            break;
        }

        let mut system_gcc_version = String::new();
        for element in dwarf.file_in_obj(test_obj.clone())? {
            system_gcc_version.push_str(&element.get_compiler_version());
            break;
        }

        /* Dwraf DW_AT_producer 
         * GNU standard version 
         */
        let gcc_version_arr = gcc_version.split(" ").collect::<Vec<_>>();
        let system_gcc_version_arr = system_gcc_version.split(" ").collect::<Vec<_>>();


        if gcc_version_arr.len() < 3 || system_gcc_version_arr.len() < 3 || gcc_version_arr[2] != system_gcc_version_arr[2] {
            return Err(Error::Compiler(format!("compiler version is different\n       debug_info compiler_version: {}\n       system compiler_version: {}", &gcc_version, &system_gcc_version)));
        }
        Ok(())
    }

    pub fn linker<P, Q>(&self, link_list: &Vec<P>, output_file: Q) -> Result<()>
    where
        P: AsRef<OsStr>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new().args(["-r", "-o"]).arg(output_file.as_ref()).args(link_list);
        let output = ExternCommand::new(&self.linker).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Compiler(format!("link object file error {}: {:?}", output.exit_code(), output.stderr())));
        };
        Ok(())
    }
}

impl Compiler {
    fn __hack(&self, register: bool) -> Result<()> {
        let ioctl_str = CString::new(format!("/dev/{}", UPATCH_DEV_NAME)).unwrap();
        let compiler_str = CString::new(self.compiler.as_os_str().as_bytes()).unwrap();
        let assembler_str = CString::new(self.assembler.as_os_str().as_bytes()).unwrap();
        let collect2_str = CString::new(self.collect2.as_os_str().as_bytes()).unwrap();
        let hack_array: [CString; UPATCH_HACK_NUM] = [compiler_str, assembler_str, collect2_str];

        unsafe{
            let fd = libc::open(ioctl_str.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return Err(Error::Mod(format!("open {:?} error", ioctl_str)));
            }
            let result = self.ioctl(fd, UPATCH_HACK_NUM, &hack_array, register);
            let ret = libc::close(fd);
            if ret < 0 {
                return Err(Error::Mod(format!("close {:?} error", ioctl_str)));
            }
            result
        }
    }

    fn ioctl(&self, fd: i32, num: usize, hack_array: &[CString; UPATCH_HACK_NUM], register: bool) -> Result<()> {
        unsafe {
            match register {
                true => {
                    let hack_request: [u64; UPATCH_HACK_NUM] = [UPATCH_REGISTER_COMPILER, UPATCH_REGISTER_ASSEMBLER, UPATCH_REGISTER_LINK];
                    for i in 0..num {
                        debug!("hack {:?}", hack_array[i]);
                        let ret = libc::ioctl(fd, hack_request[i], hack_array[i].as_ptr());
                        if ret != 0 {
                            debug!("hack {:?} error {}, try to rollback", hack_array[i], ret);
                            self.ioctl(fd, i, hack_array, false)?;
                            return Err(Error::Mod(format!("hack {:?} error {}", hack_array[i], ret)));
                        }
                    }
                },
                false => {
                    let hack_request: [u64; UPATCH_HACK_NUM] = [UPATCH_UNREGISTER_COMPILER, UPATCH_UNREGISTER_ASSEMBLER, UPATCH_UNREGISTER_LINK];
                    for i in (0..num).rev() {
                        debug!("unhack {:?}", hack_array[i]);
                        let ret = libc::ioctl(fd, hack_request[i], hack_array[i].as_ptr());
                        if ret != 0 {
                            debug!("unhack {:?} error {}", hack_array[i], ret);
                            return Err(Error::Mod(format!("unhack {:?} error {}", hack_array[i], ret)));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}