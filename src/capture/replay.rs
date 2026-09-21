//! Replay a captured exchange against an upstream and report the result.
//!
//! This is the first consumer of the `.rrlog` format: if replay works, the
//! capture/serialization contract is sound enough to build diff and report on.

use std::path::Path;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use hyper::Method;

use super::client;
use super::record::Exchange;
use super::store;

pub struct ReplayOutcome {
    pub original: Exchange,
    pub status: u16,
    pub latency_ms: u64,
}

/// Replay the exchange `id` from `file`. If `target` is given it overrides the
/// upstream stored in the capture (e.g. to replay against staging).
pub async fn run(file: &Path, id: &str, target: Option<&str>) -> Result<ReplayOutcome> {
    let original = store::find(file, id)?
        .with_context(|| format!("no capture with id `{id}` in {}", file.display()))?;

    let base = target
        .unwrap_or(&original.upstream)
        .trim_end_matches('/')
        .to_string();
    let url = format!("{}{}", base, original.request.uri);

    let method = Method::from_bytes(original.request.method.as_bytes())
        .with_context(|| format!("invalid method `{}`", original.request.method))?;
    let body = Bytes::from(original.request.body.to_bytes());

    let client = client::build_client();
    let timer = std::time::Instant::now();
    // Replay uses the same 1 MiB cap as capture unless we later thread a flag.
    let response = client::send(
        &client,
        &method,
        &url,
        &original.request.headers,
        body,
        1_048_576,
    )
    .await;
    let latency_ms = timer.elapsed().as_millis() as u64;

    let response = match response {
        Ok(r) => r,
        Err(e) => bail!("replay failed: {e:#}"),
    };

    Ok(ReplayOutcome {
        original,
        status: response.status,
        latency_ms,
    })
}
