use anyhow::Result;
use clap::{Args, ValueEnum};

use super::not_yet_implemented;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReportFormat {
    Markdown,
    Pdf,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Identifier of the captured request to turn into a bug report.
    pub request_id: String,

    /// Output format.
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub format: ReportFormat,

    /// Write the report to a file instead of stdout.
    #[arg(short, long)]
    pub output: Option<String>,
}

pub fn run(_args: ReportArgs) -> Result<()> {
    not_yet_implemented("report")
}
