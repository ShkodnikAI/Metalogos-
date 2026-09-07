// ── tests/naryad_198_dependency_conflict.rs ─────────────────────────
// Наряд №198, Contract 1: version conflict must produce an explicit,
// understandable error — NOT a silent choice.
//
// Setup:
//   app -> pkg_a (1.0.0) -> shared (1.0.0)
//   app -> pkg_b (1.0.0) -> shared (2.0.0)
//
// Two packages (pkg_a, pkg_b) both require "shared" but with different
// versions. mlogpkg v1 does not support multiple concurrent versions of
// the same package, so this MUST fail with an explicit error message
// mentioning the conflict.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mlogpkg_198_conflict_{}_{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Set up a local registry with the diamond dependency structure:
///   pkg_a -> shared@1.0.0
///   pkg_b -> shared@2.0.0
///
/// Since the registry only supports one version per package name, we
/// install shared@1.0.0. When pkg_b is resolved, it requires shared@2.0.0
/// but the registry has 1.0.0 — this is one form of conflict.
///
/// The other form is: pkg_a requires shared@1.0.0, pkg_b requires
/// shared@1.0.0 (same version) but the resolved version (from a third
/// dep) is different. Both forms must produce a clear error.
fn setup_registry_with_conflict(registry: &Path) {
    // shared@1.0.0 (only one version per package name in v1 registry).
    write_pkg(registry, "shared", "1.0.0", &[]);
    // pkg_a depends on shared@1.0.0 — matches registry.
    write_pkg(registry, "pkg_a", "1.0.0", &[("shared", "1.0.0")]);
    // pkg_b depends on shared@2.0.0 — does NOT match registry's 1.0.0.
    write_pkg(registry, "pkg_b", "1.0.0", &[("shared", "2.0.0")]);
}

fn write_pkg(registry: &Path, name: &str, version: &str, deps: &[(&str, &str)]) {
    let pkg_dir = registry.join(name);
    fs::create_dir_all(&pkg_dir).unwrap();

    let mut deps_toml = String::new();
    if !deps.is_empty() {
        deps_toml.push_str("\n[dependencies]\n");
        for (d, v) in deps {
            deps_toml.push_str(&format!("{} = \"{}\"\n", d, v));
        }
    }

    let manifest = format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n{}",
        name, version, deps_toml
    );
    fs::write(pkg_dir.join("mlog.toml"), manifest).unwrap();

    // Minimal source file so build's semantic check passes.
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

#[test]
fn naryad_198_dependency_conflict_at_build() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("build_conflict");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    setup_registry_with_conflict(&registry);

    // Write mlog.toml that depends on both pkg_a and pkg_b.
    write_manifest(
        &project_dir.join("mlog.toml"),
        "testapp",
        &[("pkg_a", "1.0.0"), ("pkg_b", "1.0.0")],
    );
    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::write(
        project_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();

    // ── Run mlogpkg build ────────────────────────────────────────────
    let output = Command::new(mlogpkg_bin())
        .arg("build")
        .env("MLOGPKG_REGISTRY", &registry)
        .current_dir(&project_dir)
        .output()
        .expect("failed to run mlogpkg");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // ── Assert: build must FAIL with an explicit conflict error ──────
    assert!(
        !output.status.success(),
        "build must fail when there's a version conflict.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // The error message must mention "conflict" and the conflicting package name.
    let combined = format!("{}\n{}", stdout, stderr);
    assert!(
        combined.to_lowercase().contains("conflict"),
        "error must mention 'conflict':\n{}",
        combined
    );
    assert!(
        combined.contains("shared"),
        "error must mention the conflicting package 'shared':\n{}",
        combined
    );
}

#[test]
fn naryad_198_dependency_conflict_at_add() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("add_conflict");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    setup_registry_with_conflict(&registry);

    // Init project, add pkg_a (no conflict yet).
    Command::new(mlogpkg_bin())
        .args(["init", "--name", "testapp"])
        .env("MLOGPKG_REGISTRY", &registry)
        .current_dir(&project_dir)
        .output()
        .expect("init failed");

    let add_a = Command::new(mlogpkg_bin())
        .args(["add", "pkg_a", "1.0.0"])
        .env("MLOGPKG_REGISTRY", &registry)
        .current_dir(&project_dir)
        .output()
        .expect("add pkg_a failed");
    assert!(
        add_a.status.success(),
        "add pkg_a should succeed (no conflict yet): stderr={}",
        String::from_utf8_lossy(&add_a.stderr)
    );

    // Now add pkg_b — this should fail because pkg_b requires shared@2.0.0
    // but pkg_a already requires shared@1.0.0.
    let add_b = Command::new(mlogpkg_bin())
        .args(["add", "pkg_b", "1.0.0"])
        .env("MLOGPKG_REGISTRY", &registry)
        .current_dir(&project_dir)
        .output()
        .expect("add pkg_b failed");

    let stdout = String::from_utf8_lossy(&add_b.stdout);
    let stderr = String::from_utf8_lossy(&add_b.stderr);

    assert!(
        !add_b.status.success(),
        "add pkg_b must fail due to version conflict.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let combined = format!("{}\n{}", stdout, stderr);
    assert!(
        combined.to_lowercase().contains("conflict"),
        "error must mention 'conflict':\n{}",
        combined
    );
    assert!(
        combined.contains("shared"),
        "error must mention 'shared':\n{}",
        combined
    );

    // mlog.toml must NOT have pkg_b added (rollback on failure).
    let manifest = fs::read_to_string(project_dir.join("mlog.toml")).unwrap();
    assert!(
        !manifest.contains("pkg_b"),
        "mlog.toml must not contain pkg_b after failed add:\n{}",
        manifest
    );
    // mlog.toml must still contain pkg_a (preserved).
    assert!(
        manifest.contains("pkg_a"),
        "mlog.toml must still contain pkg_a after failed add:\n{}",
        manifest
    );
}
