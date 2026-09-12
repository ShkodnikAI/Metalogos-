// ── Наряд №276: per-call LLM traces — METALOGOS_LLM_TRACE (ADR-0138) ──
//
// Контракты:
//   T1: env установлен → каждый LLM-вызов дописывает РОВНО одну валидную
//       JSONL-строку; имена полей — GenAI semconv (gen_ai.provider.name —
//       актуальное имя атрибута, gen_ai.system переименован upstream;
//       gen_ai.request.model; gen_ai.usage.*); status/cache/backend/ts/name.
//   T2: по умолчанию трейсинг ВЫКЛЮЧЕН: вызовы без env не трассируются,
//       включение env действует только на последующие вызовы.
//   T3: битый путь → вызовы работают (warning once, ни паники, ни ошибки).
//   T4: SmartRouter-путь → provider_name/model/provider_alias/usage из
//       реального HTTP-мока (OpenAI-формат с usage-блоком).
//   T5: cache-hit (ADR-0047) → строка с cache:"exact" без нового LLM-вызова.
//   T6: TW-программа → backend:"tw"; VM-программа → backend:"vm".
//
// Аудит-инвариант: honest data — поля, которых не дал провайдер (usage у
// мока, model у кэш-хита), ОТСУТСТВУЮТ в строке, а не выдумываются.

use metalogos::ast::*;
use metalogos::interpreter::{Interpreter, Value};
use serial_test::serial;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Fresh trace path per test (no cross-test file sharing).
fn trace_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mlog276_{}_{}", tag, std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("trace.jsonl")
}

fn read_lines(path: &std::path::Path) -> Vec<serde_json::Value> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str(l)
                .unwrap_or_else(|e| panic!("trace line is not valid JSON ({}): {}", e, l))
        })
        .collect()
}

fn eval_call_llm(prompt: &str, input: &str) -> Result<Value, String> {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp.run(Vec::new());
    interp.eval_expr(&Expr::FnCall {
        name: "call_llm".to_string(),
        args: vec![
            Expr::StringLit {
                value: prompt.to_string(),
                span: Span::unknown(),
            },
            Expr::StringLit {
                value: input.to_string(),
                span: Span::unknown(),
            },
        ],
        span: Span::unknown(),
    })
}

// ── T1 + T2: enabled → one JSONL line per call; off by default ────────

#[test]
#[serial]
fn n276_trace_enabled_one_line_per_call_and_off_by_default() {
    let _env = lock_env();
    let path = trace_path("t1");

    // Phase A (T2): env unset — two calls, then REMOVE the file so any
    // hypothetical hidden write from the untraced calls would surface.
    std::env::remove_var("METALOGOS_LLM_TRACE");
    let r = eval_call_llm("p", "a").expect("call_llm works without tracing");
    assert!(format!("{}", r).contains("[MOCK:"), "T2: mock default");
    let _ = eval_call_llm("p", "b");
    let _ = std::fs::remove_file(&path);

    // Phase B (T1): env set — exactly ONE line for ONE further call.
    std::env::set_var("METALOGOS_LLM_TRACE", &path);
    let _ = eval_call_llm("p", "c").expect("call_llm works with tracing");
    std::env::remove_var("METALOGOS_LLM_TRACE");

    let lines = read_lines(&path);
    assert_eq!(lines.len(), 1, "T1: one call → exactly one JSONL line");
    let l = &lines[0];
    assert_eq!(l["name"], "gen_ai.chat", "T1: operation name");
    assert_eq!(l["status"], "ok");
    assert_eq!(l["cache"], "miss", "T1: non-cached call is a miss");
    assert_eq!(l["backend"], "tw", "T1: interpreter thread = tree-walker");
    assert_eq!(l["gen_ai.provider.name"], "mock", "T1: legacy mock path");
    assert!(
        l.get("ts").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "T1: ts is unix ms"
    );
    assert!(l.get("latency_ms").is_some(), "T1: latency present");
    // Honest data: the mock reports no usage and no model — fields ABSENT.
    assert!(
        l.get("gen_ai.usage.input_tokens").is_none(),
        "T1: mock gives no input tokens — field must be absent, not invented"
    );
    assert!(
        l.get("gen_ai.usage.output_tokens").is_none(),
        "T1: mock gives no output tokens — field must be absent"
    );
    assert!(
        l.get("gen_ai.request.model").is_none(),
        "T1: legacy mock has no model — field must be absent"
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// ── T3: broken path → calls still work ────────────────────────────────

#[test]
#[serial]
fn n276_trace_broken_path_does_not_break_calls() {
    let _env = lock_env();
    let broken = std::env::temp_dir()
        .join(format!("mlog276_no_such_dir_{}", std::process::id()))
        .join("x.jsonl");
    std::env::set_var("METALOGOS_LLM_TRACE", &broken);
    let r = eval_call_llm("p", "i");
    std::env::remove_var("METALOGOS_LLM_TRACE");
    let out = r.expect("T3: trace write failure must NOT fail the LLM call");
    assert!(
        format!("{}", out).contains("[MOCK:"),
        "T3: mock still answers"
    );
    assert!(
        !broken.exists(),
        "T3: nothing written anywhere for an unwritable path"
    );
}

// ── T4: SmartRouter path → full provider fields + usage ───────────────

fn spawn_openai_mock() -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().unwrap().to_string();
    let h = std::thread::spawn(move || {
        use std::io::{Read, Write};
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 65536];
            let _ = stream.read(&mut buf);
            let body = r#"{"choices":[{"message":{"content":"mock reply"}}],"usage":{"prompt_tokens":11,"completion_tokens":7}}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://{}", addr), h)
}

#[test]
#[serial]
fn n276_trace_smart_router_fields_with_usage() {
    let _env = lock_env();
    let path = trace_path("t4");

    let (base, server) = spawn_openai_mock();
    let config = LlmConfigDecl {
        span: Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: Span::unknown(),
            alias: "mocka".to_string(),
            provider: "openai".to_string(),
            key: None,
            url: Some(base),
        }],
        default_model: Some("test-model-276".to_string()),
        failover: Some("manual".to_string()),
        circuit_breaker: 3,
        timeout: 5,
    };
    metalogos::llm::set_global_smart_router(metalogos::llm::SmartRouter::from_config(&config));

    std::env::set_var("METALOGOS_LLM_TRACE", &path);
    let r = eval_call_llm("p", "i");
    std::env::remove_var("METALOGOS_LLM_TRACE");
    metalogos::llm::clear_global_smart_router();

    let out = r.expect("T4: routed mock call must succeed");
    assert_eq!(
        format!("{}", out),
        "mock reply",
        "T4: OpenAI-format response parsed"
    );
    server.join().expect("T4: mock server thread");

    let lines = read_lines(&path);
    assert_eq!(lines.len(), 1, "T4: one routed call → one line");
    let l = &lines[0];
    assert_eq!(
        l["gen_ai.provider.name"], "openai",
        "T4: provider from route"
    );
    assert_eq!(l["provider_alias"], "mocka", "T4: router alias");
    assert_eq!(
        l["gen_ai.request.model"], "test-model-276",
        "T4: default_model"
    );
    assert_eq!(
        l["gen_ai.usage.input_tokens"], 11,
        "T4: usage from response"
    );
    assert_eq!(
        l["gen_ai.usage.output_tokens"], 7,
        "T4: usage from response"
    );
    assert_eq!(l["status"], "ok");
    assert_eq!(l["cache"], "miss");
    assert_eq!(l["backend"], "tw");

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// ── T5: cache hit → cache:"exact", no new LLM call ────────────────────

fn make_cached_learnable(name: &str) -> Declaration {
    // Unique prompt per process: the ADR-0047 cache key is hash(prompt, input);
    // if any layer persists the cache table, a fixed prompt would make the
    // FIRST call of a rerun a hit and break the miss→hit sequence under test.
    let prompt = format!("echo this [pid {}]", std::process::id());
    Declaration::LearnablePattern(LearnablePatternDecl {
        span: Span::unknown(),
        name: name.to_string(),
        params: vec![Param {
            span: Span::unknown(),
            name: "text".to_string(),
            type_name: "String".to_string(),
        }],
        return_type: "String".to_string(),
        prompt,
        context: None,
        context_strategy: ContextStrategy::None,
        max_context_tokens: 2000,
        max_tokens: None,
        cache: true,
        cache_ttl: 3600,
        model: None,
        conversation: None,
        distill_to: None,
        distill_after: 0,
        fallback_if: None,
    })
}

#[test]
#[serial]
fn n276_trace_cache_hit_is_exact_line_without_llm_call() {
    let _env = lock_env();
    let path = trace_path("t5");
    metalogos::llm::MockLlm::reset_call_count();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp.run(vec![make_cached_learnable("Echo276")]);

    let call = |interp: &mut Interpreter, arg: &str| {
        interp.eval_expr(&Expr::FnCall {
            name: "Echo276".to_string(),
            args: vec![Expr::StringLit {
                value: arg.to_string(),
                span: Span::unknown(),
            }],
            span: Span::unknown(),
        })
    };

    std::env::set_var("METALOGOS_LLM_TRACE", &path);
    let r1 = call(&mut interp, "hello");
    let r2 = call(&mut interp, "hello"); // identical → cache hit
    std::env::remove_var("METALOGOS_LLM_TRACE");

    assert!(r1.is_ok() && r2.is_ok(), "T5: both calls succeed");
    assert_eq!(
        format!("{}", r1.unwrap()),
        format!("{}", r2.unwrap()),
        "T5: cached result identical"
    );
    assert_eq!(
        metalogos::llm::MockLlm::call_count(),
        1,
        "T5: second call must be served from cache"
    );

    let lines = read_lines(&path);
    assert_eq!(
        lines.len(),
        2,
        "T5: miss + hit = two trace lines (got: {:?})",
        lines
    );
    assert_eq!(lines[0]["cache"], "miss", "T5: first call is a miss");
    assert_eq!(lines[1]["cache"], "exact", "T5: second call is a cache hit");
    assert_eq!(lines[1]["status"], "ok");
    assert!(
        lines[1].get("gen_ai.usage.input_tokens").is_none(),
        "T5: cache entry stores no usage — honest absence"
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// ── T6: backend tag — tw vs vm ─────────────────────────────────────────

const FLOW_PROGRAM: &str = r#"
pattern Call(x: String) -> String {
  let out = call_llm("p", x)
  return out
}
flow Main {
  input: String = "hi"
  -> Call
  -> output
}
"#;

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let program = metalogos::compiler::Compiler::new().compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

#[test]
#[serial]
fn n276_trace_backend_tag_tw_and_vm() {
    let _env = lock_env();

    // TW: interpreter thread → backend "tw".
    let tw_path = trace_path("t6tw");
    std::env::set_var("METALOGOS_LLM_TRACE", &tw_path);
    let tw_out = metalogos::run_program(FLOW_PROGRAM);
    std::env::remove_var("METALOGOS_LLM_TRACE");
    tw_out.expect("T6: TW program runs");
    let tw_lines = read_lines(&tw_path);
    assert_eq!(tw_lines.len(), 1, "T6: one TW call → one line");
    assert_eq!(tw_lines[0]["backend"], "tw", "T6: interpreter = tw");
    let _ = std::fs::remove_dir_all(tw_path.parent().unwrap());

    // VM: Vm::run sets the tag for the duration and restores it after.
    let vm_path = trace_path("t6vm");
    std::env::set_var("METALOGOS_LLM_TRACE", &vm_path);
    let vm_out = run_vm(FLOW_PROGRAM);
    std::env::remove_var("METALOGOS_LLM_TRACE");
    vm_out.expect("T6: VM program runs");
    let vm_lines = read_lines(&vm_path);
    assert_eq!(vm_lines.len(), 1, "T6: one VM call → one line");
    assert_eq!(vm_lines[0]["backend"], "vm", "T6: VM run tagged vm");
    let _ = std::fs::remove_dir_all(vm_path.parent().unwrap());
}
