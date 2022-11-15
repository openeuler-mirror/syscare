use std::collections::HashMap;

use crate::statics::*;

use super::{PatchBuilder, PatchBuilderOptions};

pub struct KernelPatchBuilder {
    build_root: String
}

impl KernelPatchBuilder {
    pub fn new(build_root: &str) -> Self {
        Self { build_root: build_root.to_owned() }
    }

    #[inline(always)]
    fn map_args(&self, options: &PatchBuilderOptions) -> Vec<String> {
        let mut result = Vec::new();

        let patch_info = &options.patch_info;

        result.push(String::from("--name"));
        result.push(patch_info.get_patch_name().get_name().to_owned());

        result.push(String::from("--sourcedir"));
        result.push(options.source_dir.to_owned());

        result.push(String::from("--output"));
        result.push(options.output_dir.to_owned());

        result.push(String::from("--vmlinux"));
        result.push(options.debug_info.to_owned());

        if let Some(config) = &options.kconfig {
            result.push(String::from("--config"));
            result.push(config.to_owned());
        }

        if let Some(jobs) = &options.kjobs {
            result.push(String::from("--jobs"));
            result.push(jobs.to_string());
        }

        // if let Some(target) = &options.ktarget {
        //     result.push(String::from("--target"));
        //     for tgt in target {
        //         result.push(tgt.to_owned());
        //     }
        // }

        if options.skip_compiler_check {
            result.push(String::from("--skip-compiler-check"));
        }

        for patch_file in patch_info.get_file_list() {
            result.push(patch_file.get_path().to_owned())
        }

        result
    }

    #[inline(always)]
    fn map_envs(&self, _options: &PatchBuilderOptions) -> HashMap<&str, String> {
        let mut result = HashMap::new();
        // if let Some(kmod_dir) = &options.kmod_dir {
        //     result.insert("USERMODBUILDDIR", kmod_dir.to_owned());
        // }
        // if let Some(kmod_flag) = &options.kmod_flag {
        //     result.insert("USERMODFLAGS",    kmod_flag.to_owned());
        // }
        // addtional envs for kpatch
        result.insert("CACHEDIR",           self.build_root.to_owned());
        result.insert("NO_PROFILING_CALLS", String::from("yes"));
        result.insert("DISABLE_AFTER_LOAD", String::from("yes"));
        result.insert("KEEP_JUMP_LABEL",    String::from("yes"));

        result
    }
}

impl PatchBuilder for KernelPatchBuilder {
    fn build_patch(&self, options: PatchBuilderOptions) -> std::io::Result<()> {
        let exit_status = KPATCH_BUILD.execve(self.map_args(&options), self.map_envs(&options))?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", KPATCH_BUILD, exit_code),
            ));
        }

        Ok(())
    }
}