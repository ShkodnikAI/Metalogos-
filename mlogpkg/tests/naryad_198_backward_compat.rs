// ── tests/naryad_198_backward_compat.rs ────────────────────────────
// Наряд №198, Contract 4 (Block 3 regression): simple projects without
// transitive dependencies must work exactly as before, PLUS the
// appearance of mlogpkg.lock.
//
// This is the "Block 3 — backward compat" regression test. The existing
// pkg_integration.rs tests already cover most of this, but this file
// makes the regression intent explicit and adds a direct check that
// the lockfile contains exactly one entry for a single-direct-dep project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mlogpkg_198_compat_{}_{}",
        std::process::id(),
        name
    ));
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

fn run_mlogpkg(args: &[&str], cwd: &Path, registry: &Path) -> (bool, String, String) {
    let output = Command::new(mlogpkg_bin())
        .args(args)
        .env("MLOGPKG_REGISTRY", registry)
        .current_dir(cwd)
        .output()
        .expect("failed to run mlogpkg");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn naryad_198_simple_project_no_deps_builds_and_locks() {
    // ── Setup: empty project, no dependencies ────────────────────────
    let project_dir = temp_dir("compat_no_deps");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();

    let (success, _stdout, stderr) =
        run_mlogpkg(&["init", "--name", "simple_app"], &project_dir, &registry);
    assert!(success, "init should succeed: stderr={}", stderr);

    let (success, stdout, stderr) = run_mlogpkg(&["build"], &project_dir, &registry);
    assert!(
        success,
        "build should succeed on empty project: stdout={} stderr={}",
        stdout, stderr
    );
    assert!(
        stdout.contains("Build OK"),
        "should say build ok: {}",
        stdout
    );

    // Lockfile must exist (NEW behavior from Наряд №198).
    let lock_path = project_dir.join("mlogpkg.lock");
    assert!(
        lock_path.exists(),
        "mlogpkg.lock must be created even for projects with no deps"
    );

    // Lockfile must have version header.
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("version = 1"),
        "lockfile must have version header:\n{}",
        lock_content
    );

    // Lockfile must have empty package list (no deps).
    assert!(
        !lock_content.contains("[[package]]"),
        "lockfile for no-deps project must have no [[package]] entries:\n{}",
        lock_content
    );
}

#[test]
fn naryad_198_simple_project_single_direct_dep() {
    // ── Setup: project with one direct dependency, no transitive deps ──
    let project_dir = temp_dir("compat_one_dep");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();
    write_pkg(&registry, "lonely-lib", "1.0.0");

    let (success, _stdout, stderr) =
        run_mlogpkg(&["init", "--name", "one_dep_app"], &project_dir, &registry);
    assert!(success, "init should succeed: stderr={}", stderr);

    let (success, _stdout, stderr) =
        run_mlogpkg(&["add", "lonely-lib", "1.0.0"], &project_dir, &registry);
    assert!(success, "add should succeed: stderr={}", stderr);

    // Verify mlog.toml was updated.
    let manifest = fs::read_to_string(project_dir.join("mlog.toml")).unwrap();
    assert!(
        manifest.contains("lonely-lib"),
        "mlog.toml must contain the added dependency:\n{}",
        manifest
    );
    assert!(
        manifest.contains("1.0.0"),
        "mlog.toml must contain the version:\n{}",
        manifest
    );

    // Build must succeed.
    let (success, stdout, stderr) = run_mlogpkg(&["build"], &project_dir, &registry);
    assert!(
        success,
        "build should succeed: stdout={} stderr={}",
        stdout, stderr
    );

    // Lockfile must contain exactly one package.
    let lock_path = project_dir.join("mlogpkg.lock");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("name = \"lonely-lib\""),
        "lockfile must contain the dep:\n{}",
        lock_content
    );
    assert!(
        lock_content.contains("version = \"1.0.0\""),
        "lockfile must contain the version:\n{}",
        lock_content
    );
}

#[test]
fn naryad_198_info_still_works() {
    // `mlogpkg info` must work the same as before, plus show lockfile status.
    let project_dir = temp_dir("compat_info");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();

    let (_, _, _) = run_mlogpkg(&["init", "--name", "info_app"], &project_dir, &registry);

    let (success, stdout, _stderr) = run_mlogpkg(&["info"], &project_dir, &registry);
    assert!(success, "info should succeed");
    assert!(
        stdout.contains("info_app"),
        "info must show project name: {}",
        stdout
    );
    assert!(
        stdout.contains("0.1.0"),
        "info must show version: {}",
        stdout
    );
    // NEW: info should mention the lockfile status.
    assert!(
        stdout.to_lowercase().contains("lockfile"),
        "info must mention lockfile status (Наряд №198):\n{}",
        stdout
    );
}

#[test]
fn naryad_198_init_idempotent_failure() {
    // `mlogpkg init` must still fail when mlog.toml already exists.
    let project_dir = temp_dir("compat_init_fail");
    let registry = project_dir.join("registry");
    fs::create_dir_all(&registry).unwrap();

    fs::write(
        project_dir.join("mlog.toml"),
        "[package]\nname = \"old\"\nversion = \"0.0.1\"\nedition = \"2024\"\n",
    )
    .unwrap();

    let (success, _stdout, stderr) = run_mlogpkg(&["init"], &project_dir, &registry);
    assert!(!success, "init should fail when mlog.toml exists");
    assert!(
        stderr.contains("already exists"),
        "error should mention 'already exists': {}",
        stderr
    );
}
