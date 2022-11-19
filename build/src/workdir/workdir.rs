use crate::util::fs;

use super::patch_root::PatchRoot;
use super::package_root::PackageRoot;

pub trait ManageWorkDir {
    fn create_all(&self) -> std::io::Result<()>;
    fn remove_all(&self) -> std::io::Result<()>;
}

pub struct WorkDir {
    base:         String,
    patch_root:   PatchRoot,
    package_root: PackageRoot,
}

impl WorkDir {
    pub fn new(base_dir: String) -> Self {
        let base         = base_dir.to_owned();
        let patch_root   = PatchRoot::new(format!("{}/patch", base_dir));
        let package_root = PackageRoot::new(format!("{}/package",  base_dir));

        Self { base, patch_root, package_root }
    }

    fn base_dir(&self) -> &str {
        &self.base
    }

    pub fn patch_root(&self) -> &PatchRoot {
        &self.patch_root
    }

    pub fn package_root(&self) -> &PackageRoot {
        &self.package_root
    }
}

impl ManageWorkDir for WorkDir {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(self.base_dir())?;
        self.patch_root.create_all()?;
        self.package_root.create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.package_root.remove_all()?;
        self.patch_root.remove_all()?;
        std::fs::remove_dir_all(self.base_dir())?;

        Ok(())
    }
}

impl std::fmt::Display for WorkDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.base_dir())
    }
}
