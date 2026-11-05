// Naryad №911 (issue #1813, wave 82 — W-K): the build integrity gate —
// the fail-closed coverage rule (ADR-0218 §4.2).
//
// The contract under test:
//   - a dependency without a digest pin refuses at build («digest pin missing»);
//   - a tampered registry package refuses BEFORE the semantic check,
//     naming the file divergence («digest pin mismatch» / «file digest divergence»);
//   - a pinned verify_key demands a VERIFIED signature (no document /
//     no trailer / wrong key — all refuse);
//   - the advisory consult (№198) rides strictly AFTER the integrity pass:
//     a tampered package with a matching advisory row refuses as INTEGRITY.
//
// This file exercises the REAL filesystem for fixtures by design (№475).
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn run_mlogpkg(args: &[&str], cwd: &Path, registry: Option<&Path>) -> (bool, String, String) {
    let mut cmd = Command::new(mlogpkg_bin());
    cmd.args(args).current_dir(cwd);
    if let Some(r) = registry {
        cmd.env("MLOGPKG_REGISTRY", r);
    }
    let out = cmd.output().expect("failed to run mlogpkg");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mlogpkg_n911_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_pkg(registry: &Path, name: &str, version: &str) {
    let pkg_dir = registry.join(name);
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(
        pkg_dir.join("mlog.toml"),
        format!("[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n", name, version),
    )
    .unwrap();
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(
        pkg_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
}

/// Pack the package at `pkg_dir` and return its digest (the printed form).
fn pack_digest(pkg_dir: &Path) -> String {
    let (ok, out, err) = run_mlogpkg(&["pack"], pkg_dir, None);
    assert!(ok, "pack should succeed in {:?}: {}", pkg_dir, err);
    out.lines()
        .next()
        .unwrap()
        .trim_start_matches("digest: ")
        .trim()
        .to_string()
}

/// Rewrite `name = "version"` into the pinned inline-table form.
fn pin_dependency(manifest_path: &Path, name: &str, version: &str, digest: &str) {
    let content = fs::read_to_string(manifest_path).unwrap();
    let bare = format!("{} = \"{}\"", name, version);
    let pinned = format!(
        "{} = {{ version = \"{}\", digest = \"{}\" }}",
        name, version, digest
    );
    assert!(
        content.contains(&bare),
        "manifest must contain the bare dep to pin: {}\n{}",
        bare,
        content
    );
    fs::write(manifest_path, content.replace(&bare, &pinned)).unwrap();
}

fn write_advisory_db(path: &Path, package: &str, version: &str) {
    let content = format!(
        "# test advisory db (№911)\n[[advisory]]\nid = \"MLOG-911\"\npackage = \"{}\"\nversion = \"{}\"\nseverity = \"high\"\ntitle = \"the canary advisory\"\ndescription = \"\"\nurl = \"\"\n",
        package, version
    );
    fs::write(path, content).unwrap();
}

#[test]
fn n911_bare_version_refuses_with_missing_pin() {
    let dir = temp_dir("bare_refuses");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lib", "1.0.0");
    fs::write(
        dir.join("mlog.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nlib = \"1.0.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();

    let (ok, _out, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "build must REFUSE a bare version");
    assert!(
        err.contains("digest pin missing") && err.contains("lib"),
        "the refusal must name the missing pin: {}",
        err
    );
    assert!(
        err.contains("[dependencies]") && err.contains("№911"),
        "the refusal must teach the pinned form: {}",
        err
    );
}

#[test]
fn n911_pinned_happy_builds() {
    let dir = temp_dir("pinned_happy");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lib", "1.0.0");
    let digest = pack_digest(&registry.join("lib"));

    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nlib = {{ version = \"1.0.0\", digest = \"{}\" }}\n",
            digest
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();

    let (ok, out, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(ok, "pinned build must succeed: {} / {}", err, out);
    assert!(out.contains("Build OK"), "stdout: {}", out);
}

#[test]
fn n911_tampered_registry_refuses_before_semantic_check() {
    let dir = temp_dir("tamper_before_check");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lib", "1.0.0");
    let digest = pack_digest(&registry.join("lib"));

    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nlib = {{ version = \"1.0.0\", digest = \"{}\" }}\n",
            digest
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    // The CONSUMER source is deliberately broken — if the semantic check
    // runs, the test fails (the integrity gate must fire FIRST).
    fs::write(dir.join("src/main.mlog"), "entity m: UnknownType = { text: \"hi\" }").unwrap();

    // Tamper the REGISTRY tree after packing.
    let src = registry.join("lib/src/main.mlog");
    let body = fs::read_to_string(&src).unwrap();
    fs::write(&src, format!("{}\n", body)).unwrap();

    let (ok, _out, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "build must REFUSE a tampered registry package");
    assert!(
        err.contains("integrity gate") || err.contains("digest pin mismatch"),
        "the refusal must be INTEGRITY-named, not semantic: {}",
        err
    );
    assert!(
        !err.contains("UnknownType"),
        "the semantic check must NOT run before the integrity gate: {}",
        err
    );
}

#[test]
fn n911_advisory_consult_rides_after_integrity() {
    let dir = temp_dir("advisory_order");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lib", "1.0.0");
    let digest = pack_digest(&registry.join("lib"));

    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nlib = {{ version = \"1.0.0\", digest = \"{}\" }}\n",
            digest
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();

    // The advisory matches lib@1.0.0 — but the tree is TAMPERED: the
    // refusal must name the INTEGRITY divergence, not the advisory.
    let db = dir.join("advisory.toml");
    write_advisory_db(&db, "lib", "1.0.0");

    let src = registry.join("lib/src/main.mlog");
    let body = fs::read_to_string(&src).unwrap();
    fs::write(&src, format!("{}\n", body)).unwrap();

    let mut cmd = Command::new(mlogpkg_bin());
    cmd.args(["build"])
        .env("MLOGPKG_REGISTRY", &registry)
        .env("MLOGPKG_ADVISORY_DB", &db)
        .current_dir(&dir);
    let out = cmd.output().expect("failed to run mlogpkg");
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!out.status.success(), "build must refuse");
    assert!(
        err.contains("integrity gate") || err.contains("digest pin mismatch"),
        "the refusal must be INTEGRITY-named: {}",
        err
    );
    assert!(
        !err.contains("MLOG-911"),
        "the advisory must NOT fire before the integrity gate: {}",
        err
    );
}

#[test]
fn n911_verify_key_pinned_demands_verified_signature() {
    let dir = temp_dir("verify_key");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lib", "1.0.0");
    let digest = pack_digest(&registry.join("lib"));

    // The signing key lives outside the registry.
    let key = dir.join("signer.hex");
    let (ok, _o, e) = run_mlogpkg(&["keygen", "--out", key.to_str().unwrap()], &dir, None);
    assert!(ok, "keygen: {}", e);

    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[registry]\nverify_key = \"signer.hex\"\n\n[dependencies]\nlib = {{ version = \"1.0.0\", digest = \"{}\" }}\n",
            digest
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();

    // 1. No package document at all → refusal.
    let (ok, _o, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "verify_key pinned + no document must refuse");
    assert!(
        err.contains("signature required"),
        "stderr: {}",
        err
    );

    // 2. Document without a signature → refusal.
    let (ok, _o, e) = run_mlogpkg(&["pack"], &registry.join("lib"), None);
    assert!(ok, "pack in registry: {}", e);
    let (ok, _o, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "verify_key pinned + unsigned document must refuse");
    assert!(err.contains("signature required"), "stderr: {}", err);

    // 3. Signed with the right key → green.
    let (ok, _o, e) = run_mlogpkg(&["sign", "--key", key.to_str().unwrap()], &registry.join("lib"), None);
    assert!(ok, "sign: {}", e);
    let (ok, out, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(ok, "signed + pinned key must build: {} / {}", err, out);
    assert!(out.contains("Build OK"), "stdout: {}", out);

    // 4. A foreign key pinned → signature mismatch.
    let stranger = dir.join("stranger.hex");
    let (ok, _o, e) = run_mlogpkg(&["keygen", "--out", stranger.to_str().unwrap()], &dir, None);
    assert!(ok, "keygen stranger: {}", e);
    let manifest = fs::read_to_string(dir.join("mlog.toml")).unwrap();
    fs::write(
        dir.join("mlog.toml"),
        manifest.replace("signer.hex", "stranger.hex"),
    )
    .unwrap();
    let (ok, _o, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "a foreign key must refuse");
    assert!(err.contains("signature mismatch"), "stderr: {}", err);
}

#[test]
fn n911_transitive_pin_required() {
    let dir = temp_dir("transitive");
    let registry = dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "leaf", "1.0.0");
    let dleaf = pack_digest(&registry.join("leaf"));

    // mid pins leaf.
    let mid_dir = registry.join("mid");
    fs::create_dir_all(&mid_dir).unwrap();
    fs::write(
        mid_dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"mid\"\nversion = \"2.0.0\"\nedition = \"2024\"\n\n[dependencies]\nleaf = {{ version = \"1.0.0\", digest = \"{}\" }}\n",
            dleaf
        ),
    )
    .unwrap();
    fs::create_dir_all(mid_dir.join("src")).unwrap();
    fs::write(mid_dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();
    let dmid = pack_digest(&mid_dir);

    // The consumer pins mid only — leaf's pin rides mid's manifest.
    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmid = {{ version = \"2.0.0\", digest = \"{}\" }}\n",
            dmid
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.mlog"), "entity v: String = \"\"\nflow Main { input: String = v -> output }\n").unwrap();

    let (ok, out, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(ok, "the transitive chain must build: {} / {}", err, out);

    // Tamper the LEAF — the integrity gate must catch it through the chain.
    let src = registry.join("leaf/src/main.mlog");
    let body = fs::read_to_string(&src).unwrap();
    fs::write(&src, format!("{}\n", body)).unwrap();
    let (ok, _o, err) = run_mlogpkg(&["build"], &dir, Some(&registry));
    assert!(!ok, "a tampered leaf must refuse through the chain");
    assert!(
        err.contains("digest pin mismatch") && err.contains("leaf"),
        "the refusal must name the leaf: {}",
        err
    );
}
