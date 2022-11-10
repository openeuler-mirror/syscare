use std::{fmt::Display, env, path::Path, process::exit};

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

    fn check(&self) {
        if self.source.is_empty() || self.build_file.is_empty() || self.debug_info.is_empty() || self.diff_file.is_empty() {
            println!("no input files");
            exit(-1);
        }
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

    pub fn read(&mut self) {
        let args: Vec<String> = env::args().collect();
        self.program.push_str(&args[0]);
        let mut i = 1;
        while i < args.len() {
            match &*args[i] {
                "-s" | "--debugsource" => {
                    i += 1;
                    if !Path::new(&args[i]).is_dir() {
                        println!("debugsource {} is not a directory", &args[i]);
                        self.usage();
                        exit(-1);
                    }
                    self.source.push_str(&args[i]);
                },
                "-b" | "--buildfile" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        println!("buildfile {} is not a file", &args[i]);
                        self.usage();
                        exit(-1);
                    }
                    self.build_file.push_str(&args[i]);
                },
                "-i" | "--debuginfo" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        println!("debuginfo {} is not a file", &args[i]);
                        self.usage();
                        exit(-1);
                    }
                    self.debug_info.push_str(&args[i]);
                },
                "-c" | "--compilerfile" => {
                    i += 1;
                    if !Path::new(&args[i]).is_file() {
                        println!("compiler {} is not a file", &args[i]);
                        self.usage();
                        exit(-1);
                    }
                    self.compiler_file.push_str(&args[i]);
                },
                "-o" | "--output" => {
                    i += 1;
                    self.output_file.push_str(&args[i]);
                },
                "-h" | "--help" => {
                    self.usage();
                },
                _ => {
                    if !Path::new(&args[i]).is_file() {
                        println!("patch {} is not a file", args[i]);
                        self.usage();
                        exit(-1);
                    }
                    self.diff_file.push(args[i].clone());
                },
            }
            i += 1;
        }
        self.check();
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