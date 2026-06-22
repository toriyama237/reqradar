//! Core data model for a captured HTTP exchange.
//!
//! This is the format every consumer (replay, diff, report) will read, so it is
//! intentionally self-contained and serde-serializable. The on-disk form is one
//! JSON object per line (see [`crate::capture::store`]).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single request/response pair observed by the proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    /// Sortable, unique identifier (`<unix_millis>-<seq>`).
    pub id: String,
    /// When the request was received by the proxy.
    pub started_at: DateTime<Utc>,
    /// Base URL the request was forwarded to (used by replay).
    pub upstream: String,
    /// Total time from receiving the request to receiving the response.
    pub latency_ms: u64,
    pub request: RequestRecord,
    /// The upstream response, if one was received.
    pub response: Option<ResponseRecord>,
    /// Set when forwarding failed (no response).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Exchange {
    /// Compact one-line summary for terminal output.
    pub fn summary(&self) -> String {
        let status = match (&self.response, &self.error) {
            (Some(r), _) => r.status.to_string(),
            (None, Some(_)) => "ERR".to_string(),
            (None, None) => "---".to_string(),
        };
        format!(
            "{}  {:<6} {:<30} -> {:>4}  {}ms",
            self.id, self.request.method, self.request.route, status, self.latency_ms
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub method: String,
    /// Full path + query as seen by the proxy.
    pub uri: String,
    /// Path only (no query), handy for grouping by route.
    pub route: String,
    pub version: String,
    pub headers: Vec<Header>,
    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRecord {
    pub status: u16,
    pub version: String,
    pub headers: Vec<Header>,
    pub body: Body,
}

/// A single header. Stored as a list (not a map) to preserve order and
/// duplicate keys (e.g. multiple `Set-Cookie`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// Request/response body. Text is kept human-readable in the log; binary falls
/// back to base64 so the format stays lossless.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "encoding", rename_all = "snake_case")]
pub enum Body {
    Empty,
    Text { text: String },
    Base64 { base64: String, len: usize },
}

impl Body {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            Body::Empty
        } else if let Ok(text) = std::str::from_utf8(bytes) {
            Body::Text {
                text: text.to_owned(),
            }
        } else {
            Body::Base64 {
                base64: BASE64.encode(bytes),
                len: bytes.len(),
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Body::Empty => Vec::new(),
            Body::Text { text } => text.clone().into_bytes(),
            Body::Base64 { base64, .. } => BASE64.decode(base64).unwrap_or_default(),
        }
    }

    #[allow(dead_code)] // part of the model API; used by consumers + tests
    pub fn len(&self) -> usize {
        match self {
            Body::Empty => 0,
            Body::Text { text } => text.len(),
            Body::Base64 { len, .. } => *len,
        }
    }

    #[allow(dead_code)] // part of the model API; used by consumers + tests
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_roundtrips_text() {
        let b = Body::from_bytes(b"hello");
        assert!(matches!(b, Body::Text { .. }));
        assert_eq!(b.to_bytes(), b"hello");
    }

    #[test]
    fn body_roundtrips_binary() {
        let raw = [0u8, 159, 146, 150];
        let b = Body::from_bytes(&raw);
        assert!(matches!(b, Body::Base64 { .. }));
        assert_eq!(b.to_bytes(), raw);
    }

    #[test]
    fn empty_body() {
        let b = Body::from_bytes(b"");
        assert!(b.is_empty());
        assert_eq!(b.to_bytes(), Vec::<u8>::new());
    }
}
