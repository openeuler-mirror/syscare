use std::path::PathBuf;

use syscare_abi::{PackageInfo, PatchInfo};

use crate::package::{ElfRelation, PackageBuildRoot};

pub struct BuildParameters {
    pub patch: PatchInfo,
    pub build_root: PackageBuildRoot,
    pub source_dir: PathBuf,
    pub spec_file: PathBuf,
    pub debuginfo_pkgs: Vec<PackageInfo>,
    pub elf_relations: Vec<ElfRelation>,
    pub jobs: usize,
    pub skip_compiler_check: bool,
    pub skip_cleanup: bool,
    pub verbose: bool,
}

impl std::fmt::Display for BuildParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "------------------------------")?;
        writeln!(f, "Build Parameters")?;
        writeln!(f, "------------------------------")?;
        writeln!(f, "patch_name:          {}", self.patch.name)?;
        writeln!(f, "patch_version:       {}", self.patch.version)?;
        writeln!(f, "patch_release:       {}", self.patch.release)?;
        writeln!(f, "build_root:          {}", self.build_root.display())?;
        writeln!(f, "source_dir:          {}", self.source_dir.display())?;
        writeln!(f, "spec_file:           {}", self.spec_file.display())?;
        writeln!(f, "jobs:                {}", self.jobs)?;
        writeln!(f, "skip_compiler_check: {}", self.skip_compiler_check)?;
        writeln!(f, "skip_cleanup:        {}", self.skip_cleanup)?;
        writeln!(f, "verbose:             {}", self.verbose)?;
        write!(f, "------------------------------")?;

        Ok(())
    }
}
