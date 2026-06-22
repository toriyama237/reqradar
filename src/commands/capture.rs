use anyhow::Result;
use clap::Args;

use super::not_yet_implemented;

#[derive(Debug, Default, Args)]
pub struct CaptureArgs {
    /// Launch the web dashboard (React/Vite) instead of the terminal TUI.
    #[arg(long)]
    pub web: bool,

    /// Address to listen on for the capturing proxy.
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: String,

    /// Port for the web dashboard (only with --web).
    #[arg(long, default_value_t = 7777)]
    pub web_port: u16,
}

pub fn run(args: CaptureArgs) -> Result<()> {
    if args.web {
        not_yet_implemented("capture --web (web dashboard)")
    } else {
        not_yet_implemented("capture (terminal TUI)")
    }
}
