// ── НАРЯД №130 Contract Tests: SSRF guard for http_get/http_post ────────
// Contracts:
//   C1: http_get("http://127.0.0.1:22") → rejected (loopback).
//   C2: http_get("http://169.254.169.254/latest/meta-data/") → rejected (cloud metadata).
//   C3: Normal external URL (public IP literal) → guard passes, no regression.
//   C4: With METALOGOS_HTTP_ALLOW_PRIVATE=1 → private address allowed.
//
// Tests exercise check_url_ssrf (DNS resolution + IP check) and
// is_blocked_address (IP-level classification) directly.
// No real HTTP requests are made — safe for offline CI.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// ── C1: Loopback IPv4 — blocked ──────────────────────────────────────────

#[test]
fn test_ssrf_blocks_loopback_ipv4() {
    // Ensure opt-out is NOT set
    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    let result = metalogos::builtins::check_url_ssrf("http://127.0.0.1:22/");
    assert!(result.is_err(), "C1: 127.0.0.1 should be blocked");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("SSRF guard"),
        "C1: error should mention SSRF guard, got: {}",
        msg
    );
    assert!(
        msg.contains("private/loopback/link-local"),
        "C1: error should mention blocked address class, got: {}",
        msg
    );
}

// ── C2: Cloud metadata endpoint — blocked ──────────────────────────────

#[test]
fn test_ssrf_blocks_cloud_metadata() {
    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    let result = metalogos::builtins::check_url_ssrf("http://169.254.169.254/latest/meta-data/");
    assert!(
        result.is_err(),
        "C2: 169.254.169.254 (cloud metadata) should be blocked"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("169.254.169.254"),
        "C2: error should contain the blocked IP, got: {}",
        msg
    );
}

// ── C3: is_blocked_address — verify IP classification + public IP passes ──

#[test]
fn test_is_blocked_address_classification() {
    let blocked = metalogos::builtins::is_blocked_address;

    // ── Blocked addresses ──
    // Loopback
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        "C3: 127.0.0.1 (loopback) must be blocked"
    );
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))),
        "C3: 127.255.255.255 (loopback) must be blocked"
    );

    // Private ranges
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
        "C3: 10.0.0.1 (private) must be blocked"
    );
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))),
        "C3: 172.16.0.1 (private) must be blocked"
    );
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))),
        "C3: 172.31.255.255 (private) must be blocked"
    );
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
        "C3: 192.168.1.1 (private) must be blocked"
    );

    // Link-local
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))),
        "C3: 169.254.1.1 (link-local) must be blocked"
    );

    // Cloud metadata (explicit check, beyond is_link_local)
    assert!(
        blocked(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))),
        "C3: 169.254.169.254 (cloud metadata) must be blocked"
    );

    // IPv6 loopback
    assert!(
        blocked(&IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))),
        "C3: ::1 (v6 loopback) must be blocked"
    );

    // IPv6 link-local
    assert!(
        blocked(&"fe80::1".parse::<IpAddr>().unwrap()),
        "C3: fe80::1 (v6 link-local) must be blocked"
    );

    // ── NOT blocked (public IPs) ──
    assert!(
        !blocked(&"8.8.8.8".parse::<IpAddr>().unwrap()),
        "C3: 8.8.8.8 (public DNS) must NOT be blocked"
    );
    assert!(
        !blocked(&"1.1.1.1".parse::<IpAddr>().unwrap()),
        "C3: 1.1.1.1 (public) must NOT be blocked"
    );
    assert!(
        !blocked(&"203.0.113.1".parse::<IpAddr>().unwrap()),
        "C3: 203.0.113.1 (documentation range, not private) must NOT be blocked"
    );
}

/// C3 extension: public IP literal URL passes the full check_url_ssrf.
/// Uses 8.8.8.8 (IP literal, no DNS lookup needed — safe in offline CI).
#[test]
fn test_ssrf_public_ip_literal_passes() {
    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    let result = metalogos::builtins::check_url_ssrf("http://8.8.8.8:80/");
    assert!(
        result.is_ok(),
        "C3: 8.8.8.8 (public IP) should pass SSRF guard"
    );
    let resolves = result.unwrap();
    // Should return pinned resolves for reqwest
    assert!(
        !resolves.is_empty(),
        "C3: should return pinned resolves to prevent DNS rebinding"
    );
    assert_eq!(
        resolves[0].0, "8.8.8.8",
        "C3: resolve domain should match host"
    );
}

// ── C4: METALOGOS_HTTP_ALLOW_PRIVATE=1 → private address allowed ────────

#[test]
fn test_ssrf_allow_private_opt_out() {
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");

    let result = metalogos::builtins::check_url_ssrf("http://127.0.0.1:22/");

    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    assert!(
        result.is_ok(),
        "C4: with METALOGOS_HTTP_ALLOW_PRIVATE=1, 127.0.0.1 should be allowed"
    );
    // When guard is disabled, returns empty resolves (reqwest uses default DNS)
    assert!(
        result.unwrap().is_empty(),
        "C4: disabled guard should return empty resolves"
    );
}

/// C4 extension: opt-out also works for cloud metadata
#[test]
fn test_ssrf_allow_private_cloud_metadata() {
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");

    let result = metalogos::builtins::check_url_ssrf("http://169.254.169.254/latest/meta-data/");

    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    assert!(
        result.is_ok(),
        "C4: with opt-out, cloud metadata URL should be allowed"
    );
}

/// Invalid URL → SSRF guard returns error (not panic)
#[test]
fn test_ssrf_invalid_url() {
    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    let result = metalogos::builtins::check_url_ssrf("not-a-url");
    assert!(
        result.is_err(),
        "invalid URL should be rejected by SSRF guard"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("SSRF guard"),
        "error should mention SSRF guard, got: {}",
        msg
    );
}

/// URL with no host → SSRF guard returns error
#[test]
fn test_ssrf_url_no_host() {
    std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE");

    let result = metalogos::builtins::check_url_ssrf("file:///etc/passwd");
    // file:// URLs have empty host in some parsers, or the host is empty
    // Either way, it should be caught
    assert!(
        result.is_err(),
        "file:// URL should be rejected (no HTTP/HTTPS host)"
    );
}
