fn main() {
    std::env::set_var("METALOGOS_STATIC_LABEL_GATE_OFF", "1");
    let src = r#"
pattern P(d: String) -> String {
  let _ = respond(http_post("http://127.0.0.1:1/x", "hi"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;
    match metalogos::check_program(src) {
        Err(e) => println!("check err: {}", e),
        Ok(r) => for f in &r.errors { println!("finding: {}", f); },
    }
}
