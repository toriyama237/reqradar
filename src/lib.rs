//! ReqRadar capture engine and CLI.
//!
//! The binary is a thin `main`. Integration tests and future consumers talk to
//! this crate: the HTTP reverse proxy, the `.rrlog` store, and replay.

pub mod capture;
pub mod cli;
pub mod commands;
pub mod redact;
