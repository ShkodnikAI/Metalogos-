// ── Наряд №281 (P2, M2): user_profile — детерминированная выжимка контейнера ──
//
// «Что мы знаем о X» одним вызовом (паттерн supermemory user-profiles):
//
//   user_profile(db_path, container) -> Struct{
//       container, count,
//       static:  List[Struct{key, value}],   // долгие факты
//       dynamic: List[Struct{key, value}],   // текущий контекст
//       buckets: Struct{<topic>: List[...]}  // произвольные топики
//   }
//
// ── Источник записей: KV-контур (memorize/kv_set) ──────────────────
// Записи контейнера — kv-записи с ключами convention:
//     container:<container>:<bucket>:<key>  →  значение (String)
// Пишутся существующими memorize/kv_set (с memory { persist: <db_path> }
// — тот же файл читает user_profile) либо напрямую в таблицу kv_store.
// Конвенция задокументирована в REFERENCE; никакого нового writer-а
// не вводится — «источник: записи контейнера (memorize/relate)».
//
// ── Детерминированность ────────────────────────────────────────────
// Сборка БЕЗ LLM-вызова: группировка по bucket, сортировка по ключу.
// LLM-синтез профиля — опционально и явно (сознательно ВНЕ скоупа
// Tier-1, громко в PR/CHANGELOG): детерминированная сборка — контракт
// «одинаковый вход → одинаковый профиль».
//
// ── Кэш с инвалидацией (ADR-0047-дух, №273-сосед) ──────────────────
// Ин-процессный кэш (HashMap) с двойным ключом инвалидации:
//   1) поколение KV-записей (счётчик kv_writes_generation, memory.rs) —
//      запись в контейнер через билтины инвалидирует мгновенно;
//   2) mtime файла — внешние записи мимо билтинов (db_execute, другой
//      процесс) ловит время модификации (консервативно, честная граница).
// Кэш — перф-оптимизация чистых чтений, семантику не меняет.
//
// ── Контейнер-изоляция ─────────────────────────────────────────────
// Префикс container:<name>: — жёсткая граница: записи другого контейнера
// ФИЗИЧЕСКИ не попадают в выборку (не «тихая выдача», а пустота). Тест:
// cross-container данные не читаются. Scope-параметр для user_profile
// сознательно не вводится (громко в PR): контейнер уже является
// изоляционной границей профиля; scope-параметр поставлен на
// vec_store/vec_search (№281), где есть разделяемая поверхность —
// таблицы одного файла.

use super::io::{sandbox_path_ex, SandboxMode};
use super::memory::kv_writes_generation;
use super::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Локальный хелпер Struct-значений (слайсовая форма; без glob-имени
/// в общем неймспейсе — лекало memory_forget.rs).
fn struct_value(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Struct {
        type_name: type_name.to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    }
}

/// Префикс container-записей (SSOT конвенции, см. шапку).
pub const CONTAINER_PREFIX: &str = "container:";

/// Запись кэша: (поколение KV-записей, mtime файла, профиль).
type CacheEntry = (u64, i64, Value);
/// Кэш профилей: (резолвнутый db_path, container) → запись.
type ProfileCache = std::collections::HashMap<(String, String), CacheEntry>;

static PROFILE_CACHE: Mutex<Option<ProfileCache>> = Mutex::new(None);

fn cache_lock() -> Result<std::sync::MutexGuard<'static, Option<ProfileCache>>, String> {
    PROFILE_CACHE
        .lock()
        .map_err(|_| "user_profile cache poisoned".to_string())
}

/// mtime файла в секундах (консервативный сигнал внешних записей).
fn file_mtime_secs(path: &std::path::Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// user_profile(db_path, container) — см. шапку модуля.
pub fn user_profile_core(args: &[Value]) -> Result<Value, String> {
    const BUILTIN: &str = "user_profile";
    if args.len() != 2 {
        return Err(format!(
            "{BUILTIN}() requires exactly 2 arguments (db_path, container), got {}",
            args.len()
        ));
    }
    let db_path = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{BUILTIN}(): argument 1 (db_path) must be a String, got {}",
                other.type_name()
            ))
        }
    };
    let container = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{BUILTIN}(): argument 2 (container) must be a String, got {}",
                other.type_name()
            ))
        }
    };
    if container.is_empty() {
        return Err(format!("{BUILTIN}(): container must not be empty"));
    }
    if container.contains(':') {
        // ':' — разделитель конвенции; контейнер с ':' сломал бы парсинг
        // ключей (container:<c>:<bucket>:<key>) — громко, fail-closed.
        return Err(format!(
            "{BUILTIN}(): container must not contain ':' (key convention separator), got '{container}'"
        ));
    }

    // Песочница ДО кэша: ключ кэша — РЕЗОЛВНУТЫЙ абсолютный путь
    // (относительный "kv.db" в разных cwd — разные файлы; кэш не имеет
    // права их путать — тест n281 ловил ровно это).
    let safe_path =
        sandbox_path_ex(&db_path, SandboxMode::ForRead).map_err(|e| format!("{BUILTIN}(): {e}"))?;
    let cache_key = (safe_path.display().to_string(), container.clone());

    // Кэш: (поколение KV, mtime файла). Попадание → клон профиля.
    {
        let guard = cache_lock()?;
        if let Some(cache) = guard.as_ref() {
            if let Some((gen, mtime, profile)) = cache.get(&cache_key) {
                let cur_gen = kv_writes_generation();
                let cur_mtime = file_mtime_secs(&safe_path);
                if *gen == cur_gen && *mtime == cur_mtime {
                    return Ok(profile.clone());
                }
            }
        }
    }

    // Чтение: только файл (песочница ForRead — файл должен существовать;
    // живой KV-контур процесса сознательно не подмешивается: профиль =
    // состояние файла, кэш согласован с mtime).
    let conn = rusqlite::Connection::open_with_flags(
        &safe_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("{BUILTIN}(): cannot open profile db '{db_path}': {e}"))?;

    // kv_store отсутствует → пустой профиль (не ошибка): «профиль без
    // записей — пустой, не ошибка» (issue #330; natural empty state).
    let kv_table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='kv_store'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("{BUILTIN}(): sqlite_master: {e}"))?;

    // container:<c>:<bucket>:<key> → value; группировка BTreeMap —
    // детерминированный порядок бакетов и записей.
    let mut statics: Vec<(String, String)> = Vec::new();
    let mut dynamics: Vec<(String, String)> = Vec::new();
    let mut others: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut count: i64 = 0;

    if kv_table > 0 {
        let prefix = format!("{CONTAINER_PREFIX}{container}:");
        let mut stmt = conn
            .prepare("SELECT key, value FROM kv_store WHERE key LIKE ?1 ORDER BY key")
            .map_err(|e| format!("{BUILTIN}(): kv query: {e}"))?;
        let like = format!("{prefix}%");
        let rows = stmt
            .query_map([like], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| format!("{BUILTIN}(): kv query: {e}"))?;
        for row in rows {
            let (key, value) = row.map_err(|e| format!("{BUILTIN}(): kv row: {e}"))?;
            let rest = match key.strip_prefix(&prefix) {
                Some(r) => r,
                None => {
                    return Err(format!(
                        "{BUILTIN}(): kv row '{key}' lost prefix '{prefix}' between query and read"
                    ))
                }
            };
            // rest = <bucket>:<key>; bucket без ':' — ключ без разделителя
            // считается битым (записан мимо конвенции) и ПРОПУСКАЕТСЯ
            // молча? НЕТ — громко: запись в контейнере, не подходящая
            // конвенции, — ошибка данных, а не мусор (fail-closed).
            let (bucket, key_part) = match rest.split_once(':') {
                Some(pair) => pair,
                None => {
                    return Err(format!(
                        "{BUILTIN}(): malformed container record '{key}' — expected container:{container}:<bucket>:<key> (bucket and key must not be empty)"
                    ))
                }
            };
            if bucket.is_empty() || key_part.is_empty() {
                return Err(format!(
                    "{BUILTIN}(): malformed container record '{key}' — bucket and key must not be empty"
                ));
            }
            count += 1;
            match bucket {
                "static" => statics.push((key_part.to_string(), value)),
                "dynamic" => dynamics.push((key_part.to_string(), value)),
                other => others
                    .entry(other.to_string())
                    .or_default()
                    .push((key_part.to_string(), value)),
            }
        }
    }

    let records = |pairs: &[(String, String)]| -> Value {
        Value::List(
            pairs
                .iter()
                .map(|(k, v)| {
                    struct_value(
                        "ProfileRecord",
                        &[
                            ("key", Value::String(k.clone())),
                            ("value", Value::String(v.clone())),
                        ],
                    )
                })
                .collect(),
        )
    };

    let buckets_value = Value::Struct {
        type_name: "ProfileBuckets".to_string(),
        fields: others
            .iter()
            .map(|(bucket, pairs)| (bucket.clone(), records(pairs)))
            .collect(),
    };

    let profile = struct_value(
        "UserProfile",
        &[
            ("container", Value::String(container.clone())),
            ("count", Value::Float(count as f64)),
            ("static", records(&statics)),
            ("dynamic", records(&dynamics)),
            ("buckets", buckets_value),
        ],
    );

    // Кладём в кэш текущее состояние.
    let gen = kv_writes_generation();
    let mtime = file_mtime_secs(&safe_path);
    let mut guard = cache_lock()?;
    let cache = guard.get_or_insert_with(std::collections::HashMap::new);
    cache.insert(cache_key, (gen, mtime, profile.clone()));

    Ok(profile)
}

/// Builtin-обвязка (сигнатура вызова из реестра).
pub(crate) fn builtin_user_profile(args: &[Value]) -> Result<Value, String> {
    user_profile_core(args)
}
