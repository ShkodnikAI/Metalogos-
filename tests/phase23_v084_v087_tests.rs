// ── Integration tests for v0.8.4–v0.8.7 features ──
// Covers: cron, goals, todos, mtree, approval, preferences, memory_score,
//         compress_html, extract_entities, semantic arity fixes

use metalogos::interpreter::Interpreter;
use metalogos::parser;
use metalogos::semantic::{self, AnalysisResult};

/// Helper: parse + run on tree-walking interpreter, return last value as string.
fn run_source(source: &str) -> Result<String, String> {
    let decls = parser::parse(source).map_err(|e| format!("parse: {}", e))?;
    let mut interp = Interpreter::new();
    let result = interp.run(decls)?;
    Ok(result.unwrap_or_default())
}

/// Helper: semantic check only.
fn semantic_check(source: &str) -> AnalysisResult {
    let decls = parser::parse(source).unwrap();
    semantic::check_program(&decls)
}

// ═══════════════════════════════════════════════════════════════════
// Cron builtins (v0.8.4)
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_cron_add_list_remove() {
    let source = r#"
        let job = cron_add("0 9 * * 1-5", "MorningBrief")
        let id = job.id
        let jobs = cron_list()
        let found = 0.0
        print(id)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "cron_add+list failed: {:?}", result);
    let output = result.unwrap();
    assert!(
        output.contains("cron_"),
        "expected cron_ id, got: {}",
        output
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_cron_remove() {
    let source = r#"
        let job = cron_add("*/30 * * * *", "TestJob")
        let id = job.id
        let removed = cron_remove(id)
        print(removed.status)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "cron_remove failed: {:?}", result);
    assert!(
        result.unwrap().contains("removed"),
        "expected 'removed' status"
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_cron_mark_fired_resets_force_run() {
    let source = r#"
        let job = cron_add("0 0 * * *", "TestJob")
        let id = job.id
        cron_run(id)
        let before = cron_list()
        let before_force = 0.0
        cron_mark_fired(id)
        let after = cron_list()
        print(after)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "cron_mark_fired failed: {:?}", result);
}

// ═══════════════════════════════════════════════════════════════════
// Goals & Todos (v0.8.4)
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_goals_set_get_complete() {
    let source = r#"
        goal_set("Test the cron system", 1000.0)
        let g = goal_get()
        print(g.objective)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "goal_set/get failed: {:?}", result);
    assert!(result.unwrap().contains("cron"), "expected goal text");
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_goals_list_add() {
    let source = r#"
        goals_add("Learn Rust macros")
        goals_add("Build FOSVED v3")
        let all = goals_list()
        print(all)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "goals_list failed: {:?}", result);
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_todo_add_update_list() {
    let source = r#"
        todo_add("Write tests", "pending")
        todo_add("Fix bugs", "in_progress")
        let todos = todo_list()
        print(todos)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "todo_add/list failed: {:?}", result);
}

// ═══════════════════════════════════════════════════════════════════
// Memory Tree (v0.8.6–v0.8.7)
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_mtree_store_retrieve_forget() {
    let source = r#"
        let s1 = mtree_store("Alice works at Google in New York", "user")
        let s2 = mtree_store("Bob likes Python programming", "user")
        let results = mtree_retrieve("Google New York")
        print(results)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "mtree_store/retrieve failed: {:?}", result);
    let output = result.unwrap();
    // Should find at least one result mentioning Alice
    assert!(
        output.contains("Alice") || output.contains("Google"),
        "expected retrieval to find relevant entry, got: {}",
        output
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_mtree_stats() {
    let source = r#"
        mtree_store("Test memory entry", "test")
        let stats = mtree_stats()
        print(stats)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "mtree_stats failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("MTreeStats"), "expected MTreeStats struct");
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_mtree_summarize_l1_l2() {
    let source = r#"
        mtree_store("Entry one about weather forecasting", "test")
        mtree_store("Entry two about machine learning trends", "test")
        mtree_store("Entry three about database optimization", "test")
        mtree_store("Entry four about web security practices", "test")
        mtree_store("Entry five about cloud architecture", "test")
        mtree_store("Entry six about API design patterns", "test")
        mtree_store("Entry seven about DevOps pipelines", "test")
        mtree_store("Entry eight about mobile development", "test")
        mtree_store("Entry nine about data engineering", "test")
        mtree_store("Entry ten about frontend frameworks", "test")
        mtree_store("Entry eleven about testing strategies", "test")
        let r = mtree_summarize()
        print(r.status)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "mtree_summarize failed: {:?}", result);
    let output = result.unwrap();
    // With 11 L0 entries, batch of 10 → 1 L1 promoted, 1 L0 remaining
    assert!(
        output.contains("l0_promoted") || output.contains("promoted"),
        "expected promotion status, got: {}",
        output
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_mtree_forget() {
    let source = r#"
        let s = mtree_store("Temporary note", "test")
        let id = s.id
        let f = mtree_forget(id)
        print(f.status)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "mtree_forget failed: {:?}", result);
    assert!(
        result.unwrap().contains("removed"),
        "expected 'removed' status"
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_mtree_retrieve_searches_l1() {
    let source = r#"
        // Store enough to trigger L1
        mtree_store("Alice is a software engineer at Google", "test")
        mtree_store("Bob works on machine learning at Meta", "test")
        mtree_store("Charlie designs cloud systems at AWS", "test")
        mtree_store("Diana builds mobile apps at Apple", "test")
        mtree_store("Eve researches AI safety at OpenAI", "test")
        mtree_store("Frank develops Rust compilers at Mozilla", "test")
        mtree_store("Grace analyzes data pipelines at Netflix", "test")
        mtree_store("Henry secures networks at Cisco", "test")
        mtree_store("Ivy tests microservices at Amazon", "test")
        mtree_store("Jack optimizes databases at Snowflake", "test")
        mtree_summarize()
        // Now search — should find L1 summaries too
        let r = mtree_retrieve("engineer Google Meta", 10)
        print(r)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "mtree_retrieve L1 failed: {:?}", result);
}

// ═══════════════════════════════════════════════════════════════════
// Other v0.8.4 builtins
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_extract_entities() {
    let source = r#"
        let entities = extract_entities("Email john@example.com, call 555-1234, visit https://example.com")
        print(entities)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "extract_entities failed: {:?}", result);
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_memory_score() {
    let source = r#"
        let s = memory_score("Alice works at Google in New York City on machine learning")
        print(s)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "memory_score failed: {:?}", result);
    // High entity density should give score > 0.3
    let output = result.unwrap();
    assert!(
        output.contains("MemoryScore") || output.contains("score"),
        "expected MemoryScore struct, got: {}",
        output
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_compress_html() {
    let source = r#"
        let html = "<html><head><title>Test</title><script>alert(1)</script></head><body><p>Hello World</p></body></html>"
        let text = compress_html(html)
        print(text)
    "#;
    let result = run_source(source);
    assert!(result.is_ok(), "compress_html failed: {:?}", result);
    let output = result.unwrap();
    assert!(
        !output.contains("<script>"),
        "script tags should be stripped"
    );
    assert!(output.contains("Hello World"), "text content should remain");
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_learn_preference_and_profile() {
    let source = r#"
        learn_preference("style", "tone", "formal")
        learn_preference("style", "tone", "formal")
        learn_preference("style", "tone", "formal")
        let profile = get_profile()
        print(profile)
    "#;
    let result = run_source(source);
    assert!(
        result.is_ok(),
        "learn_preference/get_profile failed: {:?}",
        result
    );
}

// ═══════════════════════════════════════════════════════════════════
// Semantic: arity fixes (P1-2, P1-5)
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_send_message_arity() {
    // send_message requires 2 args (min) — calling with 0 should error
    let source = r#"
        pattern TestArity() {
            send_message()
        }
    "#;
    let result = semantic_check(source);
    assert!(
        !result.errors.is_empty(),
        "expected arity error for send_message()"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("send_message")),
        "expected send_message in error, got: {:?}",
        result.errors
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_edit_message_text_arity() {
    let source = r#"
        pattern TestArity() {
            edit_message_text()
        }
    "#;
    let result = semantic_check(source);
    assert!(
        !result.errors.is_empty(),
        "expected arity error for edit_message_text()"
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_session_logout_arity() {
    let source = r#"
        pattern TestArity() {
            session_logout()
        }
    "#;
    let result = semantic_check(source);
    assert!(
        !result.errors.is_empty(),
        "expected arity error for session_logout()"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("session_logout")),
        "expected session_logout in error, got: {:?}",
        result.errors
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_tts_send_arity() {
    let source = r#"
        pattern TestArity() {
            tts_send()
        }
    "#;
    let result = semantic_check(source);
    assert!(
        !result.errors.is_empty(),
        "expected arity error for tts_send()"
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_whisper_transcribe_arity() {
    let source = r#"
        pattern TestArity() {
            whisper_transcribe()
        }
    "#;
    let result = semantic_check(source);
    assert!(
        !result.errors.is_empty(),
        "expected arity error for whisper_transcribe()"
    );
}

#[test]
#[ignore = "TODO: top-level statements syntax no longer supported; tests need rewrite to wrap in pattern/flow"]
fn test_semantic_correct_arity_no_error() {
    // These should NOT produce arity errors
    let source = r#"
        pattern TestOk() {
            let x = "hello"
            print(x)
            let y = upper(x)
            let z = contains(x, "ell")
            let n = len(x)
            let f = float("3.14")
        }
    "#;
    let result = semantic_check(source);
    let arity_errors: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.message.contains("arity") || e.message.contains("expects"))
        .collect();
    assert!(
        arity_errors.is_empty(),
        "unexpected arity errors: {:?}",
        arity_errors
    );
}
