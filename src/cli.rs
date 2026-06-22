use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands;

/// ReqRadar — capture, inspect, replay and diff HTTP requests from your terminal.
#[derive(Debug, Parser)]
#[command(name = "reqradar", version, about, long_about = None)]
pub struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start capturing HTTP traffic (TUI by default).
    Capture(commands::capture::CaptureArgs),

    /// Replay a previously captured request.
    Replay(commands::replay::ReplayArgs),

    /// Diff two captures (e.g. before/after a deploy).
    Diff(commands::diff::DiffArgs),

    /// Export a ready-to-paste bug report (Markdown/PDF).
    Report(commands::report::ReportArgs),

    /// Manage custom detection rules (YAML).
    Rules(commands::rules::RulesArgs),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            // No subcommand: default to interactive capture.
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
}
