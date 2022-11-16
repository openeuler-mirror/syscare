use std::ffi::CString;
use std::process::Command;
use std::fs::{OpenOptions, self};
use std::io::{self, Write};

use crate::dwarf::Dwarf;

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

    pub fn readlink(&self, name: String) -> io::Result<String> {
        let mut path_str = String::from_utf8(Command::new("which").arg(&name).output()?.stdout).unwrap();
        path_str.pop();
        if path_str.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("can't found compiler")));
        }
        Ok(path_str)
    }

    pub fn analyze(&mut self) -> io::Result<()> {
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
    
    pub fn hack(&self) -> io::Result<()> {
        self.__hack(true)
    }

    pub fn unhack(&self) -> io::Result<()> {
        self.__hack(false)
    }

    pub fn check_version(&self, cache_dir: &str, debug_info: &str) -> io::Result<()> {
        let tmp_dir = cache_dir.to_string() + "/test";
        fs::create_dir(&tmp_dir).unwrap();
        let test = tmp_dir.clone() + "/test.c";
        let test_obj = tmp_dir.clone() + "/test.o";
        let mut test_file = OpenOptions::new().create(true).read(true).write(true).open(&test)?;
        test_file.write_all(b"void main(void) {}")?;

        let result = Command::new("gcc")
                            .args(["-gdwarf", "-ffunction-sections", "-fdata-sections", "-c", &test, "-o", &test_obj])
                            .output()?;

        if !result.status.success(){
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("compiler build test error {}: {}", result.status, String::from_utf8(result.stderr).unwrap_or_default())));
        }

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
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("compiler version is different\n       debug_info compiler_version: {}\n       system compiler_version: {}", &gcc_version, &system_gcc_version)));
        }
        Ok(())
    }

    pub fn linker(&self, dir: &str, output_file: &str) -> io::Result<()> {
        let arr = walkdir::WalkDir::new(dir).into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .collect::<Vec<_>>();
        if arr.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no functional changes found"));
        }

        let mut build_cmd = Command::new(&self.linker_file);
        let mut build_cmd = build_cmd.arg("-r").arg("-o").arg(output_file);

        for obj in arr {       
            let name = obj.path().to_str().unwrap_or_default().to_string();
            build_cmd = build_cmd.arg(&name);
        }
        let result = build_cmd.output()?;
        if !result.status.success(){
                return Err(io::Error::new(io::ErrorKind::NotFound, format!("link obj error {}: {:?}", result.status, String::from_utf8(result.stderr).unwrap_or_default())));
        }
        Ok(())
    }
}

impl Compiler {
    fn __hack(&self, register: bool) -> io::Result<()> {
        let ioctl_str = CString::new(format!("/dev/{}", UPATCH_DEV_NAME)).unwrap();
        let compiler_str = CString::new(self.compiler_file.clone()).unwrap();
        let assembler_str = CString::new(self.as_file.clone()).unwrap();

        unsafe{
            let fd = libc::open(ioctl_str.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return Err(io::Error::new(io::ErrorKind::NotFound, format!("open {} error", format!("/dev/{}", UPATCH_DEV_NAME))));
            }
            if register {
                let ret = libc::ioctl(fd, UPATCH_REGISTER_COMPILER, compiler_str.as_ptr());
                if ret < 0 {
                    libc::close(fd);
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("hack {} error", &self.compiler_file)));
                }
                let ret = libc::ioctl(fd, UPATCH_REGISTER_ASSEMBLER, assembler_str.as_ptr());
                if ret < 0 {
                    libc::ioctl(fd, UPATCH_UNREGISTER_COMPILER, compiler_str.as_ptr());
                    libc::close(fd);
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("hack {} error", &self.as_file)));
                }
            }
            else{
                let ret = libc::ioctl(fd, UPATCH_UNREGISTER_COMPILER, compiler_str.as_ptr());
                if ret < 0 {
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("unhack {} error", &self.as_file)));
                }
                let ret = libc::ioctl(fd, UPATCH_UNREGISTER_ASSEMBLER, assembler_str.as_ptr());
                if ret < 0 {
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("unhack {} error", &self.as_file)));
                }
            }
            libc::close(fd);
        }
        Ok(())
    }
}