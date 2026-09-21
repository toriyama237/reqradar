//! Built-in detectors that run offline over captured exchanges.
//!
//! Three kinds, no YAML: HTTP 5xx, latency above a threshold, and
//! credential-looking strings in **text bodies**. Header redaction is
//! `crate::redact`; this module does not look at header values.

use crate::capture::record::{Body, Exchange};

pub const DEFAULT_SLOW_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Status5xx,
    Slow,
    SecretInBody,
}

impl Kind {
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Status5xx => "[5xx]",
            Kind::Slow => "[slow]",
            Kind::SecretInBody => "[secret]",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Status5xx => "5xx",
            Kind::Slow => "slow",
            Kind::SecretInBody => "secret",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: Kind,
    pub exchange_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub slow_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            slow_ms: DEFAULT_SLOW_MS,
        }
    }
}

pub fn scan_exchange(ex: &Exchange, cfg: &Config) -> Vec<Finding> {
    let mut out = Vec::new();

    if let Some(resp) = &ex.response {
        if resp.status >= 500 {
            out.push(Finding {
                kind: Kind::Status5xx,
                exchange_id: ex.id.clone(),
                detail: format!("HTTP {}", resp.status),
            });
        }
    }

    if ex.latency_ms >= cfg.slow_ms {
        out.push(Finding {
            kind: Kind::Slow,
            exchange_id: ex.id.clone(),
            detail: format!("{}ms (threshold {}ms)", ex.latency_ms, cfg.slow_ms),
        });
    }

    let secret = body_secret_reason(&ex.request.body).or_else(|| {
        ex.response
            .as_ref()
            .and_then(|r| body_secret_reason(&r.body))
    });
    if let Some(reason) = secret {
        out.push(Finding {
            kind: Kind::SecretInBody,
            exchange_id: ex.id.clone(),
            detail: reason.to_string(),
        });
    }

    out
}

pub fn scan_all(exchanges: &[Exchange], cfg: &Config) -> Vec<Finding> {
    exchanges
        .iter()
        .flat_map(|ex| scan_exchange(ex, cfg))
        .collect()
}

/// Tags for the live capture one-liner, in a stable order.
pub fn tags(ex: &Exchange, cfg: &Config) -> String {
    let mut seen = Vec::new();
    for f in scan_exchange(ex, cfg) {
        let tag = f.kind.tag();
        if !seen.contains(&tag) {
            seen.push(tag);
        }
    }
    seen.join(" ")
}

fn body_secret_reason(body: &Body) -> Option<&'static str> {
    let text = match body {
        Body::Text { text } => text.as_str(),
        Body::Empty | Body::Base64 { .. } => return None,
    };
    if has_bearer_token(text) {
        return Some("text body looks like it contains a Bearer token");
    }
    if has_json_password_field(text) {
        return Some("text body looks like it contains a JSON password field");
    }
    if has_form_password(text) {
        return Some("text body looks like it contains password=");
    }
    if has_aws_access_key_id(text) {
        return Some("text body looks like it contains an AWS access key id");
    }
    None
}

fn has_bearer_token(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(i) = rest.find("bearer ") {
        let after = &rest[i + 7..];
        let token_len = after
            .chars()
            .take_while(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=')
            })
            .count();
        if token_len >= 8 {
            return true;
        }
        rest = after;
    }
    false
}

fn has_json_password_field(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    for needle in ["\"password\":", "\"password\" :"] {
        if let Some(i) = lower.find(needle) {
            let after = lower[i + needle.len()..].trim_start();
            if after.starts_with('"') {
                return true;
            }
        }
    }
    false
}

fn has_form_password(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("password=")
}

fn has_aws_access_key_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return false;
    }
    bytes
        .windows(20)
        .any(|w| w.starts_with(b"AKIA") && w[4..].iter().all(|b| b.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::record::{Header, RequestRecord, ResponseRecord};
    use chrono::Utc;

    fn ex(id: &str, status: u16, latency_ms: u64, req_body: Body, resp_body: Body) -> Exchange {
        Exchange {
            id: id.into(),
            started_at: Utc::now(),
            upstream: "http://127.0.0.1:9".into(),
            latency_ms,
            request: RequestRecord {
                method: "POST".into(),
                uri: "/x".into(),
                route: "/x".into(),
                version: "HTTP/1.1".into(),
                headers: vec![Header {
                    name: "Authorization".into(),
                    value: "Bearer header-token-not-a-body-secret".into(),
                }],
                body: req_body,
            },
            response: Some(ResponseRecord {
                status,
                version: "HTTP/1.1".into(),
                headers: vec![],
                body: resp_body,
            }),
            error: None,
        }
    }

    #[test]
    fn flags_5xx() {
        let findings = scan_exchange(
            &ex("a", 503, 10, Body::Empty, Body::Empty),
            &Config::default(),
        );
        assert!(findings.iter().any(|f| f.kind == Kind::Status5xx));
        assert!(findings.iter().all(|f| f.kind != Kind::Slow));
    }

    #[test]
    fn flags_slow() {
        let findings = scan_exchange(
            &ex("b", 200, 800, Body::Empty, Body::Empty),
            &Config::default(),
        );
        assert!(findings
            .iter()
            .any(|f| f.kind == Kind::Slow && f.detail.contains("800ms")));
        assert!(findings.iter().all(|f| f.kind != Kind::Status5xx));
    }

    #[test]
    fn clean_exchange_is_silent() {
        let findings = scan_exchange(
            &ex("c", 200, 12, Body::Empty, Body::Empty),
            &Config::default(),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn authorization_header_is_not_a_body_finding() {
        let findings = scan_exchange(
            &ex("d", 200, 12, Body::Empty, Body::Empty),
            &Config::default(),
        );
        assert!(findings.iter().all(|f| f.kind != Kind::SecretInBody));
    }

    #[test]
    fn flags_bearer_in_body_without_echoing_the_token() {
        let body = Body::Text {
            text: r#"{"token":"Bearer abcdefghijklmnop"}"#.into(),
        };
        let findings = scan_exchange(&ex("e", 200, 12, body, Body::Empty), &Config::default());
        let secret = findings
            .iter()
            .find(|f| f.kind == Kind::SecretInBody)
            .unwrap();
        assert!(!secret.detail.contains("abcdefghijklmnop"));
    }

    #[test]
    fn password_in_prose_is_not_a_finding() {
        let body = Body::Text {
            text: "Please reset your password from the settings page.".into(),
        };
        let findings = scan_exchange(&ex("f", 200, 12, body, Body::Empty), &Config::default());
        assert!(findings.iter().all(|f| f.kind != Kind::SecretInBody));
    }

    #[test]
    fn flags_json_password_field() {
        let body = Body::Text {
            text: r#"{"password":"hunter2"}"#.into(),
        };
        let findings = scan_exchange(&ex("g", 200, 12, body, Body::Empty), &Config::default());
        assert!(findings.iter().any(|f| f.kind == Kind::SecretInBody));
        assert!(findings.iter().all(|f| !f.detail.contains("hunter2")));
    }

    #[test]
    fn scan_all_keeps_exchange_ids() {
        let exchanges = vec![
            ex("slow", 200, 900, Body::Empty, Body::Empty),
            ex("ok", 200, 10, Body::Empty, Body::Empty),
            ex("boom", 500, 10, Body::Empty, Body::Empty),
        ];
        let findings = scan_all(&exchanges, &Config::default());
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.exchange_id == "slow"));
        assert!(findings.iter().any(|f| f.exchange_id == "boom"));
    }

    #[test]
    fn tags_are_stable_and_unique() {
        let tagged = tags(
            &ex("h", 500, 900, Body::Empty, Body::Empty),
            &Config::default(),
        );
        assert_eq!(tagged, "[5xx] [slow]");
    }
}
