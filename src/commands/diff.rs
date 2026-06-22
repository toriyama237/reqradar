use anyhow::Result;
use clap::Args;

use super::not_yet_implemented;

#[derive(Debug, Args)]
pub struct DiffArgs {
    /// First capture (the "before").
    pub before: String,

    /// Second capture (the "after").
    pub after: String,
}

pub fn run(_args: DiffArgs) -> Result<()> {
    not_yet_implemented("diff")
}
