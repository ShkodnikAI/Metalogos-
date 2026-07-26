// ── Integration test: REPL receives 3 lines on stdin ─────────────────
// Contract: feed entity, pattern, flow lines incrementally; last line produces output.
// This tests the feed_line() path that REPL uses for persistent state.

#[test]
fn repl_integration_three_lines() {
    use metalogos::interpreter::Interpreter;

    let mut interp = Interpreter::new();

    // Line 1: entity declaration (no output expected)
    let line1 = r#"entity greeting: String = "Hello, METALOGOS!!""#;
    let result1 = metalogos::feed_line(&mut interp, line1).unwrap();
    assert!(
        result1.is_none(),
        "entity declaration should produce no output, got: {:?}",
        result1
    );

    // Line 2: pattern declaration (no output expected)
    let line2 = r#"pattern Shout(text: String) -> String { return upper(text) }"#;
    let result2 = metalogos::feed_line(&mut interp, line2).unwrap();
    assert!(
        result2.is_none(),
        "pattern declaration should produce no output, got: {:?}",
        result2
    );

    // Line 3: flow declaration (produces output)
    let line3 = r#"flow Main { input: String = greeting -> Shout -> output }"#;
    let result3 = metalogos::feed_line(&mut interp, line3).unwrap();
    assert!(result3.is_some(), "flow declaration should produce output");
    assert_eq!(result3.unwrap().trim(), "HELLO, METALOGOS!!");
}
