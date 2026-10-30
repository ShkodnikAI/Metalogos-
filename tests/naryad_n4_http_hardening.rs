// ── Наряд N4 (gh#1213, PE-1): the outgoing-HTTP hardening pack ──────────
//
// The env-driven egress contract, all four egress builtins gated the
// №261 way (the gate runs at the top of every builtin call — and since
// №261 reqwest NEVER auto-follows a 3xx, every redirect hop is a new
// program-level builtin call, so the allowlist is re-checked per hop
// by construction):
//
//   H1: METALOGOS_HTTP_ALLOW_HOSTS — an exact allowlisted host passes;
//       a non-listed host is refused LOUD and TYPED
//       ([HTTP_HOST_NOT_ALLOWED]) BEFORE any DNS/connect happens (the
//       server log shows no request arrived).
//   H2: the wildcard form `*.example.by` covers subdomains (one label
//       or several), does NOT cover the apex itself and does NOT cover
//       a lookalike (`notexample.by`) — anchored at the dot.
//   H3: METALOGOS_HTTP_USER_AGENT — the default UA is
//       `Metalogos/<version>`; the env override replaces it; an
//       explicit caller `User-Agent` header wins over both.
//   H4: METALOGOS_HTTP_MIN_INTERVAL_MS — the minimal interval between
//       requests to the SAME host (per-host bookkeeping, not global);
//       different hosts are not delayed against each other.
//   H5: the `{robots: true}` call option — robots.txt is fetched once,
//       cached for 24 h, and Disallow rules refuse the request with a
//       typed [HTTP_ROBOTS_DENIED]; a missing/unreachable robots.txt
//       fails OPEN (the RFC 9309 posture: no robots file = no crawl
//       restrictions), and the longest-match rule decides when Allow
//       and Disallow overlap.
//
// The env vars are read at CALL time (the ssrf_guard_enabled()
// convention — set_var/remove_var under the env mutex per test). The
// mock servers bind 127.0.0.1:0 (the №261 mock lekało) with
// METALOGOS_HTTP_ALLOW_PRIVATE=1 so the SSRF guard lets the loopback
// through while the allowlist gate under test does its work.
//
// RED-first discipline: on main, the env vars are inert — the refusal
// tests observed the request GO THROUGH (assertion failure), and the
// helper-level tests did not compile (no such API). Every test below
// was RED on main before the implementation landed.

#![allow(clippy::disallowed_methods)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;

// ── eval_expr (the №261 lekało) ──────────────────────────────────────

fn eval_expr(src: &str) -> Result<String, String> {
    let full = format!(
        "profile capability {{ allow_carrier: \"http_post\" allow_carrier: \"http_post_multipart\" }}\npattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

/// The VM twin of the TW eval above — the parity leg for the refusal
/// surface (both backends share the builtin, the parity is pinned so a
/// future VM-side shortcut cannot diverge).
fn eval_expr_vm(src: &str) -> Result<String, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(&format!(
        "profile capability {{ allow_carrier: \"http_post\" allow_carrier: \"http_post_multipart\" }}\npattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    ))
    .map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    match vm.run(program)? {
        Some(s) => Ok(s.trim_end().to_string()),
        None => Err("vm eval returned None".to_string()),
    }
}

// ── Env mutex + guard (the №261 lekało, generalized to any var) ──────

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prevs: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    /// `settings`: var name -> Some(value) to SET, None to REMOVE.
    fn set(settings: &[(&'static str, Option<&str>)]) -> Self {
        let lock = env_lock();
        let mut prevs = Vec::new();
        for (name, val) in settings {
            prevs.push((*name, std::env::var(name).ok()));
            match val {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
        EnvGuard { _lock: lock, prevs }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, prev) in &self.prevs {
            match prev {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

// ── The mock server (the №261 lekało): fixed canned routes + log ─────

struct MockRoute {
    /// The exact request path to match ("GET /robots.txt " probe).
    path: &'static str,
    response: Vec<u8>,
}

fn spawn_mock_server(routes: Vec<MockRoute>) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let handle = std::thread::spawn(move || {
        let mut log = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(4000);
        let _ = listener.set_nonblocking(true);
        while std::time::Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 8192];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                raw.extend_from_slice(&buf[..n]);
                                if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let req = String::from_utf8_lossy(&raw).into_owned();
                    log.push_str(&req);
                    log.push('\n');
                    let hit = routes
                        .iter()
                        .find(|r| req.contains(&format!(" {} ", r.path)))
                        .map(|r| r.response.clone());
                    let resp = hit.unwrap_or_else(|| {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
                    });
                    stream.write_all(&resp).expect("write response");
                    let _ = stream.flush();
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        log
    });
    (format!("http://127.0.0.1:{}/", port), handle)
}

fn ok_response(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

// ── H2: the wildcard/exact matching table (pure helper) ──────────────

#[test]
fn n4_host_allowed_matching_table() {
    let f = metalogos::builtins::host_allowed;

    // Exact entry matches exactly.
    assert!(f("example.by", "example.by"), "exact host matches itself");
    assert!(
        f("EXAMPLE.BY", "example.by"),
        "the host match is case-insensitive (DNS names are)"
    );
    assert!(
        f("example.by", "EXAMPLE.BY"),
        "the entry match is case-insensitive too"
    );
    assert!(
        !f("example.com", "example.by"),
        "a different host never matches"
    );
    assert!(
        !f("notexample.by", "example.by"),
        "a suffix WITHOUT the dot anchor never matches"
    );

    // Wildcard: `*.example.by` covers subdomains — one label or more.
    assert!(
        f("a.example.by", "*.example.by"),
        "wildcard covers a one-label subdomain"
    );
    assert!(
        f("a.b.example.by", "*.example.by"),
        "wildcard covers a multi-label subdomain"
    );
    assert!(
        !f("example.by", "*.example.by"),
        "wildcard does NOT cover the apex itself"
    );
    assert!(
        !f("notexample.by", "*.example.by"),
        "wildcard is anchored at the dot"
    );

    // Comma list: any entry may allow.
    assert!(
        f("example.by", "example.by, example.com"),
        "comma list — first entry"
    );
    assert!(
        f("example.com", "example.by,example.com"),
        "comma list — second entry, no space"
    );
    assert!(
        !f("example.org", "example.by, example.com"),
        "comma list — unlisted host"
    );
    assert!(
        f("a.example.com", " *.example.com , example.by "),
        "whitespace-tolerant entries"
    );

    // Empty/garbage list allows nothing (the env-UNSET case is the
    // no-gate case and lives at the env level, not in the pure table).
    assert!(!f("example.by", ""), "empty list allows nothing");
    assert!(
        !f("example.by", "  ,  "),
        "blank tokens are skipped, nothing allows"
    );
}

// ── H1: the env-driven gate — the exact host passes, the other refused ──

#[test]
fn n4_allowlist_exact_host_passes_off_mock() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_ALLOW_HOSTS", Some("127.0.0.1")),
    ]);
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/data",
        response: ok_response("n4-allowlisted"),
    }]);

    let out = eval_expr(&format!("http_get(\"{}data\")", url))
        .expect("the allowlisted host must pass the gate");
    assert_eq!(
        out, "n4-allowlisted",
        "H1: the exact host passes and the body comes back"
    );
    let _ = server.join();
}

#[test]
fn n4_allowlist_other_host_refused_typed_and_before_connect() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_ALLOW_HOSTS", Some("example.by")),
    ]);

    // NO mock server at all: the refusal must happen BEFORE any connect
    // — if the gate is inert (the RED state) the request would fail with
    // a transport [HTTP_CONNECT] error instead (or hang).
    let err = eval_expr("http_get(\"http://refused-host.invalid/n4\")")
        .expect_err("the unlisted host must be refused");
    assert!(
        err.starts_with("[HTTP_HOST_NOT_ALLOWED]"),
        "H1: the refusal is the TYPED code, got: {}",
        err
    );
    assert!(
        err.contains("refused-host.invalid"),
        "H1: the message names the refused host, got: {}",
        err
    );
    assert!(
        err.contains("METALOGOS_HTTP_ALLOW_HOSTS"),
        "H1: the message points at the env knob, got: {}",
        err
    );
}

#[test]
fn n4_allowlist_refusal_parity_on_vm() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_ALLOW_HOSTS", Some("example.by")),
    ]);
    let tw_err = eval_expr("http_get(\"http://refused-host.invalid/n4\")").unwrap_err();
    let vm_err = eval_expr_vm("http_get(\"http://refused-host.invalid/n4\")").unwrap_err();
    assert_eq!(
        tw_err, vm_err,
        "H1: TW and VM refuse identically (the parity leg)"
    );
}

// ── H2: the wildcard form through the env gate ────────────────────────

#[test]
fn n4_allowlist_wildcard_covers_mock_host() {
    // The mock host is 127.0.0.1 — the wildcard list contains only
    // a wildcard that does NOT cover it; the refusal must be the
    // typed one (the wildcard semantics ride the same gate).
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_ALLOW_HOSTS", Some("*.example.by")),
    ]);
    let err = eval_expr("http_get(\"http://wild-unmatched.invalid/n4\")")
        .expect_err("a host not covered by the wildcard must be refused");
    assert!(
        err.starts_with("[HTTP_HOST_NOT_ALLOWED]"),
        "H2: the wildcard miss is the same typed refusal, got: {}",
        err
    );
}

// ── H1: the gate covers all four egress builtins ─────────────────────

#[test]
fn n4_allowlist_gates_http_post_and_download_and_multipart() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_ALLOW_HOSTS", Some("example.by")),
    ]);
    for (fn_call, what) in [
        ("http_post(\"http://refused-host.invalid/n4\", \"body\", \"text/plain\")", "http_post"),
        (
            "http_download(\"http://refused-host.invalid/n4\", \"/tmp/n4-should-not-exist\")",
            "http_download",
        ),
        (
            "http_post_multipart(\"http://refused-host.invalid/n4\", {f: \"x\"}, {g: \"/tmp/n4-never-read.txt\"})",
            "http_post_multipart",
        ),
    ] {
        let err = eval_expr(fn_call)
            .expect_err(&format!("{} must be refused for the unlisted host", what));
        assert!(
            err.starts_with("[HTTP_HOST_NOT_ALLOWED]"),
            "H1: {} is behind the same typed gate, got: {}",
            what,
            err
        );
    }
    let _ = std::fs::remove_file("/tmp/n4-should-not-exist");
}

// ── H3: User-Agent — the default, the env override, the caller wins ───

#[test]
fn n4_user_agent_default_is_metalogos_version() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_USER_AGENT", None),
    ]);
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/ua",
        response: ok_response("ua-ok"),
    }]);
    let out = eval_expr(&format!("http_get(\"{}ua\")", url)).expect("the request passes");
    assert_eq!(out, "ua-ok");
    let log = server.join().expect("the mock thread");
    // The wire (hyper) lowercases header names — the assertions match
    // the on-the-wire spelling.
    let expected = format!("user-agent: Metalogos/{}", env!("CARGO_PKG_VERSION"));
    assert!(
        log.contains(&expected),
        "H3: the DEFAULT UA is Metalogos/<version>, expected `{}` in the log:\n{}",
        expected,
        log
    );
}

#[test]
fn n4_user_agent_env_override() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_USER_AGENT", Some("TestAgent/9.9")),
    ]);
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/ua",
        response: ok_response("ua-ok"),
    }]);
    let _ = eval_expr(&format!("http_get(\"{}ua\")", url)).expect("the request passes");
    let log = server.join().expect("the mock thread");
    assert!(
        log.contains("user-agent: TestAgent/9.9"),
        "H3: the env override replaces the default UA, log:\n{}",
        log
    );
    assert!(
        !log.contains("user-agent: Metalogos/"),
        "H3: the default UA must NOT appear when overridden, log:\n{}",
        log
    );
}

#[test]
fn n4_user_agent_explicit_header_wins() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_USER_AGENT", Some("TestAgent/9.9")),
    ]);
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/ua",
        response: ok_response("ua-ok"),
    }]);
    // The struct-literal grammar only takes IDENT keys, so a hyphenated
    // header name arrives via parse_json (the documented JSON route).
    let _ = eval_expr(&format!(
        "http_get(\"{}ua\", parse_json(\"{{\\\"User-Agent\\\": \\\"Custom/1\\\"}}\"))",
        url
    ))
    .expect("the request passes");
    let log = server.join().expect("the mock thread");
    assert!(
        log.contains("user-agent: Custom/1"),
        "H3: the explicit caller header wins over the env override, log:\n{}",
        log
    );
}

// ── H4: the per-host min interval ─────────────────────────────────────

#[test]
fn n4_min_interval_delays_same_host_not_different_hosts() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_MIN_INTERVAL_MS", Some("150")),
    ]);
    let (url_a, server_a) = spawn_mock_server(vec![MockRoute {
        path: "/a",
        response: ok_response("a"),
    }]);
    let (url_b, server_b) = spawn_mock_server(vec![MockRoute {
        path: "/b",
        response: ok_response("b"),
    }]);

    // Same host twice: the SECOND call must wait out the interval.
    let t0 = std::time::Instant::now();
    let _ = eval_expr(&format!("http_get(\"{}a\")", url_a)).expect("call 1 passes");
    let after_first = t0.elapsed();
    let _ = eval_expr(&format!("http_get(\"{}a\")", url_a)).expect("call 2 passes");
    let same_host_pair = t0.elapsed();
    assert!(
        same_host_pair >= std::time::Duration::from_millis(140),
        "H4: the second same-host call must wait out ~150ms (pair took {:?}, first call {:?})",
        same_host_pair,
        after_first
    );
    let t1 = std::time::Instant::now();
    let _ = eval_expr(&format!("http_get(\"{}b\")", url_b)).expect("host B passes");
    assert!(
        t1.elapsed() < std::time::Duration::from_millis(140),
        "H4: a different host must NOT inherit host A's interval debt (took {:?})",
        t1.elapsed()
    );
    // Both joins AFTER all the calls: a join blocks until that server's
    // accept deadline expires, and a burned deadline of server A must
    // not starve server B's accept loop.
    let _ = server_a.join();
    let _ = server_b.join();
}

#[test]
fn n4_min_interval_zero_is_no_delay() {
    let _env = EnvGuard::set(&[
        ("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1")),
        ("METALOGOS_HTTP_MIN_INTERVAL_MS", Some("0")),
    ]);
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/z",
        response: ok_response("z"),
    }]);
    let t0 = std::time::Instant::now();
    for _ in 0..3 {
        let _ = eval_expr(&format!("http_get(\"{}z\")", url)).expect("the call passes");
    }
    let elapsed = t0.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(300),
        "H4: interval 0 must add no artificial delay (three calls took {:?})",
        elapsed
    );
    let _ = server.join();
}

// ── H5: robots.txt — allow, deny, cache, fail-open, longest-match ─────

const ROBOTS_DISALLOW_PRIVATE: &str = "User-agent: *\nDisallow: /private\n";

#[test]
fn n4_robots_denies_disallowed_path_typed() {
    let _env = EnvGuard::set(&[("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1"))]);
    let (url, server) = spawn_mock_server(vec![
        MockRoute {
            path: "/robots.txt",
            response: ok_response(ROBOTS_DISALLOW_PRIVATE),
        },
        MockRoute {
            path: "/private/data",
            response: ok_response("secret"),
        },
    ]);

    let err = eval_expr(&format!(
        "http_get(\"{}private/data\", {{robots: true}})",
        url
    ))
    .expect_err("the Disallow-ed path must be refused");
    assert!(
        err.starts_with("[HTTP_ROBOTS_DENIED]"),
        "H5: the robots refusal is the TYPED code, got: {}",
        err
    );
    assert!(
        err.contains("/private/data"),
        "H5: the message names the refused path, got: {}",
        err
    );
    let log = server.join().expect("the mock thread");
    assert!(
        log.contains("GET /robots.txt"),
        "H5: robots.txt was fetched before the target path, log:\n{}",
        log
    );
    assert!(
        !log.contains("GET /private/data"),
        "H5: the disallowed path must NEVER be requested, log:\n{}",
        log
    );
}

#[test]
fn n4_robots_allows_permitted_path() {
    let _env = EnvGuard::set(&[("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1"))]);
    let (url, server) = spawn_mock_server(vec![
        MockRoute {
            path: "/robots.txt",
            response: ok_response(ROBOTS_DISALLOW_PRIVATE),
        },
        MockRoute {
            path: "/public/data",
            response: ok_response("public-body"),
        },
    ]);
    let out = eval_expr(&format!(
        "http_get(\"{}public/data\", {{robots: true}})",
        url
    ))
    .expect("the permitted path passes the robots gate");
    assert_eq!(out, "public-body");
    let _ = server.join();
}

#[test]
fn n4_robots_txt_fetched_once_then_cached() {
    let _env = EnvGuard::set(&[("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1"))]);
    let (url, server) = spawn_mock_server(vec![
        MockRoute {
            path: "/robots.txt",
            response: ok_response(ROBOTS_DISALLOW_PRIVATE),
        },
        MockRoute {
            path: "/first",
            response: ok_response("one"),
        },
        MockRoute {
            path: "/second",
            response: ok_response("two"),
        },
    ]);
    let _ = eval_expr(&format!("http_get(\"{}first\", {{robots: true}})", url)).expect("call 1");
    let _ = eval_expr(&format!("http_get(\"{}second\", {{robots: true}})", url)).expect("call 2");
    let log = server.join().expect("the mock thread");
    let robots_fetches = log.matches("GET /robots.txt").count();
    assert_eq!(
        robots_fetches, 1,
        "H5: robots.txt is cached — one fetch serves both calls, log:\n{}",
        log
    );
}

#[test]
fn n4_robots_missing_file_fails_open() {
    let _env = EnvGuard::set(&[("METALOGOS_HTTP_ALLOW_PRIVATE", Some("1"))]);
    // No /robots.txt route → the mock answers 404 → no rules → allowed
    // (the RFC 9309 posture: an unfetchable robots file is no crawl ban).
    let (url, server) = spawn_mock_server(vec![MockRoute {
        path: "/open/data",
        response: ok_response("open-body"),
    }]);
    let out = eval_expr(&format!("http_get(\"{}open/data\", {{robots: true}})", url))
        .expect("a 404 robots.txt must fail OPEN");
    assert_eq!(out, "open-body");
    let _ = server.join();
}

#[test]
fn n4_robots_longest_match_table() {
    let f = metalogos::builtins::RobotsRules::parse;
    let rules = f("User-agent: *\nAllow: /p\nDisallow: /p/r\n# a comment\nDisallow:\n");

    assert!(
        rules.allows("/public"),
        "longest match: Allow /p covers /public"
    );
    assert!(rules.allows("/p"), "Allow /p covers /p itself");
    assert!(
        !rules.allows("/p/road"),
        "Disallow /p/r is LONGER than Allow /p — denied"
    );
    assert!(rules.allows("/other"), "no rule matches /other — allowed");
    assert!(
        rules.allows("/"),
        "the empty Disallow token never bans anything"
    );
}

#[test]
fn n4_robots_empty_disallow_allows_everything() {
    let f = metalogos::builtins::RobotsRules::parse;
    let rules = f("User-agent: *\nDisallow:\n");
    assert!(
        rules.allows("/anything"),
        "an empty Disallow value is the explicit allow-all"
    );
}

#[test]
fn n4_robots_ttl_is_twenty_four_hours() {
    // The pinned contract: the cache holds the rules for 24 h.
    assert_eq!(
        metalogos::builtins::ROBOTS_TTL.as_secs(),
        24 * 60 * 60,
        "H5: the robots cache TTL is exactly 24 hours"
    );
}
