use std::fmt::Display;
use std::env;
use std::path::Path;
use std::process::exit;

use super::Result;
use super::Error;

pub struct Arg {
    pub source: String,
    pub build_file: String,
    pub debug_info: String,
    pub compiler_file: String,
    pub output_file: String,
    pub diff_file: Vec<String>,
    program: String,
}

impl Arg {
    fn usage(&self) {
        println!("Usage: {} FILE [options]", self.program);
        println!("      -h|--help:        options message");
        println!("      -s|--debugsource: Specify source directory");
        println!("      -b|--buildfile:   Specify build file");
        println!("      -i|--debuginfo:   Specify debug info");
        println!("      -c|--compiler:    Specify compiler, default gcc");
        println!("      -o|--output:      Specify output, default");
    }

    fn check(&self) -> Result<()>  {
        if self.source.is_empty() || self.build_file.is_empty() || self.debug_info.is_empty() || self.diff_file.is_empty() {
            return Err(Error::InvalidInput(format!("no input files")));
        }
        Ok(())
    }
}

impl Arg {
    pub fn new() -> Self {
        Self { 
            source: String::new(),
            build_file: String::new(), 
            debug_info: String::new(), 
            compiler_file: String::new(), 
            output_file: String::new(),
            diff_file: Vec::new(),
            program: String::new(),
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let args: Vec<String> = env::args().collect();
        self.program.push_str(&args[0]);
        let mut i = 1;
        while i < args.len() {
            match &*args[i] {
                "-s" | "--debugsource" => {
                    i += 1;
                    if !Path::new(&args[i]).is_dir() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("debugsource {} is not a directory", &args[i])));
                    }
                    self.source.push_str(&args[i]);
                },
                "-b" | "--buildfile" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("buildfile {} is not a file", &args[i])));
                    }
                    self.build_file.push_str(&args[i]);
                },
                "-i" | "--debuginfo" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("debuginfo {} is not a file", &args[i])));
                    }
                    self.debug_info.push_str(&args[i]);
                },
                "-c" | "--compilerfile" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("compiler {} is not a file", &args[i])));
                    }
                    self.compiler_file.push_str(&args[i]);
                },
                "-o" | "--output" => {
                    i += 1;
                    self.output_file.push_str(&args[i]);
                },
                "-h" | "--help" => {
                    self.usage();
                    exit(0);
                },
                _ => {
                    if !Path::new(&args[i]).is_file() {
                        self.usage();
                        return Err(Error::InvalidInput(format!("patch {} is not a file", args[i])));
                    }
                    self.diff_file.push(args[i].clone());
                },
            }
            i += 1;
        }
        self.check()
    }
}

impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source: {}, build file: {}, debug info: {}, compiler file: {}, output file: {}, diff files: {:?}", self.source, 
            self.build_file, 
            self.debug_info, 
            self.compiler_file,
            self.output_file,
            self.diff_file)
    }
}
