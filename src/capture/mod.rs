//! Capture engine: the transparent proxy, the data model, persistence, and the
//! replay consumer that validates the format.

pub mod client;
pub mod proxy;
pub mod record;
pub mod replay;
pub mod store;

use std::path::PathBuf;

/// Pick the most recently modified `.rrlog` file in `dir`, if any.
pub fn latest_capture_in(dir: impl AsRef<std::path::Path>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rrlog") {
            continue;
        }
        let modified = entry.metadata().and_then(|m| m.modified()).ok();
        if let Some(modified) = modified {
            match &newest {
                Some((t, _)) if *t >= modified => {}
                _ => newest = Some((modified, path)),
            }
        }
    }
    newest.map(|(_, p)| p)
}
