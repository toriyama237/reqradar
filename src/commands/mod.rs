pub mod capture;
pub mod diff;
pub mod replay;
pub mod report;
pub mod rules;

use anyhow::{bail, Result};

/// Phase-zero placeholder: every subcommand is scaffolded but not yet wired up.
///
/// This keeps the CLI surface stable while the underlying engine is built out
/// phase by phase (see the roadmap in README.md).
pub(crate) fn not_yet_implemented(feature: &str) -> Result<()> {
    bail!(
        "`{feature}` is not implemented yet.\n\
         ReqRadar is in phase zero (project bootstrap). Follow the roadmap in the README \
         to see when this lands: https://github.com/toriyama237/reqradar#roadmap"
    );
}
