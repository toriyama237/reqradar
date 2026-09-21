use anyhow::Result;
use clap::Parser;

use reqradar::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.init_tracing();
    cli.run()
}
