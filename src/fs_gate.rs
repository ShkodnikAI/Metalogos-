//! №475 (issue #723) — the filesystem FACADE: one gated entry point for
//! every file operation a program can influence.
//!
//! The audit 26.09 §3.1 (High) finding: the №455 gate was embedded in a
//! single function (`file_ingest_gate` — called only by `read_file` and
//! `office/text.rs`), so every other filesystem path went past it:
//!   - group A: `write_file`/`append_file`/`delete_file`/`list_dir`/
//!     `http_download` — sandbox only, NO deny-list, NO serve data-dir
//!     containment (a route could rewrite `app.mlog` — reading `*.mlog`
//!     was forbidden while WRITING it was allowed: the asymmetry the
//!     audit names);
//!   - group B: pdf.rs, `smtp_send` attachments, `config_load` — raw
//!     `std::fs` reads/writes, absolute paths included, past everything;
//!   - group C: the deny-list missed `app.db-wal/-journal/-shm` and the
//!     credential-name classes (`*.pem`, `*.key`, `id_rsa*`, …).
//!
//! The fix is the fix AT THE COMMON POINT (the naryad): every operation
//! goes through THIS module, and each function applies all three №455
//! layers plus the sandbox:
//!   1. the sandbox (№131/№252): relative-path resolution, `..` refusal,
//!      symlink-proof canonicalization (TOCTOU-safe write opens);
//!   2. the deny-list on BOTH reads and writes (the shared №455
//!      vocabulary, extended by №475 task 4) with the layer-3 allowlist
//!      crane for reads and non-critical writes;
//!   3. the serve-route data-directory containment (reads AND writes:
//!      a route handler touches only `METALOGOS_DATA_DIR`).
//!
//! Plus the №475 task-5 HARD write deny: `*.mlog`, `.env*`,
//! `metalogos.toml`, `.git/**` are refused for writes ALWAYS — the
//! allowlist crane does NOT apply (the application image integrity is
//! not a per-deployment policy).
//!
//! The ratchet (№475 task 2): `clippy.toml` disallows the raw `std::fs`
//! methods outside this module and the explicitly-justified service
//! modules (the weights store, the journal, the compile-time source
//! loaders, the docs harness) — any new bypass fails CI.

use crate::builtins::io::{
    current_exec_context, file_ingest_gate, sandbox_path, sandbox_path_ex,
    sandbox_sensitive_violation, sandbox_violation, sensitive_allowlisted, sensitive_name_match,
    sensitive_path_match, serve_data_dir_root, ExecContext, SandboxMode,
};

// ═══ The №475 hard write-deny vocabulary (task 5) ════════════════════
//
// A write (create/overwrite/append/remove) onto these names is refused
// UNCONDITIONALLY — `METALOGOS_SENSITIVE_PATH_ALLOWLIST` cannot unlock
// them. Matched on the RAW path form AND on the resolved canonical form
// (a symlink named `notes.txt` pointing at `app.mlog` refuses the same
// way), on every path component for the `.git/**` case.
fn hard_write_name(name: &str) -> bool {
    name.starts_with(".env") || name.ends_with(".mlog") || name == "metalogos.toml"
}

fn hard_write_path_hit(path: &std::path::Path) -> bool {
    if let Some(name) = path.file_name() {
        if hard_write_name(&name.to_string_lossy()) {
            return true;
        }
    }
    path.components()
        .any(|c| matches!(c, std::path::Component::Normal(c) if c == ".git"))
}

// ═══ The write-side gate (deny-list + serve containment) ════════════
//
// The read-side gate is io.rs `file_ingest_gate` (№455, messages and
// tests pinned there). The write side shares the deny vocabulary and
// the containment invariant, with write-accurate messages and the hard
// deny in front of everything.

/// Layer 0 + the raw half of layer 1 for WRITES: the hard write-deny
/// (task 5) and the raw-form deny-list with the allowlist crane. Runs
/// BEFORE path resolution — an attempt to write the application image
/// is a signal by itself, "the file does not exist" does not make it
/// safe (the №455 raw-deny posture).
pub(crate) fn precheck_write_raw(raw: &str, purpose: &str) -> Result<(), String> {
    if hard_write_path_hit(std::path::Path::new(raw)) {
        return Err(hard_write_error(purpose, raw));
    }
    if !sensitive_allowlisted(raw) && sensitive_path_match(raw) {
        return Err(sandbox_sensitive_violation(format!(
            "{}('{}'): the path matches the sensitive-path deny-list \
             (writes: .env*, *.db*, *.pem, *.key, id_rsa*, id_ed25519*, \
             .netrc, .npmrc, credentials*, *.sqlite*, .git/**, metalogos.toml, \
             .mlog/**) — set METALOGOS_SENSITIVE_PATH_ALLOWLIST=\"NAME\" to \
             allow a specific file explicitly (Naryad #455/#475)",
            purpose, raw
        )));
    }
    Ok(())
}

/// The resolved half of the write gate: the hard deny on the canonical
/// form (symlink-proof) + the resolved-name deny-list + the serve
/// data-dir containment. Call AFTER the sandbox resolved the path.
pub(crate) fn gate_write_resolved(
    raw: &str,
    resolved: &std::path::Path,
    purpose: &str,
) -> Result<(), String> {
    if hard_write_path_hit(resolved) {
        return Err(hard_write_error(purpose, raw));
    }

    let allowlisted = sensitive_allowlisted(raw);
    if !allowlisted {
        if let Some(name) = resolved
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
        {
            if sensitive_name_match(&name)
                || resolved.components().any(
                    |c| matches!(c, std::path::Component::Normal(c) if c == ".git" || c == ".mlog"),
                )
            {
                return Err(sandbox_sensitive_violation(format!(
                    "{}('{}'): the resolved path '{}' matches the sensitive-path \
                     deny-list — set METALOGOS_SENSITIVE_PATH_ALLOWLIST=\"NAME\" \
                     to allow a specific file explicitly (Naryad #455/#475)",
                    purpose,
                    raw,
                    resolved.display()
                )));
            }
        }
    }

    // №500: the write-side canonical re-check — the mirror of the
    // read side's documented canonical re-check (io.rs `file_ingest_gate`):
    // `resolved` carries the canonical PARENT plus the final component
    // (the №252 contract), so a symlink NAMED innocently but pointing at
    // the application image passes the name checks above. Canonicalize
    // the full path and re-run the hard deny and the deny-list name
    // check on the TARGET — the policy names the swap before the OS's
    // O_NOFOLLOW does (the pin: tests/naryad_500_pdf_fs_gate.rs).
    if let Ok(canonical) = resolved.canonicalize() {
        if canonical != resolved {
            if hard_write_path_hit(&canonical) {
                return Err(hard_write_error(purpose, raw));
            }
            if !allowlisted {
                if let Some(name) = canonical
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                {
                    if sensitive_name_match(&name) {
                        return Err(sandbox_sensitive_violation(format!(
                            "{}('{}'): the symlink resolves to '{}' which matches the \
                             sensitive-path deny-list (Naryad #500)",
                            purpose,
                            raw,
                            canonical.display()
                        )));
                    }
                }
            }
        }
    }

    // Layer 2: the serve-route containment — writes land in the data
    // directory only. The allowlist does NOT bypass this (containment is
    // not a name policy). Process context unchanged.
    if current_exec_context() == ExecContext::ServeRoute {
        let root = serve_data_dir_root().map_err(sandbox_sensitive_violation)?;
        if !resolved.starts_with(&root) {
            return Err(sandbox_sensitive_violation(format!(
                "{}('{}'): file writes in serve route handlers are restricted to the \
                 data directory — resolved path '{}' is outside '{}' \
                 (METALOGOS_DATA_DIR, default ./data; Naryad #475)",
                purpose,
                raw,
                resolved.display(),
                root.display()
            )));
        }
    }
    Ok(())
}

fn hard_write_error(purpose: &str, raw: &str) -> String {
    sandbox_sensitive_violation(format!(
        "{}('{}'): writes to the application image (*.mlog, .env*, \
         metalogos.toml, .git/**) are ALWAYS refused — the allowlist \
         does not apply to the application image integrity (Naryad #475)",
        purpose, raw
    ))
}

// ═══ The facade (task 1) ════════════════════════════════════════════

/// Open a file for READING through the full gate: the sandbox (№131),
/// the №455 ingest gate (deny-list raw+resolved, allowlist crane, serve
/// data-dir containment). `purpose` is pure error-message context
/// (e.g. "read_file", "smtp_send attachment").
///
/// On success returns the opened handle on the RESOLVED path; the caller
/// reads through the `std::io::Read` traits (the raw `std::fs::read*`
/// methods stay disallowed outside this module). The OS-level outcome
/// contract (soft vs loud) remains the caller's — the facade owns the
/// POLICY, not the outcome classification.
pub(crate) fn open_read(raw: &str, purpose: &str) -> Result<std::fs::File, String> {
    let resolved = sandbox_path(raw).map_err(sandbox_violation)?;
    file_ingest_gate(purpose, raw, &resolved)?;
    open_gated(&resolved)
        .map_err(|e| format!("{}('{}'): cannot open for read: {}", purpose, raw, e))
}

/// Open a file for WRITING through the full gate: the sandbox (№131/№252),
/// the hard write-deny (task 5), the deny-list with the allowlist crane,
/// the serve data-dir containment, then the TOCTOU-safe open (№252).
/// Parent directories are created best-effort on the resolved parent.
pub(crate) fn open_write(raw: &str, purpose: &str, append: bool) -> Result<std::fs::File, String> {
    precheck_write_raw(raw, purpose)?;
    let resolved = sandbox_path_ex(raw, SandboxMode::ForWrite).map_err(sandbox_violation)?;
    gate_write_resolved(raw, &resolved, purpose)?;
    if let Some(parent) = resolved.parent() {
        // Parent preparation on an already resolved+gated path
        // (best-effort, the same posture write_file had before the
        // facade). The raw call lives in THE facade (№500 ratchet).
        #[allow(clippy::disallowed_methods)]
        let _ = std::fs::create_dir_all(parent);
    }
    open_sandbox_write(&resolved, append)
}

/// Resolve, gate, and OPEN a directory listing (read-class): the
/// sandbox, the №455 ingest gate (a directory named `.git`/`.mlog`
/// refuses; the serve containment restricts listings to the data
/// directory). Returns the gated `ReadDir` handle.
pub(crate) fn read_dir(raw: &str, purpose: &str) -> Result<std::fs::ReadDir, String> {
    let resolved = sandbox_path(raw).map_err(sandbox_violation)?;
    file_ingest_gate(purpose, raw, &resolved)?;
    // №475: the only raw read_dir outside the allow-modules.
    #[allow(clippy::disallowed_methods)]
    std::fs::read_dir(&resolved).map_err(|e| format!("{}('{}'): {}", purpose, raw, e))
}

/// Open an ALREADY-GATED resolved path. The caller ran `sandbox_path`
/// and `file_ingest_gate`; the raw `File::open` lives HERE so the
/// №475 ratchet (clippy disallowed-methods) holds everywhere else.
#[allow(clippy::disallowed_methods)] // №475: the raw open is the facade's own primitive
pub(crate) fn open_gated(resolved: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(resolved)
}

/// Read the WHOLE file through the full read gate into a String.
/// Convenience for callers whose outcome contract is loud on read.
pub(crate) fn read_to_string(raw: &str, purpose: &str) -> Result<String, String> {
    use std::io::Read;
    let mut file = open_read(raw, purpose)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .map_err(|e| format!("{}('{}'): read failed: {}", purpose, raw, e))?;
    Ok(buf)
}

/// Read the WHOLE file through the full read gate into bytes.
pub(crate) fn read_bytes(raw: &str, purpose: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut file = open_read(raw, purpose)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("{}('{}'): read failed: {}", purpose, raw, e))?;
    Ok(buf)
}

/// №500: create a directory through the write gate — the program-facing
/// output dirs (pdf_extract_images/pdf_split) get the SAME vocabulary as
/// file writes: the hard write-deny, the deny-list with the crane, the
/// sandbox and the serve containment. The raw `create_dir_all` lives in
/// THE facade (the №500 ratchet extends the disallow list to it).
pub(crate) fn create_dir_all(raw: &str, purpose: &str) -> Result<(), String> {
    precheck_write_raw(raw, purpose)?;
    let resolved = sandbox_path_ex(raw, SandboxMode::ForWrite).map_err(sandbox_violation)?;
    gate_write_resolved(raw, &resolved, purpose)?;
    #[allow(clippy::disallowed_methods)] // №500: THE gated primitive itself
    std::fs::create_dir_all(&resolved).map_err(|e| format!("{}('{}'): {}", purpose, raw, e))
}

/// Write the whole byte slice through the full write gate (overwrite).
pub(crate) fn write_bytes(raw: &str, purpose: &str, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = open_write(raw, purpose, false)?;
    file.write_all(bytes)
        .map_err(|e| format!("{}('{}'): write failed: {}", purpose, raw, e))
}

// ═══ The TOCTOU-safe write open (moved verbatim from io.rs — №252) ═══
//
/// Symlink-safe open for sandbox write targets. `target` must come from
/// `sandbox_path_ex(_, SandboxMode::ForWrite)` (canonical parent + final
/// component). Two-phase open closes the final-component TOCTOU; see the
/// №252 documentation in the naryad history and the honest intermediate-
/// component boundary note (would need openat2 RESOLVE_BENEATH).
#[allow(clippy::disallowed_methods)] // №475: THE gated primitive itself — every write enters through here
pub(crate) fn open_sandbox_write(
    target: &std::path::Path,
    append: bool,
) -> Result<std::fs::File, String> {
    let attempt = if append {
        std::fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(target)
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
    };

    match attempt {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let base = std::env::current_dir()
                .map_err(|e| format!("file I/O sandbox: {}", e))?
                .canonicalize()
                .map_err(|e| format!("file I/O sandbox: {}", e))?;
            let canonical = target.canonicalize().map_err(|_| {
                sandbox_violation(format!(
                    "file I/O sandbox: cannot resolve path: '{}'",
                    target.display()
                ))
            })?;
            if !canonical.starts_with(&base) {
                return Err(sandbox_violation(format!(
                    "file I/O sandbox: resolved path escapes sandbox: '{}'",
                    target.display()
                )));
            }
            let mut opts = std::fs::OpenOptions::new();
            if append {
                opts.append(true);
            } else {
                // №254 regression (wave-3 acceptance CI, 2026-09-19): the
                // reopen of an EXISTING file used write() without
                // truncate — overwriting longer content with shorter
                // left the old bytes as a tail (a cached sidecar read
                // back as "corrupt sidecar JSON: trailing characters").
                // Overwrite mode truncates; append mode must not.
                opts.write(true).truncate(true);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.custom_flags(libc::O_NOFOLLOW);
            }
            opts.open(&canonical).map_err(|e| {
                format!(
                    "file I/O sandbox: cannot open '{}': {}",
                    target.display(),
                    e
                )
            })
        }
        Err(e) => Err(format!(
            "file I/O sandbox: cannot create '{}': {}",
            target.display(),
            e
        )),
    }
}
