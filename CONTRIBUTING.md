# Contributing to ReqRadar

## Git workflow

This repository uses **GitHub Flow**, not GitFlow.

- `main` is the only long-lived branch. It must stay green.
- All work lands through a pull request. Do not push commits to `main`.
- Branch names: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`, `docs/<slug>`, `test/<slug>`, `refactor/<slug>`, `ci/<slug>`.
- One concern per pull request. One logical change per commit.

There is no `develop` branch. Releases are tags on `main` (`vMAJOR.MINOR.PATCH`).

## Commit messages

Conventional Commits, imperative, scoped when useful:

```
feat: write markdown bug reports from a capture id
fix: refuse non-loopback binds without --allow-lan
refactor: extract the capture engine into a library crate
test: cover capture and replay against an in-process upstream
docs: document the GitHub Flow workflow
chore: set version to 0.1.0-dev
```

## Local checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI runs the same commands. A PR that fails CI is not reviewable.

## What belongs in a PR

See `.github/PULL_REQUEST_TEMPLATE.md`. In particular:

- User-visible CLI changes must update `--help` and the README.
- Anything that writes captures must consider secrets (see `SECURITY.md`).
- Do not add a web dashboard, TUI, HTTPS, or YAML rules until capture → replay → report is tested and documented.

## License

Contributions are dual-licensed MIT OR Apache-2.0, the same as the crate.
