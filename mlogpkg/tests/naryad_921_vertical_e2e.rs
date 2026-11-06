#![allow(clippy::disallowed_methods)]

// ── mlogpkg/tests/naryad_921_vertical_e2e.rs ────────────────────────
// №921 (Wave 84 — W-L2, ADR-0220 §2): the package-path e2e of the
// fail-closed vertical consumption. The naryad's done-when: a package
// declaring `[vertical]` payments at the LIVE profile pin verifies
// GREEN (mlogpkg verify, the №911 inheritance — the verdict rides
// AFTER the integrity chain); a broken digest / an unknown id refuses
// with the NAMED divergence BEFORE any effect; a manifest WITHOUT the
// `[vertical]` table verifies exactly as before (no default attach —
// zero behavioral mutations).
//
// The flow is the real binary (CARGO_BIN_EXE_mlogpkg): keygen → pack →
// sign → verify, in a temporary package directory; the key lives
// OUTSIDE the package tree (the digest document hashes the tree, the
// key is not package content).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlogpkg")
}

fn run_in(dir: &PathBuf, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the mlogpkg binary must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// A minimal verifiable package: mlog.toml (+ the optional `[vertical]`
/// declaration) + the entry point.
fn setup_package(vertical_table: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "n921-vertical-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("src")).expect("the package dir creates");
    let vertical = vertical_table.unwrap_or("");
    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"vtest\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{vertical}"
        ),
    )
    .expect("the manifest writes");
    fs::write(
        dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .expect("the entry writes");
    dir
}

fn key_outside(dir: &PathBuf, name: &str) -> PathBuf {
    let key = dir.parent().unwrap().join(name);
    let (code, _, err) = run_in(dir, &["keygen", "--out", key.to_str().unwrap()]);
    assert_eq!(code, 0, "keygen must succeed: {err}");
    key
}

fn pack_sign(dir: &PathBuf, key: &PathBuf) {
    let (code, _, err) = run_in(dir, &["pack"]);
    assert_eq!(code, 0, "pack must succeed: {err}");
    let (code, _, err) = run_in(dir, &["sign", "--key", key.to_str().unwrap()]);
    assert_eq!(code, 0, "sign must succeed: {err}");
}

#[test]
fn a_declared_vertical_at_the_live_pin_verifies_green() {
    let dir = setup_package(Some(
        "\n[vertical]\nid = \"payments\"\ndigest = \"LIVE_PIN_PLACEHOLDER\"\n",
    ));
    let key = key_outside(&dir, "n921-green-key.hex");
    // The LIVE pin: the payments profile's carrier digest (the
    // self-describing G5 pin re-derived from the live registry).
    let live_pin = metalogos::vertical::vertical_registry()[0]
        .carrier_digest
        .clone();
    let manifest = dir.join("mlog.toml");
    let text = fs::read_to_string(&manifest).unwrap().replace(
        "LIVE_PIN_PLACEHOLDER",
        &live_pin,
    );
    fs::write(&manifest, text).unwrap();
    pack_sign(&dir, &key);
    let (code, out, err) = run_in(&dir, &["verify", "--key", key.to_str().unwrap()]);
    assert_eq!(code, 0, "verify must be GREEN at the live pin:\n{out}{err}");
    assert!(
        out.contains("vertical OK: payments verified"),
        "the verify output must carry the vertical verdict line:\n{out}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_declared_vertical_with_a_broken_digest_refuses_named() {
    let dir = setup_package(Some(
        "\n[vertical]\nid = \"payments\"\ndigest = \"BROKEN_PIN_PLACEHOLDER\"\n",
    ));
    // The broken pin: 64 hex chars that do NOT match the live profile.
    let manifest = dir.join("mlog.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap()
        .replace("BROKEN_PIN_PLACEHOLDER", &format!("{}", "ab".repeat(32)));
    fs::write(&manifest, text).unwrap();
    let key = key_outside(&dir, "n921-broken-key.hex");
    pack_sign(&dir, &key);
    let (code, out, err) = run_in(&dir, &["verify", "--key", key.to_str().unwrap()]);
    assert_ne!(code, 0, "a broken digest must refuse:\n{out}{err}");
    assert!(
        err.contains("vertical: vertical digest pin mismatch"),
        "the refusal must NAME the divergence:\n{err}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_declared_vertical_with_an_unknown_id_refuses_named() {
    let dir = setup_package(Some(
        "\n[vertical]\nid = \"medicine\"\ndigest = \"UNKNOWN_ID_PLACEHOLDER\"\n",
    ));
    let manifest = dir.join("mlog.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap()
        .replace("UNKNOWN_ID_PLACEHOLDER", &format!("{}", "00".repeat(32)));
    fs::write(&manifest, text).unwrap();
    let key = key_outside(&dir, "n921-unknown-key.hex");
    pack_sign(&dir, &key);
    let (code, out, err) = run_in(&dir, &["verify", "--key", key.to_str().unwrap()]);
    assert_ne!(code, 0, "an unknown id must refuse:\n{out}{err}");
    assert!(
        err.contains("vertical: unknown vertical id"),
        "the refusal must NAME the divergence:\n{err}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_vertical_declaration_verifies_with_zero_behavioral_mutations() {
    // The negative control (ADR-0220 §2): NO `[vertical]` table →
    // verify behaves EXACTLY as before №921 — GREEN, and NO vertical
    // verdict line (nothing attaches by default).
    let dir = setup_package(None);
    let key = key_outside(&dir, "n921-none-key.hex");
    pack_sign(&dir, &key);
    let (code, out, err) = run_in(&dir, &["verify", "--key", key.to_str().unwrap()]);
    assert_eq!(code, 0, "the undeclared path must stay green:\n{out}{err}");
    assert!(
        !out.contains("vertical OK"),
        "no default attach — the undeclared manifest must NOT carry the \
         vertical verdict line:\n{out}"
    );
    fs::remove_dir_all(&dir).ok();
}
