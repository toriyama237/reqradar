//! Header redaction for anything that leaves the private `.rrlog` file.
//!
//! Captures on disk keep original values so replay can reproduce authenticated
//! requests. Reports, `--json` stdout, and any shareable export go through here.

use crate::capture::record::{Exchange, Header};

const REDACTED: &str = "[REDACTED]";

const SENSITIVE_NAMES: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-amz-security-token",
];

pub fn is_sensitive_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_NAMES.contains(&lower.as_str())
}

/// Clone headers with sensitive values replaced. Order and names are preserved.
pub fn headers(headers: &[Header]) -> Vec<Header> {
    headers
        .iter()
        .map(|h| {
            if is_sensitive_header(&h.name) {
                Header {
                    name: h.name.clone(),
                    value: REDACTED.to_string(),
                }
            } else {
                h.clone()
            }
        })
        .collect()
}

/// Copy an exchange with credential headers stripped for stdout / reports.
pub fn exchange_for_display(ex: &Exchange) -> Exchange {
    let mut out = ex.clone();
    out.request.headers = headers(&out.request.headers);
    if let Some(resp) = out.response.as_mut() {
        resp.headers = headers(&resp.headers);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_authorization_and_preserves_others() {
        let input = vec![
            Header {
                name: "Authorization".into(),
                value: "Bearer secret".into(),
            },
            Header {
                name: "Content-Type".into(),
                value: "application/json".into(),
            },
            Header {
                name: "Cookie".into(),
                value: "session=abc".into(),
            },
        ];
        let out = headers(&input);
        assert_eq!(out[0].value, REDACTED);
        assert_eq!(out[1].value, "application/json");
        assert_eq!(out[2].value, REDACTED);
    }

    #[test]
    fn leaves_harmless_headers_untouched() {
        let input = vec![Header {
            name: "X-Request-Id".into(),
            value: "abc".into(),
        }];
        assert_eq!(headers(&input)[0].value, "abc");
    }
}
