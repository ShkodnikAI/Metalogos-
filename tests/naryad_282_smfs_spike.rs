// ── tests/naryad_282_smfs_spike.rs — Наряд №282 (P3, СПАЙК, Tier 3) ─────
//
// Прототип «память как виртуальная ФС» (SMFS-аналог, issue #331):
//   1. read_file("sm:<db>/<c>/profile.md") — детерминированный дайджест
//      профиля (поверх user_profile №281), обрезка значений + указатели
//      на полные записи;
//   2. read_file("sm:<db>/<c>/<bucket>/<key>") — полное значение;
//   3. list_dir/file_exists — навигация по виртуальному пространству;
//   4. read-only: write/append/delete в зоне sm: — громко [SMFS_READ_ONLY];
//   5. песочница не расширяется: '..' — громко; активная песочница
//      forbidden=[filesystem] сильнее перехвата; реальные файлы с
//      префиксом sm: недостижимы (резервация в обе стороны);
//   6. taint: canary-детекция (№284) работает сквозь sm:-экспорт;
//   7. ДЕМО-ЗАМЕР (Go-критерий): profile.md + 2 точечных чтения
//      против «обхода всех записей» — сокращение ≥ 2×;
//   8. crosscheck TW/VM.
//
// Запуск: cargo test --test naryad_282_smfs_spike
// (БЕЗ feature-гейта: smfs — kv-контур ядра, как user_profile №281.)

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

/// kv-файл с заданными container-записями (конвенция ключей —
/// docs/ REFERENCE §4.5; запись через memorize/kv_set покрыта отдельно).
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
            rusqlite::params![k, v],
        )
        .expect("insert kv record");
    }
}

fn read_str(path: &str) -> String {
    match call_builtin("read_file", &[s(path)]) {
        Ok(Value::String(x)) => x,
        other => panic!("read_file('{path}') → String expected, got {other:?}"),
    }
}

fn list_strs(path: &str) -> Vec<String> {
    match call_builtin("list_dir", &[s(path)]) {
        Ok(Value::List(items)) => items
            .into_iter()
            .map(|v| match v {
                Value::String(x) => x,
                other => panic!("list_dir entry not String: {other:?}"),
            })
            .collect(),
        other => panic!("list_dir('{path}') → List expected, got {other:?}"),
    }
}

fn is_true(path: &str) -> bool {
    match call_builtin("file_exists", &[s(path)]) {
        Ok(Value::Bool(x)) => x,
        other => panic!("file_exists('{path}') → Bool expected, got {other:?}"),
    }
}

/// Детерминированный длинный текст (~target_chars символов).
fn long_text(seed: usize, target_chars: usize) -> String {
    let words = [
        "metalogos",
        "memory",
        "container",
        "profile",
        "vector",
        "search",
        "recall",
        "sandbox",
        "taint",
        "ledger",
    ];
    let mut out = String::new();
    let mut i = 0;
    while out.chars().count() < target_chars {
        out.push_str(&format!("{}{} ", words[(seed + i) % words.len()], i));
        i += 1;
    }
    out.trim_end().to_string()
}

const DB: &str = "kv.db";
const BIO: &str = "container:alice:static:bio";

/// Стандартный демо-контейнер: alice (4 записи, из них 1 длинная) + bob.
fn seed_standard() {
    make_kv_db(
        DB,
        &[
            ("container:alice:static:name", "Alice Smith"),
            (BIO, &long_text(3, 400)),
            ("container:alice:dynamic:mood", "ok"),
            ("container:alice:notes:n1", "note one content"),
            ("container:bob:static:name", "Bob"),
        ],
    );
}

// ── 1. profile.md — дайджест ────────────────────────────────────────────

#[test]
fn profile_md_renders_digest() {
    with_tmp_cwd(|| {
        seed_standard();
        let md = read_str("sm:kv.db/alice/profile.md");
        assert!(md.starts_with("# Profile: alice\n"), "header: {md}");
        assert!(md.contains("records: 4\n"), "count: {md}");
        assert!(md.contains("## static\n"), "static section: {md}");
        assert!(md.contains("## dynamic\n"), "dynamic section: {md}");
        assert!(md.contains("## notes\n"), "notes section: {md}");
        // Короткие значения — целиком.
        assert!(md.contains("- name: Alice Smith\n"), "short value: {md}");
        assert!(md.contains("- mood: ok\n"), "short value 2: {md}");
        // Пустой контейнер bob не смешивается (изоляция №281).
        assert!(!md.contains("Bob"), "container isolation: {md}");
    });
}

#[test]
fn digest_truncates_long_values_with_pointer() {
    with_tmp_cwd(|| {
        seed_standard();
        let md = read_str("sm:kv.db/alice/profile.md");
        // Длинное значение обрезано: дайджест-строка короче полного текста,
        // есть указатель на полное чтение (SMFS-парадигма).
        assert!(
            md.contains("(full: sm:kv.db/alice/static/bio)\n"),
            "full-record pointer: {md}"
        );
        let bio_line = md
            .lines()
            .find(|l| l.starts_with("- bio: "))
            .expect("bio digest line");
        // 160 символов дайджеста + '…' → строка заметно короче 400-симв.
        // значения (полный текст НЕ попал в профиль).
        let digest = bio_line.trim_start_matches("- bio: ");
        assert!(digest.ends_with('…'), "truncation marker: {digest}");
        assert!(digest.chars().count() < 200, "digest is compact: {digest}");
        assert!(
            !md.contains(&long_text(3, 400)),
            "full value must not leak into md"
        );
    });
}

// ── 2. Полное значение записи ───────────────────────────────────────────

#[test]
fn full_record_is_byte_exact_and_missing_is_soft() {
    with_tmp_cwd(|| {
        seed_standard();
        // Байт-в-байт равен значению в kv_store.
        assert_eq!(read_str("sm:kv.db/alice/notes/n1"), "note one content");
        assert_eq!(read_str("sm:kv.db/alice/static/bio"), long_text(3, 400));
        // Отсутствующая запись — мягкий исход (аналог отсутствующего файла, №254).
        assert_eq!(read_str("sm:kv.db/alice/notes/nope"), "");
    });
}

// ── 3. Навигация: list_dir / file_exists ────────────────────────────────

#[test]
fn list_root_shows_only_kv_dbs() {
    with_tmp_cwd(|| {
        seed_standard();
        // Не-sqlite файл и каталог не монтируются.
        std::fs::write("readme.txt", "not sqlite").expect("plain file");
        std::fs::create_dir("subdir").expect("dir");
        let dbs = list_strs("sm:");
        assert_eq!(dbs, vec!["kv.db".to_string()], "root mount: {dbs:?}");
        // sqlite-файл БЕЗ kv_store тоже не монтируется.
        let conn = rusqlite::Connection::open("empty.db").expect("open empty");
        conn.execute_batch("CREATE TABLE other (x INTEGER);")
            .expect("ddl");
        drop(conn);
        let dbs = list_strs("sm:");
        assert_eq!(
            dbs,
            vec!["kv.db".to_string()],
            "no kv_store → not mounted: {dbs:?}"
        );
    });
}

#[test]
fn list_db_and_container_are_sorted() {
    with_tmp_cwd(|| {
        seed_standard();
        // Контейнеры sorted.
        assert_eq!(
            list_strs("sm:kv.db"),
            vec!["alice".to_string(), "bob".to_string()]
        );
        // Контейнер: profile.md первым, бакеты sorted с суффиксом '/'.
        let entries = list_strs("sm:kv.db/alice");
        assert_eq!(entries[0], "profile.md", "profile.md first: {entries:?}");
        assert_eq!(
            &entries[1..],
            &[
                "dynamic/".to_string(),
                "notes/".to_string(),
                "static/".to_string()
            ]
        );
    });
}

#[test]
fn file_exists_matrix() {
    with_tmp_cwd(|| {
        seed_standard();
        assert!(is_true("sm:"), "mount root");
        assert!(is_true("sm:kv.db"), "db with kv_store");
        assert!(is_true("sm:kv.db/alice"), "existing container");
        assert!(is_true("sm:kv.db/alice/profile.md"), "profile file");
        assert!(is_true("sm:kv.db/alice/notes/n1"), "existing record");
        assert!(!is_true("sm:kv.db/ghost"), "missing container");
        assert!(
            !is_true("sm:kv.db/ghost/profile.md"),
            "profile of missing container"
        );
        assert!(!is_true("sm:kv.db/alice/notes/nope"), "missing record");
        assert!(!is_true("sm:nosuch.db/alice"), "missing db");
        // Невалидная форма в предикате — мягкое «нет» (контраст с read/list).
        assert!(!is_true("sm:/x"), "invalid form → false");
    });
}

// ── 4. Read-only монтирование ───────────────────────────────────────────

#[test]
fn write_append_delete_are_loud_read_only() {
    with_tmp_cwd(|| {
        seed_standard();
        for (op, args) in [
            (
                "write_file",
                vec![s("sm:kv.db/alice/notes/n1"), s("hacked")],
            ),
            ("append_file", vec![s("sm:kv.db/alice/notes/n1"), s("tail")]),
            ("delete_file", vec![s("sm:kv.db/alice/notes/n1")]),
            // И корень, и невалидная форма — резервация абсолютна.
            ("write_file", vec![s("sm:"), s("x")]),
            ("write_file", vec![s("sm:/x"), s("x")]),
        ] {
            let r = call_builtin(op, &args);
            let err = r.expect_err(&format!("{op} on sm: must be loud"));
            assert!(
                err.contains("[SMFS_READ_ONLY]"),
                "{op} must carry SMFS_READ_ONLY code: {err}"
            );
        }
        // Контент записи не изменился.
        assert_eq!(read_str("sm:kv.db/alice/notes/n1"), "note one content");
    });
}

// ── 5. Песочница: traversal громко; резервация; гейт сильнее ────────────

#[test]
fn traversal_and_bad_forms_are_loud() {
    with_tmp_cwd(|| {
        seed_standard();
        // '..' — нарушение в духе №131, громко с SANDBOX_VIOLATION.
        for p in [
            "sm:kv.db/../secrets",
            "sm:kv.db/alice/../notes/n1",
            "sm:../escape",
        ] {
            let r = call_builtin("read_file", &[s(p)]);
            let err = r.expect_err(&format!("'{p}' must be loud"));
            assert!(
                err.contains("[SANDBOX_VIOLATION]"),
                "'{p}' must carry SANDBOX_VIOLATION: {err}"
            );
        }
        // Невалидные формы — громко с SMFS_BAD_PATH (дефект программы).
        for p in [
            "sm:/x",                      // пустая компонента
            "sm:kv.db/al:ice/profile.md", // ':' в контейнере (конвенция)
            "sm:kv.db/alice/bucket",      // 3-я компонента — не profile.md
            "sm:kv.db/alice/b/k/extra",   // слишком глубоко
        ] {
            let r = call_builtin("read_file", &[s(p)]);
            let err = r.expect_err(&format!("'{p}' must be loud"));
            assert!(
                err.contains("[SMFS_BAD_PATH]"),
                "'{p}' must carry SMFS_BAD_PATH: {err}"
            );
        }
        // Директории не читаются файлово, файлы не листятся — громко.
        assert!(call_builtin("read_file", &[s("sm:")]).is_err());
        assert!(call_builtin("read_file", &[s("sm:kv.db")]).is_err());
        assert!(call_builtin("list_dir", &[s("sm:kv.db/alice/profile.md")]).is_err());
    });
}

#[test]
fn real_files_with_sm_prefix_cannot_be_created() {
    with_tmp_cwd(|| {
        seed_standard();
        // Резервация в обе стороны: реальный файл "sm:real.txt" через
        // builtins создать нельзя (виртуальные пути ≠ реальные файлы,
        // вопрос 2 отчёта).
        let r = call_builtin("write_file", &[s("sm:real.txt"), s("x")]);
        let err = r.expect_err("reservation must hold");
        assert!(err.contains("[SMFS_READ_ONLY]"), "{err}");
        assert!(
            !std::path::Path::new("sm:real.txt").exists(),
            "no real file created"
        );
    });
}

#[test]
fn sandbox_forbidden_filesystem_beats_smfs() {
    // Активная песочница сильнее виртуального слоя: forbidden=[filesystem]
    // отсекает sm:-пути ДО перехвата (Phase 7.5 гейт стоит до builtin_fn).
    let mut interp = metalogos::interpreter::Interpreter::new();
    let sandbox = metalogos::ast::SandboxDecl {
        span: metalogos::ast::Span::unknown(),
        name: "strict".to_string(),
        allowed: vec![],
        forbidden: vec!["filesystem".to_string()],
        timeout: 30,
    };
    interp.set_active_sandbox(sandbox);
    let result = interp.eval_expr(&metalogos::ast::Expr::FnCall {
        name: "read_file".to_string(),
        args: vec![metalogos::ast::Expr::StringLit {
            value: "sm:kv.db/alice/profile.md".to_string(),
            span: metalogos::ast::Span::unknown(),
        }],
        span: metalogos::ast::Span::unknown(),
    });
    let err = result.expect_err("smfs read must be blocked by filesystem-forbidden sandbox");
    assert!(
        err.contains("filesystem access forbidden"),
        "sandbox gate must fire before smfs: {err}"
    );
}

// ── 6. Taint: canary-детекция сквозь экспорт ────────────────────────────

#[test]
fn canary_detection_survives_smfs_export() {
    with_tmp_cwd(|| {
        // Короткое значение (<160): маркер попадает и в полную запись,
        // и в дайджест profile.md.
        let marked = metalogos::builtins::canary_insert_core("untrusted: acct-777", 1, "tail")
            .expect("canary_insert");
        make_kv_db(DB, &[("container:alice:notes:leak", &marked.marked_text)]);
        // Полная запись: canary_check находит маркер.
        let rec = read_str("sm:kv.db/alice/notes/leak");
        assert_eq!(rec, marked.marked_text, "export is byte-exact");
        let check = metalogos::builtins::canary_check_core(&rec, &marked.canary_id, "exact")
            .expect("canary_check record");
        assert!(check.leaked, "detection must survive record export");
        // Дайджест: маркер не вырезался обрезкой — детекция работает и тут.
        let md = read_str("sm:kv.db/alice/profile.md");
        let check_md = metalogos::builtins::canary_check_core(&md, &marked.canary_id, "exact")
            .expect("canary_check md");
        assert!(
            check_md.leaked,
            "detection must survive digest export: {md}"
        );
    });
}

// ── 7. ДЕМО-ЗАМЕР: Go-критерий (сокращение токенов ≥ 2×) ────────────────

#[test]
fn demo_container_token_reduction_meets_go_criterion() {
    with_tmp_cwd(|| {
        // Демо-контейнер: 20 записей в 4 бакетах, значения 500–1200 симв.
        let mut records: Vec<(String, String)> = Vec::new();
        let buckets = ["static", "dynamic", "notes", "projects"];
        for i in 0..20 {
            let bucket = buckets[i % buckets.len()];
            let key = format!("k{:02}", i);
            let target = 500 + (i % 5) * 175; // 500..1200
            let text = long_text(i + 7, target);
            records.push((format!("container:demo:{bucket}:{key}"), text));
        }
        let refs: Vec<(&str, &str)> = records
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        make_kv_db(DB, &refs);

        // Путь «обход всех файлов» (до SMFS): прочитать ВСЕ записи целиком.
        let traversal_chars: usize = records.iter().map(|(_, v)| v.chars().count()).sum();

        // Путь SMFS: cat profile.md + 2 точечных чтения интересующих записей.
        let md = read_str("sm:kv.db/demo/profile.md");
        let detail1 = read_str("sm:kv.db/demo/static/k00");
        let detail2 = read_str("sm:kv.db/demo/projects/k16");
        let smfs_chars = md.chars().count() + detail1.chars().count() + detail2.chars().count();

        // Токены ≈ chars/4 (прокси без токенизатора — задокументировано).
        let toks = |c: usize| c / 4;
        let ratio = traversal_chars as f64 / smfs_chars as f64;

        println!(
            "[smfs-demo] records=20, traversal={} chars (~{} tokens), \
             smfs(profile.md + 2 reads)={} chars (~{} tokens), reduction={:.2}x",
            traversal_chars,
            toks(traversal_chars),
            smfs_chars,
            toks(smfs_chars),
            ratio
        );
        println!("[smfs-demo] profile.md = {} chars", md.chars().count());

        // Go-критерий issue #331: сокращение ≥ 2×.
        assert!(
            ratio >= 2.0,
            "Go criterion: reduction {ratio:.2}x < 2x (traversal {traversal_chars} vs smfs {smfs_chars})"
        );
    });
}

// ── 8. Crosscheck TW/VM ─────────────────────────────────────────────────

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
fn smfs_read_is_identical_in_tw_and_vm() {
    with_tmp_cwd(|| {
        seed_standard();
        let source = r#"
pattern Read(_x: String) -> String {
  let md = read_file("sm:kv.db/alice/profile.md")
  let rec = read_file("sm:kv.db/alice/notes/n1")
  return rec + "|" + str(len(md))
}

flow Main { input: String = "x" -> Read -> output }
"#;
        let cwd = std::env::current_dir().expect("cwd");
        let tw = run_tw(source, &cwd).expect("TW run").unwrap_or_default();
        let vm = run_vm(source, &cwd).expect("VM run").unwrap_or_default();
        assert_eq!(
            tw.trim(),
            vm.trim(),
            "TW и VM обязаны дать байт-в-байт одинаковый вывод smfs-чтения"
        );
        assert!(
            tw.trim().starts_with("note one content|"),
            "TW output must start with the full record: {}",
            tw.trim()
        );
    });
}
