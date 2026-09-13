// ── Наряд №280 (P2, M2): memory_forget — управляемое забывание с границами ──
//
// Эталон дисциплины — supermemory forget-matching: сухой прогон →
// связанный список id → apply строго по ids → forgetBatchId на каждом
// стёртом. Урок №263 (карты состояния без границ) переносится напрямую:
// забывание без границ опаснее карт без границ.
//
//   memory_forget(db_path, table, query, threshold, max_forget[, dry_run[, ids]])
//       -> Struct{candidates: List[Struct{id, score}], applied, batch_id}
//
// Контракт (issue #329):
//   • Tier 1 поверх vec_search (№272): query — эмбеддинг (List[Float]),
//     кандидаты = ближайшие соседи с similarity ≥ threshold.
//   • dry_run=true (ДЕФОЛЬТ, arity 5) — ТОЛЬКО превью: состояние не
//     меняется (ни строк, ни ledger), applied=0, batch_id="".
//   • apply — СТРОГО по явному списку id из превью (arity 7,
//     dry_run=false + ids), никогда по переисканному запросу. Каждый
//     id проверяется ТОЧЕЧНО против границ превью: существует в
//     таблице + similarity(query, id) ≥ threshold — bound deletes.
//     id вне границ / несуществующий → ГРОМКАЯ ошибка ДО применения
//     (атомарность: сначала проверяются все ids, потом пишется ledger).
//   • Границы: threshold отсекает далёких кандидатов, max_forget
//     капает поражённую область (и в превью, и в apply).
//   • Soft-delete: физического удаления НЕТ; стёртые id попадают в
//     forget-ledger {table}__forgotten (id, batch_id, reason,
//     forgotten_at) — batch_id присвоен каждой стёртой записи и
//     остаётся в журнале операции (audit-след в духе №276).
//     Физический vacuum — отдельная операция владельца, не builtin.
//   • vec_search по умолчанию не возвращает забытых (include_forgotten
//     = false, дефолт) — см. vector.rs.
//   • Повторный forget того же id — no-op (не ошибка, applied=0):
//     ledger идемпотентен по id.
//
// Taint-контур (инвариант, тесты): забывание не снимает taint с
// записей, попавших в логи — memory_forget оперирует ТОЛЬКО vec0-
// таблицей и ledger; canary-маркеры (№284), taint-трекер (№274 redact)
// и журналы LLM не трогаются. Секретные записи не попадают в память
// вовсе (маскирование до памяти) — forget не обязан их «стирать».
//
// Песочница: db_path через sandbox_path_ex (№131/№252) — ForRead для
// превью, ForWrite для apply (ledger создаётся). Имя таблицы — белый
// список идентификаторов (SQL-инъекция через интерполяцию DDL
// исключена, лекало vec_store/vec_search).
//
// Auto-забывание (TTL/вытеснение updates-фактом) — v2, ВНЕ скоупа
// №280 (отдельный чекбокс issue #329, громко задекларировано в PR).

use super::canary::base32_encode_16;
use super::vector::{
    forget_ledger_ddl, forgotten_ids, open_vec_db, table_dim, validate_table_name,
    value_as_embedding,
};
use super::Value;
use crate::builtins::io::SandboxMode;

/// Префикс batch_id: MLOG-FORGET-<base32×26> — 128 бит энтропии
/// (rand 0.10, crypto-нонсы; формат-брат canary-маркера №284).
pub const FORGET_BATCH_PREFIX: &str = "MLOG-FORGET-";

/// Причина записи в ledger (константа операции №280).
const FORGET_REASON: &str = "memory_forget";

/// Локальный хелпер построения Struct-значений (слайсовая форма,
/// лекало vector.rs; НЕ глобальный core::make_struct — не светит имя
/// в общий glob-неймспейс builtins, коллизий нет). Модуль canary (№284)
/// строит литерально, здесь — тонкая обёртка ради компактности.
fn struct_value(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Struct {
        type_name: type_name.to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    }
}

/// Строковый аргумент (локальная форма vector::value_as_string — имя
/// не выносится в glob-неймспейс).
fn string_arg(builtin: &str, pos: usize, v: &Value, what: &str) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{builtin}(): argument {pos} ({what}) must be a String, got {}",
            other.type_name()
        )),
    }
}

/// DoS-граница KNN-скана — та же, что у vec_search (k ≤ 10 000).
const MAX_KNN_K: i64 = 10_000;

/// batch_id для apply-фазы: MLOG-FORGET-<base32×26>, 16 случайных байт.
fn new_batch_id() -> String {
    use rand::Rng as _;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    format!("{}{}", FORGET_BATCH_PREFIX, base32_encode_16(&bytes))
}

/// cosine similarity двух векторов (f32-компоненты, f64-аккумуляция).
/// ЕДИНАЯ точка вычисления similarity для превью И для bound-проверки
/// apply — детерминизм: кандидат превью не может «выпасть» на apply из-за
/// расхождения f32-дистанции vec0 и локального счёта (дистанция vec0
/// используется только для упорядочивания KNN, НЕ для порога).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    debug_assert_eq!(a.len(), b.len(), "dim checked by caller");
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let (x, y) = (*x as f64, *y as f64);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0; // нулевой вектор — сходство не определено, честный 0.0
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn embedding_from_blob(blob: &[u8]) -> Vec<f32> {
    let n = blob.len() / 4;
    (0..n)
        .map(|i| {
            f32::from_le_bytes([
                blob[i * 4],
                blob[i * 4 + 1],
                blob[i * 4 + 2],
                blob[i * 4 + 3],
            ])
        })
        .collect()
}

/// Валидация threshold: конечный Float в [0, 1]. similarity ∈ [-1, 1];
/// порог выше 1 отсекал бы ВСЁ (программная ошибка), ниже 0 — не граница.
fn threshold_as_f64(builtin: &str, v: &Value) -> Result<f64, String> {
    match v {
        Value::Float(f) if f.is_finite() && (0.0..=1.0).contains(f) => Ok(*f),
        other => Err(format!(
            "{builtin}(): argument 4 (threshold) must be a Float in [0, 1], got {}",
            other.type_name()
        )),
    }
}

/// Валидация max_forget: положительное целое Float в [1, 10 000]
/// (DoS-граница как у k в vec_search).
fn max_forget_as_i64(builtin: &str, v: &Value) -> Result<i64, String> {
    let f = match v {
        Value::Float(f) if f.is_finite() => *f,
        other => {
            return Err(format!(
                "{builtin}(): argument 5 (max_forget) must be a positive integer Float, got {}",
                other.type_name()
            ))
        }
    };
    if f < 1.0 || f.fract() != 0.0 {
        return Err(format!(
            "{builtin}(): max_forget must be a positive integer, got {f}"
        ));
    }
    let k = f as i64;
    if k > MAX_KNN_K {
        return Err(format!(
            "{builtin}(): max_forget = {k} exceeds the limit {MAX_KNN_K} (DoS guard)"
        ));
    }
    Ok(k)
}

/// Кандидат забывания: id + similarity (лучшая среди строк id).
struct Candidate {
    id: String,
    score: f64,
}

/// KNN-скан с ростом k: вернуть до max_forget УНИКАЛЬНЫХ id
/// (лучшая similarity на id), similarity ≥ threshold, ещё не забытых.
/// Рост k (удвоение до 10 000) нужен, когда у одного id несколько строк —
/// KNN-окно должно вместить достаточно уникальных id. Останов: окно
/// исчерпано (вернулось < k), порог отсёк хвост (монотонность дистанции)
/// или k достиг потолка.
fn knn_candidates(
    builtin: &str,
    conn: &rusqlite::Connection,
    table: &str,
    query: &[f32],
    threshold: f64,
    max_forget: i64,
    already_forgotten: &std::collections::HashSet<String>,
) -> Result<Vec<Candidate>, String> {
    let mut k = max_forget.max(1);
    loop {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, embedding FROM \"{table}\" WHERE embedding MATCH ?1 AND k = ?2"
            ))
            .map_err(|e| format!("{builtin}(): prepare KNN: {e}"))?;
        let blob: Vec<u8> = query.iter().flat_map(|f| f.to_le_bytes()).collect();
        let rows = stmt
            .query_map(rusqlite::params![blob, k], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|e| format!("{builtin}(): KNN query: {e}"))?;

        let mut candidates: Vec<Candidate> = Vec::new();
        let mut returned = 0i64;
        let mut tail_below_threshold = false;
        for row in rows {
            let (id, blob) = row.map_err(|e| format!("{builtin}(): KNN row: {e}"))?;
            returned += 1;
            let emb = embedding_from_blob(&blob);
            if emb.len() != query.len() {
                // vec0 фиксирует dim по DDL — рассогласование здесь
                // невозможно; защита от повреждённого файла, громко.
                return Err(format!(
                    "{builtin}(): stored embedding dim {} != table dim {} for table '{}'",
                    emb.len(),
                    query.len(),
                    table
                ));
            }
            let sim = cosine_similarity(query, &emb);
            if sim < threshold {
                // Строки упорядочены по distance (возрастание) —
                // следующие ещё дальше: хвост ниже порога, рост k бесполезен.
                tail_below_threshold = true;
                break;
            }
            if already_forgotten.contains(&id) {
                continue; // забытые id — не кандидаты (повторное превью честно)
            }
            if let Some(c) = candidates.iter_mut().find(|c| c.id == id) {
                if sim > c.score {
                    c.score = sim; // дедуп id: лучшая similarity
                }
            } else {
                candidates.push(Candidate { id, score: sim });
            }
        }
        let filled = candidates.len() as i64 >= max_forget;
        if filled || tail_below_threshold || returned < k || k >= MAX_KNN_K {
            candidates.truncate(max_forget as usize);
            return Ok(candidates);
        }
        k = (k * 2).min(MAX_KNN_K);
    }
}

/// Точечная bound-проверка apply: id существует в таблице и его ЛУЧШАЯ
/// similarity ≥ threshold. Это НЕ переисканный запрос — прямой
/// SELECT по id (индекс ledger/primary), ровно те же вычисления, что
/// в превью (cosine_similarity), потому граница превью воспроизводится
/// байт-в-байт. id уже забыт → Ok(None): повторный forget — no-op.
fn bound_check_one(
    builtin: &str,
    conn: &rusqlite::Connection,
    table: &str,
    id: &str,
    query: &[f32],
    threshold: f64,
) -> Result<Option<Candidate>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT embedding FROM \"{table}\" WHERE id = ?1"))
        .map_err(|e| format!("{builtin}(): prepare id lookup: {e}"))?;
    let rows = stmt
        .query_map([id], |r| r.get::<_, Vec<u8>>(0))
        .map_err(|e| format!("{builtin}(): id lookup: {e}"))?;
    let mut best: Option<f64> = None;
    let mut found = 0usize;
    for row in rows {
        let blob = row.map_err(|e| format!("{builtin}(): id row: {e}"))?;
        found += 1;
        let emb = embedding_from_blob(&blob);
        if emb.len() != query.len() {
            return Err(format!(
                "{builtin}(): stored embedding dim {} != table dim {} for table '{}'",
                emb.len(),
                query.len(),
                table
            ));
        }
        let sim = cosine_similarity(query, &emb);
        if best.is_none_or(|b| sim > b) {
            best = Some(sim);
        }
    }
    if found == 0 {
        return Err(format!(
            "{builtin}(): id '{id}' not found in table '{table}' — apply works only over ids from the dry_run preview (bound deletes)"
        ));
    }
    let sim = match best {
        Some(s) => s,
        None => {
            return Err(format!(
                "{builtin}(): id '{id}' has no stored embedding in table '{table}'"
            ))
        }
    };
    if sim < threshold {
        return Err(format!(
            "{builtin}(): id '{id}' is outside the preview bounds: similarity {sim:.4} < threshold {threshold:.4} — apply works only over ids from the dry_run preview (bound deletes)"
        ));
    }
    Ok(Some(Candidate {
        id: id.to_string(),
        score: sim,
    }))
}

/// `memory_forget(db_path, table, query, threshold, max_forget[, dry_run[, ids]])`
/// — см. шапку модуля. Ядро чистое ( ConnectionState не трогается,
/// execution.rs не требуется), builtin-обвязка ниже.
pub fn memory_forget_core(args: &[Value]) -> Result<Value, String> {
    const BUILTIN: &str = "memory_forget";
    if args.len() < 5 || args.len() > 7 {
        return Err(format!(
            "{BUILTIN}() requires 5, 6 or 7 arguments (db_path, table, query, threshold, max_forget[, dry_run[, ids]]), got {}",
            args.len()
        ));
    }
    let db_path = string_arg(BUILTIN, 1, &args[0], "db_path")?;
    let table = string_arg(BUILTIN, 2, &args[1], "table")?;
    let query = value_as_embedding(BUILTIN, 3, &args[2])?;
    if query.is_empty() {
        return Err(format!(
            "{BUILTIN}(): query embedding must not be empty (dim 0 vectors are not searchable)"
        ));
    }
    let threshold = threshold_as_f64(BUILTIN, &args[3])?;
    let max_forget = max_forget_as_i64(BUILTIN, &args[4])?;

    // dry_run: дефолт true (arity 5); 6-м аргументом — только Bool
    // (List на 6-й позиции = программист забыл dry_run — громко).
    let mut dry_run = true;
    if args.len() >= 6 {
        match &args[5] {
            Value::Bool(b) => dry_run = *b,
            Value::List(_) => {
                return Err(format!(
                    "{BUILTIN}(): argument 6 must be dry_run (Bool) — pass dry_run=false before the ids list"
                ))
            }
            other => {
                return Err(format!(
                    "{BUILTIN}(): argument 6 (dry_run) must be a Bool, got {}",
                    other.type_name()
                ))
            }
        }
    }
    // ids: только 7-м аргументом, только при dry_run=false. Apply без
    // явного списка — ГРОМКО: забывание по переисканному запросу
    // запрещено контрактом (bound deletes).
    let ids: Option<Vec<String>> = if args.len() == 7 {
        match &args[6] {
            Value::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    match item {
                        Value::String(s) => out.push(s.clone()),
                        other => {
                            return Err(format!(
                                "{BUILTIN}(): ids[{i}] must be a String, got {}",
                                other.type_name()
                            ))
                        }
                    }
                }
                Some(out)
            }
            other => {
                return Err(format!(
                    "{BUILTIN}(): argument 7 (ids) must be a List of String, got {}",
                    other.type_name()
                ))
            }
        }
    } else {
        None
    };
    if args.len() == 7 && dry_run {
        return Err(format!(
            "{BUILTIN}(): ids list is mutually exclusive with dry_run=true — pass dry_run=false to apply"
        ));
    }
    if !dry_run && ids.is_none() {
        // dry_run=false без ids — apply-запрос без списка: громко.
        return Err(format!(
            "{BUILTIN}(): apply requires an explicit ids list from the dry_run preview — call with dry_run=true first, then re-call with dry_run=false and the ids"
        ));
    }

    validate_table_name(BUILTIN, &table)?;
    if let Some(list) = &ids {
        if list.is_empty() {
            return Err(format!(
                "{BUILTIN}(): ids list must not be empty — an explicit apply of nothing is a programmer error"
            ));
        }
    }

    // Превью — ForRead (состояние не меняется), apply — ForWrite (ledger).
    let mode = if dry_run {
        SandboxMode::ForRead
    } else {
        SandboxMode::ForWrite
    };
    let conn = open_vec_db(BUILTIN, &db_path, mode)?;

    // Таблица существует?
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [&table],
            |r| r.get(0),
        )
        .map_err(|e| format!("{BUILTIN}(): sqlite_master: {e}"))?;
    if exists == 0 {
        return Err(format!(
            "{BUILTIN}(): table '{table}' not found in vector db '{db_path}'"
        ));
    }
    // Размерность запроса = размерности таблицы (громко, лекало vec_search).
    if let Some(existing) = table_dim(&conn, &table)? {
        if existing != query.len() as i64 {
            return Err(format!(
                "{BUILTIN}(): dimension mismatch for table '{table}': stored dim {existing}, query dim {} — vectors of different models/dimensions must not be mixed",
                query.len()
            ));
        }
    }

    let already_forgotten = forgotten_ids(&conn, &table)?;

    if dry_run {
        // Превью: только кандидаты, состояние не меняется.
        let candidates = knn_candidates(
            BUILTIN,
            &conn,
            &table,
            &query,
            threshold,
            max_forget,
            &already_forgotten,
        )?;
        let cand_values: Vec<Value> = candidates
            .iter()
            .map(|c| {
                struct_value(
                    "ForgetCandidate",
                    &[
                        ("id", Value::String(c.id.clone())),
                        ("score", Value::Float(c.score)),
                    ],
                )
            })
            .collect();
        return Ok(struct_value(
            "MemoryForgetResult",
            &[
                ("candidates", Value::List(cand_values)),
                ("applied", Value::Float(0.0)),
                ("batch_id", Value::String(String::new())),
            ],
        ));
    }

    // Apply: ids обязателен (проверено выше громкой ошибкой), каждый —
    // точечная bound-проверка ДО любых записей (атомарность apply).
    let ids = match ids {
        Some(list) => list,
        None => {
            return Err(format!(
                "{BUILTIN}(): apply requires an explicit ids list from the dry_run preview"
            ))
        }
    };
    let mut to_apply: Vec<Candidate> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        if !seen.insert(id.clone()) {
            continue; // дубликат в списке — не ошибка, применяется один раз
        }
        if already_forgotten.contains(id) {
            continue; // повторный forget — no-op (идемпотентность ledger)
        }
        if let Some(c) = bound_check_one(BUILTIN, &conn, &table, id, &query, threshold)? {
            to_apply.push(c);
        }
    }
    if to_apply.len() as i64 > max_forget {
        return Err(format!(
            "{BUILTIN}(): apply would forget {} ids, exceeding max_forget {max_forget} — narrow the ids list",
            to_apply.len()
        ));
    }

    // batch_id выдаётся ТОЛЬКО когда реально стёрт хотя бы один id:
    // пустой apply (все id уже забыты) — no-op без следа в журнале.
    let batch_id = if to_apply.is_empty() {
        String::new()
    } else {
        new_batch_id()
    };
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute_batch(&forget_ledger_ddl(&table))
        .map_err(|e| format!("{BUILTIN}(): cannot create forget ledger for '{table}': {e}"))?;
    {
        let mut stmt = conn
            .prepare(&format!(
                "INSERT INTO \"{table}__forgotten\" (id, batch_id, reason, forgotten_at) VALUES (?1, ?2, ?3, ?4)"
            ))
            .map_err(|e| format!("{BUILTIN}(): ledger insert: {e}"))?;
        for c in &to_apply {
            stmt.execute(rusqlite::params![c.id, batch_id, FORGET_REASON, now])
                .map_err(|e| format!("{BUILTIN}(): ledger insert '{}': {e}", c.id))?;
        }
    }

    let applied_values: Vec<Value> = to_apply
        .iter()
        .map(|c| {
            struct_value(
                "ForgetCandidate",
                &[
                    ("id", Value::String(c.id.clone())),
                    ("score", Value::Float(c.score)),
                ],
            )
        })
        .collect();
    Ok(struct_value(
        "MemoryForgetResult",
        &[
            ("candidates", Value::List(applied_values)),
            ("applied", Value::Float(to_apply.len() as f64)),
            ("batch_id", Value::String(batch_id)),
        ],
    ))
}

/// Builtin-обвязка (сигнатура вызова из реестра).
pub(crate) fn builtin_memory_forget(args: &[Value]) -> Result<Value, String> {
    memory_forget_core(args)
}
