//! Transparent reverse proxy that forwards every request to a single upstream
//! and records each exchange.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::Utc;
use http_body_util::Full;
use hyper::header::{HeaderName, HeaderValue};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::client::{self, CollectError, HttpClient};
use super::id::IdGen;
use super::record::{Body, Exchange, RequestRecord};
use super::store::Store;
use crate::redact;

pub struct ProxyConfig {
    pub listen: SocketAddr,
    /// Upstream base URL, e.g. `http://localhost:3000`.
    pub target: String,
    /// Where to write captured exchanges.
    pub out: PathBuf,
    /// Print JSON per exchange (redacted) instead of a compact summary.
    pub json: bool,
    /// Permit non-loopback listen addresses.
    pub allow_lan: bool,
    /// Max buffered request or response body, in bytes.
    pub max_body_bytes: usize,
}

pub struct RunningProxy {
    pub addr: SocketAddr,
    pub log_path: PathBuf,
    pub upstream: String,
    join: JoinHandle<Result<()>>,
}

impl RunningProxy {
    pub fn abort(&self) {
        self.join.abort();
    }
}

struct Ctx {
    base: String,
    client: HttpClient,
    store: Mutex<Store>,
    ids: IdGen,
    json: bool,
    max_body_bytes: usize,
}

/// Normalize the target into a `scheme://authority` base, rejecting anything we
/// can't forward to yet (e.g. https).
fn normalize_target(target: &str) -> Result<String> {
    let with_scheme = if target.contains("://") {
        target.to_string()
    } else {
        format!("http://{target}")
    };
    let uri: hyper::Uri = with_scheme
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

fn assert_bind_allowed(addr: SocketAddr, allow_lan: bool) -> Result<()> {
    if addr.ip().is_loopback() {
        return Ok(());
    }
    if allow_lan {
        tracing::warn!(
            %addr,
            "listening on a non-loopback address; there is no authentication on this port"
        );
        return Ok(());
    }
    anyhow::bail!(
        "refusing to bind {addr}: ReqRadar has no listen-port authentication. \
         Bind 127.0.0.1 or pass --allow-lan"
    );
}

/// Bind and serve until Ctrl-C. Used by the CLI.
pub async fn serve(config: ProxyConfig) -> Result<()> {
    let running = spawn(config).await?;
    eprintln!("ReqRadar capturing");
    eprintln!("  listen   http://{}", running.addr);
    eprintln!("  upstream {}", running.upstream);
    eprintln!("  log      {}", running.log_path.display());
    eprintln!("  (Ctrl-C to stop)\n");
    tracing::warn!(
        path = %running.log_path.display(),
        "captures store original headers and bodies; treat the file as a secrets store"
    );

    let abort = running.join.abort_handle();
    let log_path = running.log_path.clone();

    tokio::select! {
        res = running.join => match res {
            Ok(inner) => inner,
            Err(e) if e.is_cancelled() => Ok(()),
            Err(e) => Err(anyhow::anyhow!("proxy task failed: {e}")),
        },
        _ = tokio::signal::ctrl_c() => {
            abort.abort();
            eprintln!("\nStopped. Captures saved to {}", log_path.display());
            Ok(())
        }
    }
}

/// Bind the proxy and return immediately. The accept loop runs on a task.
/// Tests abort the task when they are done.
pub async fn spawn(config: ProxyConfig) -> Result<RunningProxy> {
    assert_bind_allowed(config.listen, config.allow_lan)?;
    let base = normalize_target(&config.target)?;
    let store = Store::create(&config.out)?;
    let store_path = store.path().to_path_buf();

    let ctx = Arc::new(Ctx {
        base: base.clone(),
        client: client::build_client(),
        store: Mutex::new(store),
        ids: IdGen::new(),
        json: config.json,
        max_body_bytes: config.max_body_bytes,
    });

    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;
    let addr = listener.local_addr().context("reading bound address")?;

    tracing::info!(%addr, upstream = %base, "proxy listening");

    let join = tokio::spawn(accept_loop(listener, ctx));
    Ok(RunningProxy {
        addr,
        log_path: store_path,
        upstream: base,
        join,
    })
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
                tracing::debug!("connection error: {err}");
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
    let body_bytes = match client::collect_capped(body, ctx.max_body_bytes).await {
        Ok(b) => b,
        Err(CollectError::TooLarge { max }) => {
            return Ok(payload_too_large(max));
        }
        Err(CollectError::Body(e)) => {
            return Ok(bad_gateway(format!("reading request body: {e}")));
        }
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

    let id = ctx.ids.next(started);
    let result = client::send(
        &ctx.client,
        &parts.method,
        &url,
        &req_headers,
        body_bytes,
        ctx.max_body_bytes,
    )
    .await;
    let latency_ms = timer.elapsed().as_millis() as u64;

    let (response_record, error, client_response) = match result {
        Ok(rec) => {
            let resp = build_client_response(&rec);
            (Some(rec), None, resp)
        }
        Err(e) => {
            let msg = format!("{e:#}");
            let resp = bad_gateway("ReqRadar upstream error".to_string());
            tracing::warn!(id = %id, error = %msg, "upstream forward failed");
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
            tracing::error!("failed to persist exchange {}: {e}", exchange.id);
        }
    }

    if ctx.json {
        let display = redact::exchange_for_display(&exchange);
        match serde_json::to_string(&display) {
            Ok(line) => println!("{line}"),
            Err(e) => tracing::error!("failed to serialize exchange: {e}"),
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

fn payload_too_large(max: usize) -> Response<Full<Bytes>> {
    let msg = format!("ReqRadar: request body exceeds --max-body-bytes ({max})");
    Response::builder()
        .status(StatusCode::PAYLOAD_TOO_LARGE)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(msg)))
        .expect("static 413 response is valid")
}
