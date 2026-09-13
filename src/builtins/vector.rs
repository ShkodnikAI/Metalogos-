// ── Векторный контур языка — Наряд №272, ADR-0134 (Go-вердикт спайка №271) ──
//
// Три билтина (feature-gate `vec`, off-by-default — паттерн candle/vision,
// ADR-0104 measured impact; в `portable` включён по ADR-0134 D3):
//
//   embed(text) -> List[Float]
//       Реюз EmbeddingManager (ADR-0040, без новых зависимостей).
//       Бэкенд выбирается окружением: METALOGOS_EMBEDDING_PROVIDER=openai
//       + METALOGOS_EMBEDDING_API_KEY → OpenAI text-embedding-3-small
//       (1536), иначе детерминированный TF-IDF (256+).
//
//   vec_store(db_path, table, id, embedding) -> Struct
//   vec_search(db_path, table, query_embedding, k) -> List[Struct{id, distance}]
//       Generic-билтины над vec0-виртуальными таблицами sqlite-vec
//       (KNN distance_metric=cosine). Имена domain-agnostic
//       (FEATURE_INTAKE §4-D): память Phase 4 (group_scenarios /
//       recall_from_scenario) потребляет их как фундамент, но знаниям
//       о домене здесь не место.
//
// ── SSOT эмбеддингов: один менеджер на процесс ─────────────────────
// TfidfEmbedding хранит словарь ВНУТРИ экземпляра и растит размерность
// вместе с ним (max(vocab, 256)). Создавать новый менеджер на каждый
// вызов нельзя: векторы разных вызовов окажутся в разных пространствах.
// Поэтому менеджер — процесс-глобальный (once_cell, паттерн наряда №4):
// словарь растёт консистентно, векторы одного прогона сравнимы.
// Детерминированность: TF-IDF детерминирован для той же
// последовательности embed-вызовов процесса; между процессами с разной
// историей словарь (и размерность) может отличаться — честно задокументировано.
//
// ── Auto-extension: регистрация на процесс ─────────────────────────
// sqlite3_auto_extension(sqlite3_vec_init) — канонический контракт
// sqlite-vec для rusqlite БЕЗ feature `load_extension` (ADR-0134 D2,
// факт-чек спайка №271). Регистрация глобальна: каждый НОВЫЙ Connection
// после неё получает vec0. Соединения, открытые до регистрации, vec0
// не видят — для них это безопасно (функции просто не вызываются).
//
// ── Песочница ──────────────────────────────────────────────────────
// db_path проходит sandbox_path_ex (наряды №131/№252): абсолютные пути,
// '..' и symlink-эскейпы отвергаются. Векторная БД — файл как любой
// другой: ForWrite при vec_store, ForRead при vec_search.

use super::Value;
use crate::builtins::io::{sandbox_path_ex, SandboxMode};
use crate::embeddings::EmbeddingManager;
use once_cell::sync::Lazy;
use rusqlite::ffi::sqlite3_auto_extension;
use std::sync::{Mutex, Once};

/// Глобальный SSOT-менеджер эмбеддингов (см. шапку модуля).
static EMBEDDING_MANAGER: Lazy<Mutex<EmbeddingManager>> =
    Lazy::new(|| Mutex::new(EmbeddingManager::new()));

fn register_vec_extension() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

/// Идентификатор SQL-таблицы: [A-Za-z_][A-Za-z0-9_]* — защита от SQL-инъекции
/// через имя таблицы (имя попадает в DDL напрямую, биндить его нельзя).
pub(crate) fn validate_table_name(builtin: &str, table: &str) -> Result<(), String> {
    let mut chars = table.chars();
    let ok = match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{builtin}(): invalid table name '{}': must match [A-Za-z_][A-Za-z0-9_]* (SQL identifier whitelist)",
            table
        ))
    }
}

fn value_as_string(builtin: &str, pos: usize, v: &Value, what: &str) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{builtin}(): argument {pos} ({what}) must be a String, got {}",
            type_name(other)
        )),
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "String",
        Value::Float(_) => "Float",
        Value::Bool(_) => "Bool",
        Value::List(_) => "List",
        Value::Struct { .. } => "Struct",
        _ => "other",
    }
}

pub(crate) fn value_as_embedding(builtin: &str, pos: usize, v: &Value) -> Result<Vec<f32>, String> {
    match v {
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                match item {
                    Value::Float(f) => out.push(*f as f32),
                    other => {
                        return Err(format!(
                            "{builtin}(): embedding component {i} must be a Float, got {}",
                            type_name(other)
                        ))
                    }
                }
            }
            Ok(out)
        }
        other => Err(format!(
            "{builtin}(): argument {pos} must be a List of Float (embedding), got {}",
            type_name(other)
        )),
    }
}

fn make_struct(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Struct {
        type_name: type_name.to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    }
}

// ── embed ──────────────────────────────────────────────────────────

/// `embed(text) -> List[Float]` — эмбеддинг текста через глобальный
/// SSOT-менеджер (см. шапку модуля). Tier 1: 2+ домена (память, кэш
/// №273, дедупликация, evals) — без новых зависимостей, реюз
/// `EmbeddingManager` (ADR-0040).
///
/// Модель/размерность по умолчанию: TF-IDF, dim = max(vocab, 256) —
/// детерминированно внутри процесса (та же последовательность вызовов →
/// те же векторы). `METALOGOS_EMBEDDING_PROVIDER=openai` +
/// `METALOGOS_EMBEDDING_API_KEY` → OpenAI text-embedding-3-small, dim 1536.
pub(crate) fn builtin_embed(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "embed() requires exactly 1 argument (text), got {}",
            args.len()
        ));
    }
    let text = value_as_string("embed", 1, &args[0], "text")?;
    let manager = EMBEDDING_MANAGER
        .lock()
        .map_err(|_| "embedding manager poisoned".to_string())?;
    let vec = manager.embed(&text)?;
    Ok(Value::List(
        vec.into_iter().map(|x| Value::Float(x as f64)).collect(),
    ))
}

/// Процесс-глобальный эмбеддинг-хелпер для не-билтиновых потребителей
/// (наряд №273: semantic cache в learnable-контуре) — тот же SSOT-менеджер,
/// что и у билтина `embed` (векторы сравнимы по определению).
pub(crate) fn embed_text(text: &str) -> Result<Vec<f32>, String> {
    let manager = EMBEDDING_MANAGER
        .lock()
        .map_err(|_| "embedding manager poisoned".to_string())?;
    manager.embed(text)
}

// ── vec_store / vec_search ─────────────────────────────────────────

/// Метаданные размерности по таблицам: vec_meta(table, dim). vec0
/// фиксирует dim в DDL и не умеет его менять — рассогласование должно
/// падать ГРОМКО с именами чисел, а не криптичной ошибкой vec0
/// (требование наряда: защита от смешения векторов разных моделей).
const META_DDL: &str =
    "CREATE TABLE IF NOT EXISTS vec_meta (table_name TEXT PRIMARY KEY, dim INTEGER NOT NULL);";

/// №281: биндинг таблицы к scope (контейнер-изоляция, containerTag-аналог
/// supermemory). Отдельная таблица — не расширение vec_meta (совместимость
/// с существующими бд без миграции).
const SCOPE_DDL: &str =
    "CREATE TABLE IF NOT EXISTS vec_scopes (table_name TEXT PRIMARY KEY, scope TEXT NOT NULL);";

pub(crate) fn open_vec_db(
    builtin: &str,
    db_path: &str,
    mode: SandboxMode,
) -> Result<rusqlite::Connection, String> {
    let safe_path = sandbox_path_ex(db_path, mode).map_err(|e| format!("{builtin}(): {e}"))?;
    register_vec_extension();
    let conn = rusqlite::Connection::open(&safe_path)
        .map_err(|e| format!("{builtin}(): cannot open vector db '{}': {}", db_path, e))?;
    conn.execute_batch(META_DDL)
        .map_err(|e| format!("{builtin}(): cannot init vec_meta: {}", e))?;
    Ok(conn)
}

pub(crate) fn table_dim(conn: &rusqlite::Connection, table: &str) -> Result<Option<i64>, String> {
    let mut stmt = conn
        .prepare("SELECT dim FROM vec_meta WHERE table_name = ?1")
        .map_err(|e| format!("vec_meta query: {}", e))?;
    let mut rows = stmt
        .query([table])
        .map_err(|e| format!("vec_meta query: {}", e))?;
    if let Some(row) = rows.next().map_err(|e| format!("vec_meta query: {}", e))? {
        Ok(Some(row.get(0).map_err(|e| format!("vec_meta: {}", e))?))
    } else {
        Ok(None)
    }
}

fn set_table_dim(conn: &rusqlite::Connection, table: &str, dim: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO vec_meta (table_name, dim) VALUES (?1, ?2)",
        [table, &dim.to_string()],
    )
    .map_err(|e| format!("vec_meta insert: {}", e))?;
    Ok(())
}

/// `vec_store(db_path, table, id, embedding[, payload]) -> Struct{stored, table, id, dim, rowid}`
/// — сохранить вектор в vec0-таблицу. Таблица создаётся при первом
/// сохранении (dim фиксируется по первому вектору); далее dim
/// проверяется — рассогласование = громкая ошибка. `id` — строка
/// вызывающего (не обязана быть уникальной, вернётся как есть в
/// vec_search); хранится в aux-колонке vec0 (join-free KNN).
///
/// Наряд №281 — опциональный пятый аргумент `payload`:
///   • String — текст документа для FTS5-плеча hybrid-поиска
///     (shadow-таблица `{table}__fts`, id + text; перезапись по id);
///   • Struct `{text?, scope?}` — то же + привязка таблицы к scope
///     (контейнер-изоляция, containerTag-аналог supermemory): биндинг
///     на первой записи, смена scope у существующей таблицы — громко;
///   • неизвестные поля opts — громкая ошибка (fail-closed).
pub(crate) fn builtin_vec_store(args: &[Value]) -> Result<Value, String> {
    if args.len() != 4 && args.len() != 5 {
        return Err(format!(
            "vec_store() requires 4 or 5 arguments (db_path, table, id, embedding[, payload]), got {}",
            args.len()
        ));
    }
    let db_path = value_as_string("vec_store", 1, &args[0], "db_path")?;
    let table = value_as_string("vec_store", 2, &args[1], "table")?;
    let id = value_as_string("vec_store", 3, &args[2], "id")?;
    let embedding = value_as_embedding("vec_store", 4, &args[3])?;
    // №281: разбор payload (String = text; Struct = {text?, scope?}).
    let mut store_text: Option<String> = None;
    let mut store_scope: Option<String> = None;
    if args.len() == 5 {
        match &args[4] {
            Value::String(text) => store_text = Some(text.clone()),
            Value::Struct { fields, .. } => {
                for (k, v) in fields {
                    match (k.as_str(), v) {
                        ("text", Value::String(t)) => store_text = Some(t.clone()),
                        ("scope", Value::String(sc)) => store_scope = Some(sc.clone()),
                        ("text", other) => {
                            return Err(format!(
                                "vec_store(): opts.text must be a String, got {}",
                                other.type_name()
                            ))
                        }
                        ("scope", other) => {
                            return Err(format!(
                                "vec_store(): opts.scope must be a String, got {}",
                                other.type_name()
                            ))
                        }
                        _ => {
                            return Err(format!(
                                "vec_store(): unknown opts field '{k}' — known: text, scope"
                            ))
                        }
                    }
                }
            }
            other => {
                return Err(format!(
                    "vec_store(): argument 5 (payload) must be a String (text) or Struct {{text, scope}}, got {}",
                    other.type_name()
                ))
            }
        }
    }
    if embedding.is_empty() {
        return Err(
            "vec_store(): embedding must not be empty (dim 0 vectors are not storable)".to_string(),
        );
    }
    validate_table_name("vec_store", &table)?;

    let conn = open_vec_db("vec_store", &db_path, SandboxMode::ForWrite)?;
    let dim = embedding.len() as i64;

    // Размерность: первая запись фиксирует, дальнейшие сверяются.
    match table_dim(&conn, &table)? {
        None => set_table_dim(&conn, &table, dim)?,
        Some(existing) if existing != dim => {
            return Err(format!(
                "vec_store(): dimension mismatch for table '{}': created with dim {}, got {} — vectors of different models/dimensions must not be mixed",
                table, existing, dim
            ));
        }
        Some(_) => {}
    }

    // vec0-таблица: metadata-колонка id (хранится в shadow-таблице vec0,
    // возвращается KNN-запросом, фильтруема WHERE — фундамент Phase 4).
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS \"{table}\" USING vec0(embedding float[{dim}], id text);"
    ))
    .map_err(|e| format!("vec_store(): cannot create vec0 table '{}': {}", table, e))?;

    let rowid: i64 = conn
        .query_row(
            &format!("SELECT COALESCE(MAX(rowid), 0) + 1 FROM \"{table}\""),
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("vec_store(): rowid allocation: {}", e))?;

    let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        &format!("INSERT INTO \"{table}\"(rowid, embedding, id) VALUES (?1, ?2, ?3)"),
        rusqlite::params![rowid, blob, id],
    )
    .map_err(|e| format!("vec_store(): insert: {}", e))?;

    // №281: биндинг scope (контейнер-изоляция). Первая запись фиксирует,
    // смена у существующей таблицы — громко (жёсткая граница, не тихая).
    if let Some(scope) = &store_scope {
        bind_table_scope("vec_store", &conn, &table, scope)?;
    }
    // №281: FTS5-плечо — текст документа в shadow-таблицу (перезапись
    // по id: текст последней записи id выигрывает, честно задокументировано).
    if let Some(text) = &store_text {
        upsert_fts_text("vec_store", &conn, &table, &id, text)?;
    }

    Ok(make_struct(
        "VecStoreResult",
        &[
            ("stored", Value::Float(1.0)),
            ("table", Value::String(table)),
            ("id", Value::String(id)),
            ("dim", Value::Float(dim as f64)),
            ("rowid", Value::Float(rowid as f64)),
        ],
    ))
}

/// №281: привязка таблицы к scope. Таблица без биндинга привязывается;
/// повторный биндинг тем же scope — no-op; другим — громко (жёсткая
/// граница контейнер-изоляции, «не тихая выдача»).
fn bind_table_scope(
    builtin: &str,
    conn: &rusqlite::Connection,
    table: &str,
    scope: &str,
) -> Result<(), String> {
    conn.execute_batch(SCOPE_DDL)
        .map_err(|e| format!("{builtin}(): cannot init vec_scopes: {e}"))?;
    let existing: Option<String> = conn
        .query_row(
            "SELECT scope FROM vec_scopes WHERE table_name = ?1",
            [table],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(format!("{builtin}(): vec_scopes query: {other}")),
        })?;
    match existing {
        Some(bound) if bound != scope => Err(format!(
            "{builtin}(): table '{table}' is bound to scope '{bound}', refusing to rebind to '{scope}' — scope is a hard container boundary"
        )),
        Some(_) => Ok(()),
        None => {
            conn.execute(
                "INSERT INTO vec_scopes (table_name, scope) VALUES (?1, ?2)",
                [table, scope],
            )
            .map_err(|e| format!("{builtin}(): vec_scopes insert: {e}"))?;
            Ok(())
        }
    }
}

/// №281: текст в FTS5 shadow-таблицу. id UNINDEXED — join-ключ к vec0.
fn upsert_fts_text(
    builtin: &str,
    conn: &rusqlite::Connection,
    table: &str,
    id: &str,
    text: &str,
) -> Result<(), String> {
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS \"{table}__fts\" USING fts5(id UNINDEXED, text);"
    ))
    .map_err(|e| format!("{builtin}(): cannot create fts index for '{table}': {e}"))?;
    conn.execute(&format!("DELETE FROM \"{table}__fts\" WHERE id = ?1"), [id])
        .map_err(|e| format!("{builtin}(): fts delete by id: {e}"))?;
    conn.execute(
        &format!("INSERT INTO \"{table}__fts\"(id, text) VALUES (?1, ?2)"),
        [id, text],
    )
    .map_err(|e| format!("{builtin}(): fts insert: {e}"))?;
    Ok(())
}

/// №281: проверка scope при поиске/профиле. Явный scope + таблица без
/// биндинга → громко (fail-closed); другой scope → громко (cross-scope
/// доступ запрещён контрактом — «громкая ошибка, не тихая выдача»).
pub(crate) fn check_table_scope(
    builtin: &str,
    conn: &rusqlite::Connection,
    table: &str,
    scope: &str,
) -> Result<(), String> {
    conn.execute_batch(SCOPE_DDL)
        .map_err(|e| format!("{builtin}(): cannot init vec_scopes: {e}"))?;
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_scopes'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("{builtin}(): sqlite_master: {e}"))?;
    if exists == 0 {
        return Err(format!(
            "{builtin}(): scope '{scope}' requested but table '{table}' is not scope-bound — store with vec_store(..., {{scope: \"{scope}\"}}) first"
        ));
    }
    let bound: Option<String> = conn
        .query_row(
            "SELECT scope FROM vec_scopes WHERE table_name = ?1",
            [table],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(format!("{builtin}(): vec_scopes query: {other}")),
        })?;
    match bound {
        None => Err(format!(
            "{builtin}(): scope '{scope}' requested but table '{table}' is not scope-bound — store with vec_store(..., {{scope: \"{scope}\"}}) first"
        )),
        Some(b) if b != scope => Err(format!(
            "{builtin}(): [SCOPE_VIOLATION] cross-scope access refused: table '{table}' belongs to scope '{b}', requested '{scope}'"
        )),
        Some(_) => Ok(()),
    }
}

/// `vec_search(db_path, table, query_embedding, k[, opts]) -> List[Struct{id, distance, score}]`
/// — KNN по vec0-таблице (distance_metric=cosine), ближайший первым.
/// Пустая существующая таблица → пустой List. Отсутствующая таблица /
/// рассогласование размерности → громкая ошибка. k <= 0 и
/// нецелое k — ошибки; k > 10 000 отклоняется (DoS-граница).
///
/// Опциональный пятый аргумент — два совместимых вида (тип-дискриминация):
///   • Bool — include_forgotten (наряд №280, back-compat): пост-фильтр
///     id из forget-ledger (memory_forget); дефолт false.
///   • Struct (наряд №281) `{include_forgotten?, mode?, scope?, query_text?}`:
///     mode — `"semantic"` (дефолт, чистое KNN) | `"fts"` (BM25 по
///     FTS5-shadow `{table}__fts`) | `"hybrid"` (RRF-слияние k=60 обоих
///     плеч — реюз формулы memory_store, ADR-0094/0075); scope —
///     контейнер-изоляция (cross-scope → громко, fail-closed на
///     незабинженной таблице); query_text — текстовый запрос для
///     fts/hybrid (без него — громко). Неизвестные поля opts — громко.
///
/// Форма хита: `{id, distance, score}` — score ∈ [0,1] нормализованная
/// релевантность режима (semantic: 1−distance; fts: max-нормализованный
/// bm25; hybrid: max-нормализованный RRF); в semantic distance — истинная
/// cosine-дистанция, в fts/hybrid distance = 1 − score (НЕ физическая
/// дистанция — честно задокументировано). k — размер выборки каждого
/// плеча и/или KNN ДО пост-фильтра забытых.
pub(crate) fn builtin_vec_search(args: &[Value]) -> Result<Value, String> {
    if args.len() < 4 || args.len() > 5 {
        return Err(format!(
            "vec_search() requires 4 or 5 arguments (db_path, table, query_embedding, k[, include_forgotten | opts]), got {}",
            args.len()
        ));
    }
    let db_path = value_as_string("vec_search", 1, &args[0], "db_path")?;
    let table = value_as_string("vec_search", 2, &args[1], "table")?;
    let query = value_as_embedding("vec_search", 3, &args[2])?;
    let k = match &args[3] {
        Value::Float(f) => *f,
        other => {
            return Err(format!(
                "vec_search(): argument 4 (k) must be a Float, got {}",
                type_name(other)
            ))
        }
    };
    if !k.is_finite() || k <= 0.0 || k.fract() != 0.0 {
        return Err(format!(
            "vec_search(): k must be a positive integer, got {k}"
        ));
    }
    let k = k as i64;
    if k > 10_000 {
        return Err(format!(
            "vec_search(): k = {k} exceeds the limit 10000 (DoS guard; narrow the query instead)"
        ));
    }
    // №280/№281: пятый аргумент — Bool (include_forgotten, №280) или
    // Struct opts (№281). Тип-дискриминация, оба вида совместимы.
    let mut include_forgotten = false;
    let mut mode: String = "semantic".to_string();
    let mut scope: Option<String> = None;
    let mut query_text: Option<String> = None;
    if args.len() == 5 {
        match &args[4] {
            Value::Bool(b) => include_forgotten = *b,
            Value::Struct { fields, .. } => {
                for (key, v) in fields {
                    match (key.as_str(), v) {
                        ("include_forgotten", Value::Bool(b)) => include_forgotten = *b,
                        ("mode", Value::String(m)) => {
                            if !matches!(m.as_str(), "semantic" | "fts" | "hybrid") {
                                return Err(format!(
                                    "vec_search(): unknown mode '{m}' — expected \"semantic\"|\"fts\"|\"hybrid\""
                                ));
                            }
                            mode = m.clone();
                        }
                        ("scope", Value::String(sc)) => scope = Some(sc.clone()),
                        ("query_text", Value::String(t)) => query_text = Some(t.clone()),
                        ("include_forgotten", other) => {
                            return Err(format!(
                                "vec_search(): opts.include_forgotten must be a Bool, got {}",
                                other.type_name()
                            ))
                        }
                        ("mode", other) => {
                            return Err(format!(
                                "vec_search(): opts.mode must be a String, got {}",
                                other.type_name()
                            ))
                        }
                        ("scope", other) => {
                            return Err(format!(
                                "vec_search(): opts.scope must be a String, got {}",
                                other.type_name()
                            ))
                        }
                        ("query_text", other) => {
                            return Err(format!(
                                "vec_search(): opts.query_text must be a String, got {}",
                                other.type_name()
                            ))
                        }
                        _ => {
                            return Err(format!(
                                "vec_search(): unknown opts field '{key}' — known: include_forgotten, mode, scope, query_text"
                            ))
                        }
                    }
                }
            }
            other => {
                return Err(format!(
                    "vec_search(): argument 5 must be a Bool (include_forgotten, №280) or Struct opts (№281), got {}",
                    other.type_name()
                ))
            }
        }
    }
    if matches!(mode.as_str(), "fts" | "hybrid") && query_text.is_none() {
        return Err(format!(
            "vec_search(): mode \"{mode}\" requires opts.query_text (the textual query for the FTS5 arm)"
        ));
    }
    validate_table_name("vec_search", &table)?;

    let conn = open_vec_db("vec_search", &db_path, SandboxMode::ForRead)?;

    // №281: контейнер-изоляция — явный scope сверяется с биндингом
    // таблицы ДО любых чтений (fail-closed: незабинженная таблица тоже громко).
    if let Some(scope) = &scope {
        check_table_scope("vec_search", &conn, &table, scope)?;
    }

    // Таблица существует?
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [&table],
            |r| r.get(0),
        )
        .map_err(|e| format!("vec_search(): sqlite_master: {}", e))?;
    if exists == 0 {
        return Err(format!(
            "vec_search(): table '{}' not found in vector db '{}'",
            table, db_path
        ));
    }
    // Размерность запроса = размерности таблицы (громко).
    if let Some(existing) = table_dim(&conn, &table)? {
        if existing != query.len() as i64 {
            return Err(format!(
                "vec_search(): dimension mismatch for table '{}': stored dim {}, query dim {} — vectors of different models/dimensions must not be mixed",
                table,
                existing,
                query.len()
            ));
        }
    }

    // №281: диспетч режимов. semantic — прежний путь; fts/hybrid —
    // BM25-плечо и RRF-слияние (формула memory_store, k=60).
    let mut out: Vec<Value> = match mode.as_str() {
        "semantic" => {
            let hits = knn_arm(&conn, &table, &query, k)?;
            hits.into_iter()
                .map(|(id, distance)| {
                    make_struct(
                        "VecSearchHit",
                        &[
                            ("id", Value::String(id)),
                            ("distance", Value::Float(distance)),
                            ("score", Value::Float(1.0 - distance)),
                        ],
                    )
                })
                .collect()
        }
        "fts" => {
            let text = query_text.as_deref().unwrap_or_default();
            let hits = fts_arm(&conn, &table, text, k)?;
            let max_raw = hits.iter().map(|(_, r)| *r).fold(f64::MIN, f64::max);
            hits.into_iter()
                .map(|(id, raw)| {
                    let score = if max_raw > 0.0 { raw / max_raw } else { 0.0 };
                    make_struct(
                        "VecSearchHit",
                        &[
                            ("id", Value::String(id)),
                            ("distance", Value::Float(1.0 - score)),
                            ("score", Value::Float(score)),
                        ],
                    )
                })
                .collect()
        }
        _ => {
            // hybrid: RRF-слияние векторного и FTS5-плеч (реюз формулы
            // memory_store ADR-0094/0075: score = Σ 1/(RRF_K + rank_i)).
            const RRF_K: f64 = 60.0;
            let text = query_text.as_deref().unwrap_or_default();
            let vec_hits = knn_arm(&conn, &table, &query, k)?;
            let fts_hits = fts_arm(&conn, &table, text, k)?;
            // Ранги: векторное плечо уже отсортировано по distance asc;
            // fts-плечо сортируем по raw bm25 desc (большее = релевантнее).
            let mut rrf: std::collections::HashMap<String, (f64, Option<f64>)> =
                std::collections::HashMap::new();
            for (rank, (id, distance)) in vec_hits.iter().enumerate() {
                let e = rrf.entry(id.clone()).or_insert((0.0, None));
                e.0 += 1.0 / (RRF_K + rank as f64 + 1.0);
                e.1 = Some(*distance);
            }
            let mut fts_sorted = fts_hits;
            fts_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (rank, (id, _raw)) in fts_sorted.iter().enumerate() {
                let e = rrf.entry(id.clone()).or_insert((0.0, None));
                e.0 += 1.0 / (RRF_K + rank as f64 + 1.0);
            }
            let mut merged: Vec<(String, f64, Option<f64>)> =
                rrf.into_iter().map(|(id, (s, d))| (id, s, d)).collect();
            merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            merged.truncate(k as usize);
            let max_rrf = merged.first().map(|(_, s, _)| *s).unwrap_or(0.0);
            merged
                .into_iter()
                .map(|(id, rrf_score, vec_distance)| {
                    let score = if max_rrf > 0.0 {
                        rrf_score / max_rrf
                    } else {
                        0.0
                    };
                    // distance: истинная векторная, если строка была в
                    // векторном плече; иначе 1 − score (fts-only hit).
                    let distance = vec_distance.unwrap_or(1.0 - score);
                    make_struct(
                        "VecSearchHit",
                        &[
                            ("id", Value::String(id)),
                            ("distance", Value::Float(distance)),
                            ("score", Value::Float(score)),
                        ],
                    )
                })
                .collect()
        }
    };
    // Наряд №280: пост-фильтр забытых id (soft-delete ledger от
    // memory_forget) — во ВСЕХ режимах. k — размер выборки плеча/KNN
    // ДО фильтра; после фильтра результат может быть меньше k —
    // честно задокументировано.
    if !include_forgotten {
        let forgotten = forgotten_ids(&conn, &table)?;
        if !forgotten.is_empty() {
            out.retain(|hit| match hit {
                Value::Struct { fields, .. } => match fields.get("id") {
                    Some(Value::String(id)) => !forgotten.contains(id),
                    _ => true,
                },
                _ => true,
            });
        }
    }
    Ok(Value::List(out))
}

/// Векторное плечо: KNN (vec0), строки упорядочены по distance asc
/// (ближайший первым — инвариант спайка №271, не пересортировывается).
fn knn_arm(
    conn: &rusqlite::Connection,
    table: &str,
    query: &[f32],
    k: i64,
) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, distance FROM \"{table}\" WHERE embedding MATCH ?1 AND k = ?2"
        ))
        .map_err(|e| format!("vec_search(): prepare KNN: {e}"))?;
    let blob: Vec<u8> = query.iter().flat_map(|f| f.to_le_bytes()).collect();
    let rows = stmt
        .query_map(rusqlite::params![blob, k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("vec_search(): KNN query: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        let (id, distance) = row.map_err(|e| format!("vec_search(): row: {e}"))?;
        out.push((id, distance));
    }
    Ok(out)
}

/// FTS5-плечо: bm25-ранжирование по shadow-таблице {table}__fts.
/// Возвращает (id, raw_rank), где raw = −rank (БОЛЬШЕЕ = релевантнее —
/// fts5 rank меньше-лучше, паттерн memory_store). Отсутствующий индекс —
/// громкая ошибка (fts/hybrid без текстов — программная ошибка).
fn fts_arm(
    conn: &rusqlite::Connection,
    table: &str,
    query_text: &str,
    k: i64,
) -> Result<Vec<(String, f64)>, String> {
    let fts_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            [format!("{table}__fts")],
            |r| r.get(0),
        )
        .map_err(|e| format!("vec_search(): sqlite_master: {e}"))?;
    if fts_exists == 0 {
        return Err(format!(
            "vec_search(): no FTS5 index for table '{table}' — store documents with vec_store(..., \"text\") first (fts/hybrid need a text arm)"
        ));
    }
    if query_text.trim().is_empty() {
        return Err("vec_search(): query_text must not be empty for fts/hybrid mode".to_string());
    }
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, rank FROM \"{table}__fts\" WHERE \"{table}__fts\" MATCH ?1 ORDER BY rank LIMIT ?2"
        ))
        .map_err(|e| format!("vec_search(): prepare FTS: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![query_text, k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("vec_search(): FTS query: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        let (id, rank) = row.map_err(|e| format!("vec_search(): fts row: {e}"))?;
        out.push((id, -rank)); // больше = релевантнее
    }
    Ok(out)
}

// ── Forget-ledger (наряд №280) ───────────────────────────────────
// Soft-delete: физического удаления нет; стёртые id живут в
// shadow-таблице {table}__forgotten рядом с vec0-таблицей. Ledger —
// журнал операции: batch_id каждой операции забывания виден в строках.

/// DDL forget-ledger. Имя таблицы уже прошло validate_table_name
/// (белый список идентификаторов), суффикс безопасен.
pub(crate) fn forget_ledger_ddl(table: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS \"{table}__forgotten\" (\
             id TEXT NOT NULL, \
             batch_id TEXT NOT NULL, \
             reason TEXT NOT NULL, \
             forgotten_at TEXT NOT NULL);\
         CREATE INDEX IF NOT EXISTS \"{table}__forgotten_id_idx\" \
             ON \"{table}__forgotten\"(id);"
    )
}

/// Множество уже забытых id для таблицы. Отсутствующий ledger —
/// не ошибка (пустое множество): забывать ещё нечего.
pub(crate) fn forgotten_ids(
    conn: &rusqlite::Connection,
    table: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [format!("{table}__forgotten")],
            |r| r.get(0),
        )
        .map_err(|e| format!("sqlite_master: {e}"))?;
    if exists == 0 {
        return Ok(std::collections::HashSet::new());
    }
    let mut stmt = conn
        .prepare(&format!("SELECT id FROM \"{table}__forgotten\""))
        .map_err(|e| format!("ledger query: {e}"))?;
    let mut out = std::collections::HashSet::new();
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| format!("ledger query: {e}"))?;
    for row in rows {
        out.insert(row.map_err(|e| format!("ledger row: {e}"))?);
    }
    Ok(out)
}
