use clap::Subcommand;

#[derive(Debug)]
#[derive(Subcommand)]
pub enum Command {
    /// Build a new patch
    #[command(
        disable_help_flag(true),
        subcommand_precedence_over_arg(true),
        allow_hyphen_values(true)
    )]
    Build {
        args: Vec<String>
    },
    /// Show patch detail info
    Info {
        patch_name: String
    },
    /// Show patch target info
    Target {
        patch_name: String
    },
    /// Show patch status
    Status {
        patch_name: String
    },
    /// List all installed patches
    List,
    /// Apply a patch
    Apply {
        patch_name: String
    },
    /// Remove a patch
    Remove {
        patch_name: String
    },
    /// Activate a patch
    Active {
        patch_name: String
    },
    /// Deactive a patch
    Deactive {
        patch_name: String
    },
    /// Restore all patch status from record
    Restore,
}

pub trait CommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32>;
}
