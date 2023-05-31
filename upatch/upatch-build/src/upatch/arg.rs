use std::path::PathBuf;
use std::ffi::OsString;

use clap::Parser;
use which::which;

use crate::tool::*;

#[derive(Parser, Debug)]
#[command(bin_name="upatch-build", version, term_width = 200)]
pub struct Arguments {
    /// Specify work directory
    /// will add upatch in work_dir [default: ~/.upatch]
    #[arg(short, long, default_value = None, verbatim_doc_comment)]
    pub work_dir: Option<PathBuf>,

    /// Specify source directory
    /// will modify the debug_source
    #[arg(short = 's', long, verbatim_doc_comment)]
    pub debug_source: PathBuf,

    /// Specify build source command
    #[arg(short, long)]
    pub build_source_cmd: String,

    /// Specify build patched command [default: <BUILD_SOURCE_COMMAND>]
    #[arg(long, default_value_t = String::new(), hide_default_value = true)]
    pub build_patch_cmd: String,

    /// Specify debug info list
    #[arg(short = 'i', long = "debug-info", required = true)]
    pub debug_infoes: Vec<PathBuf>,

    /// Specify elf's relative path relate to elf-dir or absolute path list.
    /// one-to-one correspondence with debug-info
    #[arg(short, long = "elf-path", required = true, verbatim_doc_comment)]
    pub elf_pathes: Vec<PathBuf>,

    /// Specify the directory of searching elf [default: <DEBUG_SOURCE>]
    #[arg(long, default_value = None, required = false)]
    pub elf_dir: Option<PathBuf>,

    /// Specify compiler [default: gcc]
    #[arg(short, long, default_value = None)]
    pub compiler: Option<Vec<PathBuf>>,

    /// Specify output directory [default: <WORK_DIR>]
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Specify output name
    #[arg(short, long, default_value = "", hide_default_value = true)]
    pub name: OsString,

    /// Skip compiler version check (not recommended)
    #[arg(long, default_value = "false")]
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Patch file(s)
    #[arg(required = true)]
    pub patches: Vec<PathBuf>
}

impl Arguments {
    pub fn new() -> Self {
        Arguments::parse()
    }
}

impl Arguments {
    pub fn check(&mut self) -> std::io::Result<()> {
        self.work_dir = Some(match &self.work_dir {
            Some(work_dir) => real_arg(work_dir)?.join("upatch"),
            #[allow(deprecated)]
            None => match std::env::home_dir() {
                Some(work_dir) => work_dir.join(".upatch"),
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("home_dir don't support BSD system"),
                )),
            },
        });

        let mut default_compiler = vec![PathBuf::from("gcc"), PathBuf::from("g++")];
        let compiler_paths = self.compiler.as_deref_mut().unwrap_or(&mut default_compiler);
        self.compiler = Some({
            for compiler_path in &mut compiler_paths.iter_mut() {
                *compiler_path = match compiler_path.exists() {
                    true => real_arg(&compiler_path)?,
                    false => which(&compiler_path).map_err(|e| std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("can't find {:?} in system: {}", compiler_path, e),
                        ))?,
                };
            }
            compiler_paths.to_vec()
        });

        self.debug_source = real_arg(&self.debug_source)?;

        for debug_info in &mut self.debug_infoes {
            *debug_info = real_arg(&debug_info)?;
        }

        for patch in &mut self.patches {
            *patch = real_arg(&patch)?;
        }

        if self.build_patch_cmd.is_empty() {
            self.build_patch_cmd = self.build_source_cmd.clone();
        }

        if !self.name.is_empty() {
            self.name.push("-");
        }

        self.elf_dir = match &self.elf_dir {
            Some(elf_dir) => Some(real_arg(elf_dir)?),
            None => Some(self.debug_source.clone()),
        };

        match self.elf_pathes.len().eq(&self.debug_infoes.len()) {
            true => {
                for elf_path in &mut self.elf_pathes {
                    *elf_path = self.elf_dir.as_ref().unwrap().join(&elf_path);
                }
            },
            false => return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{}'s elf-path don't match {}'s debug-info", self.elf_pathes.len(), self.debug_infoes.len()),
            )),
        }

        Ok(())
    }
}