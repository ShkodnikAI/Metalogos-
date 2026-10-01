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
///
/// Н455: this builtin moves file content into the program exactly like
/// `read_file` — it runs through the SAME sandbox resolution and the SAME
/// file-ingest gate (the sensitive-path deny-list + the serve-route
/// data-dir containment). Before the naryad this surface had NO sandbox
/// at all (any absolute path was readable) — the same file channel the
/// audit flagged, closed here rather than left as a bypass. The success/
/// error contract is unchanged (a missing/unreadable file stays a loud
/// `read_file_tokens(): …` error, exactly as before).
pub fn builtin_read_file_tokens(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("read_file_tokens() expects a file path (String)".to_string()),
    };
    use crate::builtins::io::{
        file_ingest_gate, sandbox_path, sandbox_sensitive_violation, sensitive_allowlisted,
        sensitive_path_match,
    };
    // Н455 layer 1: the RAW-form deny-list — loud even if the file is missing.
    if sensitive_path_match(&path) && !sensitive_allowlisted(&path) {
        return Err(sandbox_sensitive_violation(format!(
            "read_file_tokens('{}'): the path matches the sensitive-path deny-list \
             (.env*, *.db, *.sqlite*, .git/**, *.mlog, metalogos.toml, .mlog/**) — \
             set METALOGOS_SENSITIVE_PATH_ALLOWLIST=\"NAME\" to allow a specific \
             file explicitly (Naryad #455)",
            path
        )));
    }
    // The sandbox resolution is NEW (the naryad): absolute paths and `..`
    // were readable here before — the loud `[SANDBOX_VIOLATION]` is the
    // same contract every other file surface already carries.
    let safe_path = sandbox_path(&path).map_err(|e| format!("read_file_tokens(): {}", e))?;
    // Н455: the SSOT file-ingest gate (canonical deny-list + serve root).
    file_ingest_gate("read_file_tokens", &path, &safe_path)?;
    // №475: the read goes through the facade — the raw File::open lives
    // in fs_gate.rs; here io::Read over the gated handle (the loud
    // missing/unreadable contract is unchanged).
    let mut file =
        crate::fs_gate::open_gated(&safe_path).map_err(|e| format!("read_file_tokens(): {}", e))?;
    use std::io::Read;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("read_file_tokens(): {}", e))?;
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

    // Narjad №493, anchor 1: extract_entities used to be a silent stub.
    // The previous `regex_lite_find(pattern)` took ONLY the regex pattern
    // string (not the text) and returned an empty Vec unconditionally —
    // email/url/phone were never extracted, even though the docstring
    // promised the three kinds. The fix replaces the broken helper with
    // three direct scanners that take the actual text and walk it
    // manually (std-only — no external regex crate, same posture as
    // the rest of this module).

    // Email detection
    for m in find_emails(&text) {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("email".to_string())),
                ("name", Value::String(m)),
            ],
        ));
    }

    // URL detection
    for m in find_urls(&text) {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("url".to_string())),
                ("name", Value::String(m)),
            ],
        ));
    }

    // Phone detection (rough: 7-15 digits with optional +/spaces/dashes)
    for m in find_phones(&text) {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("phone".to_string())),
                ("name", Value::String(m)),
            ],
        ));
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

/// Narjad №493, anchor 1: std-only email scanner. Walks the text
/// character-by-character, finds `@`, then walks back for the local
/// part (allowed chars: `a-zA-Z0-9._%+-`) and forward for the domain
/// (allowed chars: `a-zA-Z0-9.-`). Validates that the domain has at
/// least one dot and the TLD is 2+ ASCII letters. Returns the matched
/// substrings (without surrounding punctuation).
fn find_emails(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find the next '@' from position i.
        let at_off = match text[i..].find('@') {
            Some(p) => p,
            None => break,
        };
        let at_pos = i + at_off;
        // Walk back from at_pos to find the local-part start.
        let mut start = at_pos;
        while start > 0 && is_email_local_char(bytes[start - 1]) {
            start -= 1;
        }
        // Walk forward from at_pos+1 to find the domain end.
        let mut end = at_pos + 1;
        while end < bytes.len() && is_email_domain_char(bytes[end]) {
            end += 1;
        }
        // Trim a trailing dot from the domain end (a domain cannot end
        // on a dot — the regex would not have matched a trailing dot
        // either; the manual scan over-captures one otherwise).
        while end > at_pos + 1 && bytes[end - 1] == b'.' {
            end -= 1;
        }
        // Validate: non-empty local part, domain has at least one dot,
        // the final segment after the last dot is 2+ ASCII letters.
        if start < at_pos && end > at_pos + 1 {
            let domain = &text[at_pos + 1..end];
            if let Some(dot_idx) = domain.rfind('.') {
                let tld = &domain[dot_idx + 1..];
                if tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()) {
                    results.push(text[start..end].to_string());
                    i = end;
                    continue;
                }
            }
        }
        i = at_pos + 1;
    }
    results
}

fn is_email_local_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_email_domain_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')
}

/// Narjad №493, anchor 1: std-only URL scanner. Looks for `http://`
/// or `https://` and walks forward over non-whitespace, non-`<>"`
/// characters. Trims trailing sentence punctuation (`.`, `,`, `;`,
/// `)`, etc.) that the regex `[\^s<>"]+` would not have captured
/// cleanly either, to match the user-facing intent.
fn find_urls(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while search_from <= text.len() {
        let rest = &text[search_from..];
        let off = match rest.find("http://").or_else(|| rest.find("https://")) {
            Some(p) => p,
            None => break,
        };
        let start = search_from + off;
        let scheme_end = if text[start..].starts_with("https://") {
            start + 8
        } else {
            start + 7
        };
        // Walk forward: capture everything that's not whitespace or a
        // closing angle-bracket / quote (the regex `[\^s<>"]+` shape).
        let mut end = scheme_end;
        for (idx, c) in text[scheme_end..].char_indices() {
            if c.is_whitespace() || c == '<' || c == '>' || c == '"' {
                break;
            }
            end = scheme_end + idx + c.len_utf8();
        }
        // Trim trailing sentence-punctuation the regex would have
        // captured (a `)` balance check is overkill for this heuristic;
        // the goal is no trailing `.`, `,`, `;`, `:`, `!`, `?`).
        while end > scheme_end {
            let last = &text[scheme_end..end];
            if let Some(c) = last.chars().next_back() {
                if matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')') {
                    end -= c.len_utf8();
                    continue;
                }
            }
            break;
        }
        if end > scheme_end {
            results.push(text[start..end].to_string());
            search_from = end;
        } else {
            search_from = scheme_end;
        }
    }
    results
}

/// Narjad №493, anchor 1: std-only phone scanner. Collects runs of
/// digits/separators that contain 7-15 digits with an optional leading
/// `+` and surrounding `(`, `)`, `-`, space. The regex shape was
/// `\+?[\d\s\-()]{7,15}` — the manual scan enforces the DIGIT count
/// (7..=15) rather than the run length, which is what the previous
/// post-filter already asked for.
fn find_phones(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // Start of a phone run: optional '+' or a digit, or '('.
        if c == b'+' || c.is_ascii_digit() || c == b'(' {
            let mut end = i;
            if end < bytes.len() && bytes[end] == b'+' {
                end += 1;
            }
            while end < bytes.len() && is_phone_char(bytes[end]) {
                end += 1;
            }
            // Trim trailing separators (space, '-', ')') — they would
            // have been part of the run but are not significant.
            while end > i + 1 && matches!(bytes[end - 1], b' ' | b'-' | b')') {
                end -= 1;
            }
            let candidate = &text[i..end];
            let digit_count = candidate.chars().filter(|c| c.is_ascii_digit()).count();
            // Same bounds as the previous post-filter (7..=15 digits).
            if (7..=15).contains(&digit_count) {
                // Avoid matching pure-number tokens like years ("2026")
                // — the digit count gate already excludes those, but
                // also require either a '+' or a separator to surface
                // something that looks phone-shaped.
                let has_separator = candidate.contains(['+', '-', '(', ')', ' ']);
                if has_separator || candidate.starts_with('+') {
                    results.push(candidate.to_string());
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    results
}

fn is_phone_char(b: u8) -> bool {
    b.is_ascii_digit() || matches!(b, b' ' | b'-' | b'(' | b')')
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
    // №546 (ADR-0178 §5.4–5.5): this builtin owns its own manager instance
    // and embeds the query PLUS every document — it is a seam consumer
    // exactly like embed/embed_text. The secret-family check runs on the
    // raw query Value (before the String contract), the budget consumes
    // one unit per embedding the call will perform (1 + doc count).
    crate::builtins::embed_seam::seam_secret_check(&args[0])?;
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

    // The budget rides the same scope as the work it guards: one unit per
    // embedding the call will perform (the query + every document), all
    // consumed BEFORE the first embed — one loud refusal covers the whole
    // call (fail-closed, never a partial run).
    crate::builtins::embed_seam::seam_budget_check(1 + documents.len() as u64)?;

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

#[cfg(test)]
mod naryad_493_anchor1_tests {
    // Narjad №493, anchor 1: extract_entities regression tests. The
    // function is pub(crate), so the tests live next to the source.
    use super::builtin_extract_entities;
    use crate::interpreter::Value;

    fn names_of_kind(entities: &[Value], kind: &str) -> Vec<String> {
        entities
            .iter()
            .filter_map(|e| match e {
                Value::Struct { fields, .. } => {
                    let k = match fields.get("kind") {
                        Some(Value::String(s)) => s.as_str(),
                        _ => return None,
                    };
                    if k != kind {
                        return None;
                    }
                    match fields.get("name") {
                        Some(Value::String(s)) => Some(s.clone()),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn extracts_simple_email() {
        let v = builtin_extract_entities(&[Value::String(
            "contact alice@example.com for details".to_string(),
        )])
        .expect("extract_entities must succeed");
        let emails = match &v {
            Value::List(items) => names_of_kind(items, "email"),
            _ => panic!("expected List, got {:?}", v),
        };
        assert_eq!(emails, vec!["alice@example.com".to_string()]);
    }

    #[test]
    fn extracts_multiple_emails_and_skips_invalid() {
        let v = builtin_extract_entities(&[Value::String(
            "from a@b.com and x.y@sub.example.org plus not-an-email@".to_string(),
        )])
        .expect("extract_entities must succeed");
        let emails = match &v {
            Value::List(items) => names_of_kind(items, "email"),
            _ => panic!("expected List, got {:?}", v),
        };
        assert_eq!(
            emails,
            vec!["a@b.com".to_string(), "x.y@sub.example.org".to_string()]
        );
    }

    #[test]
    fn extracts_http_and_https_urls() {
        let v = builtin_extract_entities(&[Value::String(
            "see http://example.com/page and https://secure.example.org/path?q=1 for more."
                .to_string(),
        )])
        .expect("extract_entities must succeed");
        let urls = match &v {
            Value::List(items) => names_of_kind(items, "url"),
            _ => panic!("expected List, got {:?}", v),
        };
        assert_eq!(
            urls,
            vec![
                "http://example.com/page".to_string(),
                "https://secure.example.org/path?q=1".to_string(),
            ]
        );
    }

    #[test]
    fn extracts_international_format_phone() {
        let v = builtin_extract_entities(&[Value::String(
            "call +1 (555) 123-4567 anytime".to_string(),
        )])
        .expect("extract_entities must succeed");
        let phones = match &v {
            Value::List(items) => names_of_kind(items, "phone"),
            _ => panic!("expected List, got {:?}", v),
        };
        assert_eq!(phones, vec!["+1 (555) 123-4567".to_string()]);
    }

    #[test]
    fn returns_empty_list_for_text_with_no_entities() {
        let v = builtin_extract_entities(&[Value::String(
            "just a plain sentence with no entities here".to_string(),
        )])
        .expect("extract_entities must succeed");
        match &v {
            Value::List(items) => {
                // The capitalized-words heuristic may produce 0 or more
                // entries; the email/url/phone lanes MUST be empty.
                assert!(names_of_kind(items, "email").is_empty());
                assert!(names_of_kind(items, "url").is_empty());
                assert!(names_of_kind(items, "phone").is_empty());
            }
            _ => panic!("expected List, got {:?}", v),
        }
    }
}
