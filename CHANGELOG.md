# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Library crate so the capture engine can be tested without the binary.
- Integration test: capture GET+POST through the proxy, then replay the POST.
- `reqradar report` writes a Markdown bug report with sensitive headers redacted.
- `--max-body-bytes` (default 1 MiB) and `--allow-lan` on `capture`.
- Unix `.rrlog` files created with mode `0600`.
- `-v` / `--verbose` wired to `tracing` on stderr.
- `cargo-audit` job and Dependabot for Cargo and GitHub Actions.
- CONTRIBUTING, SECURITY, issue/PR templates, CODEOWNERS.

### Changed

- Package version is `0.1.0-dev` (no release tag yet).
- README and `--help` describe the tool as it exists: HTTP capture + replay.
  Diff, YAML rules, TUI, and the web dashboard are explicitly unimplemented.
- `--json` prints a redacted exchange. The on-disk `.rrlog` keeps original values.

### Security

- Refuse non-loopback binds unless `--allow-lan` is set.
- Cap buffered request and response bodies.
- Redact credential headers in reports and JSON stdout.

[Unreleased]: https://github.com/toriyama237/reqradar/compare/HEAD...HEAD
