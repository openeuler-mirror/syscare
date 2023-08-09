use clap::Parser;

use super::{CLI_NAME, CLI_VERSION};

#[derive(Parser, Clone, Debug)]
#[clap(bin_name=CLI_NAME, version=CLI_VERSION)]
pub struct Arguments {
    /// Command name
    pub cmd_name: String,

    /// Command arguments
    pub cmd_args: Vec<String>,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
