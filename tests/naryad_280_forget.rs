// ── tests/naryad_280_forget.rs — Наряд №280 (P2, M2) ────────────────────
//
// Контрактные тесты memory_forget (issue #329, диспатч #332):
//   1. dry_run-превью (дефолт) — только кандидаты, состояние не меняется;
//   2. apply — строго по явным ids из превью (bound deletes): id вне
//      границ / несуществующий — ГРОМКАЯ ошибка ДО любых записей;
//   3. границы: threshold отсекает далёких, max_forget капирует область;
//   4. soft-delete ledger: batch_id MLOG-FORGET-* в каждой строке;
//      повторный forget того же id — no-op;
//   5. vec_search: забытые не возвращаются по умолчанию
//      (include_forgotten=true — опциональный 5-й аргумент);
//   6. taint-инвариант: забывание не снимает canary-детекцию (№284);
//   7. crosscheck TW/VM одинаковый сценарий — одинаковый результат.
//
// Запуск: cargo test --features vec --test naryad_280_forget
// Честный гейт: файл пуст без фичи `vec` (минимальный контракт).
//
// Гейт-тесты используют рукотворные векторы (состояние процесс-
// глобального SSOT-менеджера embed не трогают — паттерн n272).
// Единственный тест с embed() — TW/VM parity — единственный
// пользователь менеджера в этом процессе.

#![cfg(feature = "vec")]

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use std::sync::Mutex;

// ── Helpers ─────────────────────────────────────────────────────────────

/// Сериализация: tmp-cwd тесты делят процесс и cwd-глобалы.
static SERIAL_LOCK: Mutex<()> = Mutex::new(());

fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("builtin {name} not in BUILTIN_REGISTRY — feature vec enabled?"));
    let handler = spec.handler.expect("memory_forget builtins have handlers");
    handler(args)
}

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn with_tmp_cwd(body: impl FnOnce()) {
    let _guard = SERIAL_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("chdir");
    let _cwd = CwdGuard(prev);
    body();
}

fn f(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect())
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn fl(v: f64) -> Value {
    Value::Float(v)
}

fn b(v: bool) -> Value {
    Value::Bool(v)
}

/// vec_store с рукотворным вектором.
fn store(db: &str, table: &str, id: &str, vec: &[f64]) {
    call_builtin("vec_store", &[s(db), s(table), s(id), f(vec)])
        .unwrap_or_else(|e| panic!("vec_store {id}: {e}"));
}

/// Извлечь поле Struct-значения (паника с понятным текстом).
fn field(v: &Value, name: &str) -> Value {
    match v {
        Value::Struct { fields, .. } => fields
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no field '{name}' in {fields:?}")),
        other => panic!("expected Struct, got {other:?}"),
    }
}

fn field_f64(v: &Value, name: &str) -> f64 {
    match field(v, name) {
        Value::Float(x) => x,
        other => panic!("field '{name}' is not Float: {other:?}"),
    }
}

fn field_str(v: &Value, name: &str) -> String {
    match field(v, name) {
        Value::String(x) => x,
        other => panic!("field '{name}' is not String: {other:?}"),
    }
}

/// candidates результата → Vec<(id, score)>.
fn candidates(v: &Value) -> Vec<(String, f64)> {
    match field(v, "candidates") {
        Value::List(items) => items
            .iter()
            .map(|c| (field_str(c, "id"), field_f64(c, "score")))
            .collect(),
        other => panic!("candidates is not a List: {other:?}"),
    }
}

/// id-шники vec_search-хитов.
fn hit_ids(v: &Value) -> Vec<String> {
    match v {
        Value::List(items) => items.iter().map(|h| field_str(h, "id")).collect(),
        other => panic!("search result is not a List: {other:?}"),
    }
}

/// Прямое чтение ledger (обход билтинов — проверка «а что в файле»).
fn ledger_rows(db: &str, table: &str) -> Vec<(String, String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, batch_id, reason, forgotten_at FROM \"{table}__forgotten\" ORDER BY id"
        ))
        .expect("ledger query");
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .expect("ledger map");
    rows.map(|r| r.expect("ledger row")).collect()
}

fn ledger_exists(db: &str, table: &str) -> bool {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [format!("{table}__forgotten")],
            |r| r.get(0),
        )
        .expect("sqlite_master");
    n > 0
}

/// Стандартныйfdb: 4 документа, doc-1 ближайший к запросу.
const DB: &str = "n280.db";
const TB: &str = "docs";

fn standard_db() {
    store(DB, TB, "doc-1", &[1.0, 0.0, 0.0]);
    store(DB, TB, "doc-2", &[0.95, 0.1, 0.0]);
    store(DB, TB, "doc-3", &[0.0, 1.0, 0.0]);
    store(DB, TB, "doc-4", &[0.0, 0.0, 1.0]);
    // doc-5: тоже близкий к q (sim ≈ 0.99) — для превышения max_forget.
    store(DB, TB, "doc-5", &[0.9, 0.15, 0.0]);
}

fn search(ids_k: f64, include_forgotten: Option<bool>) -> Vec<String> {
    let mut args = vec![s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(ids_k)];
    if let Some(inc) = include_forgotten {
        args.push(b(inc));
    }
    hit_ids(&call_builtin("vec_search", &args).expect("vec_search"))
}

// ── 1. Превью (dry_run, дефолт) ─────────────────────────────────────────

#[test]
fn n280_preview_default_returns_candidates_without_mutation() {
    with_tmp_cwd(|| {
        standard_db();
        // Арность 5: dry_run=true по умолчанию.
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(10.0)],
        )
        .expect("preview");
        let cands = candidates(&r);
        assert!(!cands.is_empty(), "preview must return candidates");
        assert!(cands.iter().any(|(id, _)| id == "doc-1"));
        assert_eq!(field_f64(&r, "applied"), 0.0, "dry_run never applies");
        assert_eq!(field_str(&r, "batch_id"), "", "dry_run has no batch");
        // Состояние не меняется: строки живы, ledger не создан.
        assert!(search(10.0, None).contains(&"doc-1".to_string()));
        assert!(!ledger_exists(DB, TB), "dry_run must not create ledger");
    });
}

#[test]
fn n280_preview_candidates_sorted_by_score_desc() {
    with_tmp_cwd(|| {
        standard_db();
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.0), fl(10.0)],
        )
        .expect("preview");
        let cands = candidates(&r);
        assert!(cands.len() >= 2);
        for w in cands.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "candidates must be sorted by score desc: {cands:?}"
            );
        }
        assert!((cands[0].1 - 1.0).abs() < 1e-6, "self-match score is 1.0");
    });
}

#[test]
fn n280_preview_respects_threshold() {
    with_tmp_cwd(|| {
        standard_db();
        // sim(doc-3, q) = 0, sim(doc-4, q) = 0 → порог 0.5 отсекает.
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(10.0)],
        )
        .expect("preview");
        let cands = candidates(&r);
        assert!(cands.iter().all(|(_, score)| *score >= 0.5));
        assert!(!cands.iter().any(|(id, _)| id == "doc-3"));
        assert!(!cands.iter().any(|(id, _)| id == "doc-4"));
    });
}

#[test]
fn n280_preview_respects_max_forget() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file(DB);
        for i in 0..100 {
            let v = [1.0 - (i as f64) * 0.001, (i as f64) * 0.001, 0.0];
            store(DB, TB, &format!("row-{i:03}"), &v);
        }
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.0), fl(3.0)],
        )
        .expect("preview");
        let cands = candidates(&r);
        assert_eq!(cands.len(), 3, "max_forget caps the affected area");
        assert_eq!(cands[0].0, "row-000", "nearest first");
        assert_eq!(cands[1].0, "row-001");
        assert_eq!(cands[2].0, "row-002");
    });
}

#[test]
fn n280_preview_dedups_ids_keeping_best_score() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file(DB);
        store(DB, TB, "dup", &[1.0, 0.0, 0.0]);
        store(DB, TB, "dup", &[0.99, 0.05, 0.0]);
        store(DB, TB, "other", &[0.0, 1.0, 0.0]);
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.0), fl(10.0)],
        )
        .expect("preview");
        let cands = candidates(&r);
        let dup: Vec<(String, f64)> = cands
            .iter()
            .filter(|(id, _)| id == "dup")
            .cloned()
            .collect();
        assert_eq!(dup.len(), 1, "id deduplicated: {cands:?}");
        assert!((dup[0].1 - 1.0).abs() < 1e-6, "best score kept");
    });
}

#[test]
fn n280_preview_excludes_already_forgotten() {
    with_tmp_cwd(|| {
        standard_db();
        let apply = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .expect("apply doc-1");
        assert_eq!(field_f64(&apply, "applied"), 1.0);
        // Повторное превью: doc-1 забыт — кандидатом больше не является.
        let r = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(10.0)],
        )
        .expect("second preview");
        assert!(!candidates(&r).iter().any(|(id, _)| id == "doc-1"));
    });
}

// ── 2. Громкие ошибки apply-контракта ───────────────────────────────────

#[test]
fn n280_apply_without_ids_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
            ],
        )
        .unwrap_err();
        assert!(e.contains("explicit ids"), "loud contract: {e}");
    });
}

#[test]
fn n280_ids_with_dry_run_true_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(true),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("mutually exclusive"), "loud contract: {e}");
    });
}

#[test]
fn n280_list_as_sixth_arg_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("dry_run"), "loud guidance: {e}");
    });
}

#[test]
fn n280_apply_unknown_id_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("ghost")]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("not found"), "loud: {e}");
        assert!(e.contains("bound deletes"), "mentions the discipline: {e}");
    });
}

#[test]
fn n280_apply_below_threshold_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        // doc-3 существует, но sim = 0 < threshold 0.5 — вне границ превью.
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("doc-3")]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("outside the preview bounds"), "loud: {e}");
        // Атомарность: ошибочный apply ничего не записал.
        assert!(search(10.0, None).contains(&"doc-3".to_string()));
        assert!(!ledger_exists(DB, TB));
    });
}

#[test]
fn n280_apply_exceeding_max_forget_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(2.0),
                b(false),
                Value::List(vec![s("doc-1"), s("doc-2"), s("doc-5")]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("exceeding max_forget"), "loud: {e}");
        assert!(!ledger_exists(DB, TB), "atomic: nothing written");
    });
}

#[test]
fn n280_empty_ids_list_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("must not be empty"), "loud: {e}");
    });
}

// ── 3. Apply: основной сценарий + ledger ────────────────────────────────

#[test]
fn n280_apply_by_ids_full_cycle() {
    with_tmp_cwd(|| {
        standard_db();
        // Шаг 1: превью.
        let preview = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(10.0)],
        )
        .expect("preview");
        let ids: Vec<Value> = candidates(&preview)
            .into_iter()
            .map(|(id, _)| s(&id))
            .collect();
        assert!(!ids.is_empty());
        // Шаг 2: apply по ids из превью.
        let apply = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(ids),
            ],
        )
        .expect("apply");
        let applied = field_f64(&apply, "applied");
        let batch = field_str(&apply, "batch_id");
        assert!(applied > 0.0);
        assert!(batch.starts_with("MLOG-FORGET-"), "batch prefix: {batch}");
        assert_eq!(batch.len(), "MLOG-FORGET-".len() + 26);
        // Забытые не возвращаются по умолчанию...
        let visible = search(10.0, None);
        for (id, _) in candidates(&apply) {
            assert!(!visible.contains(&id), "{id} must be hidden after forget");
        }
        // ...но живы физически (include_forgotten=true).
        let all = search(10.0, Some(true));
        for (id, _) in candidates(&apply) {
            assert!(
                all.contains(&id),
                "{id} must survive physically (soft delete)"
            );
        }
        // Ledger содержит batch_id и причину.
        let rows = ledger_rows(DB, TB);
        assert_eq!(rows.len(), applied as usize);
        for (id, b, reason, at) in &rows {
            assert_eq!(b.as_str(), batch.as_str(), "batch_id stamped on {id}");
            assert_eq!(reason, "memory_forget");
            assert!(!at.is_empty(), "forgotten_at recorded");
        }
    });
}

#[test]
fn n280_repeated_forget_is_noop() {
    with_tmp_cwd(|| {
        standard_db();
        let ids = || Value::List(vec![s("doc-1")]);
        let first = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                ids(),
            ],
        )
        .expect("first apply");
        assert_eq!(field_f64(&first, "applied"), 1.0);
        let batch1 = field_str(&first, "batch_id");
        let second = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                ids(),
            ],
        )
        .expect("second apply");
        assert_eq!(field_f64(&second, "applied"), 0.0, "no-op, not error");
        assert_eq!(field_str(&second, "batch_id"), "", "empty batch for no-op");
        assert_eq!(ledger_rows(DB, TB).len(), 1, "ledger not duplicated");
        assert_ne!(batch1, "");
    });
}

#[test]
fn n280_batch_ids_unique_across_batches() {
    with_tmp_cwd(|| {
        standard_db();
        let apply_one = |id: &str| {
            call_builtin(
                "memory_forget",
                &[
                    s(DB),
                    s(TB),
                    f(&[1.0, 0.0, 0.0]),
                    fl(0.5),
                    fl(10.0),
                    b(false),
                    Value::List(vec![s(id)]),
                ],
            )
            .expect("apply")
        };
        let b1 = field_str(&apply_one("doc-1"), "batch_id");
        let b2 = field_str(&apply_one("doc-2"), "batch_id");
        assert_ne!(b1, b2, "each operation gets its own batch_id");
    });
}

// ── 4. vec_search: include_forgotten ────────────────────────────────────

#[test]
fn n280_vec_search_arity4_backcompat_filters_forgotten() {
    with_tmp_cwd(|| {
        standard_db();
        call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .expect("apply");
        // Арность 4 (старый контракт): забытые скрыты.
        let visible = search(10.0, None);
        assert!(!visible.contains(&"doc-1".to_string()));
        assert!(visible.contains(&"doc-2".to_string()));
    });
}

#[test]
fn n280_vec_search_include_forgotten_true_returns_all() {
    with_tmp_cwd(|| {
        standard_db();
        call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .expect("apply");
        let all = search(10.0, Some(true));
        assert!(all.contains(&"doc-1".to_string()));
        assert_eq!(all.len(), 5, "nothing physically deleted");
    });
}

#[test]
fn n280_vec_search_fifth_arg_must_be_bool() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "vec_search",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(10.0), s("yes")],
        )
        .unwrap_err();
        assert!(e.contains("include_forgotten") && e.contains("Bool"), "{e}");
    });
}

// ── 5. Валидация аргументов и границы ───────────────────────────────────

#[test]
fn n280_arity_pins() {
    with_tmp_cwd(|| {
        standard_db();
        // 4 аргумента — мало.
        assert!(call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5)]
        )
        .is_err());
        // 8 аргументов — много.
        assert!(call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(5.0),
                b(false),
                Value::List(vec![s("doc-1")]),
                b(true)
            ]
        )
        .is_err());
    });
}

#[test]
fn n280_threshold_bounds_are_loud() {
    with_tmp_cwd(|| {
        standard_db();
        for bad in [-0.5, 1.5] {
            let e = call_builtin(
                "memory_forget",
                &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(bad), fl(5.0)],
            )
            .unwrap_err();
            assert!(e.contains("threshold") && e.contains("[0, 1]"), "{e}");
        }
    });
}

#[test]
fn n280_max_forget_bounds_are_loud() {
    with_tmp_cwd(|| {
        standard_db();
        for bad in [0.0, 2.5, -3.0, 20_000.0] {
            let e = call_builtin(
                "memory_forget",
                &[s(DB), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(bad)],
            )
            .unwrap_err();
            assert!(e.contains("max_forget"), "max_forget {bad}: {e}");
        }
    });
}

#[test]
fn n280_non_string_ids_are_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![fl(1.0)]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("ids[0]"), "{e}");
    });
}

#[test]
fn n280_dimension_mismatch_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        let e = call_builtin(
            "memory_forget",
            &[s(DB), s(TB), f(&[1.0, 0.0, 0.0, 0.0]), fl(0.5), fl(5.0)],
        )
        .unwrap_err();
        assert!(e.contains("dimension mismatch"), "{e}");
    });
}

#[test]
fn n280_missing_table_is_loud() {
    with_tmp_cwd(|| {
        standard_db();
        // Файл существует, таблицы нет → громкое «not found».
        let e = call_builtin(
            "memory_forget",
            &[s(DB), s("nope"), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(5.0)],
        )
        .unwrap_err();
        assert!(e.contains("not found"), "{e}");
    });
}

#[test]
fn n280_sandbox_rejects_escapes() {
    with_tmp_cwd(|| {
        standard_db();
        for bad_db in ["../escape.db", "/etc/n280.db"] {
            let e = call_builtin(
                "memory_forget",
                &[s(bad_db), s(TB), f(&[1.0, 0.0, 0.0]), fl(0.5), fl(5.0)],
            )
            .unwrap_err();
            assert!(e.contains("sandbox"), "{bad_db}: {e}");
        }
    });
}

// ── 6. Taint-инвариант: забывание не «стирает» детекцию утечки (№284) ───

#[test]
fn n280_forget_does_not_affect_canary_detection() {
    with_tmp_cwd(|| {
        standard_db();
        // Маркер вставлен, детекция работает.
        let marked = metalogos::builtins::canary_insert_core("untrusted: acct-777", 1, "tail")
            .expect("canary_insert");
        let check1 =
            metalogos::builtins::canary_check_core(&marked.marked_text, &marked.canary_id, "exact")
                .expect("canary_check 1");
        assert!(check1.leaked);
        // Забывание в векторной памяти...
        call_builtin(
            "memory_forget",
            &[
                s(DB),
                s(TB),
                f(&[1.0, 0.0, 0.0]),
                fl(0.5),
                fl(10.0),
                b(false),
                Value::List(vec![s("doc-1")]),
            ],
        )
        .expect("apply");
        // ...НЕ снимает детекцию: canary живёт вне vec0-таблицы.
        let check2 =
            metalogos::builtins::canary_check_core(&marked.marked_text, &marked.canary_id, "exact")
                .expect("canary_check 2");
        assert!(check2.leaked, "forget must not wash out canary detection");
        // И taint-связка на месте: утечка в then-ветке с sink'ом
        // (лекало №284: call_llm → canary_check → if leaked → respond)
        // всё ещё помечается CANARY_LEAK — memory_forget на это не влияет.
        let source = r#"
pattern P(_x: String) -> String {
    let resp = call_llm("sys", "untrusted data")
    let r = canary_check(resp, "MLOG-CANARY-ABCDEFGHIJKLMNOPQRSTUVWXYZ234567")
    if (r.leaked) {
        respond(resp)
    }
    return "ok"
}
"#;
        let result = metalogos::audit_program(source).expect("audit");
        let canary_findings = result
            .findings
            .iter()
            .filter(|f| f.check_id == "CANARY_LEAK")
            .count();
        assert_eq!(canary_findings, 1, "CANARY_LEAK intact while forget exists");
    });
}

// ── 7. Crosscheck TW/VM ─────────────────────────────────────────────────

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(source).map_err(|e| format!("parse: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const FORGET_SCENARIO: &str = r#"
pattern P(_input: String) -> String {
    let q = embed("alpha beta gamma")
    let _s1 = vec_store("parity.db", "docs", "doc-1", q)
    let _s2 = vec_store("parity.db", "docs", "doc-2", embed("totally different zebra"))
    let p = memory_forget("parity.db", "docs", q, 0.5, 5)
    let a = memory_forget("parity.db", "docs", q, 0.5, 5, false, ["doc-1"])
    let hits = vec_search("parity.db", "docs", q, 10)
    return str(p.applied) + ":" + str(a.applied) + ":" + str(len(hits))
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn n280_tw_vm_parity() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("parity.db");
        let tw = run_tw(FORGET_SCENARIO).expect("TW run").expect("TW output");
        let _ = std::fs::remove_file("parity.db");
        let vm = run_vm(FORGET_SCENARIO).expect("VM run").expect("VM output");
        assert_eq!(tw, vm, "TW and VM must agree");
        assert_eq!(tw, "0:1:1", "preview applied=0, apply=1, one hit remains");
    });
}
