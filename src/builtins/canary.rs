// ── Наряд №284 (P1, M1): canary-токены недоверенного текста ────────────
//
// canary_insert(text, opts?) -> Struct{marked_text, canary_id}
// canary_check(text, canary_id, opts?) -> Struct{leaked, id, position}
//
// Runtime-детектор утечки недоверенного контента через LLM-канал.
// Промышленный паттерн: protectai/rebuff (canaryTokens), Simon Willison
// (canary words), Microsoft Spotlighting (arXiv:2403.14720, datamarking).
// В Metalogos canary связан с taint-моделью языка, а не standalone:
//   - runtime: утечка → CANARY_LEAK warning (stderr, громко) + счётчик
//     llm_usage().canary_leaks;
//   - static: в then-ветке `if (r.leaked) {...}` проверенный ответ
//     помечается TaintKind::CanaryLeak («компрометированный канал»),
//     использование такой метки в sink → audit-warning CANARY_LEAK
//     (src/audit.rs, check_canary_leak — advisory-слой audit_program).
// Детектор, НЕ гейт: решение об остановке пайплайна — за автором.
//
// Контракты:
//   - формат маркера: `MLOG-CANARY-<base32>` — 128 бит энтропии
//     (16 случайных байта → 26 символов A-Z2-7, RFC 4648 без паддинга);
//   - canary_id — ПОЛНЫЙ маркер (prefix + id); проверяется строго;
//   - громкие ошибки: пустой text, повторная вставка (уже есть маркер),
//     zero-width-символы в text ДО вставки, count вне 1..=4, неизвестные
//     position/mode/поле opts, неизвестный (не соответствующий формату)
//     canary_id, не-Struct opts, не-String text/id;
//   - устойчивость детекции: регистр, разбиение пробелами/пунктуацией
//     (режим "exact" по умолчанию); mode="zwsp" дополнительно игнорирует
//     zero-width-символы (U+200B/200C/200D/2060/FEFF) внутри маркера —
//     в "exact" они РАЗРЫВАЮТ соседство (честная граница: подозреваешь
//     zero-width-эвазию — проверяй в "zwsp");
//   - красный корпус не даёт ложных срабатываний: для совпадения нужны
//     все 36 ASCII-alnum-символов маркера по порядку;
//   - position — индекс ПЕРВОГО символа вхождения в исходном тексте
//     (в СИМВОЛАХ, не байтах), -1.0 если утечки нет;
//   - чистые ядра canary_insert_core/canary_check_core экспортированы
//     для тестов и fuzz (конвенция №256, лекало redact_string).
//
// Связь с №274 (ADR-0136): redact НЕ маскирует canary-маркеры («canary
// не считается секретом» — carve-out спанов в redact_string), а canary_id
// строго формата («секрет не считается canary» — громкая ошибка формата).

use crate::interpreter::Value;
use rand::{Rng, RngExt};

/// Префикс canary-маркера (SSOT для canary.rs, redact carve-out в
/// string.rs и тестов).
pub const CANARY_PREFIX: &str = "MLOG-CANARY-";

/// Длина base32-идентификатора: 16 байт = 128 бит → 26 символов A-Z2-7.
const CANARY_ID_CHARS: usize = 26;

const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Zero-width-семейство (форматные Cf-символы), разрывающие соседство
/// символов при сопоставлении в режиме "exact".
fn is_zero_width(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
    )
}

/// Строгая проверка формы canary_id: `MLOG-CANARY-` + ровно 26 символов
/// A-Z2-7. «Секрет не считается canary»: всё, что не соответствует
/// формату, — громкая ошибка.
pub fn is_canary_id(s: &str) -> bool {
    let id = match s.strip_prefix(CANARY_PREFIX) {
        Some(rest) => rest,
        None => return false,
    };
    id.len() == CANARY_ID_CHARS && id.bytes().all(|b| BASE32_ALPHABET.contains(&b))
}

/// base32 (RFC 4648, без паддинга) 16 байт → 26 символов A-Z2-7.
fn base32_encode_16(bytes: &[u8; 16]) -> String {
    let mut out = String::with_capacity(CANARY_ID_CHARS);
    let mut buffer: u64 = 0;
    let mut bits = 0u32;
    for &b in bytes {
        buffer = (buffer << 8) | b as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1F) as usize;
            out.push(BASE32_ALPHABET[idx] as char);
        }
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1F) as usize;
        out.push(BASE32_ALPHABET[idx] as char);
    }
    debug_assert_eq!(out.len(), CANARY_ID_CHARS);
    out
}

// ── canary_insert ───────────────────────────────────────────────────────

/// Результат canary_insert: помеченный текст + полный canary_id.
#[derive(Debug, Clone, PartialEq)]
pub struct CanaryMark {
    pub marked_text: String,
    pub canary_id: String,
}

/// Чистое ядро canary_insert (без Value-обвязки): валидации громкие,
/// маркер случайный (rand 0.10, как crypto-нонсы), вставка count
/// копий ОДНОГО id в выбранной позиции.
pub fn canary_insert_core(text: &str, count: u32, position: &str) -> Result<CanaryMark, String> {
    // Громкие ошибки (наряд №284): пустой текст, повторная вставка,
    // zero-width до вставки, count вне 1..=4, неизвестная позиция.
    if text.is_empty() {
        return Err("canary_insert() — empty text: nothing to mark".to_string());
    }
    if text.contains(CANARY_PREFIX) {
        return Err(
            "canary_insert() — text already contains a canary marker (MLOG-CANARY-*): double-marking is a loud error (№284)"
                .to_string(),
        );
    }
    if text.chars().any(is_zero_width) {
        return Err(
            "canary_insert() — text contains zero-width characters (U+200B/U+200C/U+200D/U+2060/U+FEFF) BEFORE insertion; they degrade canary_check matching hygiene (№284)"
                .to_string(),
        );
    }
    if !(1..=4).contains(&count) {
        return Err(format!(
            "canary_insert() — count must be in 1..=4, got {}",
            count
        ));
    }
    if !matches!(position, "random" | "head" | "tail") {
        return Err(format!(
            "canary_insert() — unknown position '{}': expected \"random\"|\"head\"|\"tail\"",
            position
        ));
    }

    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    let canary_id = format!("{}{}", CANARY_PREFIX, base32_encode_16(&bytes));

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    let marked_text = match position {
        "head" => {
            let mut s = String::with_capacity(text.len() + (CANARY_ID_CHARS + 13) * count as usize);
            for _ in 0..count {
                s.push_str(&canary_id);
                s.push(' ');
            }
            s.push_str(text);
            s
        }
        "tail" => {
            let mut s = String::with_capacity(text.len() + (CANARY_ID_CHARS + 13) * count as usize);
            s.push_str(text);
            for _ in 0..count {
                s.push(' ');
                s.push_str(&canary_id);
            }
            s
        }
        _ => {
            // random: count точек вставки среди 0..=n (границы между
            // символами). Если слотов меньше, чем count (крошечный текст),
            // допускаются повторы точки — маркеры встают рядом.
            let mut points: Vec<usize> = Vec::with_capacity(count as usize);
            let slots = n + 1;
            if slots as u64 >= count as u64 {
                let mut seen = std::collections::HashSet::new();
                let mut rng = rand::rng();
                while points.len() < count as usize {
                    let p = rng.random_range(0..=n);
                    if seen.insert(p) {
                        points.push(p);
                    }
                }
            } else {
                points.resize(count as usize, 0);
            }
            points.sort_unstable();
            let mut out =
                String::with_capacity(text.len() + (CANARY_ID_CHARS + 13) * count as usize);
            let mut prev = 0usize;
            for &p in &points {
                out.push_str(&chars[prev..p].iter().collect::<String>());
                out.push(' ');
                out.push_str(&canary_id);
                out.push(' ');
                prev = p;
            }
            out.push_str(&chars[prev..].iter().collect::<String>());
            out
        }
    };

    Ok(CanaryMark {
        marked_text,
        canary_id,
    })
}

// ── canary_check ────────────────────────────────────────────────────────

/// Результат canary_check: leaked + позиция первого вхождения
/// (индекс в СИМВОЛАХ исходного текста, -1 при отсутствии утечки).
#[derive(Debug, Clone, PartialEq)]
pub struct CanaryCheck {
    pub leaked: bool,
    pub position: i64,
}

/// Чистое ядро canary_check: точное вхождение + устойчивость к
/// тривиальным искажениям (регистр, разбиение пробелами/пунктуацией);
/// mode="zwsp" дополнительно игнорирует zero-width-символы.
///
/// Алгоритм: нормализация «только ASCII-alnum, строчные» с картой
/// обратных индексов; в "exact" zero-width-символы СОХРАНЯЮТСЯ в
/// нормализованном потоке (разрывая соседство — честная граница), в
/// "zwsp" — выбрасываются как разделители. Поиск — скользящее окно по
/// символам (без паник на любых входах).
pub fn canary_check_core(text: &str, canary_id: &str, mode: &str) -> Result<CanaryCheck, String> {
    if text.is_empty() {
        return Err("canary_check() — empty text: nothing to check".to_string());
    }
    if !is_canary_id(canary_id) {
        return Err(format!(
            "canary_check() — unknown canary_id '{}': expected full form MLOG-CANARY-<26 chars A-Z2-7> as returned by canary_insert (№284)",
            canary_id
        ));
    }
    if !matches!(mode, "exact" | "zwsp") {
        return Err(format!(
            "canary_check() — unknown mode '{}': expected \"exact\"|\"zwsp\"",
            mode
        ));
    }

    let zwsp_mode = mode == "zwsp";

    // Нормализованный маркер: только ASCII-alnum в нижнем регистре
    // (префиксные дефисы выброшены — тот же класс разделителей, что и в
    // тексте): "MLOG-CANARY-<id>" → "mlogcanary<id>", 36 символов.
    let needle: Vec<char> = canary_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    let mut norm: Vec<char> = Vec::with_capacity(text.len());
    let mut map: Vec<usize> = Vec::with_capacity(text.len());
    for (i, c) in text.chars().enumerate() {
        if c.is_ascii_alphanumeric() {
            norm.push(c.to_ascii_lowercase());
            map.push(i);
        } else if is_zero_width(c) {
            if zwsp_mode {
                continue; // выброшен как разделитель
            }
            // "exact": сохраняется и РАЗРЫВАЕТ соседство (граница).
            norm.push(c);
            map.push(i);
        }
        // Всё остальное (пробелы, пунктуация ASCII/Unicode, символы,
        // эмодзи, кириллица и прочие не-ASCII-alnum) — разделитель:
        // выбрасывается. Для ложного срабатывания потребовались бы все
        // 36 символов маркера по порядку — недостижимо на чистом тексте.
    }

    if norm.len() < needle.len() {
        return Ok(CanaryCheck {
            leaked: false,
            position: -1,
        });
    }

    let mut leaked = false;
    let mut position: i64 = -1;
    'outer: for start in 0..=(norm.len() - needle.len()) {
        for (offset, nc) in needle.iter().enumerate() {
            if norm[start + offset] != *nc {
                continue 'outer;
            }
        }
        leaked = true;
        position = map[start] as i64;
        break 'outer;
    }

    Ok(CanaryCheck { leaked, position })
}

// ── Value-обвязка (registry handlers) ───────────────────────────────────

/// canary_insert(text, opts?) -> Struct{marked_text: String, canary_id: String}
/// opts: {count: Float (1..=4, дефолт 1), position: String ("random"|"head"|"tail", дефолт "random")}
pub(crate) fn builtin_canary_insert(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "canary_insert() expects 1..2 arguments, got {}",
            args.len()
        ));
    }
    let text = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "canary_insert() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let (count, position) = match args.get(1) {
        None => (1u32, "random".to_string()),
        Some(Value::Struct { fields, .. }) => {
            let mut count = 1u32;
            let mut position = "random".to_string();
            for (k, v) in fields {
                match k.as_str() {
                    "count" => match v {
                        Value::Float(f) if f.fract() == 0.0 => {
                            if !(1.0..=4.0).contains(f) {
                                return Err(format!(
                                    "canary_insert() — opts.count must be in 1..=4, got {}",
                                    f
                                ));
                            }
                            count = *f as u32;
                        }
                        other => {
                            return Err(format!(
                                "canary_insert() — opts.count must be an integer in 1..=4, got {}",
                                other.type_name()
                            ))
                        }
                    },
                    "position" => match v {
                        Value::String(s) => position = s.clone(),
                        other => {
                            return Err(format!(
                                "canary_insert() — opts.position must be a String, got {}",
                                other.type_name()
                            ))
                        }
                    },
                    other => {
                        return Err(format!(
                            "canary_insert() — unknown opts field '{}': expected count|position (fail-closed, №284)",
                            other
                        ))
                    }
                }
            }
            (count, position)
        }
        Some(other) => {
            return Err(format!(
                "canary_insert() — opts must be a Struct, got {}",
                other.type_name()
            ))
        }
    };

    let mark = canary_insert_core(&text, count, &position)?;
    let mut out = std::collections::HashMap::new();
    out.insert("marked_text".to_string(), Value::String(mark.marked_text));
    out.insert("canary_id".to_string(), Value::String(mark.canary_id));
    Ok(Value::Struct {
        type_name: "CanaryMark".to_string(),
        fields: out,
    })
}

/// canary_check(text, canary_id, opts?) -> Struct{leaked: Bool, id: String, position: Float}
/// opts: {mode: String ("exact"|"zwsp", дефолт "exact")}
/// Утечка → runtime warning CANARY_LEAK (stderr) + счётчик
/// llm_usage().canary_leaks. Детектор, не гейт.
pub(crate) fn builtin_canary_check(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "canary_check() expects 2..3 arguments, got {}",
            args.len()
        ));
    }
    let text = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "canary_check() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let canary_id = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "canary_check() expected String as second arg (canary_id), got {}",
                other.type_name()
            ))
        }
    };
    let mode = match args.get(2) {
        None => "exact".to_string(),
        Some(Value::Struct { fields, .. }) => {
            let mut mode = "exact".to_string();
            for (k, v) in fields {
                match k.as_str() {
                    "mode" => match v {
                        Value::String(s) => mode = s.clone(),
                        other => {
                            return Err(format!(
                                "canary_check() — opts.mode must be a String, got {}",
                                other.type_name()
                            ))
                        }
                    },
                    other => {
                        return Err(format!(
                            "canary_check() — unknown opts field '{}': expected mode (fail-closed, №284)",
                            other
                        ))
                    }
                }
            }
            mode
        }
        Some(other) => {
            return Err(format!(
                "canary_check() — opts must be a Struct, got {}",
                other.type_name()
            ))
        }
    };

    let check = canary_check_core(&text, &canary_id, &mode)?;
    if check.leaked {
        // Связка с taint/observability-моделью (№284): громкая
        // runtime-метка утечки канала + программно читаемый счётчик.
        crate::llm::record_canary_leak();
        eprintln!(
            "warning: [CANARY_LEAK] canary '{}' leaked into the response — compromised channel: treat the response as attacker-controlled (detector, not gate; №284)",
            canary_id
        );
    }

    let mut out = std::collections::HashMap::new();
    out.insert("leaked".to_string(), Value::Bool(check.leaked));
    out.insert("id".to_string(), Value::String(canary_id));
    out.insert("position".to_string(), Value::Float(check.position as f64));
    Ok(Value::Struct {
        type_name: "CanaryCheck".to_string(),
        fields: out,
    })
}
