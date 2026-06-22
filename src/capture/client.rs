//! Outgoing HTTP forwarding, shared by the proxy and by replay.

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::header::{HeaderName, HeaderValue, HOST};
use hyper::{HeaderMap, Method, Request, Uri};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use super::record::{Body, Header, ResponseRecord};

/// Connection-pooled HTTP/1 client over plain TCP (no TLS yet — phase one
/// targets local backends).
pub type HttpClient = Client<HttpConnector, Full<Bytes>>;

pub fn build_client() -> HttpClient {
    Client::builder(TokioExecutor::new()).build_http()
}

/// Hop-by-hop headers must not be forwarded between connections.
fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Convert a hyper header map into the serializable record form.
pub fn headers_to_vec(map: &HeaderMap) -> Vec<Header> {
    map.iter()
        .map(|(k, v)| Header {
            name: k.as_str().to_string(),
            value: String::from_utf8_lossy(v.as_bytes()).into_owned(),
        })
        .collect()
}

/// Forward a request upstream and collect the full response into a record.
pub async fn send(
    client: &HttpClient,
    method: &Method,
    url: &str,
    headers: &[Header],
    body: Bytes,
) -> Result<ResponseRecord> {
    let uri: Uri = url
        .parse()
        .with_context(|| format!("invalid upstream URL: {url}"))?;
    let authority = uri.authority().map(|a| a.as_str().to_string());

    let mut builder = Request::builder().method(method.clone()).uri(uri);
    let hmap = builder
        .headers_mut()
        .expect("fresh request builder has headers");
    for h in headers {
        if is_hop_by_hop(&h.name) || h.name.eq_ignore_ascii_case("host") {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(h.name.as_bytes()),
            HeaderValue::from_str(&h.value),
        ) {
            hmap.append(name, value);
        }
    }
    // Point Host at the upstream so the backend routes correctly.
    if let Some(authority) = authority {
        if let Ok(value) = HeaderValue::from_str(&authority) {
            hmap.insert(HOST, value);
        }
    }

    let request = builder
        .body(Full::new(body))
        .context("building upstream request")?;
    let response = client
        .request(request)
        .await
        .context("forwarding request upstream")?;

    let (parts, body) = response.into_parts();
    let bytes = body
        .collect()
        .await
        .context("reading upstream response body")?
        .to_bytes();

    Ok(ResponseRecord {
        status: parts.status.as_u16(),
        version: format!("{:?}", parts.version),
        headers: headers_to_vec(&parts.headers),
        body: Body::from_bytes(&bytes),
    })
}
