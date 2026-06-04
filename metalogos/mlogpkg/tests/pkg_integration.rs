// ── mlogpkg integration tests (Phase 3.4) ────────────────────────────
//
// Contract (Наряд Phase 3.4):
//   - mlogpkg init creates mlog.toml
//   - mlogpkg add records a dependency
//   - mlogpkg build resolves deps and checks sources

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlogpkg_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    workspace_root.join("target").join("debug").join("mlogpkg")
}

fn run_mlogpkg(args: &[&str], cwd: &Path) -> (bool, String, String) {
    let bin = mlogpkg_bin();
    let mut cmd = Command::new(&bin);
    cmd.args(args).current_dir(cwd);

    let output = cmd.output().expect("failed to run mlogpkg");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mlogpkg_test_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_init_creates_mlog_toml() {
    let dir = temp_dir("init_toml");
    let (success, _stdout, stderr) = run_mlogpkg(&["init", "--name", "myproject"], &dir);
    assert!(success, "init should succeed: stderr={}", stderr);
    assert!(dir.join("mlog.toml").exists(), "mlog.toml should be created");
    assert!(dir.join("src/main.mlog").exists(), "src/main.mlog should be created");

    let content = fs::read_to_string(dir.join("mlog.toml")).unwrap();
    assert!(content.contains("name = \"myproject\""), "name should be in manifest: {}", content);
    assert!(content.contains("version = \"0.1.0\""), "version should be in manifest: {}", content);
}

#[test]
fn test_init_default_name() {
    let dir = temp_dir("init_default");
    let (success, _stdout, stderr) = run_mlogpkg(&["init"], &dir);
    assert!(success, "init should succeed: stderr={}", stderr);
    let content = fs::read_to_string(dir.join("mlog.toml")).unwrap();
    // The temp dir name contains "init_default" — which is the last component
    assert!(content.contains("init_default"), "should use dir name: {}", content);
}

#[test]
fn test_init_fails_if_exists() {
    let dir = temp_dir("init_exists");
    fs::write(dir.join("mlog.toml"), "[package]\nname = \"old\"\nversion = \"0.0.1\"\n").unwrap();
    let (success, _stdout, stderr) = run_mlogpkg(&["init"], &dir);
    assert!(!success, "init should fail when mlog.toml exists");
    assert!(stderr.contains("already exists"), "error should mention 'already exists': {}", stderr);
}

#[test]
fn test_info_no_manifest() {
    let dir = temp_dir("info_no_manifest");
    let (success, stdout, _stderr) = run_mlogpkg(&["info"], &dir);
    assert!(success, "info should succeed even without manifest");
    assert!(stdout.contains("No mlog.toml"), "should say no manifest found");
}

#[test]
fn test_info_with_manifest() {
    let dir = temp_dir("info_with");
    run_mlogpkg(&["init", "--name", "testproj"], &dir);
    let (success, stdout, _stderr) = run_mlogpkg(&["info"], &dir);
    assert!(success, "info should succeed");
    assert!(stdout.contains("testproj"), "should show project name: {}", stdout);
    assert!(stdout.contains("0.1.0"), "should show version: {}", stdout);
}

#[test]
fn test_build_with_init_project() {
    let dir = temp_dir("build_ok");
    run_mlogpkg(&["init", "--name", "buildtest"], &dir);
    let (success, stdout, stderr) = run_mlogpkg(&["build"], &dir);
    assert!(success, "build should succeed on a fresh project: stdout={} stderr={}", stdout, stderr);
    assert!(stdout.contains("Build OK"), "should say build ok");
    assert!(dir.join("mlog.lock").exists(), "mlog.lock should be created");
}

#[test]
fn test_build_detects_errors() {
    let dir = temp_dir("build_err");
    run_mlogpkg(&["init", "--name", "errtest"], &dir);
    // Write an erroneous source file
    fs::write(dir.join("src/error.mlog"), "entity m: UnknownType = { text: \"hi\" }").unwrap();
    let (success, _stdout, _stderr) = run_mlogpkg(&["build"], &dir);
    assert!(!success, "build should fail with errors");
}

#[test]
fn test_build_fails_without_manifest() {
    let dir = temp_dir("build_no_manifest");
    let (success, _stdout, stderr) = run_mlogpkg(&["build"], &dir);
    assert!(!success, "build should fail without mlog.toml");
    assert!(stderr.contains("not found"), "error should mention 'not found'");
}

#[test]
fn test_manifest_toml_format() {
    let dir = temp_dir("fmt");
    run_mlogpkg(&["init", "--name", "fmttest"], &dir);
    let content = fs::read_to_string(dir.join("mlog.toml")).unwrap();

    assert!(content.contains("[package]"), "should have [package] section");
    assert!(content.contains("edition"), "should have edition field");
    assert!(content.contains("[dependencies]"), "should have [dependencies] section");
}
