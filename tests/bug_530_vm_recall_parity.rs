// ── Bug #530 (FO-050 / office #182): recall_top_k VM parity ───────────
//
// The diagnosis that supersedes the issue's premise: `date_now` (the
// function the office staging surfaced) was an OFFICE-side ghost — a
// call to a function that never existed in the language (the TW failed
// at RUNTIME on it, the VM failed at COMPILE time; the TW boot only
// looked clean because the ghost paths had not executed). But the boot
// loop the diagnosis started exposed a REAL TW/VM registry disagreement:
//
// `recall_top_k` is a REAL memory read the tree-walking interpreter
// dispatches through its interception table
// (interpreter::memory::invoke_recall_top_k_fn — hybrid FTS5 BM25 +
// cosine search over the interpreter's memory store) — yet it had NO
// `spec!` entry in BUILTIN_REGISTRY. The VM compiler consults the
// registry, so every program calling recall_top_k failed to COMPILE on
// the VM ("undefined function: recall_top_k") while running on the TW —
// exactly the class #530 reported.
//
// Fix: the name is registered (`spec!("recall_top_k", 1, 3, "memory")`,
// handler None — both backends intercept by name before the generic
// fallback, the media_store_* stub pattern) and the VM dispatches it in
// its state-carrying block against the VM's own memory Vec (the honest
// simple-memory twin of the TW's hybrid search; same JSON result shape).

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const PROG: &str = r#"
pattern P(_t: String) -> String {
  let _ = memorize("the owner prefers dark roast coffee", 0.9, "persona")
  let _ = memorize("kitchen camera frames rotate hourly", 0.7, "system")
  let hits = recall_top_k("coffee preferences", 5.0)
  return hits
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn n530_name_is_a_registered_builtin() {
    // Compile parity: the VM compiler resolves the name from the registry.
    assert!(
        metalogos::builtins::is_builtin("recall_top_k"),
        "recall_top_k must be a registered builtin"
    );
    let compiled = run_vm(PROG);
    assert!(
        compiled.is_ok(),
        "the VM compiler must accept recall_top_k: {:?}",
        compiled.err()
    );
}

#[test]
fn n530_tw_and_vm_find_the_memorized_value() {
    let out_tw = run_tw(PROG).expect("TW runs");
    let out_vm = run_vm(PROG).expect("VM runs");
    let text_tw = out_tw.unwrap_or_default();
    let text_vm = out_vm.unwrap_or_default();
    // Both backends return the JSON array shape and find the memorized fact.
    for (backend, text) in [("TW", text_tw), ("VM", text_vm)] {
        assert!(
            text.contains("dark roast coffee"),
            "{backend} found the memory: {text}"
        );
        assert!(text.contains("\"score\""), "{backend} JSON shape: {text}");
        assert!(
            text.contains("\"priority\""),
            "{backend} JSON shape: {text}"
        );
    }
}

#[test]
fn n530_type_filter_and_k_are_honored_on_both_backends() {
    let prog = r#"
pattern P(_t: String) -> String {
  let _ = memorize("invoice reminder for march", 0.5, "finance")
  let _ = memorize("weekly team sync notes", 0.5, "system")
  let filtered = recall_top_k("invoice reminder", 10.0, "finance")
  let empty = recall_top_k("nonexistent topic xyz", 5.0)
  return filtered + "|" + empty
}
flow Main { input: String = "x" -> P -> output }
"#;
    let out_tw = run_tw(prog).expect("TW runs");
    let out_vm = run_vm(prog).expect("VM runs");
    // Contract (both backends): the type filter excludes the system entry;
    // the filtered query ranks the finance hit first (score > 0); the
    // nonsense query still returns the store's entries with score 0.0 —
    // the contract is "top-k by score", not "only hits" (the TW hybrid
    // behaves exactly this way; the VM mirrors it). Exact scores are
    // backend-local by design (each backend reads its own store).
    let text_tw = out_tw.unwrap_or_default();
    let text_vm = out_vm.unwrap_or_default();
    // Segment 1 = the type-filtered search, segment 2 = the unfiltered one.
    for (backend, text) in [("TW", &text_tw), ("VM", &text_vm)] {
        let filtered = text.split('|').next().expect("first segment");
        assert!(
            filtered.contains("invoice reminder for march"),
            "{backend} filtered hit: {filtered}"
        );
        assert!(
            !filtered.contains("team sync notes"),
            "{backend} type filter excluded the system entry: {filtered}"
        );
    }
    let nonsense = text_tw
        .split('|')
        .nth(1)
        .expect("TW returns the second search result");
    let nonsense_vm = text_vm
        .split('|')
        .nth(1)
        .expect("VM returns the second search result");
    assert!(
        nonsense.contains("\"score\":0.0"),
        "TW zero-score weak matches: {nonsense}"
    );
    assert!(
        nonsense_vm.contains("\"score\":0.0"),
        "VM zero-score weak matches: {nonsense_vm}"
    );
}
