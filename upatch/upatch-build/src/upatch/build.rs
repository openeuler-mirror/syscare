use std::{path::Path, env};
use std::process::Command;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions, File};
use std::io::{self, Read};

use walkdir::WalkDir;

use crate::arg::Arg;
use crate::dwarf::{Dwarf, DwarfCompileUnit};
use super::Compiler;
use super::Project;

pub const UPATCH_DEV_NAME: &str = "upatch";
const SYSTEM_MOUDLES: &str = "/proc/modules";
const CMD_SOURCE_ENTER: &str = "SE";
const CMD_PATCHED_ENTER: &str = "PE";
const SUPPORT_TOOL: &str = "create-diff-object";

pub struct UpatchBuild {
    cache_dir: String,
    source_dir: String,
    patch_dir: String,
    output_dir: String,
    log_file: String,
    args: Arg,
    tool_file: String,
}

impl UpatchBuild {
    pub fn new(args: Arg) -> Self {
        #![allow(deprecated)]
        let base = String::from(env::home_dir().unwrap().to_str().unwrap()); // home_dir() don't support BSD system
        let cache_dir = base.clone() + "/.upatch";
        let source_dir = cache_dir.clone() + "/source";
        let patch_dir = cache_dir.clone() + "/patch";
        let output_dir = cache_dir.clone() + "/output";
        let log_file = cache_dir.clone() + "/buildlog";
        Self {
            cache_dir,
            source_dir,
            patch_dir,
            output_dir,
            log_file,
            args,
            tool_file: String::new(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        // create .upatch directory
        self.create_dir()?;

        if self.args.output_file.is_empty() {
            self.args.output_file.push_str(&(self.cache_dir.clone() + "/result"));
        }

        // check mod
        self.check_mod()?;

        // find create-diff-object
        self.search_tool()?;

        // check compiler
        let mut compiler = Compiler::new(self.args.compiler_file.clone());
        compiler.analyze()?;
        compiler.check_version(&self.cache_dir, &self.args.debug_info)?;

        // copy source
        let dest = Path::new(&self.cache_dir);
        let project_name = &Path::new(&self.args.source).file_name().unwrap();
        let dest = dest.join(project_name);
        copy_dir::copy_dir(&self.args.source, &dest)?;

        // hack compiler
        println!("Hacking compiler");
        compiler.hack()?;

        // build source
        let build_file = &Path::new(&self.args.build_file).file_name().unwrap().to_str().unwrap();

        println!("Building original {}", project_name.to_str().unwrap());
        let project = Project::new(dest.to_str().unwrap().to_string(), build_file.to_string());
        project.build(CMD_SOURCE_ENTER, &self.source_dir)?;

        // build patch
        for patch in &self.args.diff_file {
            println!("Patching file: {}", &patch);
            project.patch(patch.clone())?;
        }

        println!("Building patched {}", project_name.to_str().unwrap());
        project.build(CMD_PATCHED_ENTER, &self.patch_dir)?;

        // unhack compiler
        println!("Unhacking compiler");
        compiler.unhack()?;

        println!("Detecting changed objects");
        // correlate obj name
        let mut source_obj: HashMap<String, String> = HashMap::new();
        let mut patch_obj: HashMap<String, String> = HashMap::new();
        let dwarf = Dwarf::new();

        self.correlate_obj(&dwarf, dest.to_str().unwrap(), &self.source_dir, &mut source_obj)?;
        self.correlate_obj(&dwarf, dest.to_str().unwrap(), &self.patch_dir, &mut patch_obj)?;

        // choose the binary's obj to create upatch file
        let binary = &Path::new(&self.args.debug_info).file_name().unwrap().to_str().unwrap();
        let binary_obj = dwarf.file_in_binary(dest.to_str().unwrap().to_string(), binary.to_string())?;
        self.correlate_diff(&source_obj, &patch_obj, &binary_obj)?;

        // clear source
        fs::remove_dir_all(&dest)?;

        // ld patchs
        compiler.linker(&self.output_dir, &self.args.output_file)?;
        println!("Building patch: {}", &self.args.output_file);
        Ok(())
    }
}

impl UpatchBuild {
    fn create_dir(&self) -> io::Result<()> {
        if Path::new(&self.cache_dir).is_dir() {
            fs::remove_dir_all(self.cache_dir.clone())?;
        }

        fs::create_dir_all(self.cache_dir.clone())?;
        fs::create_dir(self.source_dir.clone())?;
        fs::create_dir(self.patch_dir.clone())?;
        fs::create_dir(self.output_dir.clone())?;
        File::create(&self.log_file)?;
        Ok(())
    }

    fn check_mod(&self) -> io::Result<()> {
        let mut file = File::open(SYSTEM_MOUDLES)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        match contents.find(UPATCH_DEV_NAME) {
            Some(_) => Ok(()),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "can't found upatch mod in system")),
        }
    }

    fn correlate_obj(&self, dwarf: &Dwarf, comp_dir: &str, dir: &str, map: &mut HashMap<String, String>) -> io::Result<()> {
        let arr = WalkDir::new(dir).into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect::<Vec<_>>();
        for obj in arr {       
            let name = obj.path().to_str().unwrap_or_default().to_string(); 
            let result = dwarf.file_in_obj(name.clone())?;
            if result.len() == 1 && result[0].DW_AT_comp_dir.find(comp_dir) != None{
                map.insert(result[0].get_source(), name.clone());
            }
        }
        Ok(())
    }

    fn correlate_diff(&self, source_obj: &HashMap<String, String>, patch_obj: &HashMap<String, String>, binary_obj: &Vec<DwarfCompileUnit>) -> io::Result<()> {
        // TODO can print changed function
        for path in binary_obj {
            let source_name = path.get_source();
            match &patch_obj.get(&source_name) {
                Some(patch) => {
                    let output_dir =  self.output_dir.clone() + "/" + Path::new(patch).file_name().unwrap().to_str().unwrap();
                    match &source_obj.get(&source_name) {
                        Some(source) => {
                            let output = Command::new(&self.tool_file)
                                        .args(["-s", &source, "-p", &patch, "-o", &output_dir, "-r", &self.args.debug_info])
                                        .stdout(OpenOptions::new().append(true).write(true).open(&self.log_file)?)
                                        .stderr(OpenOptions::new().append(true).write(true).open(&self.log_file)?)
                                        .output()?;
                            if !output.status.success(){
                                return Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: please look {} for detail.", output.status, &self.log_file)));
                            }
                        },
                        None => { fs::copy(&source_name, output_dir)?; },
                    };
                },
                None => {},
            };
        }
        Ok(())
    }

    fn search_tool(&mut self) -> io::Result<()> {
        let arr = WalkDir::new("../").into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file() && (e.path().file_name() == Some(OsStr::new(SUPPORT_TOOL))))
                    .collect::<Vec<_>>();
        match arr.len() {
            0 => {        
                let mut path_str = String::from_utf8(Command::new("which").arg(SUPPORT_TOOL).output()?.stdout).unwrap();
                path_str.pop();
                if path_str.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::NotFound, format!("can't find supporting tools: {}", SUPPORT_TOOL)));
                }
                self.tool_file.push_str(&path_str);
            },
            1 => self.tool_file.push_str(arr[0].path().to_str().unwrap_or_default()),
            _ => return Err(io::Error::new(io::ErrorKind::NotFound, format!("../ have too many {}", SUPPORT_TOOL))),
        };
        Ok(())
    }
}