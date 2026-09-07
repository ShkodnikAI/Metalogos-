// ── tests/naryad_198_audit_finds_known_vuln.rs ─────────────────────
// Наряд №198, Contract 3: `mlogpkg audit` finds and reports a known
// vulnerability from the local advisory DB.
//
// Setup:
//   1. Create a project depending on a package "vuln-lib" version 1.2.0
//   2. Write a custom advisory DB (via MLOGPKG_ADVISORY_DB env var) that
//      lists an advisory for vuln-lib 1.2.0
//   3. Run `mlogpkg audit` — it must:
//      - find the advisory
//      - report it (exit code 1, mention the package, severity, advisory ID)
//      - NOT silently pass

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("mlogpkg_198_audit_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_pkg(registry: &Path, name: &str, version: &str) {
    let pkg_dir = registry.join(name);
    fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n",
        name, version
    );
    fs::write(pkg_dir.join("mlog.toml"), manifest).unwrap();
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(
        pkg_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
}

fn write_manifest(path: &Path, name: &str, deps: &[(&str, &str)]) {
    let mut deps_toml = String::new();
    if !deps.is_empty() {
        deps_toml.push_str("\n[dependencies]\n");
        for (d, v) in deps {
            deps_toml.push_str(&format!("{} = \"{}\"\n", d, v));
        }
    }
    let manifest = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{}",
        name, deps_toml
    );
    fs::write(path, manifest).unwrap();
}

/// Write a test advisory DB with one entry: vuln-lib 1.2.0 has a high-severity advisory.
fn write_test_advisory_db(path: &Path) {
    let content = r#"# Test advisory DB for naryad_198_audit_finds_known_vuln.rs
[[advisory]]
id = "MLOG-TEST-001"
package = "vuln-lib"
version = "1.2.0"
severity = "high"
title = "Test SQL injection in vuln-lib"
description = "vuln-lib 1.2.0 has a SQL injection vulnerability in the query builder. Upgrade to 1.3.0+."
url = "https://example.com/advisories/MLOG-TEST-001"
"#;
    fs::write(path, content).unwrap();
}

#[test]
fn naryad_198_audit_finds_known_vuln() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("vuln_found");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "vuln-lib", "1.2.0");

    // Custom advisory DB that flags vuln-lib 1.2.0.
    let advisory_db_path = project_dir.join("advisory-db.toml");
    write_test_advisory_db(&advisory_db_path);

    // Create project depending on vuln-lib 1.2.0.
    let app_dir = project_dir.join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &app_dir.join("mlog.toml"),
        "vuln_test_app",
        &[("vuln-lib", "1.2.0")],
    );

    // ── Run mlogpkg audit ────────────────────────────────────────────
    let output = Command::new(mlogpkg_bin())
        .arg("audit")
        .env("MLOGPKG_REGISTRY", &registry)
        .env("MLOGPKG_ADVISORY_DB", &advisory_db_path)
        .current_dir(&app_dir)
        .output()
        .expect("failed to run mlogpkg audit");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // ── Assert: audit must FAIL (exit 1) and report the vulnerability ─
    assert!(
        !output.status.success(),
        "audit must exit non-zero when a known vuln is found.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Must mention the vulnerable package name.
    assert!(
        stdout.contains("vuln-lib"),
        "audit output must mention the vulnerable package 'vuln-lib':\n{}",
        stdout
    );

    // Must mention the advisory ID.
    assert!(
        stdout.contains("MLOG-TEST-001"),
        "audit output must mention the advisory ID 'MLOG-TEST-001':\n{}",
        stdout
    );

    // Must mention the severity.
    assert!(
        stdout.to_uppercase().contains("HIGH"),
        "audit output must mention the severity 'HIGH':\n{}",
        stdout
    );

    // Must mention the version.
    assert!(
        stdout.contains("1.2.0"),
        "audit output must mention the version '1.2.0':\n{}",
        stdout
    );

    // Must report the count.
    assert!(
        stdout.contains("1") && stdout.to_lowercase().contains("advisory"),
        "audit output must report the count of advisories found:\n{}",
        stdout
    );
}

#[test]
fn naryad_198_audit_passes_when_no_vuln() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("vuln_none");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    // safe-lib 1.0.0 — no advisory for this version.
    write_pkg(&registry, "safe-lib", "1.0.0");

    let advisory_db_path = project_dir.join("advisory-db.toml");
    write_test_advisory_db(&advisory_db_path); // DB has vuln-lib only, not safe-lib

    let app_dir = project_dir.join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &app_dir.join("mlog.toml"),
        "safe_test_app",
        &[("safe-lib", "1.0.0")],
    );

    // ── Run mlogpkg audit ────────────────────────────────────────────
    let output = Command::new(mlogpkg_bin())
        .arg("audit")
        .env("MLOGPKG_REGISTRY", &registry)
        .env("MLOGPKG_ADVISORY_DB", &advisory_db_path)
        .current_dir(&app_dir)
        .output()
        .expect("failed to run mlogpkg audit");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // ── Assert: audit must PASS (exit 0) ──────────────────────────────
    assert!(
        output.status.success(),
        "audit must exit 0 when no known vuln matches.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(
        stdout.to_lowercase().contains("no known advisories"),
        "audit output must say 'no known advisories':\n{}",
        stdout
    );
}

#[test]
fn naryad_198_audit_version_specificity() {
    // ── Setup ────────────────────────────────────────────────────────
    // Advisory DB lists vuln-lib 1.2.0. Project uses vuln-lib 1.3.0.
    // Audit must NOT flag 1.3.0 — only 1.2.0 is vulnerable.
    let project_dir = temp_dir("vuln_version");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "vuln-lib", "1.3.0");

    let advisory_db_path = project_dir.join("advisory-db.toml");
    write_test_advisory_db(&advisory_db_path); // DB has 1.2.0 only

    let app_dir = project_dir.join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &app_dir.join("mlog.toml"),
        "version_test_app",
        &[("vuln-lib", "1.3.0")],
    );

    // ── Run mlogpkg audit ────────────────────────────────────────────
    let output = Command::new(mlogpkg_bin())
        .arg("audit")
        .env("MLOGPKG_REGISTRY", &registry)
        .env("MLOGPKG_ADVISORY_DB", &advisory_db_path)
        .current_dir(&app_dir)
        .output()
        .expect("failed to run mlogpkg audit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // ── Assert: audit must PASS (exit 0) — version 1.3.0 not in DB ────
    assert!(
        output.status.success(),
        "audit must exit 0 when version doesn't match advisory.\nstdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("MLOG-TEST-001"),
        "audit must NOT flag 1.3.0 (only 1.2.0 is vulnerable):\n{}",
        stdout
    );
}
