// ── НАРЯД №150 Contract Tests: IPv6 ULA in SSRF-guard ─────────────────
// Contracts:
//   C1: fd00::1 (ULA, fc00::/7) → blocked.
//   C2: fc00::dead:beef (ULA, fc prefix) → blocked.
//   C3: Public IPv6 (2001:4860:4860::8888) → NOT blocked (no regression).
//   C4: Existing v6 loopback/link-local still blocked (regression guard).

use std::net::IpAddr;

// ── C1: ULA fd00::1 → blocked ────────────────────────────────────────────

#[test]
fn test_ssrf_blocks_ula_fd00() {
    let blocked = metalogos::builtins::is_blocked_address;
    let ula: IpAddr = "fd00::1".parse().unwrap();
    assert!(blocked(&ula), "C1: fd00::1 (ULA) must be blocked");
}

// ── C2: ULA fc00::dead:beef (fc prefix) → blocked ───────────────────────

#[test]
fn test_ssrf_blocks_ula_fc00() {
    let blocked = metalogos::builtins::is_blocked_address;
    let ula: IpAddr = "fc00::dead:beef".parse().unwrap();
    assert!(
        blocked(&ula),
        "C2: fc00::dead:beef (ULA, fc prefix) must be blocked"
    );
}

// ── C3: Public IPv6 → NOT blocked (no regression) ──────────────────────

#[test]
fn test_ssrf_public_v6_passes() {
    let blocked = metalogos::builtins::is_blocked_address;

    // Google Public DNS IPv6
    let google: IpAddr = "2001:4860:4860::8888".parse().unwrap();
    assert!(
        !blocked(&google),
        "C3: 2001:4860:4860::8888 (public) must NOT be blocked"
    );

    // Cloudflare DNS IPv6
    let cloudflare: IpAddr = "2606:4700:4700::1111".parse().unwrap();
    assert!(
        !blocked(&cloudflare),
        "C3: 2606:4700:4700::1111 (public) must NOT be blocked"
    );

    // Documentation range 2001:db8::1 — not private, not ULA
    let doc: IpAddr = "2001:db8::1".parse().unwrap();
    assert!(
        !blocked(&doc),
        "C3: 2001:db8::1 (documentation) must NOT be blocked"
    );
}

// ── C4: Existing v6 loopback/link-local still blocked ───────────────────

#[test]
fn test_ssrf_v6_loopback_and_link_local_still_blocked() {
    let blocked = metalogos::builtins::is_blocked_address;

    // Loopback
    let lo: IpAddr = "::1".parse().unwrap();
    assert!(
        blocked(&lo),
        "C4: ::1 (v6 loopback) must still be blocked"
    );

    // Link-local
    let ll: IpAddr = "fe80::1".parse().unwrap();
    assert!(
        blocked(&ll),
        "C4: fe80::1 (v6 link-local) must still be blocked"
    );
}
