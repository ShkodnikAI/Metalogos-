#[test]
fn test_distill_to_unknown_reflex() {
    let source = r#"
learnable pattern BadDistill(input: String) -> String {
  prompt: "answer"
  distill_to: NonExistentHead
  distill_after: 5
}

flow Main { input: String = "x" -> BadDistill -> output }
"#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "should fail");
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("distill_to") && e.message.contains("NonExistentHead")));
    println!("OK");
}
