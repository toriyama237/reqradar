use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use crate::capture::{self, store};
use crate::detect;

#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Capture file to read from. Defaults to the newest .rrlog under ./captures.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Flag exchanges whose latency is at least this many milliseconds.
    #[arg(long, default_value_t = detect::DEFAULT_SLOW_MS)]
    pub slow_ms: u64,

    /// Exit with status 1 when there is at least one finding.
    #[arg(long)]
    pub fail: bool,
}

pub fn run(args: InspectArgs) -> Result<()> {
    let file = match args.file {
        Some(f) => f,
        None => capture::latest_capture_in("captures").context(
            "no --file given and no .rrlog found under ./captures; run `reqradar capture` first",
        )?,
    };

    let exchanges =
        store::read_all(&file).with_context(|| format!("reading {}", file.display()))?;
    let cfg = detect::Config {
        slow_ms: args.slow_ms,
    };
    let findings = detect::scan_all(&exchanges, &cfg);

    println!("{}", format_report(&file, exchanges.len(), &cfg, &findings));

    if args.fail && !findings.is_empty() {
        anyhow::bail!("{} finding(s)", findings.len());
    }
    Ok(())
}

fn format_report(
    file: &std::path::Path,
    n_exchanges: usize,
    cfg: &detect::Config,
    findings: &[detect::Finding],
) -> String {
    let mut out = String::new();
    if findings.is_empty() {
        out.push_str(&format!(
            "No findings in {} ({} exchanges). Thresholds: 5xx, slow>={}ms, secrets in text bodies.\n",
            file.display(),
            n_exchanges,
            cfg.slow_ms
        ));
        return out;
    }

    out.push_str(&format!(
        "{} finding(s) in {} ({} exchanges)\n\n",
        findings.len(),
        file.display(),
        n_exchanges
    ));
    for f in findings {
        out.push_str(&format!(
            "{:<6}  {}  {}\n",
            f.kind.label(),
            f.exchange_id,
            f.detail
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::record::{Body, Exchange, RequestRecord, ResponseRecord};
    use crate::capture::store::Store;
    use chrono::Utc;
    use std::fs;

    fn sample(id: &str, status: u16, latency_ms: u64) -> Exchange {
        Exchange {
            id: id.into(),
            started_at: Utc::now(),
            upstream: "http://127.0.0.1:9".into(),
            latency_ms,
            request: RequestRecord {
                method: "GET".into(),
                uri: "/x".into(),
                route: "/x".into(),
                version: "HTTP/1.1".into(),
                headers: vec![],
                body: Body::Empty,
            },
            response: Some(ResponseRecord {
                status,
                version: "HTTP/1.1".into(),
                headers: vec![],
                body: Body::Empty,
            }),
            error: None,
        }
    }

    #[test]
    fn formats_empty_and_non_empty_reports() {
        let path = std::path::Path::new("captures/session.rrlog");
        let empty = format_report(path, 3, &detect::Config::default(), &[]);
        assert!(empty.contains("No findings"));
        assert!(empty.contains("3 exchanges"));

        let findings = detect::scan_all(&[sample("1-000001", 500, 10)], &detect::Config::default());
        let filled = format_report(path, 1, &detect::Config::default(), &findings);
        assert!(filled.contains("1 finding"));
        assert!(filled.contains("1-000001"));
        assert!(!filled.contains("No findings"));
    }

    #[test]
    fn reads_findings_from_an_rrlog_without_network() {
        let dir = std::env::temp_dir().join(format!("reqradar-inspect-{}", std::process::id()));
        let path = dir.join("session.rrlog");
        let _ = fs::remove_dir_all(&dir);
        let mut store = Store::create(&path).unwrap();
        store.append(&sample("ok", 200, 10)).unwrap();
        store.append(&sample("boom", 502, 20)).unwrap();
        drop(store);

        let exchanges = store::read_all(&path).unwrap();
        let findings = detect::scan_all(&exchanges, &detect::Config::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].exchange_id, "boom");

        let _ = fs::remove_dir_all(&dir);
    }
}
