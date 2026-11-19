// ── №976 (issue #1985): the doctest-fence pin — the two fences that sat
//    broken behind green gates since 02.10/31.10 stay NON-COMPILABLE
//    doc forms (the reasons live in the fences themselves) ────────────
//
// The blocking set promised `cargo test` but never ran the Rust
// doctests; the rust-doc-tests (blocking) job runs them now, and this
// pin keeps the two migrated fences honest: a future edit that turns
// either back into a compiling Rust example must re-justify it here.

#[test]
fn n976_the_two_migrated_fences_stay_non_compilable() {
    let html = std::fs::read_to_string("src/html_label.rs").expect("html_label.rs");
    assert!(
        html.contains("//! ```text\n//! Labeled(String, Untrusted)\n//! ```"),
        "the html_label type-notation block must stay a ```text fence — \
         rustdoc compiled the indented form as Rust and broke the doc \
         build (№976)"
    );
    let algebra = std::fs::read_to_string("src/refusal_algebra.rs").expect("refusal_algebra.rs");
    assert!(
        algebra.contains("/// ```ignore\n/// // №976: SSOT dual-compile"),
        "the refusal_algebra join example must stay an ```ignore fence — \
         the file compiles under BOTH crates (the №873 #[path] seam) and \
         cannot name a crate that differs per unit (№976)"
    );
}
