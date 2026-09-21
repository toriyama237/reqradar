//! Product-path test: a real HTTP upstream, the capturing proxy, then replay.

use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use reqradar::capture::client::{self, HttpClient};
use reqradar::capture::proxy::{spawn, ProxyConfig};
use reqradar::capture::record::Header;
use reqradar::capture::{replay, store};
use tokio::net::TcpListener;

async fn echo_upstream(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().to_string();
    let (parts, body) = req.into_parts();
    let bytes = body
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();

    if path == "/health" {
        return Ok(Response::new(Full::new(Bytes::from_static(b"ok"))));
    }

    let mut builder = Response::builder().status(200);
    if let Some(ct) = parts.headers.get("content-type") {
        builder = builder.header("content-type", ct);
    }
    Ok(builder.body(Full::new(bytes)).unwrap())
}

async fn spawn_upstream() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(echo_upstream))
                    .await;
            });
        }
    });
    (addr, handle)
}

fn temp_log() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "reqradar-itest-{}-{nanos}.rrlog",
        std::process::id()
    ))
}

async fn get(client: &HttpClient, url: &str) -> u16 {
    client::send(client, &Method::GET, url, &[], Bytes::new(), 1_048_576)
        .await
        .unwrap()
        .status
}

async fn post(client: &HttpClient, url: &str, body: &str, auth: Option<&str>) -> u16 {
    let mut headers = vec![Header {
        name: "content-type".into(),
        value: "text/plain".into(),
    }];
    if let Some(token) = auth {
        headers.push(Header {
            name: "Authorization".into(),
            value: format!("Bearer {token}"),
        });
    }
    client::send(
        client,
        &Method::POST,
        url,
        &headers,
        Bytes::from(body.to_string()),
        1_048_576,
    )
    .await
    .unwrap()
    .status
}

#[tokio::test]
async fn capture_get_and_post_then_replay_post() {
    let (up_addr, up_task) = spawn_upstream().await;
    let log = temp_log();

    let proxy = spawn(ProxyConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        target: format!("http://{up_addr}"),
        out: log.clone(),
        json: false,
        allow_lan: false,
        max_body_bytes: 1_048_576,
        slow_ms: 500,
    })
    .await
    .expect("proxy binds");

    let client = client::build_client();
    let base = format!("http://{}", proxy.addr);

    assert_eq!(get(&client, &format!("{base}/health")).await, 200);
    assert_eq!(
        post(
            &client,
            &format!("{base}/echo"),
            "hello-reqradar",
            Some("secret-token")
        )
        .await,
        200
    );

    let exchanges = store::read_all(&log).expect("rrlog readable");
    assert_eq!(exchanges.len(), 2);
    assert_eq!(exchanges[0].request.route, "/health");
    assert_eq!(exchanges[1].request.method, "POST");
    assert_eq!(exchanges[1].request.body.to_bytes(), b"hello-reqradar");
    // On-disk capture keeps the secret so replay can authenticate.
    assert!(exchanges[1]
        .request
        .headers
        .iter()
        .any(|h| h.value.contains("secret-token")));

    let outcome = replay::run(&log, &exchanges[1].id, None)
        .await
        .expect("replay");
    assert_eq!(outcome.status, 200);

    proxy.abort();
    up_task.abort();
    let _ = std::fs::remove_file(&log);
}

#[tokio::test]
async fn refuses_non_loopback_without_allow_lan() {
    let err = match spawn(ProxyConfig {
        listen: "0.0.0.0:0".parse().unwrap(),
        target: "http://127.0.0.1:9".into(),
        out: temp_log(),
        json: false,
        allow_lan: false,
        max_body_bytes: 1024,
        slow_ms: 500,
    })
    .await
    {
        Ok(_) => panic!("expected non-loopback bind to be refused"),
        Err(e) => e,
    };
    let msg = format!("{err:#}");
    assert!(
        msg.contains("allow-lan") || msg.contains("loopback") || msg.contains("refusing"),
        "unexpected error: {msg}"
    );
}
