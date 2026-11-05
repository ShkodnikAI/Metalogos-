// Naryad №910 (issue #1812, wave 82 — W-K): the signed package kernel —
// pack/keygen/sign/verify over the canonical digest document (ADR-0218 §2).
//
// The contract under test:
//   - pack twice → byte-identical document (the bit-for-bit re-pack carrier);
//   - sign→verify happy path green;
//   - EVERY tamper names itself and refuses with exit 1 (fail-closed):
//     source byte flip, foreign key, missing signature, missing file,
//     manifest edit, stale document.
//
// This test file exercises the REAL filesystem for fixtures by design
// (the pkg_integration.rs form, №475).
#![allow(clippy::disallowed_methods)]

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
    let dir = std::env::temp_dir().join(format!("mlogpkg_n910_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn create_project(dir: &Path, name: &str) {
    fs::write(
        dir.join("mlog.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            name
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.mlog"),
        "entity greeting: String = \"hello\"\nflow Main { input: String = greeting -> output }\n",
    )
    .unwrap();
}

fn pack_doc(dir: &Path) -> PathBuf {
    dir.join("myproj.mlogpkg.json")
}

fn keyfile(dir: &Path, tag: &str) -> PathBuf {
    let kf = dir.join(format!("key_{}.hex", tag));
    let (ok, out, err) = run_mlogpkg(&["keygen", "--out", kf.to_str().unwrap()], dir);
    assert!(ok, "keygen should succeed: {}", err);
    assert!(out.contains("fingerprint:"), "fingerprint printed: {}", out);
    kf
}

#[test]
fn test_n910_pack_twice_byte_identical() {
    let dir = temp_dir("pack_twice");
    create_project(&dir, "myproj");
    let (ok, out1, err) = run_mlogpkg(&["pack"], &dir);
    assert!(ok, "pack should succeed: {}", err);
    assert!(out1.starts_with("digest: "), "digest printed: {}", out1);
    let bytes1 = fs::read(pack_doc(&dir)).unwrap();

    let (ok2, out2, err2) = run_mlogpkg(&["pack"], &dir);
    assert!(ok2, "re-pack should succeed: {}", err2);
    let bytes2 = fs::read(pack_doc(&dir)).unwrap();

    assert_eq!(bytes1, bytes2, "re-pack must be byte-identical (ADR-0218 §2.1)");
    assert_eq!(out1, out2, "the digest is deterministic");
}

#[test]
fn test_n910_sign_verify_happy() {
    let dir = temp_dir("sign_verify");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");

    let (ok, _, err) = run_mlogpkg(&["pack"], &dir);
    assert!(ok, "pack: {}", err);
    let (ok, out, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "sign: {}", err);
    assert!(out.starts_with("signed: "), "signed output: {}", out);

    let (ok, out, err) = run_mlogpkg(&["verify", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "verify must pass: {} / {}", err, out);
    assert!(out.contains("verify OK"), "verify output: {}", out);
}

#[test]
fn test_n910_verify_detects_source_tamper() {
    let dir = temp_dir("tamper_source");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");
    run_mlogpkg(&["pack"], &dir);
    let (ok, _, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "sign: {}", err);

    // Flip one source byte.
    let src = dir.join("src/main.mlog");
    let mut body = fs::read_to_string(&src).unwrap();
    body.push_str("// tampered\n");
    fs::write(&src, body).unwrap();

    let (ok, _, err) = run_mlogpkg(&["verify", "--key", kf.to_str().unwrap()], &dir);
    assert!(!ok, "verify must REFUSE a tampered source");
    assert!(
        err.contains("file digest divergence") || err.contains("does not match the tree"),
        "the divergence must name itself: {}",
        err
    );
}

#[test]
fn test_n910_verify_detects_foreign_key() {
    let dir = temp_dir("foreign_key");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "real");
    let stranger = keyfile(&dir, "stranger");

    run_mlogpkg(&["pack"], &dir);
    let (ok, _, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "sign: {}", err);

    let (ok, _, err) = run_mlogpkg(&["verify", "--key", stranger.to_str().unwrap()], &dir);
    assert!(!ok, "verify must REFUSE a foreign key");
    assert!(err.contains("signature mismatch"), "stderr: {}", err);
}

#[test]
fn test_n910_verify_detects_missing_signature() {
    let dir = temp_dir("no_signature");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");
    run_mlogpkg(&["pack"], &dir);

    let (ok, _, err) = run_mlogpkg(&["verify", "--key", kf.to_str().unwrap()], &dir);
    assert!(!ok, "verify must REFUSE an unsigned document");
    assert!(err.contains("no signature"), "stderr: {}", err);
}

#[test]
fn test_n910_verify_detects_missing_file() {
    let dir = temp_dir("missing_file");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");
    run_mlogpkg(&["pack"], &dir);
    let (ok, _, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "sign: {}", err);

    fs::remove_file(dir.join("src/main.mlog")).unwrap();
    let (ok, _, err) = run_mlogpkg(&["verify", "--key", kf.to_str().unwrap()], &dir);
    assert!(!ok, "verify must REFUSE a missing file");
    assert!(
        err.contains("file missing") || err.contains("does not match the tree"),
        "stderr: {}",
        err
    );
}

#[test]
fn test_n910_verify_detects_manifest_edit() {
    let dir = temp_dir("manifest_edit");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");
    run_mlogpkg(&["pack"], &dir);
    let (ok, _, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(ok, "sign: {}", err);

    // Bump the version — the manifest row moves, the signature must die.
    fs::write(
        dir.join("mlog.toml"),
        "[package]\nname = \"myproj\"\nversion = \"0.2.0\"\nedition = \"2024\"\n",
    )
    .unwrap();

    let (ok, _, err) = run_mlogpkg(&["verify", "--key", kf.to_str().unwrap()], &dir);
    assert!(!ok, "verify must REFUSE a manifest edit");
    assert!(
        err.contains("manifest divergence")
            || err.contains("does not match the tree")
            || err.contains("signature mismatch"),
        "stderr: {}",
        err
    );
}

#[test]
fn test_n910_sign_refuses_stale_document() {
    let dir = temp_dir("stale_doc");
    create_project(&dir, "myproj");
    let kf = keyfile(&dir, "a");
    run_mlogpkg(&["pack"], &dir);

    // Edit the source AFTER pack — the stored document is now stale.
    let src = dir.join("src/main.mlog");
    let body = fs::read_to_string(&src).unwrap();
    fs::write(&src, format!("{}\n", body)).unwrap();

    let (ok, _, err) = run_mlogpkg(&["sign", "--key", kf.to_str().unwrap()], &dir);
    assert!(!ok, "sign must REFUSE a stale document");
    assert!(err.contains("stale"), "stderr: {}", err);
}

#[test]
fn test_n910_keygen_permissions_0600() {
    let dir = temp_dir("keygen_perm");
    create_project(&dir, "myproj");
    let kf = dir.join("key_perm.hex");
    let (ok, _, err) = run_mlogpkg(&["keygen", "--out", kf.to_str().unwrap()], &dir);
    assert!(ok, "keygen: {}", err);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&kf).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the key file must be 0o600");
    }
    let content = fs::read_to_string(&kf).unwrap();
    assert_eq!(content.trim().len(), 64, "32 bytes as hex = 64 chars");
}

#[test]
fn test_n910_pack_refuses_no_manifest() {
    let dir = temp_dir("pack_no_manifest");
    let (ok, _, err) = run_mlogpkg(&["pack"], &dir);
    assert!(!ok, "pack must REFUSE a tree without mlog.toml");
    assert!(err.contains("mlog.toml not found"), "stderr: {}", err);
}
