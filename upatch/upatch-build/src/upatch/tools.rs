use std::path::{PathBuf, Path};

use crate::tool::*;
use crate::cmd::*;

use super::Result;
use super::Error;

const SUPPORT_DIFF: &str = "upatch-diff";
const SUPPORT_TOOL: &str = "upatch-tool";
const SUPPORT_NOTES: &str = "upatch-notes";
pub struct Tool {
    diff: PathBuf,
    tool: PathBuf,
    notes: PathBuf,
}

impl Tool {
    pub fn new() -> Self {
        Self {
            diff: PathBuf::new(),
            tool: PathBuf::new(),
            notes: PathBuf::new(),
        }
    }

    pub fn check(&mut self) -> std::io::Result<()> {
        self.diff = search_tool(SUPPORT_DIFF)?;
        self.tool = search_tool(SUPPORT_TOOL)?;
        self.notes = search_tool(SUPPORT_NOTES)?;
        Ok(())
    }

    pub fn upatch_diff<P, Q, O, D, L>(&self, source: P, patch: Q, output: O, debug_info: D, log_file: L, verbose: bool) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        O: AsRef<Path>,
        D: AsRef<Path>,
        L: AsRef<Path>,
    {
        let mut args_list = ExternCommandArgs::new()
            .arg("-s").arg(source.as_ref())
            .arg("-p").arg(patch.as_ref())
            .arg("-o").arg(output.as_ref())
            .arg("-r").arg(debug_info.as_ref());
        if verbose {
            args_list = args_list.arg("-d");
        }
        let output = ExternCommand::new(&self.diff).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Diff(format!("{}: please look {} for detail.", output.exit_code(), log_file.as_ref().display())))
        };
        Ok(())
    }

    pub fn upatch_tool<P, Q>(&self, patch: P, debug_info: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new().args(["resolve", "-b"]).arg(debug_info.as_ref()).arg("-p").arg(patch.as_ref());
        let output = ExternCommand::new(&self.tool).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::TOOL(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }

    pub fn upatch_notes<P, Q>(&self, notes: P, debug_info: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let args_list = ExternCommandArgs::new().arg("-r").arg(debug_info.as_ref()).arg("-o").arg(notes.as_ref());
        let output = ExternCommand::new(&self.notes).execvp(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::NOTES(format!("{}: {}", output.exit_code(), output.stderr())))
        };
        Ok(())
    }
}