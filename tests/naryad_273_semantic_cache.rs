// ── tests/naryad_273_semantic_cache.rs — Наряд №273 (ADR-0135) ───────
//
// Контрактные тесты semantic cache + LRU поверх ADR-0047.
// Запуск: cargo test --features vec --test naryad_273_semantic_cache
//
// Честный гейт: файл пуст без фичи `vec` (semantic-режим требует
// эмбеддинги; без фичи он даёт громкую ошибку — это проверяется
// отдельным контрактом в этом файле через API без фичи... нет:
// сам файл требует vec; контракт «без фичи — громкая ошибка» закреплён
// кодогейтом #[cfg(not(feature = "vec"))] в learnable.rs).
//
// Crosscheck-граница (документировано): кэш-контур ADR-0047/0135 — домен
// TW-интерпретатора (src/interpreter/learnable.rs — Files-список наряда).
// VM::call_llm — отдельная поверхность без кэша (дореестровая граница
// ADR-0105); расширение VM вне скоупа №273.
//
// Тесты serial: глобальные MockLlm-счётчик, SSOT-эмбеддинг-менеджер и
// env METALOGOS_LLM_CACHE_MAX — общие для процесса.

#![cfg(feature = "vec")]

use metalogos::ast::*;
use metalogos::interpreter::Interpreter;
use metalogos::interpreter::Value;
use metalogos::llm::MockLlm;
use serial_test::serial;
use std::path::PathBuf;

fn make_learnable_decl(
    name: &str,
    prompt: &str,
    cache: bool,
    ttl: u64,
    cache_semantic: bool,
    cache_threshold: f64,
) -> Declaration {
    Declaration::LearnablePattern(LearnablePatternDecl {
        span: metalogos::ast::Span::unknown(),
        name: name.to_string(),
        params: vec![Param {
            span: metalogos::ast::Span::unknown(),
            name: "text".to_string(),
            type_name: "String".to_string(),
        }],
        return_type: "String".to_string(),
        prompt: prompt.to_string(),
        context: None,
        context_strategy: ContextStrategy::None,
        max_context_tokens: 2000,
        max_tokens: None,
        cache,
        cache_ttl: ttl,
        cache_semantic,
        cache_threshold,
        model: None,
        conversation: None,
        distill_to: None,
        distill_after: 0,
        fallback_if: None,
    })
}

fn call_embed(text: &str) -> Vec<f32> {
    let spec = metalogos::builtins::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == "embed")
        .expect("embed builtin (feature vec)");
    match spec.handler.expect("handler")(&[Value::String(text.to_string())]) {
        Ok(Value::List(items)) => items
            .into_iter()
            .map(|v| match v {
                Value::Float(f) => f as f32,
                other => panic!("embed component: {other:?}"),
            })
            .collect(),
        other => panic!("embed returned {other:?}"),
    }
}

fn semantic_hits() -> f64 {
    metalogos::llm::global_llm_usage_report().cache_hits_semantic
}

fn new_interp() -> Interpreter {
    let mut i = Interpreter::new();
    i.set_base_dir(PathBuf::from("."));
    i
}

fn call_pattern(interp: &mut Interpreter, name: &str, arg: &str) -> Result<Value, String> {
    interp.eval_expr(&Expr::FnCall {
        name: name.to_string(),
        args: vec![Expr::StringLit {
            value: arg.to_string(),
            span: metalogos::ast::Span::unknown(),
        }],
        span: metalogos::ast::Span::unknown(),
    })
}

// ── 1. Точный хэш-хит приоритетен: тот же ввод → exact, не semantic ──
#[test]
#[serial]
fn naryad_273_exact_hit_priority() {
    MockLlm::reset_call_count();
    let before = semantic_hits();

    let mut interp = new_interp();
    let tmp = tempfile::tempdir().expect("tempdir");
    interp.set_memory_persist_path(Some(
        tmp.path().join("exact.db").to_string_lossy().to_string(),
    ));
    let _ = interp.run(vec![make_learnable_decl(
        "Echo",
        "echo this",
        true,
        3600,
        true,
        0.5,
    )]);

    let _ = call_pattern(&mut interp, "Echo", "hello world").expect("first call");
    assert_eq!(MockLlm::call_count(), 1, "первый вызов должен дойти до LLM");

    // Тот же ввод: exact-хит (semantic не должен срабатывать — порядок
    // ADR-0047 сохранён: few-shot → exact → semantic → LLM).
    let _ = call_pattern(&mut interp, "Echo", "hello world").expect("second call");
    assert_eq!(MockLlm::call_count(), 1, "exact-хит: LLM не вызывается");
    assert_eq!(
        semantic_hits(),
        before,
        "точный хит НЕ должен считаться semantic-хитом"
    );
}

// ── 2. Хит на перефразировке: 1 вызов LLM, semantic-счётчик растёт ──
#[test]
#[serial]
fn naryad_273_semantic_hit_on_paraphrase() {
    MockLlm::reset_call_count();
    let before = semantic_hits();

    let base = "переведи это предложение на английский язык корректно";
    let paraphrase = "переведи это предложение на английский язык дружно";
    // Замер здесь — только санити близости пары; Runtime-порог выбирается
    // ЗАВЕДОМО низким: TF-IDF IDF-дрейф (total_docs растёт с каждым embed —
    // задокументированная граница ADR-0135) делает точные границы
    // 0.919/0.921 невоспроизводимыми между замером и runtime.
    let sim = metalogos::embeddings::cosine_similarity(&call_embed(base), &call_embed(paraphrase));
    assert!(
        (0.4..1.0).contains(&sim),
        "тестовая пара должна быть семантически близкой, sim={sim}"
    );

    // Низкий порог → гарантированный хит при sim >> threshold.
    let mut interp = new_interp();
    let tmp = tempfile::tempdir().expect("tempdir");
    interp.set_memory_persist_path(Some(
        tmp.path().join("sem.db").to_string_lossy().to_string(),
    ));
    let _ = interp.run(vec![make_learnable_decl(
        "Trans",
        "translate this",
        true,
        3600,
        true,
        0.3,
    )]);

    let _ = call_pattern(&mut interp, "Trans", base).expect("base call");
    assert_eq!(MockLlm::call_count(), 1, "первый вызов — LLM");
    let _ = call_pattern(&mut interp, "Trans", paraphrase).expect("paraphrase call");
    assert_eq!(
        MockLlm::call_count(),
        1,
        "перефразировка должна быть semantic-хитом (LLM не вызывается)"
    );
    assert_eq!(
        semantic_hits() - before,
        1.0,
        "semantic-счётчик должен вырасти ровно на 1"
    );
}

// ── 3. Промах ниже порога → LLM вызывается ──────────────────────────
#[test]
#[serial]
fn naryad_273_miss_below_threshold() {
    MockLlm::reset_call_count();
    let before = semantic_hits();

    let a = "кот сидит на ковре возле дома";
    let b = "квантовый физик наблюдает за орбитой спутника";
    // 0.99: у неидентичных текстов TF-IDF sim < 0.99 всегда → гарантированный
    // промах независимо от IDF-дрейфа (см. границу в ADR-0135).
    let _sim = metalogos::embeddings::cosine_similarity(&call_embed(a), &call_embed(b));
    let mut interp = new_interp();
    let tmp = tempfile::tempdir().expect("tempdir");
    interp.set_memory_persist_path(Some(
        tmp.path().join("sem2.db").to_string_lossy().to_string(),
    ));
    let _ = interp.run(vec![make_learnable_decl(
        "Trans2",
        "translate this",
        true,
        3600,
        true,
        0.99,
    )]);

    let _ = call_pattern(&mut interp, "Trans2", a).expect("call a");
    let _ = call_pattern(&mut interp, "Trans2", b).expect("call b");
    assert_eq!(
        MockLlm::call_count(),
        2,
        "ниже порога — оба вызова уходят в LLM"
    );
    assert_eq!(semantic_hits(), before, "семантических хитов нет");
}

// ── 4. TTL старше семантики: протухшая строка не хитует ─────────────
#[test]
#[serial]
fn naryad_273_ttl_expires_semantic_entry() {
    MockLlm::reset_call_count();

    let mut interp = new_interp();
    let tmp = tempfile::tempdir().expect("tempdir");
    interp.set_memory_persist_path(Some(
        tmp.path().join("sem3.db").to_string_lossy().to_string(),
    ));
    let _ = interp.run(vec![make_learnable_decl(
        "EchoTtl",
        "echo this",
        true,
        1, // ttl = 1 second
        true,
        0.5,
    )]);

    let _ = call_pattern(&mut interp, "EchoTtl", "hello").expect("first");
    assert_eq!(MockLlm::call_count(), 1);
    std::thread::sleep(std::time::Duration::from_millis(2200)); // >2 целых секунды: TTL=1c истекает по целочисленному сравнению
    let _ = call_pattern(&mut interp, "EchoTtl", "hello").expect("second");
    assert_eq!(
        MockLlm::call_count(),
        2,
        "TTL протух и для exact, и для semantic — новый вызов LLM"
    );
}

// ── 5. LRU-эвикция ограничивает кэш ─────────────────────────────────
#[test]
#[serial]
fn naryad_273_lru_eviction() {
    std::env::set_var("METALOGOS_LLM_CACHE_MAX", "2");
    MockLlm::reset_call_count();

    let mut interp = new_interp();
    let _ = interp.run(vec![make_learnable_decl(
        "EchoLru",
        "echo this",
        true,
        3600,
        false,
        0.92,
    )]);

    // A, B, C — три разных ввода при max=2: A вытеснен (LRU), B/C в кэше.
    let _ = call_pattern(&mut interp, "EchoLru", "aaa").expect("A");
    let _ = call_pattern(&mut interp, "EchoLru", "bbb").expect("B");
    let _ = call_pattern(&mut interp, "EchoLru", "ccc").expect("C");
    assert_eq!(MockLlm::call_count(), 3);

    // Повтор B — кэш-хит (LLM не вызывается).
    let _ = call_pattern(&mut interp, "EchoLru", "bbb").expect("B again");
    assert_eq!(MockLlm::call_count(), 3, "B жив в кэше");

    // Повтор A — вытеснен → новый вызов LLM (было бы 3 без эвикции).
    let _ = call_pattern(&mut interp, "EchoLru", "aaa").expect("A again");
    assert_eq!(
        MockLlm::call_count(),
        4,
        "A вытеснен LRU — реальный вызов LLM"
    );

    std::env::remove_var("METALOGOS_LLM_CACHE_MAX");
}

// ── 6. Без persist — громкая ошибка конфигурации ────────────────────
#[test]
#[serial]
fn naryad_273_semantic_without_persist_is_loud() {
    MockLlm::reset_call_count();

    let mut interp = new_interp();
    // persist НЕ установлен.
    let _ = interp.run(vec![make_learnable_decl(
        "EchoNoPersist",
        "echo this",
        true,
        3600,
        true,
        0.5,
    )]);

    let err = call_pattern(&mut interp, "EchoNoPersist", "hello")
        .expect_err("cache_semantic без persist обязан дать громкую ошибку");
    assert!(
        err.contains("requires persistence"),
        "ошибка должна объяснять требование persist: {err}"
    );
    assert!(err.contains("llm_cache_semantic"), "{err}");
}

// ── 7. Существующий контракт ADR-0047 не сломан (exact C1-C4 живут) ──
#[test]
#[serial]
fn naryad_273_exact_cache_contract_intact() {
    // C1-эквивалент на свежем интерпретаторе: идентичные вызовы → 1 LLM.
    MockLlm::reset_call_count();
    let mut interp = new_interp();
    let _ = interp.run(vec![make_learnable_decl(
        "EchoC1",
        "echo this",
        true,
        3600,
        false,
        0.92,
    )]);
    let _ = call_pattern(&mut interp, "EchoC1", "same").expect("1");
    let _ = call_pattern(&mut interp, "EchoC1", "same").expect("2");
    assert_eq!(MockLlm::call_count(), 1, "ADR-0047 C1 сохранён");
}
