//! №855 (issue #1680, wave 74 — W-C, ADR-0204 §3/§4): the `wasm32`
//! facade of the Portable Refusal Algebra.
//!
//! The portable kernel is the DEPENDENCY-FREE algebra source of the
//! root crate — `src/refusal_algebra.rs` (the join/meet lattice over
//! the frozen 14-word `DENY_REASONS` vocabulary, the verdicts) —
//! included here verbatim via `#[path]` (ONE SSOT source, two compile
//! targets; no crate edge, no language-tree pull). This crate adds
//! only the consumption boundary: a JSON request/response contract
//! behind a minimal C ABI.
//!
//! # Build
//!
//! ```text
//! rustup target add wasm32-unknown-unknown
//! cargo build -p refusal-wasm --target wasm32-unknown-unknown --release
//! # → target/wasm32-unknown-unknown/release/refusal_wasm.wasm
//! ```
//!
//! # The C-ABI contract
//!
//! | export | signature | meaning |
//! |---|---|---|
//! | `rw_pool_len` | `() -> u32` | the input-pool size in bytes (65536) |
//! | `rw_pool_base` | `() -> u32` | the ABSOLUTE linear-memory address of the input pool |
//! | `rw_alloc` | `(len: u32) -> u32` | reserve `len` pool bytes; returns the ABSOLUTE address for the request, `u32::MAX` on overflow |
//! | `rw_eval` | `(ptr: u32, len: u32) -> u64` | evaluate the request at the ABSOLUTE address; returns `(response_addr << 32) \| response_len` |
//! | `rw_reset` | `() -> ()` | reset the pool cursor (between requests) |
//!
//! The host writes the request bytes into the exported linear memory at
//! the `rw_alloc` address, calls `rw_eval`, then reads the response from
//! `response_addr` (the low 32 bits of the packed result give the length,
//! the high 32 the address in the same linear memory). `refusal-wasm/smoke.mjs`
//! is the working consumer (node, no Rust in the loop) — the CI
//! `wasm-facade (blocking)` job runs it on every push.
//!
//! # The JSON contract
//!
//! Request: `{"op": "join" | "meet" | "verdict", "reasons": ["WORD", ...]}`
//! — `join` folds the MORE SEVERE cause, `meet` the LESS SEVERE, an
//! empty `reasons` list is `Allow`.
//!
//! Response: `{"verdict": "allow"}` |
//! `{"verdict": "deny", "code": "WORD", "cause": "WORD"}` |
//! `{"verdict": "error", "detail": "..."}` — the error verdict is the
//! fail-closed loud refusal for a malformed request or an unknown
//! reason word (the algebra is CLOSED over the frozen vocabulary,
//! ADR-0204 §6: no new codes).
//!
//! # Honest limits (ADR-0204 §6)
//!
//! The facade computes verdicts over the causes it is GIVEN — it does
//! not observe the world (no IO, no network, no clock), it does not
//! record to the Ledger (the host decides what to do with a verdict),
//! and it does not compile the METALOGOS language. Publishing the
//! artifact as an npm/crate package is the owner's act, not this
//! naryad's.

// The facade consumes the parse/join/meet surface of the shared SSOT
// source; the Verdict type and the rank accessor stay for the root
// crate — the facade-side dead-code is silenced (the #[path] seam's
// only trade, documented here).
#[allow(dead_code)]
#[path = "../../src/refusal_algebra.rs"]
mod refusal_algebra;

use serde::Deserialize;
use serde::Serialize;

/// The request shape (`serde`): one algebra operation over a list of
/// reason words.
#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    reasons: Vec<String>,
}

/// The response shape: `allow` (no code), `deny` (the frozen code +
/// cause), `error` (the fail-closed loud refusal).
#[derive(Debug, PartialEq, Serialize)]
struct Response {
    verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

fn allow() -> Response {
    Response {
        verdict: "allow".to_string(),
        code: None,
        cause: None,
        detail: None,
    }
}

fn deny(word: &str) -> Response {
    Response {
        verdict: "deny".to_string(),
        code: Some(word.to_string()),
        cause: Some(word.to_string()),
        detail: None,
    }
}

fn error(detail: &str) -> Response {
    Response {
        verdict: "error".to_string(),
        code: None,
        cause: None,
        detail: Some(detail.to_string()),
    }
}

/// Fold the requested reasons through the algebra. The error verdict is
/// fail-closed: the FIRST unknown word refuses the whole request with
/// the loud detail (no silent skipping).
fn evaluate_request(req: &Request) -> Response {
    let mut parsed = Vec::with_capacity(req.reasons.len());
    for word in &req.reasons {
        match refusal_algebra::Reason::parse(word) {
            Some(r) => parsed.push(r),
            None => return error(&format!("unknown reason word: {word}")),
        }
    }
    let folded = match parsed.first() {
        None => return allow(),
        Some(first) => parsed[1..].iter().fold(*first, |acc, r| {
            if req.op == "meet" {
                refusal_algebra::meet(acc, *r)
            } else {
                refusal_algebra::join(acc, *r)
            }
        }),
    };
    deny(folded.word())
}

/// The pure host-side entry (also the unit-test seam): request bytes in,
/// response bytes out. A malformed request is an `error` verdict, never
/// a panic.
fn evaluate(req: &[u8]) -> Vec<u8> {
    let response = match serde_json::from_slice::<Request>(req) {
        Ok(parsed) => {
            if parsed.op != "join" && parsed.op != "meet" && parsed.op != "verdict" {
                error(&format!(
                    "unknown op: {} (expected join|meet|verdict)",
                    parsed.op
                ))
            } else {
                evaluate_request(&parsed)
            }
        }
        Err(e) => error(&format!("malformed request JSON: {e}")),
    };
    response.render()
}

// ── The C-ABI boundary (wasm32-unknown-unknown) ─────────────────────
//
// The input pool: a fixed static buffer the host writes into via the
// exported linear memory (`rw_alloc` returns the offset). The output
// buffer: a fresh Vec per call — the packed result carries its
// offset+length, and the Vec stays alive (the static) until the host
// reads it. wasm32 is single-threaded here; the Mutexes are the
// clippy-clean interior mutability, never contended.

use std::sync::Mutex;

const POOL_LEN: usize = 65_536;

struct Pool {
    buf: [u8; POOL_LEN],
    cursor: usize,
}

static POOL: Mutex<Pool> = Mutex::new(Pool {
    buf: [0; POOL_LEN],
    cursor: 0,
});
static OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn pool() -> std::sync::MutexGuard<'static, Pool> {
    POOL.lock().unwrap_or_else(|p| p.into_inner())
}

/// The input-pool size in bytes.
#[no_mangle]
pub extern "C" fn rw_pool_len() -> u32 {
    POOL_LEN as u32
}

/// The ABSOLUTE linear-memory address of the input pool.
#[no_mangle]
pub extern "C" fn rw_pool_base() -> u32 {
    pool().buf.as_ptr() as u32
}

/// Reserve `len` bytes of the input pool; returns the ABSOLUTE
/// linear-memory address for the request bytes, `u32::MAX` when the
/// pool is exhausted.
#[no_mangle]
pub extern "C" fn rw_alloc(len: u32) -> u32 {
    let mut p = pool();
    if len as usize > POOL_LEN.saturating_sub(p.cursor) {
        return u32::MAX;
    }
    let addr = (p.buf.as_ptr() as u32).wrapping_add(p.cursor as u32);
    p.cursor += len as usize;
    addr
}

/// Reset the input-pool cursor (between requests).
#[no_mangle]
pub extern "C" fn rw_reset() {
    pool().cursor = 0;
}

/// Evaluate the request at the ABSOLUTE (32-bit) linear-memory address;
/// returns `(response_addr << 32) | response_len` — both into the
/// exported linear memory. The address arithmetic is 32-bit on BOTH
/// sides (`wasm32` semantics; the host build truncates identically, so
/// the host unit tests exercise the same path). A malformed request or
/// an out-of-pool address yields the `error` verdict response, never a
/// trap.
#[no_mangle]
pub extern "C" fn rw_eval(ptr: u32, len: u32) -> u64 {
    let req: Vec<u8> = {
        let p = pool();
        let base = p.buf.as_ptr() as u32;
        let rel = ptr.checked_sub(base);
        match rel {
            Some(s) if (s as usize) + (len as usize) <= POOL_LEN => {
                p.buf[s as usize..(s as usize + len as usize)].to_vec()
            }
            _ => {
                let mut out = OUT.lock().unwrap_or_else(|e| e.into_inner());
                *out = error("input address out of the pool bounds").render();
                let rlen = out.len();
                let roff = out.as_ptr() as u64;
                return (roff << 32) | rlen as u64;
            }
        }
    };
    let mut out = OUT.lock().unwrap_or_else(|e| e.into_inner());
    *out = evaluate(&req);
    let rlen = out.len();
    let roff = out.as_ptr() as u64;
    (roff << 32) | rlen as u64
}

impl Response {
    fn render(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap_or_else(|_| {
            br#"{"verdict":"error","detail":"response serialization failed"}"#.to_vec()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_json(v: serde_json::Value) -> serde_json::Value {
        serde_json::from_slice(&evaluate(v.to_string().as_bytes())).unwrap()
    }

    #[test]
    fn join_folds_the_more_severe_cause() {
        let r = eval_json(serde_json::json!({
            "op": "join",
            "reasons": ["SECRET_LEAK", "SINK_CLEARANCE"]
        }));
        assert_eq!(r["verdict"], "deny");
        assert_eq!(r["code"], "SINK_CLEARANCE");
        assert_eq!(r["cause"], "SINK_CLEARANCE");
    }

    #[test]
    fn meet_folds_the_less_severe_cause() {
        let r = eval_json(serde_json::json!({
            "op": "meet",
            "reasons": ["SECRET_LEAK", "SINK_CLEARANCE"]
        }));
        assert_eq!(r["verdict"], "deny");
        assert_eq!(r["code"], "SECRET_LEAK");
    }

    #[test]
    fn empty_reasons_are_allow_and_unknown_words_refuse_loud() {
        let r = eval_json(serde_json::json!({"op": "verdict", "reasons": []}));
        assert_eq!(r["verdict"], "allow");
        let r = eval_json(serde_json::json!({"op": "verdict", "reasons": ["NOT_A_REASON"]}));
        assert_eq!(r["verdict"], "error");
        assert!(r["detail"].as_str().unwrap().contains("NOT_A_REASON"));
    }

    #[test]
    fn malformed_requests_fail_closed() {
        let r: serde_json::Value = serde_json::from_slice(&evaluate(b"not json at all")).unwrap();
        assert_eq!(r["verdict"], "error");
        let r = eval_json(serde_json::json!({"op": "explode", "reasons": []}));
        assert_eq!(r["verdict"], "error");
    }

    #[test]
    fn the_c_abi_roundtrip_works_on_the_host() {
        // The same path the wasm smoke drives: rw_alloc → write →
        // rw_eval → read the packed response.
        let raw = br#"{"op":"join","reasons":["IRREVERSIBLE_NO_GRANT","SECRET_LEAK"]}"#;
        rw_reset();
        let base = pool().buf.as_ptr() as u32;
        let addr = rw_alloc(raw.len() as u32);
        assert_ne!(addr, u32::MAX);
        assert_eq!(addr, base);
        // The host writes into the "linear memory" at the ABSOLUTE address.
        pool().buf[0..raw.len()].copy_from_slice(raw);
        let packed = rw_eval(addr, raw.len() as u32);
        let rlen = (packed & 0xffff_ffff) as usize;
        let out = OUT.lock().unwrap_or_else(|e| e.into_inner());
        let parsed: serde_json::Value = serde_json::from_slice(&out[..rlen]).unwrap();
        assert_eq!(parsed["verdict"], "deny");
        assert_eq!(parsed["code"], "IRREVERSIBLE_NO_GRANT");
    }

    #[test]
    fn the_pool_bounds_refuse_loud_not_trap() {
        rw_reset();
        // The address is computed BEFORE the call — the pool guard must
        // not be alive across rw_eval (a non-reentrant Mutex would
        // deadlock the caller).
        let bad = pool().buf.as_ptr() as u32 + POOL_LEN as u32 + 1;
        let packed = rw_eval(bad, 4);
        let rlen = (packed & 0xffff_ffff) as usize;
        let out = OUT.lock().unwrap_or_else(|e| e.into_inner());
        let parsed: serde_json::Value = serde_json::from_slice(&out[..rlen]).unwrap();
        assert_eq!(parsed["verdict"], "error");
        assert!(parsed["detail"].as_str().unwrap().contains("bounds"));
    }
}
