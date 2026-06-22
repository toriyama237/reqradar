//! `.rrlog` storage: one JSON object per line (JSON Lines).
//!
//! Phase-zero decision: keep persistence dead simple and append-only. JSON Lines
//! is greppable, diff-friendly and trivial to stream. A richer embedded store
//! (e.g. sled) can come later behind this same API.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::record::Exchange;

/// Append-only writer for a `.rrlog` file.
pub struct Store {
    path: PathBuf,
    file: File,
}

impl Store {
    /// Open (creating parent dirs and the file if needed) for appending.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory {}", parent.display()))?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening {} for writing", path.display()))?;
        Ok(Self { path, file })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one exchange as a single JSON line and flush.
    pub fn append(&mut self, exchange: &Exchange) -> Result<()> {
        let line = serde_json::to_string(exchange).context("serializing exchange")?;
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}

/// Read every exchange from a `.rrlog` file.
pub fn read_all(path: impl AsRef<Path>) -> Result<Vec<Exchange>> {
    let path = path.as_ref();
    let file =
        File::open(path).with_context(|| format!("opening capture file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let exchange: Exchange = serde_json::from_str(&line)
            .with_context(|| format!("parsing {} line {}", path.display(), i + 1))?;
        out.push(exchange);
    }
    Ok(out)
}

/// Find a single exchange by id.
pub fn find(path: impl AsRef<Path>, id: &str) -> Result<Option<Exchange>> {
    Ok(read_all(path)?.into_iter().find(|e| e.id == id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::record::{Body, RequestRecord};
    use chrono::Utc;

    fn sample(id: &str) -> Exchange {
        Exchange {
            id: id.to_string(),
            started_at: Utc::now(),
            upstream: "http://localhost:3000".to_string(),
            latency_ms: 12,
            request: RequestRecord {
                method: "GET".to_string(),
                uri: "/health".to_string(),
                route: "/health".to_string(),
                version: "HTTP/1.1".to_string(),
                headers: vec![],
                body: Body::Empty,
            },
            response: None,
            error: None,
        }
    }

    #[test]
    fn append_then_read_roundtrips() {
        let dir = std::env::temp_dir().join(format!("rrlog-test-{}", std::process::id()));
        let path = dir.join("session.rrlog");
        let _ = fs::remove_dir_all(&dir);

        let mut store = Store::create(&path).unwrap();
        store.append(&sample("1-000001")).unwrap();
        store.append(&sample("1-000002")).unwrap();

        let all = read_all(&path).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "1-000001");

        let found = find(&path, "1-000002").unwrap();
        assert!(found.is_some());
        assert!(find(&path, "nope").unwrap().is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
