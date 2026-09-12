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
fn validate_table_name(builtin: &str, table: &str) -> Result<(), String> {
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

fn value_as_embedding(builtin: &str, pos: usize, v: &Value) -> Result<Vec<f32>, String> {
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

// ── vec_store / vec_search ─────────────────────────────────────────

/// Метаданные размерности по таблицам: vec_meta(table, dim). vec0
/// фиксирует dim в DDL и не умеет его менять — рассогласование должно
/// падать ГРОМКО с именами чисел, а не криптичной ошибкой vec0
/// (требование наряда: защита от смешения векторов разных моделей).
const META_DDL: &str =
    "CREATE TABLE IF NOT EXISTS vec_meta (table_name TEXT PRIMARY KEY, dim INTEGER NOT NULL);";

fn open_vec_db(
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

fn table_dim(conn: &rusqlite::Connection, table: &str) -> Result<Option<i64>, String> {
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

/// `vec_store(db_path, table, id, embedding) -> Struct{stored, table, id, dim, rowid}`
/// — сохранить вектор в vec0-таблицу. Таблица создаётся при первом
/// сохранении (dim фиксируется по первому вектору); далее dim
/// проверяется — рассогласование = громкая ошибка. `id` — строка
/// вызывающего (не обязана быть уникальной, вернётся как есть в
/// vec_search); хранится в aux-колонке vec0 (join-free KNN).
pub(crate) fn builtin_vec_store(args: &[Value]) -> Result<Value, String> {
    if args.len() != 4 {
        return Err(format!(
            "vec_store() requires exactly 4 arguments (db_path, table, id, embedding), got {}",
            args.len()
        ));
    }
    let db_path = value_as_string("vec_store", 1, &args[0], "db_path")?;
    let table = value_as_string("vec_store", 2, &args[1], "table")?;
    let id = value_as_string("vec_store", 3, &args[2], "id")?;
    let embedding = value_as_embedding("vec_store", 4, &args[3])?;
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

/// `vec_search(db_path, table, query_embedding, k) -> List[Struct{id, distance}]`
/// — KNN по vec0-таблице (distance_metric=cosine), ближайший первым.
/// Пустая существующая таблица → пустой List. Отсутствующая таблица /
/// рассогласование размерности → громкая ошибка. k <= 0 и
/// нецелое k — ошибки; k > 10 000 отклоняется (DoS-граница).
pub(crate) fn builtin_vec_search(args: &[Value]) -> Result<Value, String> {
    if args.len() != 4 {
        return Err(format!(
            "vec_search() requires exactly 4 arguments (db_path, table, query_embedding, k), got {}",
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
    validate_table_name("vec_search", &table)?;

    let conn = open_vec_db("vec_search", &db_path, SandboxMode::ForRead)?;

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

    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, distance FROM \"{table}\" WHERE embedding MATCH ?1 AND k = ?2"
        ))
        .map_err(|e| format!("vec_search(): prepare KNN: {}", e))?;
    let blob: Vec<u8> = query.iter().flat_map(|f| f.to_le_bytes()).collect();
    let rows = stmt
        .query_map(rusqlite::params![blob, k], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("vec_search(): KNN query: {}", e))?;

    // vec0 KNN возвращает строки, упорядоченные по distance (ближайший
    // первым) — инвариант спайка №271; здесь он не пересортировывается.
    let mut out = Vec::new();
    for row in rows {
        let (id, distance) = row.map_err(|e| format!("vec_search(): row: {}", e))?;
        out.push(make_struct(
            "VecSearchHit",
            &[
                ("id", Value::String(id)),
                ("distance", Value::Float(distance)),
            ],
        ));
    }
    Ok(Value::List(out))
}
