use anyhow::Result;
use clap::{Args, Subcommand};

use super::not_yet_implemented;

#[derive(Debug, Args)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub command: RulesCommand,
}

#[derive(Debug, Subcommand)]
pub enum RulesCommand {
    /// List the loaded detection rules.
    List,

    /// Validate a YAML rules file.
    Check {
        /// Path to the YAML rules file.
        path: String,
    },
}

pub fn run(args: RulesArgs) -> Result<()> {
    match args.command {
        RulesCommand::List => not_yet_implemented("rules list"),
        RulesCommand::Check { .. } => not_yet_implemented("rules check"),
    }
}
