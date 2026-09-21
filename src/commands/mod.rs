pub mod capture;
pub mod diff;
pub mod inspect;
pub mod replay;
pub mod report;
pub mod rules;

use anyhow::{bail, Result};

/// Features that are on the CLI surface so the command names stay stable, but
/// are not implemented in this version.
pub(crate) fn not_yet_implemented(feature: &str) -> Result<()> {
    bail!(
        "`{feature}` is not implemented in 0.1.0-dev.\n\
         Shipped today: `capture`, `replay`, `report` (Markdown), `inspect`.\n\
         See the roadmap: https://github.com/toriyama237/reqradar#roadmap"
    );
}
