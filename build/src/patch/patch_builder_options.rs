use crate::cli::CliArguments;

use super::PatchInfo;

pub struct PatchBuilderOptions {
    pub patch_info: PatchInfo,
    pub source_dir: String,
    pub debug_info: String,
    pub output_dir: String,
    pub kconfig: Option<String>,
    pub kjobs: usize,
    // pub ktarget: Option<Vec<String>>,
    // pub kmod_dir: Option<String>,
    // pub kmod_flag: Option<String>,
    pub build_entry: Option<String>,
    pub skip_compiler_check: bool,
}

impl PatchBuilderOptions {
    pub fn new(patch_info: &PatchInfo, args: &CliArguments, output_dir: &str) -> std::io::Result<Self> {
        Ok(Self {
            patch_info:          patch_info.to_owned(),
            source_dir:          args.source.to_string(),
            debug_info:          args.debug_info.as_ref().unwrap().to_string(),
            output_dir:          output_dir.to_owned(),
            kconfig:             args.kconfig.to_owned(),
            kjobs:               args.kjobs,
            // ktarget:             args.ktarget.to_owned(),
            // kmod_dir:            args.kmod_dir.to_owned(),
            // kmod_flag:           args.kmod_flag.to_owned(),
            build_entry:         args.build_entry.to_owned(),
            skip_compiler_check: args.skip_compiler_check,
        })
    }
}
