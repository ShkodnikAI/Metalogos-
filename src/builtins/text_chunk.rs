// ── Наряд №285 (P2, feature/memory): text_chunk — структура-осознанное
// чанкование для RAG-пайплайна ──
//
// Первая стадия RAG (№272 дал embed/vec_store/vec_search): производитель
// чанков. Промышленный паттерн — langchain text-splitters:
//   * RecursiveCharacterTextSplitter — каскад разделителей + overlap +
//     слияние мелких кусков;
//   * MarkdownHeaderTextSplitter — секции с header-path метаданными.
//
//   text_chunk(text, strategy, opts?) -> List<Struct{
//       index, text, chars, tokens, header_path?   // header_path — markdown
//   }>
//
// strategies:
//   "markdown"  — h1–h3 → секции (header_path "H1 > H2 > H3"); длинная
//                 секция режется каскадом «абзац → перенос → пробел»,
//                 строки заголовков не рвутся (заголовок длиннее бюджета —
//                 громкая ошибка, fail-closed);
//   "paragraph" — блоки по двойному переносу; слияние мелких в пределах
//                 бюджета; длинные блоки — каскад ниже;
//   "fixed"     — окна фиксированного бюджета с overlap.
//
// opts: max_chars (дефолт 1200), overlap (дефолт 100, СИМВОЛЫ),
//       max_tokens? — если задан, бюджет считается token_count (реюз
//       счётчика №-memory: кириллица /2, латиница /4), overlap остаётся
//       в символах (окнирование всегда посимвольно).
//
// Семантика: каскад «заголовок → абзац (\n\n) → перенос (\n) → пробел»;
// атомы меньше бюджета сливаются жадно; overlap применяется при жёстком
// окнировании (fixed и длинные атомы нижнего уровня) — хвост окна входит
// в начало следующего. Пустой/короткий текст → 1 чанк, не ошибка.
//
// Громкие ошибки (дефекты программы): неизвестный strategy; overlap >=
// max_chars (и overlap >= max_tokens на token-пути); max_tokens <= 0;
// max_chars <= 0; неизвестные поля opts; opts не Struct; заголовок
// длиннее бюджета (не рвём); бюджет меньше одного символа.
//
// feature-гейта нет: чистая строковая функция (категория "string"),
// vec-интеграционный тест — под cfg(feature = "vec").

use super::memory::token_count_estimate;
use super::Value;

use super::core::expect_string_arg;

/// Дефолты opts (issue #340).
const DEFAULT_MAX_CHARS: f64 = 1200.0;
const DEFAULT_OVERLAP: f64 = 100.0;

/// Метрика бюджета: символы или токены (token_count-реюз).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Metric {
    Chars,
    Tokens,
}

impl Metric {
    fn len(&self, s: &str) -> f64 {
        match self {
            Metric::Chars => s.chars().count() as f64,
            Metric::Tokens => token_count_estimate(s),
        }
    }
}

struct ChunkOpts {
    max_chars: f64,
    overlap: f64,
    max_tokens: Option<f64>,
}

/// Парс opts: Struct{max_chars?, overlap?, max_tokens?}; неизвестные
/// поля / не-Float значения / не-Struct — громко.
fn parse_opts(builtin: &str, arg: &Value) -> Result<ChunkOpts, String> {
    let fields = match arg {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "{builtin}(): opts must be a Struct, got {}",
                other.type_name()
            ))
        }
    };
    let mut max_chars: Option<f64> = None;
    let mut overlap: Option<f64> = None;
    let mut max_tokens: Option<f64> = None;
    for (k, v) in fields {
        match k.as_str() {
            "max_chars" => max_chars = Some(opt_float(builtin, k, v)?),
            "overlap" => overlap = Some(opt_float(builtin, k, v)?),
            "max_tokens" => max_tokens = Some(opt_float(builtin, k, v)?),
            other => {
                return Err(format!(
                    "{builtin}(): unknown opts field '{other}' (allowed: max_chars, overlap, max_tokens)"
                ))
            }
        }
    }
    Ok(ChunkOpts {
        max_chars: max_chars.unwrap_or(DEFAULT_MAX_CHARS),
        overlap: overlap.unwrap_or(DEFAULT_OVERLAP),
        max_tokens,
    })
}

fn opt_float(builtin: &str, key: &str, v: &Value) -> Result<f64, String> {
    match v {
        Value::Float(x) => Ok(*x),
        other => Err(format!(
            "{builtin}(): opts.{key} must be a Number, got {}",
            other.type_name()
        )),
    }
}

/// Валидации бюджета (громко, fail-closed).
fn validate(builtin: &str, o: &ChunkOpts) -> Result<(Metric, f64, f64), String> {
    if o.max_chars <= 0.0 {
        return Err(format!(
            "{builtin}(): opts.max_chars must be > 0, got {}",
            o.max_chars
        ));
    }
    if o.overlap < 0.0 {
        return Err(format!(
            "{builtin}(): opts.overlap must be >= 0, got {}",
            o.overlap
        ));
    }
    if o.overlap >= o.max_chars {
        return Err(format!(
            "{builtin}(): opts.overlap ({}) must be < opts.max_chars ({})",
            o.overlap, o.max_chars
        ));
    }
    match o.max_tokens {
        None => Ok((Metric::Chars, o.max_chars, o.overlap)),
        Some(t) => {
            if t <= 0.0 {
                return Err(format!(
                    "{builtin}(): opts.max_tokens must be > 0, got {}",
                    t
                ));
            }
            if o.overlap >= t {
                return Err(format!(
                    "{builtin}(): opts.overlap ({}) must be < opts.max_tokens ({}) (token budget mode)",
                    o.overlap, t
                ));
            }
            Ok((Metric::Tokens, t, o.overlap))
        }
    }
}

// ── Каскадная нарезка (RecursiveCharacterTextSplitter-дух) ──────────

/// Один атомарный кусок с сохранённым ведущим разделителем (кроме первого).
struct Atom {
    text: String,
}

fn is_header_line(s: &str) -> bool {
    let t = s.trim_start();
    let hashes = t.chars().take_while(|c| *c == '#').count();
    (1..=3).contains(&hashes) && t[hashes..].starts_with(' ')
}

/// Каскад: если кусок в бюджете — атом; иначе режем первым разделителем
/// (с сохранением его в начале дочерних кусков), рекурсивно дальше;
/// исчерпали разделители — жёсткие окна с overlap.
/// `header_guard` — громкий отказ при рвании заголовочной строки.
fn split_atoms(
    builtin: &str,
    text: &str,
    seps: &[&str],
    metric: Metric,
    budget: f64,
    overlap: f64,
    header_guard: bool,
) -> Result<Vec<Atom>, String> {
    if metric.len(text) <= budget {
        return Ok(vec![Atom {
            text: text.to_string(),
        }]);
    }
    if let Some((first, rest)) = seps.split_first() {
        let mut atoms: Vec<Atom> = Vec::new();
        let pieces: Vec<&str> = text.split(*first).collect();
        for (i, piece) in pieces.iter().enumerate() {
            // Сохраняем разделитель в НАЧАЛЕ следующего куска (склейка
            // восстанавливает текст; сепаратор принадлежит контенту ниже).
            let owned: String = if i == 0 {
                (*piece).to_string()
            } else {
                format!("{first}{piece}")
            };
            if owned.is_empty() {
                continue;
            }
            let sub = split_atoms(builtin, &owned, rest, metric, budget, overlap, header_guard)?;
            atoms.extend(sub);
        }
        return Ok(atoms);
    }
    // Разделители исчерпаны — жёсткие окна с overlap (посимвольно).
    // Заголовочные строки не рвём никогда: громко.
    if header_guard && is_header_line(text) && !text.contains('\n') {
        return Err(format!(
            "[TEXT_CHUNK_HEADER_TOO_LONG] {builtin}: header line is longer than the chunk budget ({}) and would have to be torn: '{}'…",
            metric.len(text),
            text.chars().take(60).collect::<String>()
        ));
    }
    let windows = hard_windows(text, metric, budget, overlap, builtin)?;
    Ok(windows.into_iter().map(|w| Atom { text: w }).collect())
}

/// Жёсткие окна: максимум символов с метрикой ≤ budget, шаг
/// (длина окна − overlap). Окна всегда по границе чаров.
fn hard_windows(
    text: &str,
    metric: Metric,
    budget: f64,
    overlap: f64,
    builtin: &str,
) -> Result<Vec<String>, String> {
    let chars: Vec<char> = text.chars().collect();
    let step_back = overlap.max(0.0) as usize;
    let mut out: Vec<String> = Vec::new();
    let mut start: usize = 0;
    while start < chars.len() {
        // Растим окно до нарушения бюджета (линейный скан с накоплением
        // строки — прототипно прост и детерминирован).
        let mut best_end = start; // даже 1 символ может нарушить бюджет
        let mut end = start;
        while end < chars.len() {
            end += 1;
            let s: String = chars[start..end].iter().collect();
            if metric.len(&s) > budget {
                break;
            }
            best_end = end;
        }
        if best_end == start {
            return Err(format!(
                "[TEXT_CHUNK_BUDGET_TOO_SMALL] {builtin}: budget is smaller than one character of the input"
            ));
        }
        out.push(chars[start..best_end].iter().collect());
        if best_end >= chars.len() {
            break;
        }
        // Гарантия прогресса: перекрытие не может сдвинуть окно назад.
        let next = best_end.saturating_sub(step_back).max(start + 1);
        start = next;
    }
    Ok(out)
}

/// Жадное слияние соседних атомов в пределах бюджета (RecursiveCharacter
/// merge) с overlap-сцепкой: при закрытии чанка его ХВОСТ (до overlap
/// символов, выровненный по пробелу) переносится в начало следующего.
/// Сцепка применяется только если влезает в бюджет — бюджетный инвариант
/// «ни один чанк не превышает бюджета» сильнее сцепки (граница
/// задокументирована). Атом длиннее бюджета здесь невозможен
/// (см. split_atoms/hard_windows).
fn merge_atoms(atoms: Vec<Atom>, metric: Metric, budget: f64, overlap: f64) -> Vec<String> {
    let step_back = overlap.max(0.0) as usize;
    let mut out: Vec<String> = Vec::new();
    let mut cur: Option<String> = None;
    for atom in atoms {
        match cur.take() {
            None => cur = Some(atom.text),
            Some(c) => {
                let joined = format!("{c}{}", atom.text);
                if metric.len(&joined) <= budget {
                    cur = Some(joined);
                } else {
                    // Закрываем чанк; хвост (выровненный по пробелу) — carry.
                    let carry = overlap_tail(&c, step_back);
                    out.push(c);
                    // carry + атом может превышать бюджет → тогда сцепку
                    // сбрасываем (инвариант бюджета сильнее).
                    let with_carry = format!("{carry}{}", atom.text);
                    if metric.len(&with_carry) <= budget {
                        cur = Some(with_carry);
                    } else {
                        cur = Some(atom.text);
                    }
                }
            }
        }
    }
    if let Some(c) = cur {
        out.push(c);
    }
    out
}

/// Хвост чанка до `step_back` символов, выровненный по пробелу (сцепка
/// не рвёт слова; если пробела нет — сырой хвост).
fn overlap_tail(chunk: &str, step_back: usize) -> String {
    if step_back == 0 {
        return String::new();
    }
    let total = chunk.chars().count();
    let from = total.saturating_sub(step_back);
    let tail: String = chunk.chars().skip(from).collect();
    match tail.find(' ') {
        Some(pos) => tail[pos + 1..].to_string(),
        None => tail,
    }
}

// ── Стратегии ───────────────────────────────────────────────────────

/// markdown: секции h1–h3 с header_path; длинные секции — каскад
/// «абзац → перенос → пробел»; заголовочная строка — первый атом секции
/// (не рвётся; громкая ошибка, если длиннее бюджета).
fn chunk_markdown(
    builtin: &str,
    text: &str,
    metric: Metric,
    budget: f64,
    overlap: f64,
) -> Result<Vec<(String, String)>, String> {
    // 1) Секции по заголовочным строкам (h1–h3, только в начале строки).
    struct Section {
        header_path: String,
        lines: Vec<String>,
    }
    let mut sections: Vec<Section> = Vec::new();
    let mut stack: [(usize, String); 3] =
        [(0, String::new()), (0, String::new()), (0, String::new())]; // (уровень-занятость, текст)

    for line in text.lines() {
        if is_header_line(line) {
            let level = line.trim_start().chars().take_while(|c| *c == '#').count();
            let title = line.trim_start()[level..].trim().to_string();
            // Закрываем текущую секцию, открываем новую с путём-стеком.
            stack[level - 1] = (1usize, title);
            for s in stack.iter_mut().skip(level) {
                *s = (0, String::new());
            }
            let path: Vec<&str> = stack
                .iter()
                .take(level)
                .filter(|(set, _)| *set == 1)
                .map(|(_, t)| t.as_str())
                .collect();
            sections.push(Section {
                header_path: path.join(" > "),
                lines: vec![line.to_string()],
            });
        } else {
            match sections.last_mut() {
                Some(sec) => sec.lines.push(line.to_string()),
                None => sections.push(Section {
                    header_path: String::new(), // преамбула до первого заголовка
                    lines: vec![line.to_string()],
                }),
            }
        }
    }
    if sections.is_empty() {
        // Пустой текст: lines() пуст → 1 чанк с пустым текстом (контракт).
        sections.push(Section {
            header_path: String::new(),
            lines: vec![String::new()],
        });
    }

    // 2) Секция в бюджете → 1 чанк; длиннее — каскад по телу.
    //    Заголовок резервирует место в бюджете первого чанка
    //    (header + "\n" + тело ≤ budget) — заголовок не рвётся и
    //    не оставляется мусорным чанком без тела.
    let mut out: Vec<(String, String)> = Vec::new();
    for sec in sections {
        let joined = sec.lines.join("\n");
        if metric.len(&joined) <= budget {
            out.push((sec.header_path, joined));
            continue;
        }
        // Тело без заголовочной строки.
        let (header, body) = match sec.lines.split_first() {
            Some((h, rest)) if is_header_line(h) => (Some(h.as_str()), rest.join("\n")),
            _ => (None, joined.clone()),
        };
        // Заголовок длиннее бюджета сам по себе — рвать нельзя (№340),
        // значит данные дефектны: громко (fail-closed).
        if let Some(h) = header {
            let h_len = metric.len(h) + 1.0; // + перенос
            if h_len > budget {
                return Err(format!(
                    "[TEXT_CHUNK_HEADER_TOO_LONG] {builtin}: header line ({}) exceeds the chunk budget ({}) and would have to be torn: '{}'…",
                    h_len,
                    budget,
                    h.chars().take(60).collect::<String>()
                ));
            }
        }
        let body_budget = match header {
            Some(h) => budget - metric.len(h) - 1.0,
            None => budget,
        };
        let seps = ["\n\n", "\n", " "];
        let atoms = split_atoms(builtin, &body, &seps, metric, body_budget, overlap, true)?;
        let mut merged = merge_atoms(atoms, metric, body_budget, overlap);
        // Первый чанк секции начинается с заголовочной строки (целиком).
        if let Some(h) = header {
            match merged.first_mut() {
                Some(first) => {
                    if !first.starts_with(h) {
                        *first = format!("{h}\n{first}");
                    }
                }
                None => merged.insert(0, h.to_string()),
            }
        }
        for chunk in merged {
            out.push((sec.header_path.clone(), chunk));
        }
    }
    Ok(out)
}

/// paragraph: блоки по двойному переносу; слияние мелких; длинные
/// блоки — каскад «перенос → пробел → окна».
fn chunk_paragraph(
    builtin: &str,
    text: &str,
    metric: Metric,
    budget: f64,
    overlap: f64,
) -> Result<Vec<String>, String> {
    let seps = ["\n\n", "\n", " "];
    let atoms = split_atoms(builtin, text, &seps, metric, budget, overlap, false)?;
    Ok(merge_atoms(atoms, metric, budget, overlap))
}

/// fixed: окна бюджета с overlap (посимвольно, детерминированно).
fn chunk_fixed(
    builtin: &str,
    text: &str,
    metric: Metric,
    budget: f64,
    overlap: f64,
) -> Result<Vec<String>, String> {
    hard_windows(text, metric, budget, overlap, builtin)
}

// ── Builtin ─────────────────────────────────────────────────────────

pub(crate) fn builtin_text_chunk(args: &[Value]) -> Result<Value, String> {
    const BUILTIN: &str = "text_chunk";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{BUILTIN}() requires 2 or 3 arguments (text, strategy, opts?), got {}",
            args.len()
        ));
    }
    let text = expect_string_arg(BUILTIN, args, 0)?;
    let strategy = expect_string_arg(BUILTIN, args, 1)?;
    let opts = if args.len() == 3 {
        parse_opts(BUILTIN, &args[2])?
    } else {
        ChunkOpts {
            max_chars: DEFAULT_MAX_CHARS,
            overlap: DEFAULT_OVERLAP,
            max_tokens: None,
        }
    };
    let (metric, budget, overlap) = validate(BUILTIN, &opts)?;

    enum Kind {
        Markdown,
        Paragraph,
        Fixed,
    }
    let kind = match strategy.as_str() {
        "markdown" => Kind::Markdown,
        "paragraph" => Kind::Paragraph,
        "fixed" => Kind::Fixed,
        other => {
            return Err(format!(
                "{BUILTIN}(): unknown strategy '{other}' (allowed: \"markdown\", \"paragraph\", \"fixed\")"
            ))
        }
    };

    // Пустой/короткий текст → 1 чанк (контракт «не ошибка»).
    let chunks: Vec<(String, String)> = match kind {
        Kind::Markdown => {
            if text.is_empty() {
                vec![(String::new(), String::new())]
            } else {
                chunk_markdown(BUILTIN, &text, metric, budget, overlap)?
            }
        }
        Kind::Paragraph => {
            if text.is_empty() {
                vec![(String::new(), String::new())]
            } else {
                chunk_paragraph(BUILTIN, &text, metric, budget, overlap)?
                    .into_iter()
                    .map(|c| (String::new(), c))
                    .collect()
            }
        }
        Kind::Fixed => {
            if text.is_empty() {
                vec![(String::new(), String::new())]
            } else {
                chunk_fixed(BUILTIN, &text, metric, budget, overlap)?
                    .into_iter()
                    .map(|c| (String::new(), c))
                    .collect()
            }
        }
    };

    let is_markdown = matches!(kind, Kind::Markdown);
    let values: Vec<Value> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, (header_path, chunk_text))| {
            let chars = chunk_text.chars().count() as f64;
            let tokens = token_count_estimate(&chunk_text);
            let mut fields: Vec<(&str, Value)> = vec![
                ("index", Value::Float(i as f64)),
                ("text", Value::String(chunk_text)),
                ("chars", Value::Float(chars)),
                ("tokens", Value::Float(tokens)),
            ];
            if is_markdown {
                fields.push(("header_path", Value::String(header_path)));
            }
            Value::Struct {
                type_name: "TextChunk".to_string(),
                fields: fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            }
        })
        .collect();
    Ok(Value::List(values))
}
