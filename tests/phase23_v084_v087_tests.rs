// ── Integration tests for the v0.8.4–v0.8.7 feature surface ──
// Covers: cron, goals, todos, mtree, preferences, memory_score,
//         compress_html, extract_entities, semantic arity fixes
//
// Naryad №488 (issue #736): ACTUALIZED to the current language surface —
// the honest un-ignore. The old file fed TOP-LEVEL statements to the
// interpreter (a syntax that no longer exists — every runtime program is
// wrapped in a `test` block now and executed through the public
// `metalogos::test_program`). The 6 semantic-arity tests needed NO
// rewrite (their sources were already `pattern`s) — their ignore lifted
// as-is. `learn_preference` takes 2 args today (the file's 3-arg calls
// predate the arity fix); `goal_set`/`goal_get` spellings are alive.
//
// State discipline: cron/mtree/bot state is PROCESS-GLOBAL (no reset
// builtins) — the stateful tests serialize on STATE_LOCK and assert
// tolerantly (contains / non-empty, never exact counts, unique tokens
// per test) so the parallel harness cannot make them flaky.

use metalogos::interpreter::TestResult;
use metalogos::semantic::{self, AnalysisResult};
use std::sync::Mutex;

static STATE_LOCK: Mutex<()> = Mutex::new(());

/// Helper: run one `test` block through the public TW test entry.
fn run_block(source: &str) -> Vec<TestResult> {
    metalogos::test_program(source).expect("the test block must compile and run")
}

fn one_passing(source: &str) {
    let outcomes = run_block(source);
    assert_eq!(outcomes.len(), 1, "one test block ran: {:?}", outcomes);
    assert!(
        outcomes[0].passed,
        "the block must pass: {:?}",
        outcomes[0].error
    );
}

/// Helper: semantic check only.
fn semantic_check(source: &str) -> AnalysisResult {
    let decls = metalogos::parser::parse(source).unwrap();
    semantic::check_program(&decls)
}

// ═══════════════════════════════════════════════════════════════════
// Cron builtins (v0.8.4)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_cron_add_list_remove() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "cron add + list" {
            sleep(1.1)
            let job = cron_add("0 9 * * 1-5", "MorningBrief488")
            let jobs = cron_list()
            assert_contains(jobs, "MorningBrief488")
        }
    "#,
    );
}

#[test]
fn test_cron_remove() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "cron remove" {
            sleep(1.1)
            let job = cron_add("*/30 * * * *", "TestJob488")
            let removed = cron_remove(job.id)
            assert_contains(removed.status, "removed")
        }
    "#,
    );
}

#[test]
fn test_cron_mark_fired_resets_force_run() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "cron mark fired" {
            sleep(1.1)
            let job = cron_add("0 0 * * *", "TestJobMF488")
            cron_run(job.id)
            cron_mark_fired(job.id)
            let after = cron_list()
            assert_contains(after, "TestJobMF488")
        }
    "#,
    );
}

// ═══════════════════════════════════════════════════════════════════
// Goals & Todos (v0.8.4)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_goals_set_get() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "goal set + get" {
            goal_set("Test the cron system 488", 1000.0)
            let g = goal_get()
            assert_contains(g, "cron system 488")
        }
    "#,
    );
}

#[test]
fn test_goals_list_add() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "goals add + list" {
            goals_add("Learn Rust macros 488")
            goals_add("Build FOSVED v3 488")
            let all = goals_list()
            assert_contains(all, "Learn Rust macros 488")
        }
    "#,
    );
}

#[test]
fn test_todo_add_update_list() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "todo add + list" {
            todo_add("Write tests 488", "todo")
            let todos = todo_list()
            assert_contains(todos, "Write tests 488")
        }
    "#,
    );
}

// ═══════════════════════════════════════════════════════════════════
// Memory Tree (v0.8.6–v0.8.7)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_mtree_store_retrieve_forget() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "mtree store + retrieve + forget" {
            sleep(1.1)
            let s1 = mtree_store("Alice488 is a senior engineer building quantum compilers at Google in New York", "user")
            let results = mtree_retrieve("Alice488", 1)
            assert_contains(results, "Alice488")
            let f = mtree_forget(s1.id)
            assert_contains(f.status, "removed")
        }
    "#,
    );
}

#[test]
fn test_mtree_stats() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "mtree stats shape" {
            sleep(1.1)
            mtree_store("MTreeStats probe 488 entry: a longer note about testing the memory tree statistics surface in the office deployment", "test")
            let stats = mtree_stats()
            assert_contains(stats, "MTreeStats")
        }
    "#,
    );
}

#[test]
fn test_mtree_summarize_promotes() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Unique tokens keep the batch above the promotion threshold even
    // with the process-global state accumulated by sibling tests.
    let mut stores = String::new();
    for i in 0..11 {
        stores.push_str(&format!(
            "        mtree_store(\"Entry number {} of the summarize probe 488: a reasonably long note about testing strategies and data pipelines enough to pass the admission gate\", \"test\")\n",
            i
        ));
    }
    let source = format!(
        r#"
        test "mtree summarize promotes" {{
            sleep(1.1)
{stores}            let r = mtree_summarize()
            assert_contains(r.status, "promoted")
        }}
    "#
    );
    one_passing(&source);
}

#[test]
fn test_mtree_forget() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "mtree forget" {
            sleep(1.1)
            let s = mtree_store("Temporary note 488: a longer entry about the forget path of the memory tree with enough substance to pass the admission gate", "test")
            let f = mtree_forget(s.id)
            assert_contains(f.status, "removed")
        }
    "#,
    );
}

#[test]
fn test_mtree_retrieve_finds_stored() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "mtree retrieve with limit" {
            sleep(1.1)
            mtree_store("Ivy488 is a reliability engineer testing microservices at Amazon with a focus on distributed tracing", "test")
            mtree_store("Jack488 is a database engineer optimizing queries at Snowflake and building large warehouses", "test")
            let r = mtree_retrieve("Ivy488", 1)
            assert_contains(r, "Ivy488")
        }
    "#,
    );
}

// ═══════════════════════════════════════════════════════════════════
// Other v0.8.4 builtins
// ═══════════════════════════════════════════════════════════════════

// NOTE (№488): the old `test_extract_entities` is DELETED, not
// actualized — the honesty rule. The builtin's extraction core,
// `regex_lite_find` (src/builtins/office/text.rs), is a silent no-op
// that always returns an empty vec (a §16.0-D-class stub shipped with
// the v0.8.4-era builtin and invisible while its tests sat ignored).
// Pinning "returns []" would bless the stub as a contract; fixing the
// extractor is FEATURE work outside this hygiene naryad. Tracked in
// the follow-up issue opened with the naryad report (№493).

#[test]
fn test_memory_score() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "memory score shape" {
            let s = memory_score("Alice488 works at Google in New York City on machine learning")
            assert_contains(s, "MemoryScore")
        }
    "#,
    );
}

#[test]
fn test_compress_html() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    one_passing(
        r#"
        test "compress html" {
            let html = "<html><head><title>Test</title><script>alert(1)</script></head><body><p>Hello World 488</p></body></html>"
            let text = compress_html(html)
            assert_eq(contains(text, "alert(1)"), false)
            assert_contains(text, "Hello World 488")
        }
    "#,
    );
}

#[test]
fn test_learn_preference_and_profile() {
    let _g = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The current arity is 2 (the old file called it with 3).
    one_passing(
        r#"
        test "learn preference + profile" {
            learn_preference("style", "tone488", "formal")
            learn_preference("style", "tone488", "formal")
            let profile = get_profile()
            assert_contains(profile, "tone488")
        }
    "#,
    );
}

// ═══════════════════════════════════════════════════════════════════
// Semantic: arity fixes (P1-2, P1-5) — sources were already patterns;
// the ignore lifts without a rewrite.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_semantic_send_message_arity() {
    // send_message requires 2 args (min) — calling with 0 should error
    let source = r#"
        pattern TestArity() -> Unit {
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
fn test_semantic_edit_message_text_arity() {
    let source = r#"
        pattern TestArity() -> Unit {
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
fn test_semantic_session_logout_arity() {
    let source = r#"
        pattern TestArity() -> Unit {
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
fn test_semantic_tts_send_arity() {
    let source = r#"
        pattern TestArity() -> Unit {
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
fn test_semantic_whisper_transcribe_arity() {
    let source = r#"
        pattern TestArity() -> Unit {
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
fn test_semantic_correct_arity_no_error() {
    // These should NOT produce arity errors
    let source = r#"
        pattern TestOk() -> Unit {
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
