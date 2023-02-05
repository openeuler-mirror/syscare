use clap::Parser;
use which::which;

use crate::tool::*;

#[derive(Parser, Debug)]
#[command(bin_name="upatch-build", version, term_width = 200)]
pub struct Arguments {
    /// Specify work directory
    /// will delete the work_dir [default: ~/.upatch]
    #[arg(short, long, default_value_t = String::new(), hide_default_value = true, verbatim_doc_comment)]
    pub work_dir: String,

    /// Specify source directory
    /// will modify the debug_source
    #[arg(short = 's', long, verbatim_doc_comment)]
    pub debug_source: String,

    /// Specify build source command
    #[arg(short, long)]
    pub build_source_cmd: String,

    /// Specify build patched command [default: <BUILD_SOURCE_COMMAND>]
    #[arg(long, default_value_t = String::new(), hide_default_value = true)]
    pub build_patch_cmd: String,

    /// Specify debug info
    #[arg(short = 'i', long)]
    pub debug_info: String,

    /// Specify compiler [default: gcc]
    #[arg(short, long, default_value_t = String::new(), hide_default_value = true)]
    pub compiler: String,

    /// Specify running file name
    #[arg(short, long)]
    pub elf_name: String,

    /// Specify output directory [default: <WORK_DIR>]
    #[arg(short, long, default_value_t = String::new(), hide_default_value = true)]
    pub output_dir: String,

    /// Specify output name [default: <ELF_NAME>]
    #[arg(short = 'n', long = "name", default_value_t = String::new(), hide_default_value = true)]
    pub patch_name: String,

    /// Skip compiler version check (not recommended)
    #[arg(long, default_value = "false")]
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Patch file(s)
    #[arg(required = true)]
    pub patches: Vec<String>
}

impl Arguments {
    pub fn new() -> Self {
        Arguments::parse()
    }

    pub fn check(&mut self) -> std::io::Result<()> {
        #![allow(deprecated)]
        if self.work_dir.is_empty() {
            match std::env::home_dir() {
                Some(work_dir) => self.work_dir = format!("{}/{}", work_dir.display(), ".upatch"),
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("home_dir don't support BSD system"),
                )),
            };
        }

        if self.compiler.is_empty() {
            match which("gcc") {
                Ok(compiler) => self.compiler = stringtify(compiler),
                Err(e) => return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("can't find gcc in system: {}", e),
                )),
            };
        }

        self.debug_source = stringtify(real_arg(&self.debug_source)?);
        self.debug_info = stringtify(real_arg(&self.debug_info)?);
        self.compiler = stringtify(real_arg(&self.compiler)?);

        for patch in &mut self.patches {
            *patch = stringtify(real_arg(patch.as_str())?);
        }

        if self.build_patch_cmd.is_empty() {
            self.build_patch_cmd = self.build_source_cmd.clone();
        }

        if self.patch_name.is_empty() {
             self.patch_name = self.elf_name.clone();
        }
        if self.output_dir.is_empty() {
            self.output_dir = self.work_dir.clone();
        }

        Ok(())
    }
}