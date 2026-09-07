// ── tests/naryad_198_lockfile_determinism.rs ────────────────────────
// Наряд №198, Contract 2: identical mlog.toml → identical mlogpkg.lock.
//
// Resolves the same project twice (in two separate dirs) and verifies
// that the resulting mlogpkg.lock files are byte-for-byte identical.
// This guarantees that builds are reproducible — the same dependency
// manifest always produces the same lockfile.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("mlogpkg_198_lock_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
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
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(
        pkg_dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
}

fn setup_registry(registry: &Path) {
    // Build a 3-level transitive graph so the test exercises the full
    // resolution, not just direct deps:
    //   app -> lib_a (1.0.0) -> lib_b (2.0.0) -> lib_c (3.0.0)
    //   app -> lib_d (1.5.0) -> lib_c (3.0.0)  (diamond — same version)
    write_pkg(registry, "lib_c", "3.0.0", &[]);
    write_pkg(registry, "lib_b", "2.0.0", &[("lib_c", "3.0.0")]);
    write_pkg(registry, "lib_a", "1.0.0", &[("lib_b", "2.0.0")]);
    write_pkg(registry, "lib_d", "1.5.0", &[("lib_c", "3.0.0")]);
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

fn build_project_and_get_lockfile(project_dir: &Path, registry: &Path) -> String {
    // Run `mlogpkg build` in project_dir, then read mlogpkg.lock.
    let output = Command::new(mlogpkg_bin())
        .arg("build")
        .env("MLOGPKG_REGISTRY", registry)
        .current_dir(project_dir)
        .output()
        .expect("failed to run mlogpkg build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "build failed in {:?}:\nstdout: {}\nstderr: {}",
        project_dir,
        stdout,
        stderr
    );

    let lock_path = project_dir.join("mlogpkg.lock");
    fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", lock_path, e))
}

#[test]
fn naryad_198_lockfile_determinism_two_builds() {
    // ── Setup shared registry ────────────────────────────────────────
    let project_dir = temp_dir("det_main");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    setup_registry(&registry);

    // ── Build #1 ─────────────────────────────────────────────────────
    let dir1 = temp_dir("det_build1");
    fs::create_dir_all(dir1.join("src")).unwrap();
    fs::write(
        dir1.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &dir1.join("mlog.toml"),
        "det_test",
        &[("lib_a", "1.0.0"), ("lib_d", "1.5.0")],
    );

    let lock1 = build_project_and_get_lockfile(&dir1, &registry);

    // ── Build #2 (separate dir, same mlog.toml) ─────────────────────
    let dir2 = temp_dir("det_build2");
    fs::create_dir_all(dir2.join("src")).unwrap();
    fs::write(
        dir2.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &dir2.join("mlog.toml"),
        "det_test",
        &[("lib_a", "1.0.0"), ("lib_d", "1.5.0")],
    );

    let lock2 = build_project_and_get_lockfile(&dir2, &registry);

    // ── Assert byte-for-byte identical ───────────────────────────────
    assert_eq!(
        lock1, lock2,
        "lockfile must be byte-for-byte identical for the same mlog.toml.\n\
         lock1:\n{}\n\nlock2:\n{}",
        lock1, lock2
    );
}

#[test]
fn naryad_198_lockfile_determinism_rebuild_same_dir() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("det_rebuild");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    setup_registry(&registry);

    let dir = project_dir.join("project");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    write_manifest(
        &dir.join("mlog.toml"),
        "det_rebuild_test",
        &[("lib_a", "1.0.0"), ("lib_d", "1.5.0")],
    );

    // ── Build #1 ─────────────────────────────────────────────────────
    let lock1 = build_project_and_get_lockfile(&dir, &registry);

    // ── Build #2 in same dir (rebuild) ──────────────────────────────
    // Delete the lockfile first to ensure it's regenerated, not just
    // left intact from build #1.
    fs::remove_file(dir.join("mlogpkg.lock")).unwrap();

    let lock2 = build_project_and_get_lockfile(&dir, &registry);

    // ── Assert byte-for-byte identical ───────────────────────────────
    assert_eq!(
        lock1, lock2,
        "rebuild in same dir must produce identical lockfile.\n\
         lock1:\n{}\n\nlock2:\n{}",
        lock1, lock2
    );
}

#[test]
fn naryad_198_lockfile_includes_transitive_deps() {
    // ── Setup ────────────────────────────────────────────────────────
    let project_dir = temp_dir("lock_transitive");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    setup_registry(&registry);

    let dir = project_dir.join("project");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.mlog"),
        "entity v: String = \"\"\nflow Main { input: String = v -> output }\n",
    )
    .unwrap();
    // Direct deps: lib_a, lib_d. Transitive: lib_b, lib_c.
    write_manifest(
        &dir.join("mlog.toml"),
        "transitive_test",
        &[("lib_a", "1.0.0"), ("lib_d", "1.5.0")],
    );

    let lock = build_project_and_get_lockfile(&dir, &registry);

    // ── Assert lockfile contains all 4 packages ─────────────────────
    // (lib_a, lib_b, lib_c, lib_d — direct + transitive).
    for name in &["lib_a", "lib_b", "lib_c", "lib_d"] {
        assert!(
            lock.contains(&format!("name = \"{}\"", name)),
            "lockfile must contain transitive dep '{}':\n{}",
            name,
            lock
        );
    }

    // ── Assert lockfile has the version header ───────────────────────
    assert!(
        lock.contains("version = 1"),
        "lockfile must have version header:\n{}",
        lock
    );

    // ── Assert alphabetical ordering ─────────────────────────────────
    // (BTreeMap serialization gives deterministic ordering.)
    let lib_a_pos = lock.find("name = \"lib_a\"").unwrap_or(usize::MAX);
    let lib_b_pos = lock.find("name = \"lib_b\"").unwrap_or(usize::MAX);
    let lib_c_pos = lock.find("name = \"lib_c\"").unwrap_or(usize::MAX);
    let lib_d_pos = lock.find("name = \"lib_d\"").unwrap_or(usize::MAX);
    assert!(
        lib_a_pos < lib_b_pos && lib_b_pos < lib_c_pos && lib_c_pos < lib_d_pos,
        "lockfile must list packages in alphabetical order:\n{}",
        lock
    );
}
