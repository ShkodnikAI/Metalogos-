// ── №976 (issue #1985): the doctest-fence pin — the two doc fences the
//    blocking set never ran stay pinned in their FIXED forms ──────────
//
// The blocking set promised `cargo test` but never ran the Rust
// doctests; the rust-doc-tests (blocking) job runs them now, and this
// pin keeps the two migrated fences honest:
//
//   1. src/html_label.rs — the type-notation block was an INDENTED doc
//      block that rustdoc compiled as Rust (`Labeled(String, Untrusted)`
//      is not a term: E0423/E0425, the green-gate blind spot since the
//      file was written). It must stay an explicit ```text fence.
//
//   2. src/refusal_algebra.rs — the join example is the №873 SSOT
//      dual-compile case: the file is included VERBATIM by BOTH crates
//      (metalogos and refusal-wasm via the `#[path]` seam — verified:
//      the compiling form PASSED under metalogos and FAILED under
//      refusal-wasm, `use metalogos::` does not resolve there). It
//      must stay a ```ignore fence with the reason IN THE FENCE; the
//      semantic pin lives in the module's unit tests (naryad_854).
//
// The blocking-checks.tsv cell binds to this file by name
// (naryad_976_doctest_fences) — the cell dies when the file is renamed.
//
// Naryad #475's fs_gate ratchet targets PRODUCTION I/O paths; this test
// reads the two SOURCE files it pins by design — the scoped allow
// mirrors the naryad_481/465 test posture (naryad_507/517).
#![allow(clippy::disallowed_methods)]

#[test]
fn naryad_976_doctest_fences() {
    let html = std::fs::read_to_string("src/html_label.rs").expect("html_label.rs");
    assert!(
        html.contains("//! ```text\n//! Labeled(String, Untrusted)\n//! ```"),
        "the html_label type-notation block must stay a ```text fence — \
         rustdoc compiled the indented form as Rust and broke the doc \
         build (№976)"
    );
    assert!(
        !html.contains("\n    Labeled(String, Untrusted)"),
        "no indented pseudo-fence may come back in html_label — rustdoc \
         would compile it as Rust again (№976)"
    );

    let algebra = std::fs::read_to_string("src/refusal_algebra.rs").expect("refusal_algebra.rs");
    assert!(
        algebra.contains("/// ```ignore\n/// // №976: SSOT dual-compile"),
        "the refusal_algebra join example must stay an ```ignore fence — \
         the file compiles under BOTH crates (the №873 #[path] seam) and \
         cannot name a crate that differs per unit (№976)"
    );
    let wasm = std::fs::read_to_string("refusal-wasm/src/lib.rs").expect("refusal-wasm/src/lib.rs");
    assert!(
        wasm.contains("#[path = \"../../src/refusal_algebra.rs\"]"),
        "the refusal-wasm seam is the reason the fence is ignore — the \
         pin dies if the seam moves, and the fence then needs its \
         re-justification (№976)"
    );
}
