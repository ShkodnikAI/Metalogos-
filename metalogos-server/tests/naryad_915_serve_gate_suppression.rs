// ── tests/naryad_915_serve_gate_suppression.rs ───────────────────────
// №915 (issue #1819, patch 0.38.3 — [process][bugfix]): the №897
// per-module suppression surface, now on the SERVE and MCP-SERVE startup
// gates — the third leg of the check ↔ run ↔ serve agreement.
//
// The bug (main ee5a5c03 → 8bcee429, v0.38.2): №897 fixed the RUN gate
// (src/lib.rs:252 — `filter_suppressed_unchecked_per_module`), but the
// №523 serve gate (metalogos-server/src/server.rs:811) still filtered
// against the ENTRY source text only — an allow-comment sitting in an
// IMPORTED module was invisible to it, so a program clean under
// `mlog check` (and running under `mlog run` since №897) refused at
// `mlog serve` with "Compilation error (Naryad #523): semantic findings
// block serve startup". The №557 mcp-serve gate held the SAME dirt in a
// harder form: NO suppression filter at all (the №802-era "no source
// threading" note) — even the ENTRY file's own sanctioned comments were
// invisible there.
//
// The fix: every startup gate applies the same
// `filter_suppressed_unchecked_per_module` over the SAME import groups
// (`resolve_import_groups`) — each finding is sanctioned only by its
// own module's allow-comments; the per-line quota and the
// non-empty-reason rule are unchanged; one module's comment cannot
// launder another module's access (pinned by T3).
//
// RED-before (the №897 file's form): T1/T3/T4 refused on v0.38.2 with
// "[UNCHECKED_RESULT] … block serve startup" / "block mcp-serve
// startup" — the module's own sanctioned comment never matched.
//
// The serve fixtures resolve imports from the cwd (the gate's base_dir
// is "." — the SAME rule the runtime loader uses in serve mode), so the
// module files are written into the cwd under n915-unique names and
// removed on exit (the №897 fixtures-in-the-real-filesystem posture;
// the names are unique per test — the suite runs the tests in
// parallel).

#![allow(clippy::disallowed_methods)] // the №199 form: fixtures write the REAL filesystem by design

#[cfg(feature = "server")]
use std::time::Duration;

/// The sanctioned module (the №897 specimen, byte-for-byte the same
/// shape): the try result's `.value` is read on the SAME line as its
/// trailing allow-comment.
const DEP_SANCTIONED: &str = "\
pattern ReadSanctioned(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value // allow(unchecked_result): the sanctioned read — the redact(\"hash_only\") barrier sits downstream (naryad #802 form)
}
";

/// The same module WITHOUT the allow-comment — the negative arm of the
/// regression pair (the suppression must not appear from nowhere).
const DEP_UNSANCTIONED: &str = "\
pattern ReadSanctioned(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value
}
";

/// The unsanctioned SIBLING for the laundering pin — a DIFFERENT pattern
/// name, so the two modules' declarations never collide in the merged
/// surface (the collision dedup would drop the second landing and mute
/// the pin).
const DEP_DIRTY: &str = "\
pattern ReadDirty(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value
}
";

fn write_module(name: &str, body: &str) {
    std::fs::write(name, body).unwrap_or_else(|e| panic!("fixture module {name}: {e}"));
}

fn remove_module(name: &str) {
    let _ = std::fs::remove_file(name);
}

/// The serve entry: imports the module and CALLS its pattern from the
/// route body — the repro shape (the finding fires from the MODULE's
/// body at the static gate, before any bind).
fn serve_entry(module: &str, pattern: &str) -> String {
    format!(
        "import {module}\n\nmlogserver {{\n  port: 0\n  route \"/ping\" method=GET {{\n    let v = {pattern}(\"x\")\n    return respond(v)\n  }}\n}}\n"
    )
}

// ── T1: the regression pair, positive arm — the server STARTS ────────

/// T1: an imported module with a sanctioned read → the №523 serve gate
/// passes and `run_server` reaches its serving phase (RED-before: the
/// startup refused with "block serve startup" — the module's comment
/// never matched the entry-only source). The proof is the task's
/// LIVENESS: the gate refusal is synchronous-before-bind (T2 shows it
/// lands in well under the window), so a future that is still running
/// after the window has passed the gate and is serving.
#[cfg(feature = "server")]
#[tokio::test]
async fn n915_imported_module_allow_comment_starts_serve() {
    const MOD: &str = "n915_serve_ok.mlog";
    write_module(MOD, DEP_SANCTIONED);
    let src = serve_entry("n915_serve_ok", "ReadSanctioned");

    let mut handle = Some(tokio::spawn({
        let src = src.clone();
        async move { metalogos_server::server::run_server(&src).await }
    }));
    let mut refused: Option<String> = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if handle.as_ref().expect("task handle").is_finished() {
            // The startup ENDED inside the window — for a refused program
            // that is exactly the bug shape. Recover the refusal for the
            // failure message.
            refused = match handle.take().expect("task handle").await {
                Err(joined) => Some(format!("join error: {joined}")),
                Ok(Err(e)) => Some(e.to_string()),
                Ok(Ok(())) => Some("run_server returned Ok unexpectedly".to_string()),
            };
            break;
        }
    }
    if let Some(h) = handle.as_ref() {
        if !h.is_finished() {
            h.abort();
        }
    }
    remove_module(MOD);

    assert!(
        refused.is_none(),
        "the module's own allow-comment must let the serve startup pass — refused with: {:?}",
        refused
    );
}

// ── T2: the regression pair, negative arm — startup BLOCKED ──────────

/// T2: the same module WITHOUT the comment → `mlog serve` still refuses
/// at startup with the stable code (the suppression is not granted
/// silently; the №523 posture is intact).
#[cfg(feature = "server")]
#[tokio::test]
async fn n915_imported_module_without_comment_blocks_serve() {
    const MOD: &str = "n915_serve_nok.mlog";
    write_module(MOD, DEP_UNSANCTIONED);
    let src = serve_entry("n915_serve_nok", "ReadSanctioned");

    let err = metalogos_server::server::run_server(&src)
        .await
        .expect_err("the unsanctioned module read must block the serve startup");
    remove_module(MOD);

    let text = err.to_string();
    assert!(
        text.contains("UNCHECKED_RESULT"),
        "the refusal carries the stable code, got: {}",
        text
    );
    assert!(
        text.contains("Naryad #523"),
        "the refusal is the №523 serve gate's, got: {}",
        text
    );
    assert!(
        text.contains("serve startup"),
        "the refusal names the serve surface, got: {}",
        text
    );
}

// ── T3: cross-module laundering stays closed on the serve path ───────

/// T3: one module's allow-comment must NOT launder another module's
/// identical access at serve startup — the sanctioned sibling runs
/// clean, the unsanctioned one still blocks (the №802 laundering rule,
/// per module, now pinned on serve too — the proof that the gate really
/// applies the GROUPS form, not a merged-source hash).
#[cfg(feature = "server")]
#[tokio::test]
async fn n915_serve_one_module_comment_cannot_launder_another() {
    const MOD_OK: &str = "n915_serve_launder_ok.mlog";
    const MOD_DIRTY: &str = "n915_serve_launder_dirty.mlog";
    write_module(MOD_OK, DEP_SANCTIONED);
    write_module(MOD_DIRTY, DEP_DIRTY);
    let src = serve_entry("n915_serve_launder_dirty", "ReadDirty");

    let err = metalogos_server::server::run_server(&src)
        .await
        .expect_err("the dirty module's finding must survive at serve startup");
    remove_module(MOD_OK);
    remove_module(MOD_DIRTY);

    assert!(
        err.to_string().contains("UNCHECKED_RESULT"),
        "the refusal carries the stable code, got: {}",
        err
    );
}

// ── T4: the mcp-serve gate — the same pair, positive arm ─────────────

/// T4: the №557 gate with the source threaded (№915) — a program whose
/// sanctioning comment sits in an IMPORTED module passes mcp-serve
/// startup (RED-before: the unfiltered surface refused it).
#[test]
fn n915_mcp_imported_module_allow_comment_passes() {
    const MOD: &str = "n915_mcp_ok.mlog";
    write_module(MOD, DEP_SANCTIONED);
    let src = "import n915_mcp_ok\n\ntool greets {\n  hello(name: String) -> String {\n    return ReadSanctioned(name)\n  }\n}\n";
    let declarations = metalogos::parser::parse(src).expect("the fixture must parse");

    let verdict = metalogos_server::mcp_server::enforce_semantic_startup(&declarations, src);
    remove_module(MOD);

    verdict.expect("the module's own allow-comment must pass the mcp-serve startup gate");
}

// ── T5: the mcp-serve gate — the same pair, negative arm ─────────────

/// T5: the same module WITHOUT the comment → mcp-serve still refuses at
/// startup (the №557 posture is intact — the filter never grants
/// silently).
#[test]
fn n915_mcp_imported_module_without_comment_blocks() {
    const MOD: &str = "n915_mcp_nok.mlog";
    write_module(MOD, DEP_UNSANCTIONED);
    let src = "import n915_mcp_nok\n\ntool greets {\n  hello(name: String) -> String {\n    return ReadSanctioned(name)\n  }\n}\n";
    let declarations = metalogos::parser::parse(src).expect("the fixture must parse");

    let err = metalogos_server::mcp_server::enforce_semantic_startup(&declarations, src)
        .expect_err("the unsanctioned module read must block the mcp-serve startup");
    remove_module(MOD);

    assert!(
        err.contains("UNCHECKED_RESULT"),
        "the refusal carries the stable code, got: {}",
        err
    );
    assert!(
        err.contains("Naryad #557"),
        "the refusal is the №557 mcp-serve gate's, got: {}",
        err
    );
}

// ── T6: the ENTRY file's own comment works on mcp-serve now ──────────

/// T6: the harder half of the mcp-serve dirt — the №802-era unfiltered
/// surface ignored even the ENTRY file's own sanctioned comments; with
/// the source threaded, the entry's own comment sanctions its own
/// finding (the check ↔ mcp-serve agreement the №802 note requires).
#[test]
fn n915_mcp_entry_own_comment_passes() {
    let src = "pattern ReadHere(_input: String) -> String {\n  let r = try trim(\"hello\")\n  return r.value // allow(unchecked_result): the sanctioned read in the entry itself\n}\n\ntool greets {\n  hello(name: String) -> String {\n    return ReadHere(name)\n  }\n}\n";
    let declarations = metalogos::parser::parse(src).expect("the fixture must parse");
    metalogos_server::mcp_server::enforce_semantic_startup(&declarations, src)
        .expect("the entry's own allow-comment must pass the mcp-serve startup gate");
}
