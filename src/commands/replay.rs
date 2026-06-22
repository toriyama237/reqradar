use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use crate::capture::{self, replay};

#[derive(Debug, Args)]
pub struct ReplayArgs {
    /// Identifier of the captured request to replay.
    pub request_id: String,

    /// Capture file to read from. Defaults to the newest .rrlog under ./captures.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Override the target host (e.g. to replay against staging).
    #[arg(long)]
    pub target: Option<String>,
}

pub fn run(args: ReplayArgs) -> Result<()> {
    let file = match args.file {
        Some(f) => f,
        None => capture::latest_capture_in("captures").context(
            "no --file given and no .rrlog found under ./captures; run `reqradar capture` first",
        )?,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting async runtime")?;

    let outcome = runtime.block_on(replay::run(&file, &args.request_id, args.target.as_deref()))?;

    let original_status = outcome
        .original
        .response
        .as_ref()
        .map(|r| r.status.to_string())
        .unwrap_or_else(|| "n/a".to_string());

    println!("Replayed {}", outcome.original.id);
    println!(
        "  {} {}",
        outcome.original.request.method, outcome.original.request.uri
    );
    println!(
        "  status   {} -> {} (original -> replay)",
        original_status, outcome.status
    );
    println!(
        "  latency  {}ms -> {}ms",
        outcome.original.latency_ms, outcome.latency_ms
    );

    Ok(())
}
