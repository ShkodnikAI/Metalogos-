//! metalogos-server — the transport crate: the HTTP server, the MCP
//! server and the `mlog` CLI binary (the №472 roadmap stage 2 made
//! physical; naryad №567, Wave 26, issue #931).
//!
//! №567 moved the contour 1:1 out of the language crate:
//!
//! - `server` — the axum runtime (`mlog serve`): ServerState, the
//!   route machinery, the session/CSRF/rate-limit substrate, the
//!   serve-e2e harness (`run_test_server*`, both backends);
//! - `mcp_server` — the MCP transports (stdio / http / sse);
//! - `mcp_policy` — the tool policy compile layer (№316 SSOT);
//! - `bin/mlog.rs` — the CLI binary itself (the name and the surface
//!   are unchanged).
//!
//! # Why the bin moved (the structural finding)
//!
//! The Cargo package graph forbids the cycle `metalogos ->
//! metalogos-server -> metalogos`. The №545 precedent (metalogos-reflex)
//! dodged it because the reflex crate is a LEAF (Value-free by design);
//! the transport consumes the language core by essence (Value,
//! Compiler, Vm, the builtin registry). The only cycle-free shape keeps
//! the consumer edge inside THIS package: the bin links the language
//! crate for the core and its own lib for the transport. The root
//! crate became a pure library — it no longer carries axum/tokio at
//! all, which satisfies the №278 no-default-features criterion a
//! fortiori (the №567 verification: `cargo tree -e features`).
//!
//! # The no-API-change contract
//!
//! Every `metalogos::server::*` / `metalogos::mcp_server::*` /
//! `metalogos::mcp_policy::*` path became
//! `metalogos_server::...`; the 38 consumer test files moved WITH the
//! modules in the same change (the paths re-bound mechanically, the
//! assertions untouched). The dependency direction is
//! TRANSPORT -> CORE (this crate depends on the language crate; the
//! language crate depends on NOTHING here — enforced by Cargo).

pub mod mcp_policy;
pub mod mcp_server;

/// The axum HTTP runtime (`mlog serve`) — feature-gated exactly like
/// the root crate gated it before the split; `--no-default-features`
/// yields the mcp-stdio-only profile.
#[cfg(feature = "server")]
pub mod server;

/// Serve a .mlog program as an HTTP server.
/// Parses the source, finds the mlogserver block, and starts Axum.
#[cfg(feature = "server")]
pub async fn serve_program(source: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run_server(source).await
}
