// ── Наряд №282 (P3, research/memory, СПАЙК — Tier 3): SMFS-аналог ──
//
// «Память как виртуальная файловая система» (supermemory SMFS, доки smfs):
// контейнер памяти доступен через ОБЫЧНЫЕ файловые builtins языка как
// виртуальное read-only пространство с префиксом `sm:`:
//
//   sm:                                   → корень монтирования (list_dir:
//                                           db-файлы корня песочницы, у
//                                           которых есть таблица kv_store)
//   sm:<db>                               → контейнеры БД (list_dir)
//   sm:<db>/<container>                   → profile.md + бакеты (list_dir)
//   sm:<db>/<container>/profile.md        → детерминированный дайджест
//                                           профиля (read_file, №281
//                                           user_profile поверх)
//   sm:<db>/<container>/<bucket>/<key>    → полное значение одной записи
//                                           (read_file)
//
// ── Ключевые свойства (по постановке issue #331) ────────────────────
// 1. Доступ ШТАТНЫМИ builtins: перехват встроен в read_file/write_file/
//    append_file/delete_file/file_exists/list_dir (src/builtins/io.rs) —
//    песочница №131/№252/№254 не расширяется: виртуальные пути вообще
//    не трогают диск (кроме открытия самой БД профиля — через штатный
//    sandbox_path_ex ForRead).
// 2. Read-only: запись/удаление в виртуальном пространстве — громкая
//    ошибка [SMFS_READ_ONLY]. Запись в память языка остаётся за
//    memorize/kv_set (один канал записи — вопрос 1 отчёта спайка).
// 3. Резервация префикса: путь, начинающийся с "sm:", обрабатывается
//    ТОЛЬКО виртуально. Реальные файлы с именем "sm:..." через файловые
//    builtins недостижимы (и создать их через builtins нельзя) —
//    виртуальные пути ≠ реальные файлы, коллизия исключена (вопрос 2).
// 4. Детерминизм: рендер профиля — без LLM (БЕЗ генерации; вопрос 4:
//    блокировка — время одного SQLite-запроса, не «чтение-генерация»
//    ADR-0096; LLM-сводка — отдельный продукт-вопрос, громко вне спайка).
// 5. Taint (вопрос 3): экспорт читает те же строки БД, что и штатный
//    user_profile №281 — НОВОГО канала не создаёт; canary-детекция
//    (№284) работает сквозь экспорт (тест в naryad_282_smfs_spike.rs).
//
// Статус: прототип спайка. Живёт на ветке `naryad-282-smfs-profile`,
// в main НЕ мержится (лекало №271: draft-PR — артефакт доказательства);
// в main попадают отчёт docs/research/ и черновик ADR.

use super::io::{sandbox_path_ex, sandbox_violation, SandboxMode};
use super::profile::user_profile_core;
use super::Value;

/// Префикс виртуального SMFS-пространства (SSOT; резервация в обе
/// стороны — см. шапку).
pub(crate) const SMFS_PREFIX: &str = "sm:";

/// Файл дайджеста в корне контейнера (SMFS: «cat profile.md»).
pub(crate) const SMFS_PROFILE_FILE: &str = "profile.md";

/// Обрезка значения в дайджесте (символы, не байты).
const DIGEST_CHARS: usize = 160;

/// Максимум файлов корня, проверяемых list_dir("sm:") (прототипная
/// граница скана; отсортированный порядок — детерминизм).
const ROOT_SCAN_LIMIT: usize = 512;

// ── Диагностические коды (конвенция ADR-0131: код — контракт) ──

fn err_bad_path(path: &str, why: &str) -> String {
    format!(
        "[SMFS_BAD_PATH] smfs: {why}: '{path}' (virtual layout: \
         sm:<db>/<container>/{SMFS_PROFILE_FILE} or \
         sm:<db>/<container>/<bucket>/<key>)"
    )
}

fn err_is_dir(path: &str) -> String {
    format!("[SMFS_IS_DIR] smfs: '{path}' is a virtual directory — use list_dir('{path}')")
}

fn err_is_file(path: &str) -> String {
    format!("[SMFS_IS_FILE] smfs: '{path}' is a virtual file — use read_file('{path}')")
}

/// Громкий отказ записи/удаления в виртуальном пространстве (вопрос 1:
/// запись в память — memorize/kv_set, не файловые builtins).
fn err_read_only(op: &str, path: &str) -> String {
    format!(
        "[SMFS_READ_ONLY] {op}('{path}'): sm: virtual memory filesystem is \
         read-only in the spike — write to memory via memorize/kv_set \
         (spike naryad-282, ADR draft)"
    )
}

// ── Модель виртуального пути ────────────────────────────────────────

/// Разобранный виртуальный путь (валидная форма; существование записи/
/// контейнера/БД проверяется операцией).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SmPath {
    /// `sm:` — корень монтирования.
    Mount,
    /// `sm:<db>` — БД профиля (файл корня песочницы).
    Db(String),
    /// `sm:<db>/<container>` — контейнер (директория).
    Container(String, String),
    /// `sm:<db>/<container>/profile.md` — дайджест профиля.
    Profile(String, String),
    /// `sm:<db>/<container>/<bucket>/<key>` — полное значение записи.
    Record {
        db: String,
        container: String,
        bucket: String,
        key: String,
    },
}

/// `true`, если путь принадлежит зарезервированной виртуальной зоне.
/// Один бит решения на ВСЕХ перехватах io.rs — резервируем префикс
/// в обе стороны (чтение виртуально, реальное «sm:...» недостижимо).
pub(crate) fn is_virtual(path: &str) -> bool {
    path.starts_with(SMFS_PREFIX)
}

/// Разбор виртуального пути. Невалидная ФОРМА — громкая ошибка
/// (дефект программы); «..» — нарушение в духе песочницы №131.
pub(crate) fn parse(path: &str) -> Result<SmPath, String> {
    if !is_virtual(path) {
        return Err(err_bad_path(path, "not an sm: path"));
    }
    let rest = &path[SMFS_PREFIX.len()..];
    if rest.is_empty() {
        return Ok(SmPath::Mount);
    }
    let comps: Vec<&str> = rest.split('/').collect();
    for c in &comps {
        if c.is_empty() {
            return Err(err_bad_path(path, "empty path component"));
        }
        if *c == ".." {
            // Виртуальное пространство не уходит за пределы монтирования:
            // тот же класс нарушения, что в реальной песочнице №131.
            return Err(sandbox_violation(format!(
                "smfs: path traversal ('..') not allowed: '{path}'"
            )));
        }
    }
    match comps.len() {
        1 => Ok(SmPath::Db(comps[0].to_string())),
        2 => {
            let (db, container) = (comps[0].to_string(), comps[1].to_string());
            validate_container(path, &container)?;
            Ok(SmPath::Container(db, container))
        }
        3 => {
            let (db, container) = (comps[0].to_string(), comps[1].to_string());
            validate_container(path, &container)?;
            if comps[2] == SMFS_PROFILE_FILE {
                Ok(SmPath::Profile(db, container))
            } else {
                Err(err_bad_path(
                    path,
                    &format!(
                        "third component must be '{SMFS_PROFILE_FILE}', \
                         full records are sm:<db>/<container>/<bucket>/<key>"
                    ),
                ))
            }
        }
        4 => {
            let (db, container, bucket, key) = (
                comps[0].to_string(),
                comps[1].to_string(),
                comps[2].to_string(),
                comps[3].to_string(),
            );
            validate_container(path, &container)?;
            if bucket.is_empty() {
                return Err(err_bad_path(path, "bucket must not be empty"));
            }
            if key.is_empty() {
                return Err(err_bad_path(path, "key must not be empty"));
            }
            Ok(SmPath::Record {
                db,
                container,
                bucket,
                key,
            })
        }
        n => Err(err_bad_path(
            path,
            &format!("too deep ({n} components under sm:)"),
        )),
    }
}

/// Контейнер: непуст, без ':' (разделитель конвенции ключей — как в
/// user_profile №281, fail-closed) и без '/' (split уже исключил,
/// страховка от будущих правок парсера).
fn validate_container(path: &str, container: &str) -> Result<(), String> {
    if container.is_empty() {
        return Err(err_bad_path(path, "container must not be empty"));
    }
    if container.contains(':') {
        return Err(err_bad_path(
            path,
            "container must not contain ':' (key convention separator)",
        ));
    }
    Ok(())
}

// ── БД профиля ──────────────────────────────────────────────────────

/// Открыть kv-БД (только чтение) через штатную песочницу. db-компонента
/// виртуального пути — имя файла в корне песочницы (одна компонента).
fn open_profile_db(db_file: &str) -> Result<rusqlite::Connection, String> {
    let safe = sandbox_path_ex(db_file, SandboxMode::ForRead)
        .map_err(|e| format!("[SMFS_DB] smfs: db '{db_file}': {e}"))?;
    rusqlite::Connection::open_with_flags(&safe, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("[SMFS_DB] smfs: cannot open db '{db_file}': {e}"))
}

/// Есть ли в БД таблица kv_store (мягкий предикат: нет таблицы/не sqlite
/// → false — файл просто не виден в монтировании).
fn has_kv_store(conn: &rusqlite::Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='kv_store'",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// Имена контейнеров БД (детерминированно: sorted + dedup).
fn containers_in_db(conn: &rusqlite::Connection) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if !has_kv_store(conn) {
        return out;
    }
    if let Ok(mut stmt) =
        conn.prepare("SELECT DISTINCT key FROM kv_store WHERE key LIKE 'container:%'")
    {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for row in rows.flatten() {
                // container:<c>:<bucket>:<key> → имя контейнера до ':'.
                // Битые строки конвенции в НАВИГАЦИИ пропускаются молча:
                // list_dir показывает пространство, а не валидирует данные
                // (валидация — в user_profile/read, fail-closed там).
                if let Some(rest) = row.strip_prefix(super::profile::CONTAINER_PREFIX) {
                    if let Some(c) = rest.split(':').next() {
                        if !c.is_empty() && !out.iter().any(|x| x == c) {
                            out.push(c.to_string());
                        }
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

// ── Операции: read_file ─────────────────────────────────────────────

/// Перехват read_file для виртуальных путей.
pub(crate) fn read(path: &str) -> Result<Value, String> {
    match parse(path)? {
        SmPath::Mount | SmPath::Db(_) | SmPath::Container(_, _) => Err(err_is_dir(path)),
        SmPath::Profile(db, container) => {
            let md = render_profile_md(&db, &container)?;
            Ok(Value::String(md))
        }
        SmPath::Record {
            db,
            container,
            bucket,
            key,
        } => {
            let conn = open_profile_db(&db)?;
            let kv_key = format!(
                "{}{container}:{bucket}:{key}",
                super::profile::CONTAINER_PREFIX
            );
            let value: Result<String, _> =
                conn.query_row("SELECT value FROM kv_store WHERE key = ?1", [kv_key], |r| {
                    r.get(0)
                });
            match value {
                Ok(v) => Ok(Value::String(v)),
                // Нет записи — мягкий исход, как отсутствующий файл (№254).
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Value::String(String::new())),
                Err(e) => Err(format!("[SMFS_DB] smfs: read '{path}': {e}")),
            }
        }
    }
}

// ── Операции: list_dir ──────────────────────────────────────────────

/// Перехват list_dir для виртуальных путей.
pub(crate) fn list(path: &str) -> Result<Value, String> {
    match parse(path)? {
        SmPath::Mount => {
            // Корень монтирования: db-файлы корня песочницы с kv_store.
            let base = std::env::current_dir().map_err(|e| format!("[SMFS_DB] smfs: cwd: {e}"))?;
            let mut names: Vec<String> = std::fs::read_dir(&base)
                .map_err(|e| format!("[SMFS_DB] smfs: read_dir: {e}"))?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            names.sort();
            names.truncate(ROOT_SCAN_LIMIT);
            let mut dbs: Vec<Value> = Vec::new();
            for name in names {
                if name == SMFS_PROFILE_FILE {
                    continue;
                }
                let conn = match rusqlite::Connection::open_with_flags(
                    base.join(&name),
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                ) {
                    Ok(c) => c,
                    Err(_) => continue, // не sqlite — не монтируется
                };
                if has_kv_store(&conn) {
                    dbs.push(Value::String(name));
                }
            }
            Ok(Value::List(dbs))
        }
        SmPath::Db(db) => {
            let conn = open_profile_db(&db)?;
            let containers = containers_in_db(&conn);
            Ok(Value::List(
                containers.into_iter().map(Value::String).collect(),
            ))
        }
        SmPath::Container(db, container) => {
            let conn = open_profile_db(&db)?;
            let mut entries = vec![Value::String(SMFS_PROFILE_FILE.to_string())];
            // Бакеты контейнера: bucket-часть записей container:<c>:<b>:<k>.
            let prefix = format!("{}{container}:", super::profile::CONTAINER_PREFIX);
            if has_kv_store(&conn) {
                if let Ok(mut stmt) =
                    conn.prepare("SELECT DISTINCT key FROM kv_store WHERE key LIKE ?1")
                {
                    let like = format!("{prefix}%");
                    if let Ok(rows) = stmt.query_map([like], |r| r.get::<_, String>(0)) {
                        let mut buckets: Vec<String> = rows
                            .flatten()
                            .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
                            .filter_map(|rest| rest.split(':').next().map(str::to_string))
                            .filter(|b| !b.is_empty())
                            .collect();
                        buckets.sort();
                        buckets.dedup();
                        // Суффикс «/» — виртуальная директория (аналог ls -F).
                        entries.extend(buckets.into_iter().map(|b| Value::String(format!("{b}/"))));
                    }
                }
            }
            Ok(Value::List(entries))
        }
        SmPath::Profile(_, _) | SmPath::Record { .. } => Err(err_is_file(path)),
    }
}

// ── Операции: file_exists ───────────────────────────────────────────

/// Перехват file_exists для виртуальных путей (мягкий предикат).
pub(crate) fn exists(path: &str) -> Value {
    let sm = match parse(path) {
        Ok(sm) => sm,
        // Невалидная форма в предикате — просто «нет» (file_exists
        // не бросает; контраст: read/list дают громкую ошибку).
        Err(_) => return Value::Bool(false),
    };
    let ok = match sm {
        SmPath::Mount => true,
        SmPath::Db(db) => open_profile_db(&db)
            .map(|c| has_kv_store(&c))
            .unwrap_or(false),
        SmPath::Container(db, container) => container_exists(&db, &container),
        SmPath::Profile(db, container) => container_exists(&db, &container),
        SmPath::Record {
            db,
            container,
            bucket,
            key,
        } => {
            let conn = match open_profile_db(&db) {
                Ok(c) => c,
                Err(_) => return Value::Bool(false),
            };
            if !has_kv_store(&conn) {
                return Value::Bool(false);
            }
            let kv_key = format!(
                "{}{container}:{bucket}:{key}",
                super::profile::CONTAINER_PREFIX
            );
            conn.query_row(
                "SELECT 1 FROM kv_store WHERE key = ?1",
                [kv_key],
                |_| Ok(()),
            )
            .is_ok()
        }
    };
    Value::Bool(ok)
}

fn container_exists(db: &str, container: &str) -> bool {
    let conn = match open_profile_db(db) {
        Ok(c) => c,
        Err(_) => return false,
    };
    if !has_kv_store(&conn) {
        return false;
    }
    conn.query_row(
        "SELECT 1 FROM kv_store WHERE key LIKE ?1 LIMIT 1",
        [format!("{}{container}:%", super::profile::CONTAINER_PREFIX)],
        |_| Ok(()),
    )
    .is_ok()
}

// ── Операции: write/append/delete ───────────────────────────────────

/// Перехват write_file/append_file/delete_file — read-only монтирование.
pub(crate) fn read_only_reject(op: &str, path: &str) -> String {
    // Форму проверяем тоже: даже невалидный путь в зоне sm: недостижим
    // как реальный файл — резервация абсолютна.
    let _ = parse(path);
    err_read_only(op, path)
}

// ── Рендер дайджеста профиля (детерминированный, БЕЗ LLM) ───────────

/// Первая строка значения, обрезанная до DIGEST_CHARS по границе чара.
fn digest_line(value: &str) -> String {
    let first_line = value.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= DIGEST_CHARS {
        return first_line.to_string();
    }
    let cut: String = first_line.chars().take(DIGEST_CHARS).collect();
    format!("{cut}…")
}

/// profile.md — детерминированный дайджест (поверх user_profile №281;
/// кэш №281 переиспользуется автоматически).
fn render_profile_md(db: &str, container: &str) -> Result<String, String> {
    let args = [
        Value::String(db.to_string()),
        Value::String(container.to_string()),
    ];
    let profile = user_profile_core(&args)?;

    let fields = match &profile {
        Value::Struct { fields, .. } => fields,
        other => return Err(format!("[SMFS_DB] smfs: profile is not Struct: {other:?}")),
    };
    let count = match fields.get("count") {
        Some(Value::Float(f)) => *f as i64,
        _ => 0,
    };

    // <container>/<bucket>/<key> для указателей на полные записи.
    let full_ref = |bucket: &str, key: &str| -> String {
        format!("{SMFS_PREFIX}{db}/{container}/{bucket}/{key}")
    };

    let mut out = String::new();
    out.push_str(&format!("# Profile: {container}\n"));
    out.push_str(&format!("records: {count}\n"));

    let section = |out: &mut String, name: &str, records: &Value| -> Result<(), String> {
        let items = match records {
            Value::List(items) => items,
            other => {
                return Err(format!(
                    "[SMFS_DB] smfs: profile section '{name}' is not List: {other:?}"
                ))
            }
        };
        if items.is_empty() {
            return Ok(()); // пустые секции не рендерим — токены
        }
        out.push_str(&format!("\n## {name}\n"));
        for item in items {
            let (key, value) = match item {
                Value::Struct { fields, .. } => match (fields.get("key"), fields.get("value")) {
                    (Some(Value::String(k)), Some(Value::String(v))) => (k, v),
                    _ => {
                        return Err(format!(
                            "[SMFS_DB] smfs: profile record in '{name}' malformed"
                        ))
                    }
                },
                other => {
                    return Err(format!(
                        "[SMFS_DB] smfs: profile record in '{name}' is not Struct: {other:?}"
                    ))
                }
            };
            let digest = digest_line(value);
            out.push_str(&format!("- {key}: {digest}\n"));
            if value.len() > digest.len() {
                // Обрезано → указатель на полное чтение (SMFS-парадигма:
                // обзор в profile.md, детали — точечным чтением файла).
                out.push_str(&format!("  (full: {})\n", full_ref(name, key)));
            }
        }
        Ok(())
    };

    if let Some(st) = fields.get("static") {
        section(&mut out, "static", st)?;
    }
    if let Some(dy) = fields.get("dynamic") {
        section(&mut out, "dynamic", dy)?;
    }
    if let Some(Value::Struct {
        fields: buckets, ..
    }) = fields.get("buckets")
    {
        for (bucket, records) in buckets {
            // Порядок бакетов детерминирован (BTreeMap в user_profile).
            section(&mut out, bucket, records)?;
        }
    }

    Ok(out)
}
