// ── tests/naryad_285_text_chunk.rs — Наряд №285 (P2, feature/memory) ────
//
// Контракт (issue #340): text_chunk(text, strategy, opts?) — структура-
// осознанное чанкование для RAG-пайплайна:
//   1. strategies markdown|paragraph|fixed; markdown-секции несут
//      header_path ("H1 > H2 > H3"); строки заголовков не рвутся;
//   2. бюджет: ни один чанк не превышает max_chars (или max_tokens при
//      заданном token-бюджете — реюз token_count);
//   3. overlap сцеплен (фрагмент на шве присутствует в обоих чанках);
//   4. громкие ошибки: неизвестный strategy, overlap >= max_chars,
//      max_tokens <= 0, неизвестные opts-поля; пустой/короткий → 1 чанк;
//   5. idempotent-прогон; TW/VM parity;
//   6. [feature vec] интеграция text_chunk → embed → vec_store →
//      vec_search — ближайшие секции markdown-документа.
//
// Запуск: cargo test --test naryad_285_text_chunk
// (чистая строковая функция — без feature-гейта; vec-часть под cfg.)

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
#[cfg(feature = "vec")]
use serial_test::serial;
use std::sync::Mutex;

// ── Helpers ─────────────────────────────────────────────────────────────

static SERIAL_LOCK: Mutex<()> = Mutex::new(());

fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("builtin {name} not in BUILTIN_REGISTRY"));
    let handler = spec.handler.expect("builtin has handler");
    handler(args)
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

#[cfg(feature = "vec")]
fn f(v: f64) -> Value {
    Value::Float(v)
}

fn opts(fields: Vec<(&str, f64)>) -> Value {
    Value::Struct {
        type_name: "Opts".to_string(),
        fields: fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), Value::Float(v)))
            .collect(),
    }
}

fn chunks(v: &Value) -> Vec<(f64, String, f64, f64, Option<String>)> {
    match v {
        Value::List(items) => items
            .iter()
            .map(|c| match c {
                Value::Struct { fields, .. } => {
                    let index = match fields.get("index") {
                        Some(Value::Float(x)) => *x,
                        other => panic!("chunk index not Float: {other:?}"),
                    };
                    let text = match fields.get("text") {
                        Some(Value::String(x)) => x.clone(),
                        other => panic!("chunk text not String: {other:?}"),
                    };
                    let chars = match fields.get("chars") {
                        Some(Value::Float(x)) => *x,
                        other => panic!("chunk chars not Float: {other:?}"),
                    };
                    let tokens = match fields.get("tokens") {
                        Some(Value::Float(x)) => *x,
                        other => panic!("chunk tokens not Float: {other:?}"),
                    };
                    let hp = fields.get("header_path").map(|v| match v {
                        Value::String(x) => x.clone(),
                        other => panic!("header_path not String: {other:?}"),
                    });
                    (index, text, chars, tokens, hp)
                }
                other => panic!("chunk is not Struct: {other:?}"),
            })
            .collect(),
        other => panic!("text_chunk result is not List: {other:?}"),
    }
}

fn with_lock(body: impl FnOnce()) {
    let _g = SERIAL_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    body();
}

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn with_tmp_cwd(body: impl FnOnce()) {
    let _g = SERIAL_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("chdir");
    let _cwd = CwdGuard(prev);
    body();
}

/// Детерминированный длинный текст-абзац (~target_chars символов).
fn long_paragraph(seed: usize, target_chars: usize) -> String {
    let words = [
        "контейнер",
        "память",
        "секция",
        "документ",
        "поиск",
        "вектор",
        "профиль",
        "структура",
        "бюджет",
        "граница",
    ];
    let mut out = String::new();
    let mut i = 0;
    while out.chars().count() < target_chars {
        out.push_str(&format!("{}-{} ", words[(seed + i) % words.len()], i));
        i += 1;
    }
    out.trim_end().to_string()
}

/// Демо-документ: преамбула + h1 с двумя h2, у каждого длинное тело.
fn demo_markdown() -> String {
    format!(
        "Вводная преамбула документа.\n\n# Животные\n\n## Кошки\n\n{}\n\n## Собаки\n\n{}\n\n# Астрономия\n\n{}",
        long_paragraph(1, 2600),
        long_paragraph(2, 2600),
        long_paragraph(3, 2600),
    )
}

// ── 1. markdown: секции + header_path ──────────────────────────────────

#[test]
fn markdown_sections_carry_header_path() {
    with_lock(|| {
        let doc = "# Животные\n\n## Кошки\n\nтекст про кошек\n\n## Собаки\n\nтекст про собак";
        let r = call_builtin("text_chunk", &[s(doc), s("markdown")]).expect("text_chunk");
        let cs = chunks(&r);
        assert_eq!(cs.len(), 3, "3 секции: {cs:?}");
        assert_eq!(cs[0].4.as_deref(), Some("Животные"));
        assert_eq!(cs[1].4.as_deref(), Some("Животные > Кошки"));
        assert_eq!(cs[2].4.as_deref(), Some("Животные > Собаки"));
        assert!(cs[1].1.contains("текст про кошек"));
        assert!(cs[2].1.contains("текст про собак"));
    });
}

#[test]
fn markdown_header_path_three_levels() {
    with_lock(|| {
        let doc = "# A\n\n## B\n\n### C\n\nглубокий текст\n\n## B2\n\nвторой";
        let r = call_builtin("text_chunk", &[s(doc), s("markdown")]).expect("text_chunk");
        let cs = chunks(&r);
        let paths: Vec<&str> = cs.iter().filter_map(|c| c.4.as_deref()).collect();
        assert!(paths.contains(&"A > B > C"), "paths: {paths:?}");
        assert!(paths.contains(&"A > B2"), "paths: {paths:?}");
        // h2 после h3 сбрасывает третий уровень.
        assert!(
            !paths.iter().any(|p| p.contains("A > B > C >")),
            "no stale level: {paths:?}"
        );
    });
}

#[test]
fn markdown_preamble_has_empty_header_path() {
    with_lock(|| {
        let doc = "преамбула до заголовка\n\n# Раздел\n\nтело";
        let r = call_builtin("text_chunk", &[s(doc), s("markdown")]).expect("text_chunk");
        let cs = chunks(&r);
        assert_eq!(cs[0].4.as_deref(), Some(""), "preamble: {cs:?}");
        assert_eq!(cs[1].4.as_deref(), Some("Раздел"));
    });
}

#[test]
fn markdown_long_section_splits_and_keeps_header_path() {
    with_lock(|| {
        let doc = demo_markdown();
        let r = call_builtin(
            "text_chunk",
            &[s(&doc), s("markdown"), opts(vec![("max_chars", 900.0)])],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        assert!(
            cs.len() > 3,
            "long sections must split: {} chunks",
            cs.len()
        );
        // Кошачьи чанки — все с одним header_path.
        let cats: Vec<&(f64, String, f64, f64, Option<String>)> = cs
            .iter()
            .filter(|c| c.4.as_deref() == Some("Животные > Кошки"))
            .collect();
        assert!(
            cats.len() >= 2,
            "2600-симв секция при бюджете 900 → ≥2 чанка"
        );
        for c in &cats {
            assert!(c.2 <= 900.0, "budget: chunk chars {} > 900", c.2);
        }
    });
}

#[test]
fn markdown_header_lines_are_not_torn() {
    with_lock(|| {
        let doc = demo_markdown();
        let r = call_builtin(
            "text_chunk",
            &[s(&doc), s("markdown"), opts(vec![("max_chars", 900.0)])],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        // Заголовок первого чанка каждой секции: начинается с # -строки целиком.
        let first_cat = cs
            .iter()
            .find(|c| c.4.as_deref() == Some("Животные > Кошки"))
            .expect("cats section");
        assert!(
            first_cat.1.starts_with("## Кошки\n") || first_cat.1.starts_with("## Кошки"),
            "header line must lead its section chunk: '{}…'",
            first_cat.1.chars().take(40).collect::<String>()
        );
        assert!(first_cat.1.contains("\n"), "body follows the header");
    });
}

// ── 2. Бюджетные инварианты ─────────────────────────────────────────────

#[test]
fn budget_invariant_across_strategies() {
    with_lock(|| {
        let doc = demo_markdown();
        for strategy in ["markdown", "paragraph", "fixed"] {
            let r = call_builtin(
                "text_chunk",
                &[
                    s(&doc),
                    s(strategy),
                    opts(vec![("max_chars", 700.0), ("overlap", 80.0)]),
                ],
            )
            .expect(strategy);
            for (i, _, c, _, _) in chunks(&r) {
                assert!(
                    c <= 700.0,
                    "{strategy}: chunk {i} exceeds budget: {c} > 700"
                );
            }
        }
    });
}

#[test]
fn token_budget_uses_token_count_ssot() {
    with_lock(|| {
        let doc = demo_markdown();
        let r = call_builtin(
            "text_chunk",
            &[
                s(&doc),
                s("paragraph"),
                opts(vec![
                    ("max_chars", 5000.0),
                    ("max_tokens", 300.0),
                    ("overlap", 50.0),
                ]),
            ],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        assert!(cs.len() > 1, "token budget must force splitting");
        for (i, text, _, tokens, _) in &cs {
            assert!(*tokens <= 300.0, "chunk {i}: tokens {tokens} > 300");
            // Реюз: tokens чанка == token_count(текста чанка) байт-в-байт.
            let tc = call_builtin("token_count", &[s(text)]).expect("token_count");
            match tc {
                Value::Float(x) => assert_eq!(*tokens, x, "SSOT parity chunk {i}"),
                other => panic!("token_count not Float: {other:?}"),
            }
        }
    });
}

#[test]
fn index_sequence_is_dense_from_zero() {
    with_lock(|| {
        let doc = demo_markdown();
        let r = call_builtin(
            "text_chunk",
            &[s(&doc), s("paragraph"), opts(vec![("max_chars", 600.0)])],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        for (i, (index, _, _, _, _)) in cs.iter().enumerate() {
            assert_eq!(*index, i as f64, "index must be 0..n-1");
        }
    });
}

// ── 3. Overlap-сцепка ───────────────────────────────────────────────────

#[test]
fn fixed_overlap_keeps_seam_in_both_chunks() {
    with_lock(|| {
        let doc = long_paragraph(5, 3000);
        let r = call_builtin(
            "text_chunk",
            &[
                s(&doc),
                s("fixed"),
                opts(vec![("max_chars", 800.0), ("overlap", 100.0)]),
            ],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        assert!(cs.len() >= 3, "3000/800 → ≥3 окна: {}", cs.len());
        for w in cs.windows(2) {
            // carry = хвост окна 1, выровненный по пробелу — целые слова
            // на шве присутствуют в начале окна 2.
            let words0: Vec<&str> = w[0].1.split_whitespace().collect();
            let tail_words: String = words0[words0.len().saturating_sub(4)..].join(" ");
            assert!(
                w[1].1.contains(&tail_words),
                "overlap seam: tail '{tail_words}' must appear in next chunk '{}'…",
                w[1].1.chars().take(80).collect::<String>()
            );
        }
    });
}

#[test]
fn paragraph_long_block_overlap_is_glued() {
    with_lock(|| {
        // Один абзац длиннее бюджета → окна с overlap внутри абзаца.
        let doc = long_paragraph(7, 2500);
        let r = call_builtin(
            "text_chunk",
            &[
                s(&doc),
                s("paragraph"),
                opts(vec![("max_chars", 900.0), ("overlap", 120.0)]),
            ],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        assert!(cs.len() >= 2);
        // Сцепка: последние ЦЕЛЫЕ слова чанка 1 (carry выровнен по пробелу)
        // присутствуют в начале чанка 2.
        let words1: Vec<&str> = cs[0].1.split_whitespace().collect();
        let tail_words: String = words1[words1.len().saturating_sub(3)..].join(" ");
        let head2: String = cs[1].1.chars().take(300).collect();
        assert!(
            head2.contains(&tail_words),
            "overlap must glue long-paragraph windows: tail '{tail_words}' not in chunk2 head '{head2}'"
        );
    });
}

// ── 4. paragraph: блоки и слияние мелких ────────────────────────────────

#[test]
fn paragraph_splits_on_double_newline() {
    with_lock(|| {
        let doc = "блок один\n\nблок два\n\nблок три";
        let r = call_builtin("text_chunk", &[s(doc), s("paragraph")]).expect("text_chunk");
        let cs = chunks(&r);
        assert_eq!(
            cs.len(),
            1,
            "мелкие блоки сливаются в пределах дефолт-бюджета 1200"
        );
        assert!(cs[0].1.contains("блок один") && cs[0].1.contains("блок три"));
    });
}

#[test]
fn paragraph_keeps_separate_blocks_when_over_budget() {
    with_lock(|| {
        let doc = format!("{}\n\n{}", long_paragraph(1, 800), long_paragraph(2, 800));
        // Бюджет 500: каждый блок сам длиннее → отдельные окна, без склейки.
        let r = call_builtin(
            "text_chunk",
            &[
                s(&doc),
                s("paragraph"),
                opts(vec![("max_chars", 500.0), ("overlap", 40.0)]),
            ],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        assert!(
            cs.len() >= 3,
            "two 800-char blocks / 500 budget → ≥3 чанка: {}",
            cs.len()
        );
    });
}

// ── 5. fixed: окна и шаг ────────────────────────────────────────────────

#[test]
fn fixed_windows_cover_all_input() {
    with_lock(|| {
        let doc = long_paragraph(9, 2000);
        let r = call_builtin(
            "text_chunk",
            &[
                s(&doc),
                s("fixed"),
                opts(vec![("max_chars", 600.0), ("overlap", 100.0)]),
            ],
        )
        .expect("text_chunk");
        let cs = chunks(&r);
        // Покрытие: конкатенация (без overlap-хвостов) покрывает весь текст —
        // проверяем, что последний чанк доходит до конца текста.
        let tail_of_doc: String = doc.chars().skip(doc.chars().count() - 60).collect();
        let last = cs.last().expect("non-empty");
        assert!(
            last.1.contains(&tail_of_doc),
            "last window must reach the end"
        );
        // Хвост документа без overlap-потерь: окно step=500, overlap=100 → шаг 500.
        assert!(
            cs.len() >= 4,
            "2000 chars / step 500 → ≥4 окна: {}",
            cs.len()
        );
    });
}

// ── 6. Громкие ошибки и мягкие исходы ───────────────────────────────────

#[test]
fn loud_errors_are_fail_closed() {
    with_lock(|| {
        // Неизвестный strategy.
        let e = call_builtin("text_chunk", &[s("текст"), s("recursive")])
            .expect_err("unknown strategy must be loud");
        assert!(e.contains("unknown strategy"), "{e}");
        // overlap >= max_chars.
        let e = call_builtin(
            "text_chunk",
            &[
                s("текст"),
                s("fixed"),
                opts(vec![("max_chars", 100.0), ("overlap", 100.0)]),
            ],
        )
        .expect_err("overlap == max_chars must be loud");
        assert!(e.contains("overlap"), "{e}");
        // max_tokens <= 0.
        let e = call_builtin(
            "text_chunk",
            &[s("текст"), s("fixed"), opts(vec![("max_tokens", 0.0)])],
        )
        .expect_err("max_tokens <= 0 must be loud");
        assert!(e.contains("max_tokens"), "{e}");
        // max_chars <= 0 (громко в духе fail-closed; расширение постановки).
        let e = call_builtin(
            "text_chunk",
            &[s("текст"), s("fixed"), opts(vec![("max_chars", -5.0)])],
        )
        .expect_err("max_chars <= 0 must be loud");
        assert!(e.contains("max_chars"), "{e}");
        // Неизвестное opts-поле (паттерн №280/№284: opts строгие).
        let bad = Value::Struct {
            type_name: "Opts".to_string(),
            fields: [
                ("max_chars".to_string(), Value::Float(100.0)),
                ("magic".to_string(), Value::Float(1.0)),
            ]
            .into_iter()
            .collect(),
        };
        let e = call_builtin("text_chunk", &[s("текст"), s("fixed"), bad])
            .expect_err("unknown opts field must be loud");
        assert!(e.contains("unknown opts field"), "{e}");
        // opts не Struct.
        let e = call_builtin("text_chunk", &[s("текст"), s("fixed"), s("x")])
            .expect_err("opts must be Struct");
        assert!(e.contains("opts must be a Struct"), "{e}");
        // Арность.
        assert!(
            call_builtin("text_chunk", &[s("текст")]).is_err(),
            "1 arg — loud"
        );
        assert!(
            call_builtin(
                "text_chunk",
                &[s("т"), s("fixed"), Value::Float(1.0), Value::Float(2.0)]
            )
            .is_err(),
            "4 args — loud"
        );
    });
}

#[test]
fn empty_and_short_text_yield_one_chunk() {
    with_lock(|| {
        for strategy in ["markdown", "paragraph", "fixed"] {
            let r = call_builtin("text_chunk", &[s(""), s(strategy)]).expect(strategy);
            let cs = chunks(&r);
            assert_eq!(cs.len(), 1, "{strategy}: empty → 1 чанк");
            assert_eq!(cs[0].1, "", "{strategy}: empty chunk text");
            assert_eq!(cs[0].2, 0.0, "{strategy}: 0 chars");
            assert_eq!(cs[0].3, 0.0, "{strategy}: 0 tokens");
        }
        // Короткий текст → 1 чанк без ошибок.
        let r = call_builtin("text_chunk", &[s("короткий текст"), s("markdown")]).expect("short");
        assert_eq!(chunks(&r).len(), 1);
    });
}

#[test]
fn chunking_is_idempotent() {
    with_lock(|| {
        let doc = demo_markdown();
        let a_val = call_builtin(
            "text_chunk",
            &[s(&doc), s("markdown"), opts(vec![("max_chars", 900.0)])],
        )
        .expect("run 1");
        let b_val = call_builtin(
            "text_chunk",
            &[s(&doc), s("markdown"), opts(vec![("max_chars", 900.0)])],
        )
        .expect("run 2");
        // Детерминированные кортежи (Debug-формат Struct-полей не упорядочен —
        // HashMap-порядок), сравнение по контракту.
        assert_eq!(
            chunks(&a_val),
            chunks(&b_val),
            "idempotent: same input → same chunks"
        );
    });
}

// ── 7. Crosscheck TW/VM ─────────────────────────────────────────────────

fn run_tw(source: &str, base: &std::path::Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base.to_path_buf())
}

fn run_vm(source: &str, base: &std::path::Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

#[test]
fn text_chunk_is_identical_in_tw_and_vm() {
    with_tmp_cwd(|| {
        let source = r#"
pattern Chunk(_x: String) -> String {
  let doc = "intro\n\n# A\n\nalpha body text\n\n# B\n\nbeta body text"
  let parts = text_chunk(doc, "markdown")
  let first = parts[0]
  let n = len(parts)
  return str(n) + ":" + first.text
}

flow Main { input: String = "x" -> Chunk -> output }
"#;
        let cwd = std::env::current_dir().expect("cwd");
        let tw = run_tw(source, &cwd).expect("TW run").unwrap_or_default();
        let vm = run_vm(source, &cwd).expect("VM run").unwrap_or_default();
        assert_eq!(
            tw.trim(),
            vm.trim(),
            "TW и VM обязаны дать байт-в-байт одинаковый вывод"
        );
        assert!(tw.trim().starts_with("3:"), "3 секции: {}", tw.trim());
    });
}

// ── 8. [feature vec] Интеграция: chunk → embed → store → search ─────────

#[cfg(feature = "vec")]
#[test]
#[serial]
fn rag_pipeline_chunk_embed_store_search() {
    with_tmp_cwd(|| {
        // Демо-документ с тематическими секциями.
        let doc = format!(
            "# Животные\n\n{}\n\n# Астрономия\n\n{}\n\n# Кулинария\n\n{}",
            long_paragraph(11, 400).replace("контейнер", "кот мурлычет"),
            long_paragraph(12, 400).replace("вектор", "телескоп звезда орбита планета"),
            long_paragraph(13, 400).replace("структура", "суп борщ кухня рецепт"),
        );
        let r = call_builtin("text_chunk", &[s(&doc), s("markdown")]).expect("text_chunk");
        let cs = chunks(&r);
        assert!(cs.len() >= 3, "3 секции → ≥3 чанка");

        // Каждая секция → embed → vec_store (id = header_path, text-поле).
        for (index, text, _, _, header_path) in &cs {
            let emb = call_builtin("embed", &[s(text)]).expect("embed");
            let id = format!(
                "{}#{index}",
                header_path
                    .clone()
                    .unwrap_or_else(|| "preamble".to_string())
            );
            let stored = call_builtin(
                "vec_store",
                &[s("rag.db"), s("sections"), s(&id), emb, s(text)],
            )
            .expect("vec_store");
            match stored {
                Value::Struct { fields, .. } => {
                    assert!(
                        fields.contains_key("stored"),
                        "vec_store contract: {fields:?}"
                    );
                }
                other => panic!("vec_store not Struct: {other:?}"),
            }
        }

        // Поиск: запрос про кошек → ближайшая секция «Животные».
        let q = call_builtin("embed", &[s("кот мурлычет дома животные питомцы")]).expect("embed q");
        let hits = call_builtin(
            "vec_search",
            &[s("rag.db"), s("sections"), q, f(3.0), Value::Bool(true)],
        )
        .expect("vec_search");
        match hits {
            Value::List(items) if !items.is_empty() => {
                let top = match &items[0] {
                    Value::Struct { fields, .. } => match fields.get("id") {
                        Some(Value::String(id)) => id.clone(),
                        other => panic!("hit id: {other:?}"),
                    },
                    other => panic!("hit not Struct: {other:?}"),
                };
                assert_eq!(
                    top, "Животные#0",
                    "nearest section must be the animals one, got {top}"
                );
            }
            other => panic!("vec_search must return hits: {other:?}"),
        }
    });
}
