# ReqRadar

**Capture, inspect, replay HTTP — then paste a bug report.** Reverse proxy in front of a local backend. Not a TUI. Not a dashboard. HTTP only.

[![CI](https://github.com/toriyama237/reqradar/actions/workflows/ci.yml/badge.svg)](https://github.com/toriyama237/reqradar/actions/workflows/ci.yml)

> **Status: 0.1.0-dev.** `capture`, `replay`, and Markdown `report` work. Diff, YAML rules, TUI, web UI, and HTTPS do not.

## Why

Debugging an HTTP integration today means juggling `curl`, verbose logs, a heavy GUI proxy, and captures you cannot replay. ReqRadar is the short path from “there is a network bug” to “here is the exchange, replay it, paste this into the ticket.”

## Quick start

```bash
cargo install --path .
reqradar capture --target http://localhost:3000 --listen 127.0.0.1:8080
```

Point the client at the proxy:

```bash
curl http://127.0.0.1:8080/api/health
curl -X POST http://127.0.0.1:8080/api/login -d '{"user":"bob"}'
```

Live one-liners go to stdout; the session is appended to `captures/session-<ts>.rrlog`.

```bash
reqradar replay <id>                          # newest .rrlog under ./captures
reqradar replay <id> --file session.rrlog
reqradar replay <id> --target http://127.0.0.1:3000

reqradar report <id>                          # Markdown on stdout, secrets redacted
reqradar report <id> -o bug.md
```

`--json` on `capture` prints each exchange as JSON on stdout with credential headers redacted. The on-disk `.rrlog` keeps original values so replay can authenticate.

## CLI

| Command | Status |
| --- | --- |
| `reqradar capture --target <url>` | Shipped |
| `reqradar replay <id>` | Shipped |
| `reqradar report <id>` | Shipped (Markdown; PDF is not) |
| `reqradar capture --web` | Not implemented |
| `reqradar diff` | Not implemented |
| `reqradar rules` | Not implemented |

Useful `capture` flags:

- `--listen 127.0.0.1:8080` — non-loopback binds require `--allow-lan`
- `--max-body-bytes` — default 1 MiB; larger bodies return 413
- `--out path.rrlog`
- `-v` / `-vv` / `-vvv` — tracing on stderr

## Architecture

```
client  -->  ReqRadar (HTTP/1 reverse proxy)  -->  upstream
                 |
                 +-- append Exchange as JSON Lines (.rrlog, mode 0600)
                 +-- replay / report read the same file
```

The crate is a library (`reqradar`) plus a thin binary. Integration tests drive `spawn()` against an in-process upstream.

## Security

`.rrlog` files contain original headers and bodies, including cookies and `Authorization`. They are created `0600` on Unix and `captures/` is gitignored. Treat them as secrets. Reports redact a fixed list of credential headers. Details: [SECURITY.md](SECURITY.md).

## Development

Requires Rust 1.82+ (CI uses stable).

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Workflow: [CONTRIBUTING.md](CONTRIBUTING.md). GitHub Flow (`main` + PR). No `develop` branch.

## Roadmap

Shipped in this development line: capture, `.rrlog`, replay, Markdown report, bind guard, body cap, header redaction on shareable output.

Next, in order:

1. Built-in detectors (5xx, slow requests, leaked `Authorization` in report bodies)
2. `diff` of two `.rrlog` files
3. YAML rules
4. TUI, then `--web`
5. HTTPS / `brew` / crates.io only after the CLI loop is boring

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
