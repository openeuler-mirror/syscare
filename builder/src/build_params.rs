use syscare_abi::{PackageInfo, PatchFile, PatchType};

pub struct BuildParameters {
    pub source_pkg: PackageInfo,
    pub debuginfo_pkgs: Vec<PackageInfo>,
    pub patch_uuid: String,
    pub patch_name: String,
    pub patch_version: String,
    pub patch_release: u32,
    pub patch_arch: String,
    pub patch_type: PatchType,
    pub patch_description: String,
    pub patch_files: Vec<PatchFile>,
}

impl std::fmt::Display for BuildParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "------------------------------")?;
        writeln!(f, "Source package")?;
        writeln!(f, "------------------------------")?;
        writeln!(f, "{}", self.source_pkg)?;
        writeln!(f, "------------------------------")?;
        for debuginfo_pkg in &self.debuginfo_pkgs {
            writeln!(f, "Debuginfo package")?;
            writeln!(f, "------------------------------")?;
            writeln!(f, "{}", debuginfo_pkg)?;
            writeln!(f, "------------------------------")?;
        }
        writeln!(f, "Syscare Patch")?;
        writeln!(f, "------------------------------")?;
        writeln!(f, "uuid:        {}", self.patch_uuid)?;
        writeln!(f, "name:        {}", self.patch_name)?;
        writeln!(f, "version:     {}", self.patch_version)?;
        writeln!(f, "release:     {}", self.patch_release)?;
        writeln!(f, "arch:        {}", self.patch_arch)?;
        writeln!(f, "type:        {}", self.patch_type)?;
        writeln!(f, "description: {}", self.patch_description)?;
        write!(f, "------------------------------")?;

        Ok(())
    }
}
