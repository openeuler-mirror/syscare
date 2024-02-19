use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use lazy_static::lazy_static;
use syscare_common::util::ext_cmd::{ExternCommand, ExternCommandArgs};

lazy_static! {
    static ref TAR: ExternCommand = ExternCommand::new("tar");
}

pub struct TarPackage {
    path: PathBuf,
}

impl TarPackage {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn compress<P, S>(&self, root_dir: P, target: S) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-czf")
                .arg(self.path.as_path())
                .arg("-C")
                .arg(root_dir.as_ref())
                .arg(target)
                .arg("--restrict"),
        )?
        .check_exit_code()?;

        Ok(())
    }

    pub fn decompress<P>(&self, output_dir: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        if !self.path.is_file() {
            bail!("File {} is not exist", self.path.display());
        }

        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-xf")
                .arg(self.path.as_path())
                .arg("-C")
                .arg(output_dir.as_ref())
                .arg("--no-same-owner")
                .arg("--no-same-permissions")
                .arg("--restrict"),
        )?
        .check_exit_code()?;

        Ok(())
    }
}
