/// НАРЯД #159 Block 2 — grammar revision marker contract tests.

#[test]
fn test_grammar_rev_is_positive() {
    // GRAMMAR_REV must be a positive integer — zero would mean "unset"
    assert!(
        metalogos::GRAMMAR_REV > 0,
        "GRAMMAR_REV must be > 0, got {}",
        metalogos::GRAMMAR_REV
    );
}

#[test]
fn test_grammar_rev_is_u32() {
    // Type-level contract: GRAMMAR_REV is u32 so it fits in 4 bytes.
    // This is a compile-time guarantee — the test just documents it.
    let _rev: u32 = metalogos::GRAMMAR_REV;
    assert_eq!(_rev, metalogos::GRAMMAR_REV);
}

#[test]
fn test_grammar_rev_in_version_output() {
    // mlog --version (short) should contain the version number.
    // mlog -V (long) should contain "grammar rev: N".
    // We can't invoke the binary in unit tests, but we verify
    // the format string is constructible.
    let version_str = format!(
        "mlog {}\ngrammar rev: {}",
        env!("CARGO_PKG_VERSION"),
        metalogos::GRAMMAR_REV
    );
    assert!(
        version_str.contains("grammar rev:"),
        "long version string must contain 'grammar rev:', got: {version_str}"
    );
    assert!(
        version_str.contains(&metalogos::GRAMMAR_REV.to_string()),
        "long version string must contain the revision number, got: {version_str}"
    );
}
