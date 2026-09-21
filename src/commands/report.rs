use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

use crate::capture::{self, store};
use crate::redact;

use super::not_yet_implemented;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReportFormat {
    Markdown,
    Pdf,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Identifier of the captured request to turn into a bug report.
    pub request_id: String,

    /// Capture file to read from. Defaults to the newest .rrlog under ./captures.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub format: ReportFormat,

    /// Write the report to a file instead of stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn run(args: ReportArgs) -> Result<()> {
    match args.format {
        ReportFormat::Pdf => not_yet_implemented("report --format pdf"),
        ReportFormat::Markdown => write_markdown(args),
    }
}

fn write_markdown(args: ReportArgs) -> Result<()> {
    let file = match args.file {
        Some(f) => f,
        None => capture::latest_capture_in("captures").context(
            "no --file given and no .rrlog found under ./captures; run `reqradar capture` first",
        )?,
    };

    let exchange = store::find(&file, &args.request_id)?.with_context(|| {
        format!(
            "no capture with id `{}` in {}",
            args.request_id,
            file.display()
        )
    })?;

    let markdown = render_markdown(&exchange);

    match args.output {
        Some(path) => {
            std::fs::write(&path, markdown)
                .with_context(|| format!("writing report to {}", path.display()))?;
        }
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(markdown.as_bytes())?;
        }
    }
    Ok(())
}

fn render_markdown(ex: &crate::capture::record::Exchange) -> String {
    let req_headers = redact::headers(&ex.request.headers);
    let resp_headers = ex
        .response
        .as_ref()
        .map(|r| redact::headers(&r.headers))
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str("# ReqRadar bug report\n\n");
    out.push_str(&format!("- **id:** `{}`\n", ex.id));
    out.push_str(&format!(
        "- **captured_at:** {}\n",
        ex.started_at.to_rfc3339()
    ));
    out.push_str(&format!("- **upstream:** {}\n", ex.upstream));
    out.push_str(&format!("- **latency_ms:** {}\n", ex.latency_ms));
    if let Some(err) = &ex.error {
        out.push_str(&format!("- **forward_error:** {err}\n"));
    }
    out.push('\n');

    let findings = crate::detect::scan_exchange(ex, &crate::detect::Config::default());
    if !findings.is_empty() {
        out.push_str("## Findings\n\n");
        for f in &findings {
            out.push_str(&format!("- **{}:** {}\n", f.kind.label(), f.detail));
        }
        out.push('\n');
    }

    out.push_str("## Request\n\n");
    out.push_str("```http\n");
    out.push_str(&format!("{} {}\n", ex.request.method, ex.request.uri));
    for h in &req_headers {
        out.push_str(&format!("{}: {}\n", h.name, h.value));
    }
    out.push('\n');
    out.push_str(&body_text(&ex.request.body));
    out.push_str("\n```\n\n");

    out.push_str("## Response\n\n");
    match &ex.response {
        None => out.push_str("_No upstream response._\n"),
        Some(resp) => {
            out.push_str("```http\n");
            out.push_str(&format!("HTTP {} {}\n", resp.status, resp.version));
            for h in &resp_headers {
                out.push_str(&format!("{}: {}\n", h.name, h.value));
            }
            out.push('\n');
            out.push_str(&body_text(&resp.body));
            out.push_str("\n```\n");
        }
    }

    out.push_str("\n---\n\n");
    out.push_str("_Sensitive headers redacted. On-disk `.rrlog` still holds original values._\n");
    out
}

fn body_text(body: &crate::capture::record::Body) -> String {
    match body {
        crate::capture::record::Body::Empty => String::new(),
        crate::capture::record::Body::Text { text } => text.clone(),
        crate::capture::record::Body::Base64 { len, .. } => {
            format!("<binary body, {len} bytes, omitted from report>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::record::{Body, Exchange, Header, RequestRecord, ResponseRecord};
    use chrono::Utc;

    #[test]
    fn report_redacts_authorization() {
        let ex = Exchange {
            id: "1-000001".into(),
            started_at: Utc::now(),
            upstream: "http://127.0.0.1:3000".into(),
            latency_ms: 4,
            request: RequestRecord {
                method: "GET".into(),
                uri: "/secret".into(),
                route: "/secret".into(),
                version: "HTTP/1.1".into(),
                headers: vec![Header {
                    name: "Authorization".into(),
                    value: "Bearer real-token".into(),
                }],
                body: Body::Empty,
            },
            response: Some(ResponseRecord {
                status: 200,
                version: "HTTP/1.1".into(),
                headers: vec![],
                body: Body::Text { text: "ok".into() },
            }),
            error: None,
        };
        let md = render_markdown(&ex);
        assert!(md.contains("[REDACTED]"));
        assert!(!md.contains("real-token"));
        assert!(md.contains("GET /secret"));
    }

    #[test]
    fn report_includes_5xx_finding_without_body_secret_from_headers() {
        let ex = Exchange {
            id: "1-000002".into(),
            started_at: Utc::now(),
            upstream: "http://127.0.0.1:3000".into(),
            latency_ms: 4,
            request: RequestRecord {
                method: "GET".into(),
                uri: "/fail".into(),
                route: "/fail".into(),
                version: "HTTP/1.1".into(),
                headers: vec![Header {
                    name: "Authorization".into(),
                    value: "Bearer real-token".into(),
                }],
                body: Body::Empty,
            },
            response: Some(ResponseRecord {
                status: 503,
                version: "HTTP/1.1".into(),
                headers: vec![],
                body: Body::Text {
                    text: "nope".into(),
                },
            }),
            error: None,
        };
        let md = render_markdown(&ex);
        assert!(md.contains("## Findings"));
        assert!(md.contains("**5xx:** HTTP 503"));
        assert!(!md.contains("real-token"));
        assert!(!md.contains("## Findings\n\n- **secret:"));
    }
}
