// ── tests/naryad_281_profile.rs — Наряд №281 (P2, M2) ──────────────────
//
// Контракт (issue #330, диспатч #332):
//   1. user_profile(db_path, container) — детерминированная выжимка
//      static/dynamic/buckets из KV-записей container:<c>:<bucket>:<key>;
//      профиль без записей — ПУСТОЙ, не ошибка; кэш инвалидируется записью;
//   2. scope-изоляция (контейнер-граница, containerTag-аналог):
//      cross-scope доступ — ГРОМКАЯ ошибка, не тихая выдача;
//   3. hybrid-контракт vec_search: mode semantic|fts|hybrid (RRF k=60);
//      hybrid ≥ max(плечей) по recall на фикс. наборе (таблица в PR);
//   4. crosscheck TW/VM.
//
// Запуск: cargo test --features vec --test naryad_281_profile
// Честный гейт: без фичи `vec` файл пуст (user_profile — kv-контур,
// но scope/hybrid-половина живёт в vec-билтинах; единый файл).

#![cfg(feature = "vec")]

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
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

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}
fn f(v: f64) -> Value {
    Value::Float(v)
}
fn b(v: bool) -> Value {
    Value::Bool(v)
}
fn fvec(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect())
}

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
fn list_len(v: &Value) -> usize {
    match v {
        Value::List(items) => items.len(),
        other => panic!("expected List, got {other:?}"),
    }
}
fn hit_ids(v: &Value) -> Vec<String> {
    match v {
        Value::List(items) => items.iter().map(|h| field_str(h, "id")).collect(),
        other => panic!("expected List, got {other:?}"),
    }
}

/// kv-файл с заданными container-записями (напрямую sqlite — конвенция
/// ключей задокументирована; запись через memorize/kv_set покрыта
/// отдельно + в TW/VM parity).
fn make_kv_db(path: &str, records: &[(&str, &str)]) {
    let _ = std::fs::remove_file(path);
    let conn = rusqlite::Connection::open(path).expect("open kv db");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .expect("kv_store DDL");
    for (k, v) in records {
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
            [k, v],
        )
        .expect("insert");
    }
}

fn profile(db: &str, container: &str) -> Value {
    call_builtin("user_profile", &[s(db), s(container)]).expect("user_profile")
}

fn opts(pairs: &[(&str, Value)]) -> Value {
    Value::Struct {
        type_name: "SearchOpts".to_string(),
        fields: pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    }
}

/// Стандартный документный корпус (4 docs): векторное и текстовое плечо
/// согласованы частично — для eval-теста hybrid.
fn corpus_db() -> &'static str {
    "n281.db"
}

fn store_corpus() {
    let db = corpus_db();
    let _ = std::fs::remove_file(db);
    // d1: вектор-релевантный, текст НЕ про sqlite
    call_builtin(
        "vec_store",
        &[
            s(db),
            s("docs"),
            s("d1"),
            fvec(&[1.0, 0.0]),
            s("metalogos vector language runtime"),
        ],
    )
    .expect("d1");
    // d2: текст-шум
    call_builtin(
        "vec_store",
        &[
            s(db),
            s("docs"),
            s("d2"),
            fvec(&[0.0, 1.0]),
            s("cooking pasta carbonara recipe"),
        ],
    )
    .expect("d2");
    // d3: второй вектор-релевантный
    call_builtin(
        "vec_store",
        &[
            s(db),
            s("docs"),
            s("d3"),
            fvec(&[0.9, 0.1]),
            s("vector database embeddings search"),
        ],
    )
    .expect("d3");
    // d4: текст-релевантный (sqlite), вектор далеко
    call_builtin(
        "vec_store",
        &[
            s(db),
            s("docs"),
            s("d4"),
            fvec(&[0.0, 1.0]),
            s("sqlite performance tuning guide"),
        ],
    )
    .expect("d4");
}

// ── 1. user_profile: базовый контракт ───────────────────────────────────

#[test]
fn n281_profile_empty_is_empty_not_error() {
    with_tmp_cwd(|| {
        make_kv_db("kv.db", &[]);
        let p = profile("kv.db", "alice");
        assert_eq!(field_str(&p, "container"), "alice");
        assert_eq!(field_f64(&p, "count"), 0.0);
        assert_eq!(list_len(&field(&p, "static")), 0, "empty static");
        assert_eq!(list_len(&field(&p, "dynamic")), 0, "empty dynamic");
        let buckets = field(&p, "buckets");
        match &buckets {
            Value::Struct { fields, .. } => assert!(fields.is_empty(), "no buckets"),
            other => panic!("buckets must be Struct, got {other:?}"),
        }
    });
}

#[test]
fn n281_profile_missing_kv_table_is_empty() {
    with_tmp_cwd(|| {
        // Файл без kv_store (естественное «пустое» состояние) — пустой профиль.
        let _ = std::fs::remove_file("fresh.db");
        rusqlite::Connection::open("fresh.db").expect("create empty db");
        let p = profile("fresh.db", "alice");
        assert_eq!(field_f64(&p, "count"), 0.0);
    });
}

#[test]
fn n281_profile_groups_static_dynamic_buckets() {
    with_tmp_cwd(|| {
        make_kv_db(
            "kv.db",
            &[
                ("container:alice:static:email", "alice@example.com"),
                ("container:alice:static:lang", "ru"),
                ("container:alice:dynamic:topic", "rust vectors"),
                ("container:alice:hobbies:fish", "goldfish named Byte"),
                ("container:alice:hobbies:plant", "monstera"),
            ],
        );
        let p = profile("kv.db", "alice");
        assert_eq!(field_f64(&p, "count"), 5.0);
        let st = field(&p, "static");
        assert_eq!(list_len(&st), 2);
        let st0 = match &st {
            Value::List(items) => items[0].clone(),
            other => panic!("static must be List, got {other:?}"),
        };
        assert_eq!(field_str(&st0, "key"), "email", "sorted by key");
        let dyn_ = field(&p, "dynamic");
        assert_eq!(list_len(&dyn_), 1);
        let d0 = match &dyn_ {
            Value::List(items) => items[0].clone(),
            other => panic!("dynamic must be List, got {other:?}"),
        };
        assert_eq!(field_str(&d0, "value"), "rust vectors");
        let buckets = field(&p, "buckets");
        let hobbies = field(&buckets, "hobbies");
        assert_eq!(list_len(&hobbies), 2, "arbitrary topics become buckets");
    });
}

#[test]
fn n281_profile_cross_container_isolation() {
    with_tmp_cwd(|| {
        make_kv_db(
            "kv.db",
            &[
                ("container:alice:static:secret", "alice-data"),
                ("container:bob:static:note", "bob-data"),
            ],
        );
        let alice = profile("kv.db", "alice");
        assert_eq!(field_f64(&alice, "count"), 1.0);
        // Данные bob ФИЗИЧЕСКИ не видны в профиле alice (не «тихая выдача»).
        let st = field(&alice, "static");
        match &st {
            Value::List(items) => {
                for item in items {
                    let v = field_str(item, "value");
                    assert_ne!(v, "bob-data", "cross-container leak!");
                }
            }
            other => panic!("static must be List, got {other:?}"),
        }
        let bob = profile("kv.db", "bob");
        assert_eq!(field_f64(&bob, "count"), 1.0);
        let bob_st = field(&bob, "static");
        let bob0 = match &bob_st {
            Value::List(items) => items[0].clone(),
            other => panic!("static must be List, got {other:?}"),
        };
        assert_eq!(field_str(&bob0, "value"), "bob-data");
    });
}

#[test]
fn n281_profile_malformed_record_is_loud() {
    with_tmp_cwd(|| {
        make_kv_db("kv.db", &[("container:alice:bogus", "no-bucket-colon")]);
        let e = call_builtin("user_profile", &[s("kv.db"), s("alice")]).unwrap_err();
        assert!(e.contains("malformed container record"), "{e}");
    });
}

#[test]
fn n281_profile_container_with_colon_is_loud() {
    with_tmp_cwd(|| {
        let e = call_builtin("user_profile", &[s("kv.db"), s("a:b")]).unwrap_err();
        assert!(e.contains("must not contain ':'"), "{e}");
    });
}

#[test]
fn n281_profile_cache_invalidation_on_kv_write() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("live.db");
        // Живой контур: persist + kv_set через билтины.
        metalogos::builtins::init_kv_persist("live.db").expect("persist");
        let p1 = profile("live.db", "alice");
        assert_eq!(field_f64(&p1, "count"), 0.0, "empty at first");
        // Запись в контейнер инвалидирует кэш (поколение KV-записей).
        call_builtin("kv_set", &[s("container:alice:static:email"), s("a@x.io")]).expect("kv_set");
        let p2 = profile("live.db", "alice");
        assert_eq!(field_f64(&p2, "count"), 1.0, "cache invalidated by write");
        let st2 = field(&p2, "static");
        let s2 = match &st2 {
            Value::List(items) => items[0].clone(),
            other => panic!("static must be List, got {other:?}"),
        };
        assert_eq!(
            field_str(&s2, "value"),
            "a@x.io",
            "new record visible after invalidation"
        );
    });
}

#[test]
fn n281_profile_sandbox_and_arity() {
    with_tmp_cwd(|| {
        // Песочница: абсолютный путь — громко.
        let e = call_builtin("user_profile", &[s("/etc/passwd"), s("alice")]).unwrap_err();
        assert!(e.contains("sandbox"), "{e}");
        // Арность ровно 2.
        assert!(call_builtin("user_profile", &[s("kv.db")]).is_err());
        assert!(call_builtin("user_profile", &[s("kv.db"), s("a"), s("c")]).is_err());
    });
}

// ── 2. Scope-изоляция (containerTag-аналог) ─────────────────────────────

#[test]
fn n281_scope_bind_and_search_ok() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("sc.db");
        call_builtin(
            "vec_store",
            &[
                s("sc.db"),
                s("t"),
                s("x1"),
                fvec(&[1.0, 0.0]),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .expect("store with scope");
        let hits = call_builtin(
            "vec_search",
            &[
                s("sc.db"),
                s("t"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .expect("search with matching scope");
        assert_eq!(hit_ids(&hits), vec!["x1".to_string()]);
    });
}

#[test]
fn n281_scope_cross_access_is_loud() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("sc.db");
        call_builtin(
            "vec_store",
            &[
                s("sc.db"),
                s("t"),
                s("x1"),
                fvec(&[1.0, 0.0]),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .expect("store acme");
        // Чужой scope — ГРОМКО, не тихая выдача.
        let e = call_builtin(
            "vec_search",
            &[
                s("sc.db"),
                s("t"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("scope", s("competitor"))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("SCOPE_VIOLATION"), "{e}");
        assert!(
            e.contains("acme") && e.contains("competitor"),
            "names both scopes: {e}"
        );
    });
}

#[test]
fn n281_scope_unbound_table_is_loud() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("plain.db");
        call_builtin(
            "vec_store",
            &[s("plain.db"), s("t"), s("x1"), fvec(&[1.0, 0.0])],
        )
        .expect("store without scope");
        // Fail-closed: явный scope на незабинженной таблице — громко.
        let e = call_builtin(
            "vec_search",
            &[
                s("plain.db"),
                s("t"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("not scope-bound"), "{e}");
    });
}

#[test]
fn n281_scope_rebind_is_loud() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("sc.db");
        call_builtin(
            "vec_store",
            &[
                s("sc.db"),
                s("t"),
                s("x1"),
                fvec(&[1.0, 0.0]),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .expect("bind acme");
        let e = call_builtin(
            "vec_store",
            &[
                s("sc.db"),
                s("t"),
                s("x2"),
                fvec(&[0.0, 1.0]),
                opts(&[("scope", s("other"))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("refusing to rebind"), "{e}");
    });
}

#[test]
fn n281_scope_legacy_calls_unaffected() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("sc.db");
        call_builtin(
            "vec_store",
            &[
                s("sc.db"),
                s("t"),
                s("x1"),
                fvec(&[1.0, 0.0]),
                opts(&[("scope", s("acme"))]),
            ],
        )
        .expect("bind acme");
        // Без scope — прежнее поведение (№272/№280 контракты).
        let hits = call_builtin(
            "vec_search",
            &[s("sc.db"), s("t"), fvec(&[1.0, 0.0]), f(5.0)],
        )
        .expect("legacy search");
        assert_eq!(hit_ids(&hits), vec!["x1".to_string()]);
    });
}

// ── 3. fts / hybrid режимы ──────────────────────────────────────────────

#[test]
fn n281_fts_mode_returns_lexical_hits() {
    with_tmp_cwd(|| {
        store_corpus();
        let hits = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("mode", s("fts")), ("query_text", s("sqlite"))]),
            ],
        )
        .expect("fts search");
        let ids = hit_ids(&hits);
        assert_eq!(ids, vec!["d4".to_string()], "lexical arm finds d4 only");
        let h0 = match &hits {
            Value::List(items) => items[0].clone(),
            other => panic!("expected List, got {other:?}"),
        };
        let score = field_f64(&h0, "score");
        assert!(
            (score - 1.0).abs() < 1e-9,
            "max-normalized top score = 1.0, got {score}"
        );
        let dist = field_f64(&h0, "distance");
        assert!(
            (dist - (1.0 - score)).abs() < 1e-9,
            "distance = 1 - score in fts mode"
        );
    });
}

#[test]
fn n281_fts_without_text_index_is_loud() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("nt.db");
        call_builtin("vec_store", &[s("nt.db"), s("t"), s("x1"), fvec(&[1.0])]).expect("no text");
        let e = call_builtin(
            "vec_search",
            &[
                s("nt.db"),
                s("t"),
                fvec(&[1.0]),
                f(5.0),
                opts(&[("mode", s("fts")), ("query_text", s("anything"))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("no FTS5 index"), "{e}");
    });
}

#[test]
fn n281_fts_requires_query_text() {
    with_tmp_cwd(|| {
        store_corpus();
        let e = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("mode", s("fts"))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("query_text"), "{e}");
    });
}

#[test]
fn n281_empty_query_text_is_loud() {
    with_tmp_cwd(|| {
        store_corpus();
        let e = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("mode", s("fts")), ("query_text", s("  "))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("must not be empty"), "{e}");
    });
}

#[test]
fn n281_unknown_opts_keys_are_loud() {
    with_tmp_cwd(|| {
        store_corpus();
        let e = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("bogus", b(true))]),
            ],
        )
        .unwrap_err();
        assert!(e.contains("unknown opts field"), "{e}");
        // vec_store: тоже fail-closed.
        let e2 = call_builtin(
            "vec_store",
            &[
                s(corpus_db()),
                s("docs"),
                s("d9"),
                fvec(&[1.0, 0.0]),
                opts(&[("bogus", b(true))]),
            ],
        )
        .unwrap_err();
        assert!(e2.contains("unknown opts field"), "{e2}");
    });
}

#[test]
fn n281_semantic_mode_unchanged() {
    with_tmp_cwd(|| {
        store_corpus();
        let legacy = call_builtin(
            "vec_search",
            &[s(corpus_db()), s("docs"), fvec(&[1.0, 0.0]), f(2.0)],
        )
        .expect("legacy");
        let via_opts = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(2.0),
                opts(&[("mode", s("semantic"))]),
            ],
        )
        .expect("opts semantic");
        // Идентичные id и distance (score — добавленное поле №281).
        assert_eq!(hit_ids(&legacy), hit_ids(&via_opts), "same ranking");
        match (&legacy, &via_opts) {
            (Value::List(a), Value::List(bv)) => {
                for (ha, hb) in a.iter().zip(bv.iter()) {
                    assert!((field_f64(ha, "distance") - field_f64(hb, "distance")).abs() < 1e-12);
                }
            }
            other => panic!("expected Lists, got {other:?}"),
        }
        assert_eq!(hit_ids(&legacy), vec!["d1".to_string(), "d3".to_string()]);
    });
}

/// DoD-2: hybrid ≥ max(плечей) по recall на фикс. наборе.
///
/// Корпус: d1 вектор-релевантен (vec [1,0]), d4 текст-релевантен
/// ("sqlite", вектор далеко). Запрос: vec [1,0] + text "sqlite", k=2.
/// Истина (info need «дай и векторные, и про sqlite»): {d1, d4}.
///
/// | Плечо    | Hits @2    | Recall vs {d1, d4} |
/// |----------|------------|--------------------|
/// | semantic | d1, d3     | 1/2 (только d1)    |
/// | fts      | d4         | 1/2 (только d4)    |
/// | hybrid   | d1, d4     | **2/2** ≥ max(1/2) |
#[test]
fn n281_hybrid_beats_both_arms_recall() {
    with_tmp_cwd(|| {
        store_corpus();
        let k = 2.0;
        let sem = hit_ids(
            &call_builtin(
                "vec_search",
                &[s(corpus_db()), s("docs"), fvec(&[1.0, 0.0]), f(k)],
            )
            .expect("semantic arm"),
        );
        let fts = hit_ids(
            &call_builtin(
                "vec_search",
                &[
                    s(corpus_db()),
                    s("docs"),
                    fvec(&[1.0, 0.0]),
                    f(k),
                    opts(&[("mode", s("fts")), ("query_text", s("sqlite"))]),
                ],
            )
            .expect("fts arm"),
        );
        let hybrid = hit_ids(
            &call_builtin(
                "vec_search",
                &[
                    s(corpus_db()),
                    s("docs"),
                    fvec(&[1.0, 0.0]),
                    f(k),
                    opts(&[("mode", s("hybrid")), ("query_text", s("sqlite"))]),
                ],
            )
            .expect("hybrid"),
        );
        // Плеча поодиночке теряют по половине истины.
        assert_eq!(sem, vec!["d1".to_string(), "d3".to_string()]);
        assert_eq!(fts, vec!["d4".to_string()]);
        // Hybrid покрывает ОБА мира одним вызовом: {d1, d4}.
        let truth_semantic = ["d1".to_string()];
        let truth_fts = ["d4".to_string()];
        let count = |hits: &[String], truth: &[String]| -> usize {
            hits.iter().filter(|h| truth.contains(h)).count()
        };
        let r_sem = count(&sem, &truth_semantic) + count(&sem, &truth_fts);
        let r_fts = count(&fts, &truth_semantic) + count(&fts, &truth_fts);
        let r_hyb = count(&hybrid, &truth_semantic) + count(&hybrid, &truth_fts);
        assert!(
            r_hyb >= r_sem.max(r_fts),
            "hybrid recall {r_hyb} must be >= max(semantic {r_sem}, fts {r_fts})"
        );
        assert_eq!(r_hyb, 2, "hybrid finds both worlds: {hybrid:?}");
        assert!(
            hybrid.contains(&"d4".to_string()),
            "lexical-relevant d4 found"
        );
        assert!(
            hybrid.contains(&"d1".to_string()),
            "vector-relevant d1 kept"
        );
    });
}

#[test]
fn n281_forget_filter_works_in_all_modes() {
    with_tmp_cwd(|| {
        store_corpus();
        // Забываем d1 (soft-delete из №280).
        call_builtin(
            "memory_forget",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(0.5),
                f(10.0),
                b(false),
                Value::List(vec![s("d1")]),
            ],
        )
        .expect("forget d1");
        // hybrid: d1 скрыт (include_forgotten дефолт false).
        let hits = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(10.0),
                opts(&[("mode", s("hybrid")), ("query_text", s("sqlite vector"))]),
            ],
        )
        .expect("hybrid after forget");
        let ids = hit_ids(&hits);
        assert!(
            !ids.contains(&"d1".to_string()),
            "forgotten hidden in hybrid: {ids:?}"
        );
        // include_forgotten=true через opts — d1 возвращается.
        let all = call_builtin(
            "vec_search",
            &[
                s(corpus_db()),
                s("docs"),
                fvec(&[1.0, 0.0]),
                f(10.0),
                opts(&[
                    ("mode", s("hybrid")),
                    ("query_text", s("sqlite vector")),
                    ("include_forgotten", b(true)),
                ]),
            ],
        )
        .expect("hybrid include_forgotten");
        assert!(hit_ids(&all).contains(&"d1".to_string()));
    });
}

#[test]
fn n281_vec_store_payload_forms() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("pl.db");
        // 5-й String — текст.
        call_builtin(
            "vec_store",
            &[
                s("pl.db"),
                s("t"),
                s("a"),
                fvec(&[1.0, 0.0]),
                s("hello world"),
            ],
        )
        .expect("text payload");
        // 5-й Struct — text + scope.
        call_builtin(
            "vec_store",
            &[
                s("pl.db"),
                s("t"),
                s("b"),
                fvec(&[0.0, 1.0]),
                opts(&[("text", s("goodbye world")), ("scope", s("acme"))]),
            ],
        )
        .expect("opts payload");
        // Неверный тип payload — громко.
        let e = call_builtin(
            "vec_store",
            &[s("pl.db"), s("t"), s("c"), fvec(&[1.0, 1.0]), b(true)],
        )
        .unwrap_err();
        assert!(e.contains("payload"), "{e}");
        // FTS-плечо собрано из обоих текстов.
        let hits = call_builtin(
            "vec_search",
            &[
                s("pl.db"),
                s("t"),
                fvec(&[1.0, 0.0]),
                f(5.0),
                opts(&[("mode", s("fts")), ("query_text", s("world"))]),
            ],
        )
        .expect("fts over payloads");
        let ids = hit_ids(&hits);
        assert!(
            ids.contains(&"a".to_string()) && ids.contains(&"b".to_string()),
            "{ids:?}"
        );
    });
}

// ── 4. Crosscheck TW/VM ─────────────────────────────────────────────────

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

const PROFILE_HYBRID_SCENARIO: &str = r#"
pattern P(_input: String) -> String {
    let _w1 = kv_set("container:alice:static:email", "alice@example.com")
    let _w2 = kv_set("container:alice:dynamic:topic", "rust")
    let p = user_profile("n281kv.db", "alice")
    let _s1 = vec_store("n281vec.db", "docs", "d1", [1.0, 0.0], "metalogos vector language")
    let _s2 = vec_store("n281vec.db", "docs", "d2", [0.0, 1.0], "cooking pasta carbonara")
    let hits = vec_search("n281vec.db", "docs", [1.0, 0.0], 5, {mode: "fts", query_text: "vector"})
    return str(p.count) + ":" + str(p.static[0].key) + ":" + str(len(hits))
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn n281_tw_vm_parity() {
    with_tmp_cwd(|| {
        let _ = std::fs::remove_file("n281kv.db");
        let _ = std::fs::remove_file("n281vec.db");
        metalogos::builtins::init_kv_persist("n281kv.db").expect("persist");
        let tw = run_tw(PROFILE_HYBRID_SCENARIO)
            .expect("TW run")
            .expect("TW output");
        let _ = std::fs::remove_file("n281vec.db");
        let vm = run_vm(PROFILE_HYBRID_SCENARIO)
            .expect("VM run")
            .expect("VM output");
        assert_eq!(tw, vm, "TW and VM must agree");
        assert_eq!(
            tw, "2:email:1",
            "profile 2 records, static[0]=email, fts finds d1"
        );
    });
}
