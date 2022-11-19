use std::fmt::Display;
use std::env;
use std::path::Path;
use std::process::exit;

use super::Result;
use super::Error;
use super::{stringtify, realpath};

pub struct Arg {
    pub work_dir: String,
    pub source: String,
    pub build_command: String,
    pub debug_info: String,
    pub compiler_file: String,
    pub elf_name: String,
    pub output: String,
    pub patch_name: String,
    pub diff_file: Vec<String>,
    program: String,
    pub skip_compiler_check: bool,
    pub rpmbuild: bool,
}

impl Arg {
    fn usage(&self) {
        println!("Usage: {} FILE [options]", self.program);
        println!("      -h|--help:              options message");
        println!("      -w|--workdir:           Specify work directory, default ~/.upatch/");
        println!("      -s|--debugsource:       Specify source directory");
        println!("      -b|--buildcommand:      Specify build cmd");
        println!("      -i|--debuginfo:         Specify debug info");
        println!("      -c|--compiler:          Specify compiler, default gcc");
        println!("      -e|--elfname:           Specify running file name");
        println!("      -o|--output:            Specify output directory, default ~/.upatch/");
        println!("      -n|--name:              Specify output directory, default elfname");
        println!("      --skip-compiler-check:  Specify skip_compiler_check, default false");
        println!("      --rpmbuild:             Specify rpmbuild, default false");
    }

    fn check(&mut self) -> Result<()>  {
        if self.source.is_empty() ||
            self.debug_info.is_empty() ||
            self.diff_file.is_empty() ||
            self.elf_name.is_empty() ||
            (self.build_command.is_empty() && !self.rpmbuild) {
            self.usage();
            return Err(Error::InvalidInput(format!("no input files")));
        }
        Ok(())
    }
}

impl Arg {
    pub fn new() -> Self {
        Self {
            work_dir: String::new(),
            source: String::new(),
            build_command: String::new(),
            debug_info: String::new(),
            elf_name: String::new(),
            compiler_file: String::new(),
            output: String::new(),
            patch_name: String::new(),
            diff_file: Vec::new(),
            program: String::new(),
            skip_compiler_check: false,
            rpmbuild: false,
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let args: Vec<String> = env::args().collect();
        self.program.push_str(&args[0]);
        let mut i = 1;
        while i < args.len() {
            match &*args[i] {
                "-w" | "--workdir" => {
                    i += 1;
                    if !Path::new(&args[i]).is_dir() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("workdir {} is not a directory", &args[i])));
                    }
                    self.work_dir.push_str(&*stringtify(realpath(&args[i])?));
                },
                "-s" | "--debugsource" => {
                    i += 1;
                    if !Path::new(&args[i]).is_dir() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("debugsource {} is not a directory", &args[i])));
                    }
                    self.source.push_str(&*stringtify(realpath(&args[i])?));
                },
                "-b" | "--buildcommand" => {
                    i += 1;
                    self.build_command.push_str(&args[i]);
                },
                "-i" | "--debuginfo" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("debuginfo {} is not a file", &args[i])));
                    }
                    self.debug_info.push_str(&*stringtify(realpath(&args[i])?));
                },
                "-e" | "--elfname" => {
                    i += 1;
                    self.elf_name.push_str(&args[i]);
                },
                "-c" | "--compiler" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("compiler {} is not a file", &args[i])));
                    }
                    self.compiler_file.push_str(&*stringtify(realpath(&args[i])?));
                },
                "-o" | "--output" => {
                    i += 1;
                    if !Path::new(&args[i]).is_dir() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("output {} is not a file", &args[i])));
                    }
                    self.output.push_str(&*stringtify(realpath(&args[i])?));
                },
                "-n" | "--name" => {
                    i += 1;
                    self.patch_name.push_str(&args[i]);
                },
                "-h" | "--help" => {
                    self.usage();
                    exit(0);
                },
                "--skip-compiler-check" => {
                    self.skip_compiler_check = true;
                },
                "--rpmbuild" => {
                    self.rpmbuild = true;
                }
                _ => {
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("patch {} is not a file", args[i])));
                    }
                    self.diff_file.push(stringtify(realpath(&args[i])?));
                },
            }
            i += 1;
        }
        self.check()
    }
}

impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "work_dir: {}, source: {}, build command: {}, debug info: {}, compiler file: {}, elf_name{}, output: {}, patch_name{}, diff files: {:?}, skip_compiler_check: {}, rpmbuild: {}",
            self.work_dir,
            self.source,
            self.build_command,
            self.debug_info,
            self.compiler_file,
            self.elf_name,
            self.output,
            self.patch_name,
            self.diff_file,
            self.skip_compiler_check,
            self.rpmbuild,
            )
    }
}