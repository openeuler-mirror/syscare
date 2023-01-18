use log::info;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;

        let mut output = format!("{:<35} {:<35} {:<12}", "PackageName", "PatchName", "PatchStatus");
        for patch in patch_manager.get_patch_list() {
            output.push_str(&format!("\n{:<35} {:<35} {:<12}",
                patch.get_target(),
                patch.get_simple_name(),
                patch.get_status()
            ));
        }

        info!("{}", output);
        Ok(0)
    }
}
