// ── tests/naryad_272_vec_search.rs — Наряд №272 (ADR-0134) ──────────
//
// Контрактные тесты векторного контура: embed / vec_store / vec_search.
// Запуск: cargo test --features vec --test naryad_272_vec_search
//
// Честный гейт: файл пуст без фичи `vec` (сборка без фичи зелёная —
// минимальный контракт наряда «Без фичи — все существующие тесты зелёные»).
//
// Тесты serial: embed() использует процесс-глобальный SSOT-менеджер
// (TF-IDF словарь/total_docs растут от вызова к вызову) — детерминизм
// внутри файла требует сериализации. Гейт-тесты используют рукотворные
// векторы и состояние менеджера не трогают.

#![cfg(feature = "vec")]

use metalogos::interpreter::Value;
use serial_test::serial;

/// Вызов билтина через публичный SSOT-реестр (заодно проверяет, что
/// фича `vec` реально включила записи в BUILTIN_REGISTRY).
fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let spec = metalogos::builtins::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("builtin {name} not in BUILTIN_REGISTRY — feature vec enabled?"));
    let handler = spec.handler.expect("vec builtins have handlers");
    handler(args)
}

/// Panic-safe возврат cwd: Drop срабатывает при unwind — упавший тест
/// не оставляет процесс в удалённом tempdir (каскад NotFound в остальных).
struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn f(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect())
}

fn with_tmp_cwd(body: impl FnOnce()) {
    let dir = tempfile::tempdir().expect("tempdir");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("chdir");
    let _guard = CwdGuard(prev);
    body();
}

fn hit_field(v: &Value, i: usize, field: &str) -> Value {
    match v {
        Value::List(items) => match &items[i] {
            Value::Struct { fields, .. } => fields
                .get(field)
                .cloned()
                .unwrap_or_else(|| panic!("hit {i}: no field '{field}'")),
            other => panic!("hit {i} is not a Struct: {other:?}"),
        },
        other => panic!("result is not a List: {other:?}"),
    }
}

fn hit_id(v: &Value, i: usize) -> String {
    match hit_field(v, i, "id") {
        Value::String(s) => s,
        other => panic!("hit id is not a String: {other:?}"),
    }
}

fn hit_distance(v: &Value, i: usize) -> f64 {
    match hit_field(v, i, "distance") {
        Value::Float(d) => d,
        other => panic!("hit distance is not a Float: {other:?}"),
    }
}

// ── 1. DoD: embed → vec_store → vec_search → ближайший id ──────────
#[test]
#[serial]
fn naryad_272_roundtrip_embed_store_search() {
    with_tmp_cwd(|| {
        // Малый корпус: суммарно < 256 различных токенов → dim TF-IDF = 256
        // у всех вызовов, dim-гейт не мешает (см. REFERENCE про дрейф).
        let cat = call_builtin(
            "embed",
            &[Value::String(
                "кот сидит на ковре возле дома мурлычет тихо".to_string(),
            )],
        )
        .expect("embed cat");
        let astro = call_builtin(
            "embed",
            &[Value::String(
                "телескоп наблюдает за орбитой спутника через облака планеты".to_string(),
            )],
        )
        .expect("embed astro");

        let stored = call_builtin(
            "vec_store",
            &[
                Value::String("vec_demo.db".to_string()),
                Value::String("docs".to_string()),
                Value::String("cat-doc".to_string()),
                cat.clone(),
            ],
        )
        .expect("store cat-doc");
        call_builtin(
            "vec_store",
            &[
                Value::String("vec_demo.db".to_string()),
                Value::String("docs".to_string()),
                Value::String("astro-doc".to_string()),
                astro,
            ],
        )
        .expect("store astro-doc");

        // stored: Struct { stored: 1.0, id: "cat-doc", dim, rowid, table }
        match &stored {
            Value::Struct { fields, .. } => {
                match fields.get("stored") {
                    Some(Value::Float(v)) => assert_eq!(*v, 1.0),
                    other => panic!("stored field wrong: {other:?}"),
                }
                match fields.get("id") {
                    Some(Value::String(s)) => assert_eq!(s, "cat-doc"),
                    other => panic!("id field wrong: {other:?}"),
                }
            }
            other => panic!("vec_store result is not a Struct: {other:?}"),
        }

        // Запрос — тот же текст, что у cat-doc (лексически идентичен).
        let query = call_builtin(
            "embed",
            &[Value::String(
                "кот сидит на ковре возле дома мурлычет тихо".to_string(),
            )],
        )
        .expect("embed query");
        let res = call_builtin(
            "vec_search",
            &[
                Value::String("vec_demo.db".to_string()),
                Value::String("docs".to_string()),
                query,
                Value::Float(2.0),
            ],
        )
        .expect("vec_search");

        assert_eq!(hit_id(&res, 0), "cat-doc", "top-1 должен быть cat-doc");
        assert!(
            hit_distance(&res, 0) <= hit_distance(&res, 1),
            "KNN: ближайший первым (0: {}, 1: {})",
            hit_distance(&res, 0),
            hit_distance(&res, 1)
        );
    });
}

// ── 2. Порядок KNN на рукотворных векторах ─────────────────────────
#[test]
#[serial]
fn naryad_272_knn_order_ascending() {
    with_tmp_cwd(|| {
        for (id, vec) in [
            ("near", vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("far", vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ] {
            call_builtin(
                "vec_store",
                &[
                    Value::String("order.db".to_string()),
                    Value::String("items".to_string()),
                    Value::String(id.to_string()),
                    f(&vec),
                ],
            )
            .expect("store");
        }

        // Запрос — невырожденный, ближе к "near": cos(q,near)=0.894, cos(q,far)=0.447.
        let res = call_builtin(
            "vec_search",
            &[
                Value::String("order.db".to_string()),
                Value::String("items".to_string()),
                f(&[1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
                Value::Float(2.0),
            ],
        )
        .expect("search");

        assert_eq!(hit_id(&res, 0), "near");
        assert!(hit_distance(&res, 0) <= hit_distance(&res, 1));
    });
}

// ── 3. Пустая таблица → пустой List (без ошибки) ───────────────────
#[test]
#[serial]
fn naryad_272_empty_table_returns_empty_list() {
    with_tmp_cwd(|| {
        // Существующая, но пустая vec0-таблица: создаём напрямую через
        // rusqlite (билтины создают таблицу только при первой записи).
        {
            use rusqlite::ffi::sqlite3_auto_extension;
            use std::sync::Once;
            static ONCE: Once = Once::new();
            ONCE.call_once(|| unsafe {
                sqlite3_auto_extension(Some(std::mem::transmute(
                    sqlite_vec::sqlite3_vec_init as *const (),
                )));
            });
            let conn = rusqlite::Connection::open("empty.db").expect("open");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS vec_meta (table_name TEXT PRIMARY KEY, dim INTEGER NOT NULL);
                 INSERT INTO vec_meta (table_name, dim) VALUES ('void', 4);
                 CREATE VIRTUAL TABLE void USING vec0(embedding float[4], id text);",
            )
            .expect("create empty vec0 table");
        }

        let res = call_builtin(
            "vec_search",
            &[
                Value::String("empty.db".to_string()),
                Value::String("void".to_string()),
                f(&[1.0, 0.0, 0.0, 0.0]),
                Value::Float(3.0),
            ],
        )
        .expect("search on empty table");

        match res {
            Value::List(items) => assert_eq!(items.len(), 0, "пустая таблица → пустой List"),
            other => panic!("expected List, got {other:?}"),
        }
    });
}

// ── 4. Рассогласование размерности — громко ────────────────────────
#[test]
#[serial]
fn naryad_272_dimension_mismatch_is_loud() {
    with_tmp_cwd(|| {
        call_builtin(
            "vec_store",
            &[
                Value::String("dim.db".to_string()),
                Value::String("t".to_string()),
                Value::String("a".to_string()),
                f(&[1.0, 0.0, 0.0, 0.0]), // dim 4
            ],
        )
        .expect("store dim-4");

        let err = call_builtin(
            "vec_store",
            &[
                Value::String("dim.db".to_string()),
                Value::String("t".to_string()),
                Value::String("b".to_string()),
                f(&[1.0, 0.0]), // dim 2 — другое «модельное пространство»
            ],
        )
        .expect_err("store с другой размерностью обязан упасть");
        assert!(
            err.contains("dimension mismatch"),
            "ошибка должна называть dimension mismatch: {err}"
        );
        assert!(
            err.contains("created with dim 4") && err.contains("got 2"),
            "{err}"
        );

        // Поиск с запросом чужой размерности — тоже громко.
        let err = call_builtin(
            "vec_search",
            &[
                Value::String("dim.db".to_string()),
                Value::String("t".to_string()),
                f(&[0.5; 9]),
                Value::Float(1.0),
            ],
        )
        .expect_err("search с чужой размерностью обязан упасть");
        assert!(err.contains("dimension mismatch"), "{err}");
    });
}

// ── 5. Песочница: эскейпы и SQL-инъекция через имя таблицы ─────────
#[test]
#[serial]
fn naryad_272_sandbox_rejects_escapes() {
    with_tmp_cwd(|| {
        // Path traversal
        let err = call_builtin(
            "vec_store",
            &[
                Value::String("../escape.db".to_string()),
                Value::String("t".to_string()),
                Value::String("a".to_string()),
                f(&[1.0, 0.0]),
            ],
        )
        .expect_err("traversal обязан быть отвергнут");
        assert!(err.contains("sandbox"), "{err}");

        // Абсолютный путь
        let err = call_builtin(
            "vec_search",
            &[
                Value::String("/etc/passwd".to_string()),
                Value::String("t".to_string()),
                f(&[1.0, 0.0]),
                Value::Float(1.0),
            ],
        )
        .expect_err("абсолютный путь обязан быть отвергнут");
        assert!(err.contains("sandbox"), "{err}");

        // Имя таблицы — SQL-инъекция через DDL
        let err = call_builtin(
            "vec_store",
            &[
                Value::String("inj.db".to_string()),
                Value::String("t); DROP TABLE vec_meta; --".to_string()),
                Value::String("a".to_string()),
                f(&[1.0, 0.0]),
            ],
        )
        .expect_err("имя таблицы с SQL-символами обязано упасть");
        assert!(err.contains("invalid table name"), "{err}");
    });
}

// ── 6. Лимиты k: 0 / отрицательное / нецелое — ошибки ──────────────
#[test]
#[serial]
fn naryad_272_k_limits_are_loud() {
    with_tmp_cwd(|| {
        for bad_k in [0.0, -3.0, 2.5] {
            let err = call_builtin(
                "vec_search",
                &[
                    Value::String("k.db".to_string()),
                    Value::String("t".to_string()),
                    f(&[1.0, 0.0]),
                    Value::Float(bad_k),
                ],
            )
            .expect_err(&format!("k={bad_k} обязан быть ошибкой"));
            assert!(err.contains("k must be a positive integer"), "{err}");
        }
    });
}

// ── 7. Отсутствующая таблица — громко ──────────────────────────────
#[test]
#[serial]
fn naryad_272_missing_table_is_loud() {
    with_tmp_cwd(|| {
        // Файл создаём первым store (поиск требует существующий файл —
        // песочница ForRead канонизирует путь); таблицы "nowhere" в нём нет.
        call_builtin(
            "vec_store",
            &[
                Value::String("existing.db".to_string()),
                Value::String("real".to_string()),
                Value::String("a".to_string()),
                f(&[1.0, 0.0]),
            ],
        )
        .expect("store to create the db file");

        let err = call_builtin(
            "vec_search",
            &[
                Value::String("existing.db".to_string()),
                Value::String("nowhere".to_string()),
                f(&[1.0, 0.0]),
                Value::Float(1.0),
            ],
        )
        .expect_err("поиск по несуществующей таблице обязан упасть");
        assert!(err.contains("not found"), "{err}");
    });
}

// ── 8. Crosscheck TW/VM: программа embed → vec_store → vec_search ──
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
#[serial]
fn naryad_272_example_green_in_tw_and_vm() {
    with_tmp_cwd(|| {
        let source = r#"
pattern Setup() -> String {
  let e1 = embed("кот сидит на ковре возле дома мурлычет тихо")
  let e2 = embed("телескоп наблюдает за орбитой спутника через облака планеты")
  let s1 = vec_store("demo.db", "docs", "cat-doc", e1)
  let s2 = vec_store("demo.db", "docs", "astro-doc", e2)
  return str(s1.stored) + str(s2.stored)
}

pattern Ask() -> String {
  let q = embed("кот сидит на ковре возле дома мурлычет тихо")
  let hits = vec_search("demo.db", "docs", q, 2)
  let h = hits[0]
  return h.id
}

pattern RunAll(_input: String) -> String {
  return Setup() + "|" + Ask()
}

flow Main { input: String = "x" -> RunAll -> output }
"#;

        let tw = run_tw(source, &dir_cwd())
            .expect("TW run")
            .unwrap_or_default();
        let vm = run_vm(source, &dir_cwd())
            .expect("VM run")
            .unwrap_or_default();

        let tw_trim = tw.trim();
        let vm_trim = vm.trim();
        assert!(
            tw_trim.ends_with("|cat-doc"),
            "TW output должен закончиться top-1 cat-doc: {tw_trim}"
        );
        assert_eq!(
            tw_trim, vm_trim,
            "TW и VM обязаны дать байт-в-байт одинаковый вывод"
        );
    });
}

/// cwd на момент вызова (внутри with_tmp_cwd это tempdir).
fn dir_cwd() -> std::path::PathBuf {
    std::env::current_dir().expect("cwd")
}

// ── 9. Статический пин арностей (паттерн наряда №279) ──────────────
#[test]
#[serial]
fn naryad_272_arities_pinned() {
    // embed(text) — ровно 1
    assert!(metalogos::builtins::check_builtin_arity("embed", 1).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("embed", 0).is_err());
    assert!(metalogos::builtins::check_builtin_arity("embed", 2).is_err());
    // vec_store(db_path, table, id, embedding) — ровно 4
    assert!(metalogos::builtins::check_builtin_arity("vec_store", 4).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("vec_store", 3).is_err());
    assert!(metalogos::builtins::check_builtin_arity("vec_store", 5).is_err());
    // vec_search(db_path, table, query, k) — ровно 4
    assert!(metalogos::builtins::check_builtin_arity("vec_search", 4).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("vec_search", 3).is_err());
    assert!(metalogos::builtins::check_builtin_arity("vec_search", 5).is_err());
}
