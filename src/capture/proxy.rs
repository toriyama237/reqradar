//! Transparent reverse proxy that forwards every request to a single upstream
//! and records each exchange.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Full};
use hyper::header::{HeaderName, HeaderValue};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use super::client::{self, HttpClient};
use super::record::{Body, Exchange, RequestRecord};
use super::store::Store;

pub struct ProxyConfig {
    pub listen: SocketAddr,
    /// Upstream base URL, e.g. `http://localhost:3000`.
    pub target: String,
    /// Where to write captured exchanges.
    pub out: PathBuf,
    /// Print full JSON per exchange instead of a compact summary.
    pub json: bool,
}

struct Ctx {
    base: String,
    client: HttpClient,
    store: Mutex<Store>,
    seq: AtomicU64,
    json: bool,
}

impl Ctx {
    fn next_id(&self, started: DateTime<Utc>) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("{}-{:06}", started.timestamp_millis(), seq)
    }
}

/// Normalize the target into a `scheme://authority` base, rejecting anything we
/// can't forward to yet (e.g. https).
fn normalize_target(target: &str) -> Result<String> {
    let with_scheme = if target.contains("://") {
        target.to_string()
    } else {
        format!("http://{target}")
    };
    let uri: Uri = with_scheme
        .parse()
        .with_context(|| format!("invalid --target: {target}"))?;
    let scheme = uri.scheme_str().unwrap_or("http");
    if scheme != "http" {
        anyhow::bail!("only http upstreams are supported for now (got {scheme}://)");
    }
    let authority = uri
        .authority()
        .with_context(|| format!("--target needs a host: {target}"))?
        .clone();
    Ok(format!("http://{authority}"))
}

pub async fn serve(config: ProxyConfig) -> Result<()> {
    let base = normalize_target(&config.target)?;
    let store = Store::create(&config.out)?;
    let store_path = store.path().to_path_buf();

    let ctx = Arc::new(Ctx {
        base: base.clone(),
        client: client::build_client(),
        store: Mutex::new(store),
        seq: AtomicU64::new(1),
        json: config.json,
    });

    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;

    eprintln!("ReqRadar capturing");
    eprintln!("  listen   http://{}", config.listen);
    eprintln!("  upstream {base}");
    eprintln!("  log      {}", store_path.display());
    eprintln!("  (Ctrl-C to stop)\n");

    tokio::select! {
        res = accept_loop(listener, ctx) => res,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nStopped. Captures saved to {}", store_path.display());
            Ok(())
        }
    }
}

async fn accept_loop(listener: TcpListener, ctx: Arc<Ctx>) -> Result<()> {
    loop {
        let (stream, _peer) = listener.accept().await.context("accepting connection")?;
        let io = TokioIo::new(stream);
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| handle(req, ctx.clone()));
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                eprintln!("connection error: {err}");
            }
        });
    }
}

async fn handle(
    req: Request<hyper::body::Incoming>,
    ctx: Arc<Ctx>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let started = Utc::now();
    let timer = Instant::now();

    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => return Ok(bad_gateway(format!("reading request body: {e}"))),
    };

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let route = parts.uri.path().to_string();
    let url = format!("{}{}", ctx.base, path_and_query);
    let req_headers = client::headers_to_vec(&parts.headers);

    let request_record = RequestRecord {
        method: parts.method.to_string(),
        uri: path_and_query,
        route,
        version: format!("{:?}", parts.version),
        headers: req_headers.clone(),
        body: Body::from_bytes(&body_bytes),
    };

    let id = ctx.next_id(started);
    let result = client::send(&ctx.client, &parts.method, &url, &req_headers, body_bytes).await;
    let latency_ms = timer.elapsed().as_millis() as u64;

    let (response_record, error, client_response) = match result {
        Ok(rec) => {
            let resp = build_client_response(&rec);
            (Some(rec), None, resp)
        }
        Err(e) => {
            let msg = format!("{e:#}");
            let resp = bad_gateway(format!("ReqRadar upstream error: {msg}"));
            (None, Some(msg), resp)
        }
    };

    let exchange = Exchange {
        id,
        started_at: started,
        upstream: ctx.base.clone(),
        latency_ms,
        request: request_record,
        response: response_record,
        error,
    };

    if let Ok(mut store) = ctx.store.lock() {
        if let Err(e) = store.append(&exchange) {
            eprintln!("failed to persist exchange {}: {e}", exchange.id);
        }
    }

    if ctx.json {
        match serde_json::to_string(&exchange) {
            Ok(line) => println!("{line}"),
            Err(e) => eprintln!("failed to serialize exchange: {e}"),
        }
    } else {
        println!("{}", exchange.summary());
    }

    Ok(client_response)
}

/// Build the response sent back to the client from the recorded upstream
/// response, dropping hop-by-hop / length headers (hyper recomputes them).
fn build_client_response(rec: &super::record::ResponseRecord) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::from_u16(rec.status).unwrap_or(StatusCode::BAD_GATEWAY));
    for h in &rec.headers {
        let lower = h.name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "content-length" | "transfer-encoding" | "connection"
        ) {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(h.name.as_bytes()),
            HeaderValue::from_str(&h.value),
        ) {
            builder = builder.header(name, value);
        }
    }
    builder
        .body(Full::new(Bytes::from(rec.body.to_bytes())))
        .unwrap_or_else(|_| bad_gateway("failed to build response".to_string()))
}

fn bad_gateway(message: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(message)))
        .expect("static 502 response is valid")
}
