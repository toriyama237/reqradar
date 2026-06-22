use anyhow::Result;
use clap::Args;

use super::not_yet_implemented;

#[derive(Debug, Args)]
pub struct ReplayArgs {
    /// Identifier of the captured request to replay.
    pub request_id: String,

    /// Override the target host (e.g. to replay against staging).
    #[arg(long)]
    pub target: Option<String>,
}

pub fn run(_args: ReplayArgs) -> Result<()> {
    not_yet_implemented("replay")
}
