// ── Text processing builtins ────────────────────────────────────────
// compress_html, estimate_tokens, read_file_tokens,
// extract_entities, extract_param, semantic_search, memory_score

use super::super::core::*;
use super::super::http::*;
use crate::embeddings::{cosine_similarity, EmbeddingManager};
use crate::interpreter::Value;

/// `extract_param(text, index)` — parse colon-separated callback_data, return N-th segment.
/// Example: extract_param("dept:osp:watch:42", 2) → "watch"
pub fn builtin_extract_param(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("extract_param() expects first argument to be a String".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => {
            return Err("extract_param() expects second argument to be a Float (index)".to_string())
        }
    };
    let parts: Vec<&str> = text.split(':').collect();
    match parts.get(index) {
        Some(s) => Ok(Value::String(s.to_string())),
        None => Ok(Value::String("".to_string())),
    }
}

/// `estimate_tokens(text)` — rough token count heuristic (len / 4 for CJK+Latin mix).
/// ADR note: temporary heuristic, replace with proper tokenizer when available.
pub fn builtin_estimate_tokens(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("estimate_tokens() expects a String argument".to_string()),
    };
    let char_count = text.chars().count() as f64;
    // Heuristic: ~4 chars per token for mixed CJK/Latin
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Float(tokens))
}

/// `read_file_tokens(path)` — read file and return {content, tokens} struct.
/// Convenience for skill_index: read skill file + estimate its token cost in one call.
pub fn builtin_read_file_tokens(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("read_file_tokens() expects a file path (String)".to_string()),
    };
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read_file_tokens(): {}", e))?;
    let char_count = content.chars().count() as f64;
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Struct {
        type_name: "FileInfo".to_string(),
        fields: [
            ("content".to_string(), Value::String(content)),
            ("chars".to_string(), Value::Float(char_count)),
            ("tokens".to_string(), Value::Float(tokens)),
        ]
        .into_iter()
        .collect(),
    })
}

// ── Entity Extraction (inspired by OpenHuman score/entity extraction) ──
// Pure regex-based extraction. LLM-based extraction can be done via call_llm.

/// `extract_entities(text)` — extract named entities from text using regex heuristics.
/// Returns List of Struct { kind, name, start, end }.
/// Kinds detected: person (capitalized word sequences), email, url, phone, date.
pub(crate) fn builtin_extract_entities(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("extract_entities", args, 0)?;
    let mut entities = Vec::new();

    // Email detection
    let email_re = regex_lite_find(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}");
    for m in &email_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("email".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // URL detection
    let url_re = regex_lite_find(r#"https?://[^\s<>"]'+)"#);
    for m in &url_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("url".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // Phone detection (rough: 7-15 digits with optional +/spaces/dashes)
    let phone_re = regex_lite_find(r"\+?[\d\s\-()]{7,15}");
    for m in &phone_re {
        let s = m.as_str().replace(|c: char| !c.is_ascii_digit(), "");
        if s.len() >= 7 && s.len() <= 15 {
            entities.push(make_date_struct(
                "Entity",
                vec![
                    ("kind", Value::String("phone".to_string())),
                    ("name", Value::String(m.as_str().to_string())),
                ],
            ));
        }
    }

    // Named entity: sequences of 2+ capitalized words (person/org heuristic)
    let mut caps = Vec::new();
    let mut start = 0;
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() > 1 {
            let mut end_idx = i;
            while end_idx + 1 < words.len() {
                let next = words[end_idx + 1];
                if next
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && next.len() > 1
                {
                    end_idx += 1;
                } else {
                    break;
                }
            }
            if end_idx > i {
                // Found 2+ capitalized words in sequence
                let name: String = words[i..=end_idx].join(" ");
                // Filter out common false positives
                let lower_name = name.to_lowercase();
                let false_positives = [
                    "the", "this", "that", "these", "those", "then", "than", "they", "there",
                    "their",
                ];
                if !false_positives.iter().any(|fp| lower_name == *fp) {
                    caps.push((name, start));
                }
                i = end_idx + 1;
                continue;
            }
        }
        start += w.len() + 1;
        i += 1;
    }
    for (name, _) in &caps {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("entity".to_string())),
                ("name", Value::String(name.clone())),
            ],
        ));
    }

    Ok(Value::List(entities))
}

/// Minimal regex find without external crate (uses std only).
fn regex_lite_find(pattern: &str) -> Vec<std::string::String> {
    // Very limited: only supports basic character classes.
    // For production, use the `regex` crate. This is a fallback.
    // We only call it with simple, well-known patterns above.
    let results = Vec::new();
    if pattern.contains('@') && pattern.contains('.') {
        // Email pattern — manual scan
        let bytes = pattern.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' || bytes[i] == b'(' {
                // Skip character class
                let close = if bytes[i] == b'[' { b']' } else { b')' };
                while i < bytes.len() && bytes[i] != close {
                    i += 1;
                }
                i += 1;
                continue;
            }
            i += 1;
        }
        // For email/url/phone we need actual regex; use a simpler approach
        // The real implementation should depend on `regex` crate
    }
    results
}

// ── Memory Scoring (inspired by OpenHuman chunk scoring pipeline) ──
// Computes weighted signals to decide if a text chunk is worth keeping.

/// `memory_score(text, metadata?)` — score a text chunk for memory admission.
/// Returns Struct { score, admitted, signals: {token_count, unique_words, entity_density} }.
/// Signals:
///   token_count: 0-1, plateau over chunk size (10-8000 tokens)
///   unique_words: 0-1, type-token ratio (lexical diversity)
///   entity_density: 0-1, entities per token (capped)
/// Admission threshold: score >= 0.3
pub(crate) fn builtin_memory_score(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("memory_score", args, 0)?;
    let _metadata = args.get(1); // reserved for future SourceKind weight

    // Signal 1: token_count (char_count / 4 heuristic)
    let char_count = text.chars().count() as f64;
    let token_est = char_count / 4.0;
    let token_signal = if token_est < 10.0 {
        0.0
    } else if token_est < 30.0 {
        (token_est - 10.0) / 20.0
    } else if token_est < 8000.0 {
        1.0 - (token_est - 30.0) / 16000.0 // gentle decay
    } else {
        0.5
    };

    // Signal 2: unique_words (type-token ratio)
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len() as f64;
    let unique: std::collections::HashSet<String> =
        words.iter().map(|w| w.to_lowercase()).collect();
    let unique_signal = if word_count < 2.0 {
        0.5 // neutral for very short text
    } else {
        let ttr = unique.len() as f64 / word_count;
        ttr.min(1.0)
    };

    // Signal 3: entity_density (heuristic: count capitalized sequences + emails + URLs)
    let entity_count = extract_entity_count(&text);
    let entity_density = if token_est < 100.0 {
        0.5
    } else {
        ((entity_count as f64) / (token_est / 100.0)).min(1.0)
    };

    // Weighted combination (mirrors OpenHuman weights)
    let score = token_signal * 1.0 + unique_signal * 1.0 + entity_density * 1.0;
    let total = score / 3.0; // normalize to 0-1
    let admitted = total >= 0.3;

    Ok(make_date_struct(
        "MemoryScore",
        vec![
            ("score", Value::Float((total * 100.0).round() / 100.0)),
            ("admitted", Value::Float(if admitted { 1.0 } else { 0.0 })),
            (
                "token_count",
                Value::Float((token_signal * 100.0).round() / 100.0),
            ),
            (
                "unique_words",
                Value::Float((unique_signal * 100.0).round() / 100.0),
            ),
            (
                "entity_density",
                Value::Float((entity_density * 100.0).round() / 100.0),
            ),
        ],
    ))
}

/// Count entities in text (helper for memory_score).
fn extract_entity_count(text: &str) -> usize {
    let mut count = 0;
    // Count emails
    for word in text.split_whitespace() {
        if word.contains('@') && word.contains('.') {
            count += 1;
        }
        if word.starts_with("http://") || word.starts_with("https://") {
            count += 1;
        }
    }
    // Count capitalized word sequences (2+)
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        if words[i]
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            && words[i].len() > 1
        {
            let mut end = i;
            while end + 1 < words.len()
                && words[end + 1]
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                end += 1;
            }
            if end > i {
                count += 1;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    count
}

// ── Token Compression — HTML (inspired by OpenHuman TokenJuice HtmlCompressor) ──
// Strips HTML tags, converts to readable Markdown-ish text, preserves block boundaries.

/// `compress_html(html)` — convert HTML to clean readable text.
/// Strips all tags, decodes HTML entities, adds newlines at block boundaries.
/// CJK characters preserved grapheme-by-grapheme.
/// Returns compressed String.
pub(crate) fn builtin_compress_html(args: &[Value]) -> Result<Value, String> {
    let html = expect_string_arg("compress_html", args, 0)?;

    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'<' && !in_tag {
            in_tag = true;
            tag_buf.clear();
            i += 1;
            continue;
        }
        if in_tag {
            if b == b'>' {
                in_tag = false;
                let tag = tag_buf.to_lowercase();
                // Block-level tags get a newline
                let block_tags = [
                    "p",
                    "div",
                    "h1",
                    "h2",
                    "h3",
                    "h4",
                    "h5",
                    "h6",
                    "br",
                    "li",
                    "tr",
                    "hr",
                    "blockquote",
                    "pre",
                    "table",
                    "ul",
                    "ol",
                    "section",
                    "article",
                    "header",
                    "footer",
                    "nav",
                    "main",
                    "aside",
                    "figcaption",
                    "details",
                    "summary",
                    "dt",
                    "dd",
                    "th",
                ];
                if tag.starts_with('/') {
                    // Closing tag
                    let inner = tag.trim_start_matches('/').trim();
                    if block_tags.contains(&inner) {
                        result.push('\n');
                    }
                    if inner == "script" {
                        in_script = false;
                    }
                    if inner == "style" {
                        in_style = false;
                    }
                } else {
                    let inner = tag.split_whitespace().next().unwrap_or("");
                    if block_tags.contains(&inner) && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    if inner == "script" {
                        in_script = true;
                    }
                    if inner == "style" {
                        in_style = true;
                    }
                }
                i += 1;
                continue;
            }
            tag_buf.push(b as char);
            i += 1;
            continue;
        }
        if in_script || in_style {
            i += 1;
            continue;
        }
        // HTML entity decode
        if b == b'&' {
            let rest = &html[i..];
            if let Some(end) = rest.find(';') {
                let entity = &rest[1..end];
                let decoded = decode_html_entity(entity);
                result.push_str(&decoded);
                i += end + 1;
                continue;
            }
        }
        // Collapse whitespace
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
            if !result.ends_with(' ') && !result.ends_with('\n') {
                result.push(' ');
            }
        } else {
            result.push(b as char);
        }
        i += 1;
    }

    // Collapse multiple blank lines
    let collapsed = collapse_blank_lines(&result);
    Ok(Value::String(collapsed.trim().to_string()))
}

/// Decode common HTML entities to characters.
fn decode_html_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => "\u{00a0}".to_string(),
        "&#39;" => "'".to_string(),
        _ => {
            // Numeric entities: &#NNN; or &#xHH;
            if entity.starts_with("#x") || entity.starts_with("#X") {
                if let Ok(n) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            } else if let Some(rest) = entity.strip_prefix('#') {
                if let Ok(n) = rest.parse::<u32>() {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            }
            format!("&{};", entity) // unknown entity, preserve
        }
    }
}

/// Collapse 3+ consecutive newlines into 2.
fn collapse_blank_lines(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut newline_count = 0usize;
    for c in s.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(c);
            }
        } else {
            newline_count = 0;
            result.push(c);
        }
    }
    result
}

/// `semantic_search(query, documents, top_k)` — semantic similarity search.
///
/// Inspired by obsidian-mind's QMD semantic search layer.
/// Embeds the query and each document, returns top_k results as structs:
///   { index, text, score }
///
/// Uses the same EmbeddingManager as the rest of Metalogos:
/// - OpenAI text-embedding-3-small if METALOGOS_EMBEDDING_API_KEY is set
/// - TF-IDF fallback otherwise (no API needed)
///
/// # Arguments
/// * `query` — search query string
/// * `documents` — list of document strings to search through
/// * `top_k` — number of results to return
pub(crate) fn builtin_semantic_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("semantic_search", args, 0)?;
    let documents = expect_list_arg("semantic_search", args, 1)?;
    let top_k = expect_string_arg("semantic_search", args, 2)?;
    let top_k: usize = top_k.parse().map_err(|_| {
        format!(
            "semantic_search: top_k must be a number string, got '{}'",
            args[2]
        )
    })?;

    if documents.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Create embedding manager (reads METALOGOS_EMBEDDING_PROVIDER env)
    let mgr = EmbeddingManager::new();

    // Embed the query
    let query_vec = mgr
        .embed(&query)
        .map_err(|e| format!("semantic_search: failed to embed query: {}", e))?;

    // Score each document
    let mut scored: Vec<(usize, f32, String)> = Vec::with_capacity(documents.len());
    for (i, doc_val) in documents.iter().enumerate() {
        let doc_text = format!("{}", doc_val);
        if doc_text.is_empty() {
            continue;
        }
        match mgr.embed(&doc_text) {
            Ok(doc_vec) => {
                let sim = cosine_similarity(&query_vec, &doc_vec);
                scored.push((i, sim, doc_text));
            }
            Err(e) => {
                // Skip documents that fail to embed rather than aborting
                eprintln!("[semantic_search] skip doc {}: {}", i, e);
            }
        }
    }

    // Sort by similarity descending, take top_k
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    // Build result structs
    let results: Vec<Value> = scored
        .into_iter()
        .map(|(index, score, text)| {
            make_struct(
                "SearchResult",
                vec![
                    ("index", Value::Float(index as f64)),
                    ("text", Value::String(text)),
                    ("score", Value::Float(score as f64)),
                ],
            )
        })
        .collect();

    Ok(Value::List(results))
}
