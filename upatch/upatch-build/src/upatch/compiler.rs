use std::ffi::CString;
use std::process::Command;
use std::fs::{OpenOptions, self};
use std::io::Write;

use crate::dwarf::Dwarf;
use crate::tool::*;
use crate::upatch::{ExternCommand, verbose};

use super::Result;
use super::Error;

const UPATCH_REGISTER_COMPILER: u64 = 1074324737;
const UPATCH_UNREGISTER_COMPILER: u64 = 1074324738;
const UPATCH_REGISTER_ASSEMBLER: u64 = 1074324739;
const UPATCH_UNREGISTER_ASSEMBLER: u64 = 1074324740;
use super::UPATCH_DEV_NAME;

pub struct Compiler {
    compiler_file: String,
    as_file: String,
    linker_file: String,
}

impl Compiler {
    pub fn new(compiler_file: String) -> Self {
        Self {
            compiler_file,
            as_file: String::new(),
            linker_file: String::new(),
        }
    }

    pub fn readlink(&self, name: String) -> Result<String> {
        let mut path_str = String::from_utf8(Command::new("which").arg(&name).output()?.stdout).unwrap();
        path_str.pop();
        if path_str.is_empty() {
            return Err(Error::Compiler(format!("can't found compiler")));
        }
        Ok(path_str)
    }

    pub fn analyze(&mut self) -> Result<()> {
        if self.compiler_file.is_empty() {
            self.compiler_file.push_str("gcc");
        }
        self.compiler_file = self.readlink(self.compiler_file.clone())?;
        println!("Using compiler at: {}", &self.compiler_file);

        self.as_file = String::from_utf8(Command::new(&self.compiler_file).arg("-print-prog-name=as").output()?.stdout).unwrap();
        self.as_file.pop();
        self.as_file = self.readlink(self.as_file.clone())?;

        self.linker_file = String::from_utf8(Command::new(&self.compiler_file).arg("-print-prog-name=ld").output()?.stdout).unwrap();
        self.linker_file.pop();
        self.linker_file = self.readlink(self.linker_file.clone())?;
        Ok(())
    }    
    
    pub fn hack(&self) -> Result<()> {
        self.__hack(true)
    }

    pub fn unhack(&self) -> Result<()> {
        self.__hack(false)
    }

    pub fn check_version(&self, cache_dir: &str, debug_info: &str) -> Result<()> {
        let tmp_dir = format!("{}/test", &cache_dir);
        fs::create_dir(&tmp_dir).unwrap();
        let test = format!("{}/test.c", &tmp_dir);
        let test_obj = format!("{}/test.o", &tmp_dir);
        let mut test_file = OpenOptions::new().create(true).read(true).write(true).open(&test)?;
        test_file.write_all(b"void main(void) {}")?;

        let args_list = vec!["-gdwarf", "-ffunction-sections", "-fdata-sections", "-c", &test, "-o", &test_obj];
        let output = ExternCommand::new(&self.compiler_file).execvp(args_list)?;
        match output.exit_status().success() {
            true => (),
            false => return Err(Error::Compiler(format!("compiler build test error {}: {}", output.exit_code(), output.stderr())))
        };

        let dwarf = Dwarf::new();
        let mut gcc_version = String::new();
        for element in dwarf.file_in_obj(debug_info.to_string())? {
            gcc_version.push_str(&element.get_compiler_version());
            break;
        }

        let mut system_gcc_version = String::new();
        for element in dwarf.file_in_obj(test_obj.clone())? {
            system_gcc_version.push_str(&element.get_compiler_version());
            break;
        }
        fs::remove_dir_all(&tmp_dir)?;

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

    pub fn linker(&self, dir: &str, output_file: &str) -> Result<()> {
        let arr = list_all_files_ext(dir, "o", false)?;
        if arr.is_empty() {
            return Err(Error::Compiler(format!("no functional changes found")));
        }

        let mut args_list = vec!["-r", "-o", output_file];
        let arr = arr.iter().map(|x| -> String {stringtify(x)}).rev().collect::<Vec<String>>();
        for i in 0..arr.len() {
            args_list.push(&arr[i]);
        }
        let output = ExternCommand::new(&self.linker_file).execvp(args_list)?;
        match output.exit_status().success() {
            true => verbose(output.stdout()),
            false => return Err(Error::Compiler(format!("link obj error {}: {:?}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }
}

impl Compiler {
    fn __hack(&self, register: bool) -> Result<()> {
        let ioctl_str = CString::new(format!("/dev/{}", UPATCH_DEV_NAME)).unwrap();
        let compiler_str = CString::new(self.compiler_file.clone()).unwrap();
        let assembler_str = CString::new(self.as_file.clone()).unwrap();

        unsafe{
            let fd = libc::open(ioctl_str.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return Err(Error::Mod(format!("open {} error", format!("/dev/{}", UPATCH_DEV_NAME))));
            }
            if register {
                let ret = libc::ioctl(fd, UPATCH_REGISTER_COMPILER, compiler_str.as_ptr());
                if ret < 0 {
                    libc::close(fd);
                    return Err(Error::Mod(format!("hack {} error", &self.compiler_file)));
                }
                let ret = libc::ioctl(fd, UPATCH_REGISTER_ASSEMBLER, assembler_str.as_ptr());
                if ret < 0 {
                    libc::ioctl(fd, UPATCH_UNREGISTER_COMPILER, compiler_str.as_ptr());
                    libc::close(fd);
                    return Err(Error::Mod(format!("hack {} error", &self.as_file)));
                }
            }
            else{
                let ret = libc::ioctl(fd, UPATCH_UNREGISTER_COMPILER, compiler_str.as_ptr());
                if ret < 0 {
                    return Err(Error::Mod(format!("unhack {} error", &self.as_file)));
                }
                let ret = libc::ioctl(fd, UPATCH_UNREGISTER_ASSEMBLER, assembler_str.as_ptr());
                if ret < 0 {
                    return Err(Error::Mod(format!("unhack {} error", &self.as_file)));
                }
            }
            libc::close(fd);
        }
        Ok(())
    }
}