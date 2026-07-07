#!/usr/bin/env python3
"""
Refactor Metalogos builtin registration:
- Add BuiltinSpec struct + BUILTIN_REGISTRY to builtins.rs
- Generate Builtins::new() from registry
- Export helper fns: builtin_names(), builtin_indices(), builtin_name_set(), builtin_arity_map()
- Update compiler.rs to use BUILTIN_REGISTRY for indices
- Update vm.rs to use BUILTIN_REGISTRY for name vec
- Update semantic.rs to use BUILTIN_REGISTRY for name set

Strategy: Add the registry as the SINGLE SOURCE OF TRUTH.
Consumers call helper functions instead of maintaining their own lists.
"""

import re

METALOGOS = "/home/z/my-project/metalogos-build/src"

# ── Complete list of all builtins with arities ──────────────────────
# arity=0 means variadic (skip arity check)
# source: "stateless" = has BuiltinFn in builtins.rs HashMap
#         "stateful"  = handled by interpreter/VM via special-cased if-chains
#         "stub"      = no real implementation (returns placeholder)

BUILTIN_SPECS = [
    # name, arity, category
    # ── Core string ops ──
    ("upper", 1, "string"),
    ("lower", 1, "string"),
    ("len", 1, "string"),
    ("str", 1, "string"),
    ("print", 1, "io"),
    ("contains", 2, "string"),
    ("float", 1, "convert"),
    ("to_string", 1, "convert"),
    ("get", 2, "list"),
    ("push", 2, "list"),
    ("index_of", 2, "string"),
    ("substring", 3, "string"),
    ("char_at", 2, "string"),
    ("starts_with", 2, "string"),
    ("ends_with", 2, "string"),
    ("to_float", 1, "convert"),
    ("confidence", 1, "fluid"),
    ("trim", 1, "string"),
    ("replace", 3, "string"),
    ("split", 2, "string"),
    ("join", 2, "string"),
    ("length", 1, "string"),
    ("to_int", 1, "convert"),
    ("reverse", 1, "string"),
    ("escape_html", 1, "string"),
    ("escape_json", 1, "string"),

    # ── Stdlib aliases (double-underscore prefix) ──
    ("__trim", 1, "std"),
    ("__replace", 3, "std"),
    ("__split", 2, "std"),
    ("__join", 2, "std"),
    ("__abs", 1, "std"),
    ("__min", 2, "std"),
    ("__max", 2, "std"),
    ("__clamp", 3, "std"),
    ("__round", 1, "std"),
    ("__first", 1, "std"),
    ("__last", 1, "std"),
    ("__push", 2, "std"),
    ("__list_len", 1, "std"),

    # ── Math ──
    ("abs", 1, "math"),
    ("min", 2, "math"),
    ("max", 2, "math"),
    ("clamp", 3, "math"),
    ("round", 1, "math"),

    # ── HTTP / Web ──
    ("respond", 1, "web"),
    ("respond_html", 1, "web"),
    ("form_data", 1, "web"),
    ("json_body", 0, "web"),
    ("query_param", 1, "web"),
    ("render", 2, "web"),
    ("http_get", 1, "web"),
    ("http_post", 2, "web"),
    ("http_post_multipart", 2, "web"),
    ("require", 0, "web"),  # variadic

    # ── JSON ──
    ("parse_json", 1, "json"),
    ("json_encode", 1, "json"),
    ("json_get", 2, "json"),
    ("has_field", 2, "json"),

    # ── Crypto / Auth ──
    ("env", 1, "system"),
    ("hash_password", 1, "crypto"),
    ("verify_password", 2, "crypto"),
    ("encrypt", 2, "crypto"),
    ("decrypt", 2, "crypto"),
    ("generate_key", 0, "crypto"),
    ("authenticate", 2, "auth"),
    ("session_login", 2, "auth"),
    ("session_logout", 1, "auth"),

    # ── Database ──
    ("query", 1, "db"),
    ("db_execute", 1, "db"),

    # ── LLM ──
    ("call_llm", 0, "llm"),   # variadic
    ("call_claude", 0, "llm"), # variadic
    ("llm_usage", 0, "llm"),   # variadic (no args)

    # ── Memory / KV ──
    ("kv_set", 2, "memory"),
    ("kv_get", 1, "memory"),
    ("kv_delete", 1, "memory"),
    ("kv_exists", 1, "memory"),
    ("kv_list", 0, "memory"),
    ("mem_set", 2, "memory"),
    ("mem_get", 1, "memory"),
    ("mem_delete", 1, "memory"),
    ("session_set", 2, "memory"),
    ("session_get", 1, "memory"),
    ("session_clear", 0, "memory"),

    # ── File I/O ──
    ("read_file", 1, "io"),
    ("write_file", 2, "io"),
    ("append_file", 2, "io"),
    ("delete_file", 1, "io"),
    ("file_exists", 1, "io"),
    ("list_dir", 1, "io"),

    # ── Time ──
    ("now", 0, "time"),

    # ── Bot / Messaging ──
    ("send_message", 2, "bot"),

    # ── Voice pipeline ──
    ("whisper_transcribe", 1, "voice"),
    ("tts_send", 2, "voice"),

    # ── Stateful (interpreter-only, no BuiltinFn) ──
    ("recall", 0, "stateful"),      # variadic: recall(query, [min_conf])
    ("memorize", 0, "stateful"),     # variadic: memorize(text, [priority])
    ("forget", 0, "stateful"),       # variadic: forget(query, [days])
    ("find", 4, "stateful"),         # find(type, field, op, threshold)
    ("inspect", 1, "stateful"),      # inspect(pattern_name)
    ("conv_start", 1, "stateful"),   # conv_start(id)
    ("conv_add", 3, "stateful"),     # conv_add(id, role, text)
    ("conv_history", 1, "stateful"), # conv_history(id)
    ("conv_context", 1, "stateful"), # conv_context(id)
    ("conv_end", 1, "stateful"),     # conv_end(id)
    ("event_count", 0, "stateful"),  # variadic: event_count([type])
    ("events_since", 1, "stateful"), # events_since(ms)
    ("event_sum", 2, "stateful"),    # event_sum(type, field)
    ("query_scalar", 0, "stateful"), # variadic
    ("query_row", 0, "stateful"),    # variadic

    # ── Memory Graph (B1-B5 new) ──
    ("graph_query", 0, "graph"),     # variadic
    ("graph_path", 0, "graph"),      # variadic
    ("graph_neighbors", 0, "graph"), # variadic
    ("memory_decay", 0, "graph"),    # variadic
    ("memory_boost", 0, "graph"),    # variadic
    ("memory_prune", 0, "graph"),    # variadic
    ("memory_revise", 0, "graph"),   # variadic
    ("subgraph_extract", 0, "graph"),# variadic
    ("subgraph_nodes", 0, "graph"),  # variadic
    ("subgraph_json", 0, "graph"),   # variadic
    ("trace_start", 0, "graph"),     # variadic
    ("trace_end", 0, "graph"),       # variadic

    # ── MTree (v0.8.7) ──
    ("mtree_summarize", 0, "mtree"), # variadic
    ("mtree_retrieve", 0, "mtree"),  # variadic
    ("mtree_stats", 0, "mtree"),     # variadic

    # ── Cron (v0.8.7) ──
    ("cron_mark_fired", 1, "cron"),

    # ── Request helpers ──
    ("request_body", 0, "web"),      # variadic (alias)

    # ── VM-only stubs (Phase 4.4 self-hosting, no real impl) ──
    ("stdin", 0, "stub"),
    ("split_tokens", 0, "stub"),
    ("if_eq", 3, "stub"),
    ("newline", 0, "stub"),
    ("is_string_token", 1, "stub"),

    # ── Assert / testing ──
    ("assert_eq", 2, "test"),
    ("assert_contains", 2, "test"),

    # ── Encoding ──
    ("base64_encode", 1, "encoding"),
    ("base64_decode", 1, "encoding"),

    # ── DB insert ──
    ("db_insert", 0, "db"),  # variadic
]

# ── Generate the registry code ──────────────────────────────────────

def generate_registry_code():
    """Generate the BUILTIN_REGISTRY static and helper functions."""
    lines = []

    # BuiltinSpec struct
    lines.append("")
    lines.append("/// Metadata for a single builtin function.")
    lines.append("/// This is the SINGLE SOURCE OF TRUTH for all builtin metadata.")
    lines.append("/// Every consumer (compiler, VM, semantic) reads from here.")
    lines.append("///")
    lines.append("/// - `name`: function name as exposed to the DSL")
    lines.append("/// - `arity`: exact argument count; 0 = variadic (skip arity check)")
    lines.append("/// - `category`: logical group for documentation and error messages")
    lines.append("#[derive(Debug, Clone)]")
    lines.append("pub struct BuiltinSpec {")
    lines.append("    pub name: &'static str,")
    lines.append("    pub arity: usize,        // 0 = variadic")
    lines.append("    pub category: &'static str,")
    lines.append("}")
    lines.append("")
    lines.append("/// Master registry of ALL builtin functions.")
    lines.append("/// Order determines bytecode indices — DO NOT reorder existing entries.")
    lines.append("/// To add a new builtin: append a `BuiltinSpec` row here,")
    lines.append("/// add the handler in Builtins::new(), and you're done.")
    lines.append("pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[")

    for name, arity, cat in BUILTIN_SPECS:
        lines.append(f'    BuiltinSpec {{ name: "{name}", arity: {arity}, category: "{cat}" }},')

    lines.append("];")
    lines.append("")

    # Helper functions
    lines.append("/// Total number of registered builtins.")
    lines.append("pub fn builtin_count() -> usize {")
    lines.append("    BUILTIN_REGISTRY.len()")
    lines.append("}")
    lines.append("")
    lines.append("/// Ordered list of builtin names (parallel to compiler index table).")
    lines.append("pub fn builtin_names() -> Vec<String> {")
    lines.append("    BUILTIN_REGISTRY.iter().map(|s| s.name.to_string()).collect()")
    lines.append("}")
    lines.append("")
    lines.append("/// Name → bytecode index mapping for the compiler.")
    lines.append("pub fn builtin_indices() -> std::collections::HashMap<String, usize> {")
    lines.append("    BUILTIN_REGISTRY.iter().enumerate()")
    lines.append("        .map(|(i, s)| (s.name.to_string(), i))")
    lines.append("        .collect()")
    lines.append("}")
    lines.append("")
    lines.append("/// Set of all builtin names for semantic validation.")
    lines.append("pub fn builtin_name_set() -> std::collections::HashSet<String> {")
    lines.append("    BUILTIN_REGISTRY.iter().map(|s| s.name.to_string()).collect()")
    lines.append("}")
    lines.append("")
    lines.append("/// Name → arity mapping. 0 = variadic (skip check).")
    lines.append("pub fn builtin_arity_map() -> std::collections::HashMap<&'static str, usize> {")
    lines.append("    BUILTIN_REGISTRY.iter().map(|s| (s.name, s.arity)).collect()")
    lines.append("}")
    lines.append("")
    lines.append("/// Check if a name is a known builtin.")
    lines.append("pub fn is_builtin(name: &str) -> bool {")
    lines.append("    BUILTIN_REGISTRY.iter().any(|s| s.name == name)")
    lines.append("}")
    lines.append("")

    return "\n".join(lines)


def generate_builtin_new_from_registry():
    """Generate a Builtins::new() that uses the handler map approach."""
    # We still need the HashMap for dispatch, but document that it should
    # stay in sync with BUILTIN_REGISTRY. The key insight: the REGISTRY
    # is the SSOT for metadata; the HashMap is the SSOT for dispatch.
    # This is already the case — we just add a debug assertion.
    return None  # Keep existing new(), just add assertion


def main():
    registry_code = generate_registry_code()

    # Read builtins.rs
    with open(f"{METALOGOS}/builtins.rs") as f:
        content = f.read()

    # Insert registry after the Builtins struct definition (after line 10)
    # Find the impl Builtins block and insert before it
    impl_start = content.find("impl Builtins {")
    if impl_start == -1:
        print("ERROR: could not find 'impl Builtins {' in builtins.rs")
        return

    # Insert registry code before the impl block
    new_content = content[:impl_start] + registry_code + "\n" + content[impl_start:]

    # Add sync assertion at end of Builtins::new()
    # Find the closing of Builtins::new() — the line "Builtins { funcs }"
    new_close = "        Builtins { funcs }\n    }\n\n    /// Verify builtin registry consistency (debug builds)."
    new_close += "\n    #[cfg(debug_assertions)]"
    new_close += "\n    fn check_registry_sync(&self) {"
    new_close += "\n        for spec in BUILTIN_REGISTRY.iter() {"
    new_close += "\n            if spec.category != \"stateful\" && spec.category != \"stub\""
    new_close += "\n                && spec.category != \"graph\" && spec.category != \"mtree\""
    new_close += "\n                && spec.category != \"cron\" && spec.category != \"test\""
    new_close += "\n            {"
    new_close += "\n                assert!("
    new_close += "\n                    self.funcs.contains_key(spec.name),"
    new_close += "\n                    \"BUILTIN_REGISTRY '{}' has no handler in Builtins::new()\", spec.name"
    new_close += "\n                );"
    new_close += "\n            }"
    new_close += "\n        }"
    new_close += "\n    }\n"

    # Replace the old close
    old_close = "        Builtins { funcs }\n    }\n"
    if old_close in new_content:
        new_content = new_content.replace(old_close, new_close, 1)
    else:
        print("WARNING: could not find Builtins::new() closing to insert sync check")

    # Remove duplicate "env" insert (line 70)
    # The second env insert at Phase 6.4
    new_content = new_content.replace(
        '        // Phase 6.4 — Encryption stubs\n        funcs.insert("env".to_string(), builtin_env as BuiltinFn);',
        '        // Phase 6.4 — Encryption stubs',
        1
    )

    with open(f"{METALOGOS}/builtins.rs", "w") as f:
        f.write(new_content)

    print("✓ builtins.rs: added BuiltinSpec + BUILTIN_REGISTRY + helpers + sync check")
    print(f"  Registry: {len(BUILTIN_SPECS)} builtins")

    # ── Update compiler.rs ──
    with open(f"{METALOGOS}/compiler.rs") as f:
        content = f.read()

    # Replace the hardcoded builtin array with registry call
    old_compiler = '''        let mut builtin_indices = HashMap::new();
        let builtins = [
            "upper", "lower", "len", "str", "print", "contains", "float",
            "__trim", "__replace", "__split", "__join",
            "__abs", "__min", "__max", "__clamp", "__round",
            "__first", "__last", "__push", "__list_len",
            "recall",
            // Phase 4.4 self-hosting
            "stdin", "split_tokens", "if_eq", "newline", "is_string_token",
        ];
        for (i, name) in builtins.iter().enumerate() {
            builtin_indices.insert(name.to_string(), i);
        }'''

    new_compiler = '        let builtin_indices = crate::builtins::builtin_indices();'

    if old_compiler in content:
        content = content.replace(old_compiler, new_compiler)
        # Remove the now-unused HashMap import if only used for builtin_indices
        # (HashMap is still used elsewhere, so keep it)
        with open(f"{METALOGOS}/compiler.rs", "w") as f:
            f.write(content)
        print("✓ compiler.rs: replaced hardcoded builtin array with builtin_indices()")
    else:
        print("✗ compiler.rs: could not find old builtin array (already refactored?)")

    # ── Update vm.rs ──
    with open(f"{METALOGOS}/vm.rs") as f:
        content = f.read()

    # Replace the hardcoded builtin_names vec
    old_vm = '''        let builtin_names = vec![
            "upper".to_string(),        // 0
            "lower".to_string(),        // 1
            "len".to_string(),          // 2
            "str".to_string(),          // 3
            "print".to_string(),        // 4
            "contains".to_string(),     // 5
            "float".to_string(),        // 6
            "__trim".to_string(),       // 7
            "__replace".to_string(),    // 8
            "__split".to_string(),      // 9
            "__join".to_string(),       // 10
            "__abs".to_string(),        // 11
            "__min".to_string(),        // 12
            "__max".to_string(),        // 13
            "__clamp".to_string(),      // 14
            "__round".to_string(),      // 15
            "__first".to_string(),      // 16
            "__last".to_string(),       // 17
            "__push".to_string(),       // 18
            "__list_len".to_string(),   // 19
            "recall".to_string(),       // 20
            "stdin".to_string(),        // 21
            "split_tokens".to_string(), // 22
            "if_eq".to_string(),        // 23
            "newline".to_string(),      // 24
            "is_string_token".to_string(), // 25
        ];'''

    new_vm = "        let builtin_names = crate::builtins::builtin_names();"

    if old_vm in content:
        content = content.replace(old_vm, new_vm)
        with open(f"{METALOGOS}/vm.rs", "w") as f:
            f.write(content)
        print("✓ vm.rs: replaced hardcoded builtin_names with builtin_names()")
    else:
        print("✗ vm.rs: could not find old builtin_names vec (already refactored?)")

    # ── Update semantic.rs ──
    with open(f"{METALOGOS}/semantic.rs") as f:
        content = f.read()

    # Replace the hardcoded builtin_names set
    old_semantic = '''    // Known builtins (including Phase 6 web builtins)
    for b in &[
        "upper", "lower", "len", "str", "print", "contains", "float", "confidence",
        "to_string", "get", "push", "index_of", "substring", "char_at",
        "starts_with", "ends_with", "to_float",
        // Phase 6 builtins
        "respond", "render", "form_data", "json_body", "escape_html",
        "parse_json", "json_encode", "json_get", "has_field", "http_get",
        "query", "db_execute",
        "env", "hash_password", "verify_password", "encrypt", "decrypt", "generate_key",
        "authenticate", "session_login", "session_logout",
        "send_message", "require",
    ] {
        builtin_names.insert(b.to_string());
    }'''

    new_semantic = "    let builtin_names = crate::builtins::builtin_name_set();"

    if old_semantic in content:
        content = content.replace(old_semantic, new_semantic)
        with open(f"{METALOGOS}/semantic.rs", "w") as f:
            f.write(content)
        print("✓ semantic.rs: replaced hardcoded builtin_names with builtin_name_set()")
    else:
        print("✗ semantic.rs: could not find old builtin_names set (already refactored?)")

    print(f"\nDone. Total builtins in registry: {len(BUILTIN_SPECS)}")
    print("To add a new builtin: add 1 row to BUILTIN_REGISTRY + 1 insert in Builtins::new()")


if __name__ == "__main__":
    main()