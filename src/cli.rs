use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::commands;

/// ReqRadar — capture, inspect and replay HTTP from your terminal.
#[derive(Debug, Parser)]
#[command(name = "reqradar", version, about, long_about = None)]
pub struct Cli {
    /// Increase logging verbosity on stderr (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Reverse-proxy HTTP traffic to an upstream and write a .rrlog.
    Capture(commands::capture::CaptureArgs),

    /// Replay a previously captured request.
    Replay(commands::replay::ReplayArgs),

    /// Diff two captures (not implemented yet).
    Diff(commands::diff::DiffArgs),

    /// Export a Markdown bug report from a captured request.
    Report(commands::report::ReportArgs),

    /// Manage custom detection rules (not implemented yet).
    Rules(commands::rules::RulesArgs),
}

impl Cli {
    pub fn init_tracing(&self) {
        let default = match self.verbose {
            0 => "reqradar=warn",
            1 => "reqradar=info",
            2 => "reqradar=debug",
            _ => "reqradar=trace",
        };
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init();
    }

    pub fn run(self) -> Result<()> {
        match self.command {
            None => commands::capture::run(commands::capture::CaptureArgs::default()),
            Some(Command::Capture(args)) => commands::capture::run(args),
            Some(Command::Replay(args)) => commands::replay::run(args),
            Some(Command::Diff(args)) => commands::diff::run(args),
            Some(Command::Report(args)) => commands::report::run(args),
            Some(Command::Rules(args)) => commands::rules::run(args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_capture_web_flag() {
        let cli = Cli::parse_from(["reqradar", "capture", "--web"]);
        assert!(matches!(cli.command, Some(Command::Capture(_))));
    }

    #[test]
    fn capture_help_does_not_claim_a_tui() {
        let mut buf = Vec::new();
        Cli::command()
            .write_help(&mut buf)
            .expect("help is writable");
        let help = String::from_utf8(buf).expect("help is utf-8");
        assert!(
            !help.to_ascii_lowercase().contains("tui"),
            "help still mentions a TUI:\n{help}"
        );
    }
}
