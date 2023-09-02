use std::{ffi::OsStr, path::Path};

use anyhow::Result;

use syscare_common::util::ext_cmd::{ExternCommand, ExternCommandArgs};

const TAR: ExternCommand = ExternCommand::new("tar");

pub struct TarPackage;

impl TarPackage {
    pub fn compress<P, Q, S>(tar_file: P, root_dir: Q, target: S) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-czf")
                .arg(tar_file.as_ref())
                .arg("-C")
                .arg(root_dir.as_ref())
                .arg(target)
                .arg("--restrict"),
        )?
        .check_exit_code()?;

        Ok(())
    }

    pub fn decompress<P, Q>(tar_file: P, output_dir: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-xf")
                .arg(tar_file.as_ref())
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
