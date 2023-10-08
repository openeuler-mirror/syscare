use std::path::PathBuf;

use syscare_abi::{PackageInfo, PatchFile, PatchType};

use crate::{package::PackageBuildRoot, workdir::WorkDir};

#[derive(Debug, Clone)]
pub struct BuildEntry {
    pub target_pkg: PackageInfo,
    pub build_source: PathBuf,
    pub build_spec: PathBuf,
}

pub struct BuildParameters {
    pub workdir: WorkDir,
    pub pkg_build_root: PackageBuildRoot,
    pub build_entry: BuildEntry,
    pub kernel_build_entry: Option<BuildEntry>,
    pub patch_name: String,
    pub patch_type: PatchType,
    pub patch_version: String,
    pub patch_release: u32,
    pub patch_arch: String,
    pub patch_description: String,
    pub patch_files: Vec<PatchFile>,
    pub jobs: usize,
    pub skip_compiler_check: bool,
    pub skip_cleanup: bool,
    pub verbose: bool,
}

impl std::fmt::Display for BuildParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let e = &self.build_entry;

        writeln!(f, "------------------------------")?;
        writeln!(f, "Build Parameters")?;
        writeln!(f, "------------------------------")?;
        writeln!(f, "patch_name:          {}", self.patch_name)?;
        writeln!(f, "patch_type:          {}", self.patch_type)?;
        writeln!(f, "patch_version:       {}", self.patch_version)?;
        writeln!(f, "patch_release:       {}", self.patch_release)?;
        writeln!(f, "patch_arch:          {}", self.patch_arch)?;
        writeln!(f, "patch_description:   {}", self.patch_description)?;
        writeln!(f, "build_source:        {}", e.build_source.display())?;
        writeln!(f, "build_spec:          {}", e.build_spec.display())?;
        if let Some(k) = &self.kernel_build_entry {
            writeln!(f, "kernel_source:       {}", k.build_source.display())?;
            writeln!(f, "kernel_spec:         {}", k.build_spec.display())?;
        }
        writeln!(f, "jobs:                {}", self.jobs)?;
        writeln!(f, "skip_compiler_check: {}", self.skip_compiler_check)?;
        writeln!(f, "skip_cleanup:        {}", self.skip_cleanup)?;
        writeln!(f, "verbose:             {}", self.verbose)?;
        if !self.patch_files.is_empty() {
            writeln!(f, "patch_files:")?;
            for patch_file in &self.patch_files {
                writeln!(f, "* {}", patch_file.name.to_string_lossy())?;
            }
        }
        write!(f, "------------------------------")?;

        Ok(())
    }
}
