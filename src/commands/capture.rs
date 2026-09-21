use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Args;

use crate::capture::proxy::{self, ProxyConfig};

use super::not_yet_implemented;

const DEFAULT_MAX_BODY_BYTES: usize = 1_048_576;

#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// Upstream backend to forward captured traffic to (e.g. http://localhost:3000).
    #[arg(long)]
    pub target: Option<String>,

    /// Address the capturing proxy listens on.
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: String,

    /// File to write captured exchanges to (.rrlog). Defaults to captures/session-<ts>.rrlog.
    #[arg(short, long)]
    pub out: Option<PathBuf>,

    /// Print one JSON object per exchange on stdout (sensitive headers redacted).
    #[arg(long)]
    pub json: bool,

    /// Launch the web dashboard (not implemented yet).
    #[arg(long)]
    pub web: bool,

    /// Port for the web dashboard (only with --web).
    #[arg(long, default_value_t = 7777)]
    pub web_port: u16,

    /// Allow binding a non-loopback address. ReqRadar has no auth on the listen port.
    #[arg(long)]
    pub allow_lan: bool,

    /// Maximum request or response body buffered in memory (bytes).
    #[arg(long, default_value_t = DEFAULT_MAX_BODY_BYTES)]
    pub max_body_bytes: usize,
}

impl Default for CaptureArgs {
    fn default() -> Self {
        Self {
            target: None,
            listen: "127.0.0.1:8080".into(),
            out: None,
            json: false,
            web: false,
            web_port: 7777,
            allow_lan: false,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }
}

pub fn run(args: CaptureArgs) -> Result<()> {
    if args.web {
        return not_yet_implemented("capture --web (web dashboard)");
    }

    let target = args.target.context(
        "missing --target: ReqRadar needs a backend to forward to, e.g. \
         `reqradar capture --target http://localhost:3000`",
    )?;

    let listen: SocketAddr = args
        .listen
        .parse()
        .with_context(|| format!("invalid --listen address: {}", args.listen))?;

    let out = args.out.unwrap_or_else(|| {
        let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        PathBuf::from(format!("captures/session-{stamp}.rrlog"))
    });

    let config = ProxyConfig {
        listen,
        target,
        out,
        json: args.json,
        allow_lan: args.allow_lan,
        max_body_bytes: args.max_body_bytes,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting async runtime")?;
    runtime.block_on(proxy::serve(config))
}
