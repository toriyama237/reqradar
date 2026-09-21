# Security

ReqRadar is a local debugging proxy. It sees whatever the client sends, including
credentials, cookies, and request bodies.

## What the tool stores

`.rrlog` files keep **original** headers and bodies so `reqradar replay` can
reproduce authenticated requests. Files are created with mode `0600` on Unix.
Treat a capture directory like a secrets store: do not commit it, do not paste
it into a ticket.

`captures/` is gitignored.

## What is redacted

Shareable output (`reqradar report`, and `--json` stdout) replaces values of
`Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`, `X-Api-Key`,
`X-Auth-Token`, and `X-Amz-Security-Token` with `[REDACTED]`.

Redaction is not a guarantee that a body contains no secrets. Do not publish
reports from production traffic without reading them.

## Network exposure

The proxy listens on `127.0.0.1` by default. Binding a non-loopback address
requires `--allow-lan`. ReqRadar is not an internet-facing reverse proxy:
there is no TLS terminator, no authn on the listen port, and no request
authorization.

Bodies larger than `--max-body-bytes` (1 MiB by default) are rejected so a
single response cannot fill RAM.

## Reporting a vulnerability

Use [GitHub private advisories](https://github.com/toriyama237/reqradar/security/advisories/new).
Do not open a public issue for a vulnerability in capture storage, redaction,
or bind behaviour.
