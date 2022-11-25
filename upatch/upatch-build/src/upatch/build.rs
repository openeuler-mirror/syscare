use std::{path::Path, env};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::string::String;

use crate::tool::*;
use crate::dwarf::{Dwarf, DwarfCompileUnit};

use super::Arg;
use super::Compiler;
use super::Project;
use super::Result;
use super::Error;
use super::ExternCommand;
use super::{set_log_file, set_verbose, verbose};

pub const UPATCH_DEV_NAME: &str = "upatch";
const SYSTEM_MOUDLES: &str = "/proc/modules";
const CMD_SOURCE_ENTER: &str = "SE";
const CMD_PATCHED_ENTER: &str = "PE";
const SUPPORT_DIFF: &str = "upatch-diff";
const SUPPORT_TOOL: &str = "upatch-tool";

pub struct UpatchBuild {
    cache_dir: String,
    source_dir: String,
    patch_dir: String,
    output_dir: String,
    log_file: String,
    args: Arg,
    diff_file: String,
    tool_file: String,
}

impl UpatchBuild {
    pub fn new() -> Self {
        Self {
            cache_dir: String::new(),
            source_dir: String::new(),
            patch_dir: String::new(),
            output_dir: String::new(),
            log_file: String::new(),
            args: Arg::new(),
            diff_file: String::new(),
            tool_file: String::new(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.args.read()?;

        // create .upatch directory
        self.create_dir()?;
        set_log_file(&self.log_file)?;
        set_verbose(self.args.verbose)?;

        if self.args.patch_name.is_empty() {
            self.args.patch_name.push_str(&self.args.elf_name);
        }
        if self.args.output.is_empty() {
            self.args.output.push_str(&self.cache_dir);
        }

        // check mod
        self.check_mod()?;

        // find upatch-diff and upatch-tool
        self.diff_file = search_tool(SUPPORT_DIFF)?;
        self.tool_file = search_tool(SUPPORT_TOOL)?;

        // check compiler
        let mut compiler = Compiler::new(self.args.compiler_file.clone());
        compiler.analyze()?;
        if !self.args.skip_compiler_check {
            compiler.check_version(&self.cache_dir, &self.args.debug_info)?;
        }

        // copy source
        let project_name = &Path::new(&self.args.source).file_name().unwrap();

        // hack compiler
        println!("Hacking compiler");
        compiler.hack()?;

        // build source
        println!("Building original {}", project_name.to_str().unwrap());
        let project = Project::new(self.args.source.clone());
        project.build(CMD_SOURCE_ENTER, &self.source_dir, self.args.build_source_command.clone())?;

        // build patch
        for patch in &self.args.diff_file {
            println!("Patching file: {}", &patch);
            project.patch(patch.clone())?;
        }

        println!("Building patched {}", project_name.to_str().unwrap());
        project.build(CMD_PATCHED_ENTER, &self.patch_dir, self.args.build_patch_command.clone())?;

        // unhack compiler
        println!("Unhacking compiler");
        compiler.unhack()?;

        println!("Detecting changed objects");
        // correlate obj name
        let mut source_obj: HashMap<String, String> = HashMap::new();
        let mut patch_obj: HashMap<String, String> = HashMap::new();
        let dwarf = Dwarf::new();

        self.correlate_obj(&dwarf, self.args.source.as_str(), &self.source_dir, &mut source_obj)?;
        self.correlate_obj(&dwarf, self.args.source.as_str(), &self.patch_dir, &mut patch_obj)?;

        // choose the binary's obj to create upatch file
        let binary_obj = dwarf.file_in_binary(self.args.source.clone(), self.args.elf_name.clone())?;
        self.create_diff(&source_obj, &patch_obj, &binary_obj)?;

        // ld patchs
        let output_file = format!("{}/{}", &self.args.output, &self.args.patch_name);
        compiler.linker(&self.output_dir, &output_file)?;
        self.upatch_tool(&output_file)?;
        println!("Building patch: {}", &output_file);
        Ok(())
    }
}

impl UpatchBuild {
    fn create_dir(&mut self) -> Result<()> {
        #![allow(deprecated)]
        if self.args.work_dir.is_empty(){
            // home_dir() don't support BSD system
            self.cache_dir.push_str(&format!("{}/{}", env::home_dir().unwrap().to_str().unwrap(), ".upatch"));
        }
        else{
            self.cache_dir.push_str(&self.args.work_dir);
        }

        self.source_dir.push_str(&format!("{}/{}", &self.cache_dir, "source"));
        self.patch_dir.push_str(&format!("{}/{}", &self.cache_dir, "patch"));
        self.output_dir.push_str(&format!("{}/{}", &self.cache_dir, "output"));
        self.log_file.push_str(&format!("{}/{}", &self.cache_dir, "build.log"));

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

    fn check_mod(&self) -> Result<()> {
        let mut file = File::open(SYSTEM_MOUDLES)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        match contents.find(UPATCH_DEV_NAME) {
            Some(_) => Ok(()),
            None => Err(Error::Mod(format!("can't found upatch mod in system"))),
        }
    }

    fn correlate_obj(&self, dwarf: &Dwarf, comp_dir: &str, dir: &str, map: &mut HashMap<String, String>) -> Result<()> {
        let arr = list_all_files_ext(dir, "o", false)?;
        for obj in arr {       
            let name = stringtify(obj);
            let result = dwarf.file_in_obj(name.clone())?;
            if result.len() == 1 && result[0].DW_AT_comp_dir.find(comp_dir) != None{
                map.insert(result[0].get_source(), name.clone());
            }
        }
        Ok(())
    }

    fn create_diff(&self, source_obj: &HashMap<String, String>, patch_obj: &HashMap<String, String>, binary_obj: &Vec<DwarfCompileUnit>) -> Result<()> {
        for path in binary_obj {
            let source_name = path.get_source();
            match &patch_obj.get(&source_name) {
                Some(patch) => {
                    let output_dir = format!("{}/{}", &self.output_dir, file_name(patch)?);
                    match &source_obj.get(&source_name) {
                        Some(source) => self.upatch_diff(source, patch, &output_dir)?,
                        None => { fs::copy(&patch, output_dir)?; },
                    };
                },
                None => {},
            };
        }
        Ok(())
    }

    fn upatch_diff(&self, source: &str, patch: &str, output_dir: &str) -> Result<()> {
        let mut args_list = vec!["-s", &source, "-p", &patch, "-o", &output_dir, "-r", &self.args.debug_info];
        if self.args.verbose {
            args_list.push("-d");
        }
        let output = ExternCommand::new(&self.diff_file).execvp(args_list)?;
        match output.exit_status().success() {
            true => verbose(output.stdout()),
            false => return Err(Error::Diff(format!("{}: please look {} for detail.", output.exit_code(), &self.log_file)))
        };
        Ok(())
    }

    fn upatch_tool(&self, patch: &str) -> Result<()> {
        let args_list = vec!["resolve", "-b", &self.args.debug_info, "-p", patch];
        let output = ExternCommand::new(&self.tool_file).execvp(args_list)?;
        match output.exit_status().success() {
            true => verbose(output.stdout()),
            false => return Err(Error::TOOL(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }
}
