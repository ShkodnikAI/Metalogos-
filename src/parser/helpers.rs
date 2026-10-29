use pest::iterators::Pair;
use std::collections::HashMap;

use super::*;

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a string from a pair (works for IDENT, type_name, COLON, etc.).
pub(super) fn pair_str(pair: &Pair<Rule>) -> String {
    pair.as_str().to_string()
}

/// Collect inner pairs into a Vec for easy index access.
pub(super) fn children_of<'a>(pair: &'a Pair<'a, Rule>) -> Vec<Pair<'a, Rule>> {
    pair.clone().into_inner().collect()
}

/// Find the first child matching a rule, return its string.
pub(super) fn find_child_str(children: &[Pair<Rule>], rule: Rule) -> Option<String> {
    children
        .iter()
        .find(|c| c.as_rule() == rule)
        .map(|c| pair_str(c))
}

/// Find the first child matching a rule and return it.
pub(super) fn find_child<'a>(children: &'a [Pair<'a, Rule>], rule: Rule) -> Option<Pair<'a, Rule>> {
    children.iter().find(|c| c.as_rule() == rule).cloned()
}

/// Наряд №322 (ADR-0154): extract a label annotation (`label_ann` rule)
/// into an `ast::LabelAnn`. The pair's text spans `<...>`; the raw
/// annotation body is the text between the brackets, trimmed.
pub(super) fn parse_label_ann(pair: &Pair<'_, Rule>) -> crate::ast::LabelAnn {
    let span = crate::ast::Span::from_pest(pair.as_span());
    let text = pair.as_str();
    let raw = text
        .strip_prefix('<')
        .and_then(|t| t.strip_suffix('>'))
        .unwrap_or(text)
        .trim()
        .to_string();
    crate::ast::LabelAnn { span, raw }
}

/// Наряд №324 (ADR-0154 §9): extract an effect trail (`effect_trail`
/// rule) into an `ast::EffectAnn`. The pair's text spans `⟨...⟩`; the
/// raw trail body is the text between the brackets, trimmed.
pub(super) fn parse_effect_ann(pair: &Pair<'_, Rule>) -> crate::ast::EffectAnn {
    let span = crate::ast::Span::from_pest(pair.as_span());
    let text = pair.as_str();
    let raw = text
        .strip_prefix('⟨')
        .and_then(|t| t.strip_suffix('⟩'))
        .unwrap_or(text)
        .trim()
        .to_string();
    crate::ast::EffectAnn { span, raw }
}

// ── MlogServer (Phase 6.1) ─────────────────────────────────────

// ── Template (Phase 6.2) ─────────────────────────────────────

/// The lexical walk shared by the template pre-pass's two searches
/// (the `template` keyword search and the opening-brace search).
///
/// Наряд №835 (gh#1633, wave 70): the pre-pass used to scan the RAW
/// source, so a standalone `template` inside a `//` comment (or a
/// string literal) passed the №86 identifier-boundary guard and started
/// a PHANTOM template-body extraction — the next `{` after the comment
/// (typically the FOLLOWING pattern's opening brace) was replaced with
/// a placeholder, pest parsed a husk with the body's calls invisible,
/// and every body-walking gate (SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION,
/// TAINT_*) went blind for the affected unit. The fix contract: the
/// pre-pass sees the source through the SAME lexical state pest does —
/// the COMMENT rule (`//` to end-of-line), the STRING_LITERAL rule
/// (double quotes, `\`-escapes) and the MULTILINE_STRING rule (triple
/// quotes, no escapes) host no keywords and no body braces.
///
/// Unicode safety: the walker maintains `i` at a CHAR BOUNDARY.
/// - ASCII steps advance by `ch.len_utf8()` from `src[i..].chars()`;
/// - the comment/string skip loops advance BYTE-wise, but they only
///   COMPARE against ASCII bytes (`\n`, `\r`, `"`, `\`, `{`): a UTF-8
///   continuation byte is always >= 0x80 and never equals an ASCII
///   byte, so the loops stop only on real ASCII boundaries;
/// - every `&str` slice (`src[i..]`, `starts_with`) happens only where
///   `bytes[i]` holds an ASCII value — and any byte position holding
///   an ASCII value IS a char boundary (multibyte sequences consist
///   solely of bytes >= 0x80).
///
/// The scan convention inside a template BODY stays raw (the balanced
/// brace count below does NOT use this walker): the body region is
/// raw CONTENT where mlog lexical rules do not apply — HTML/CSS/JS
/// bodies legitimately contain `//` (protocol-relative URLs) and
/// quotes (attributes), so lexical skipping there would corrupt the
/// extraction. Only the searches in CODE regions (the keyword and the
/// opening brace, both before the body starts) are lexically aware.
fn next_code_occurrence(src: &str, from: usize, needle: &str) -> Option<usize> {
    let bytes = src.as_bytes();
    let nlen = needle.len();
    // The №86 identifier-boundary guard is meaningful only for an
    // alphanumeric needle (the keyword-vs-identifier question). The
    // brace search uses the same walker WITHOUT the guard — `{` may
    // legitimately touch identifier characters (`foo{`, `{x`).
    let ident_guard_applies = needle
        .as_bytes()
        .first()
        .map(|&b| b.is_ascii_alphabetic())
        .unwrap_or(false);
    let mut i = from;
    while i < src.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // COMMENT: `//` to end-of-line (the pest COMMENT rule).
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                    i += 1;
                }
            }
            b'"' if i + 2 < bytes.len() && bytes[i + 1] == b'"' && bytes[i + 2] == b'"' => {
                // MULTILINE_STRING: `"""…"""` — no escapes in the grammar.
                i += 3;
                i += src[i..].find("\"\"\"")? + 3;
            }
            b'"' => {
                // STRING_LITERAL: `"` … `"` with `\`-escapes (the
                // ESCAPE_SEQ rule — the escape byte is ASCII, the
                // defensive byte-skip never slices the &str).
                i += 1;
                let mut closed = false;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        closed = true;
                        break;
                    } else {
                        i += 1;
                    }
                }
                if !closed {
                    return None; // unterminated: nothing after it is code
                }
            }
            _ if src[i..].starts_with(needle) => {
                // Candidate. The №86 identifier-boundary guard keeps its
                // byte-level form (keyword needles only): `needle` is
                // ASCII, so both neighbors sit at char boundaries.
                let prev_is_alnum = i > 0
                    && bytes
                        .get(i - 1)
                        .map(|&b| b.is_ascii_alphanumeric())
                        .unwrap_or(false);
                let next_is_ident = bytes
                    .get(i + nlen)
                    .map(|&b| b.is_ascii_alphanumeric() || b == b'_')
                    .unwrap_or(false);
                if ident_guard_applies && (prev_is_alnum || next_is_ident) {
                    // Part of a longer identifier — step past the first
                    // byte and keep walking (the pre-existing №86
                    // semantics).
                    i += 1;
                } else {
                    return Some(i);
                }
            }
            _ => {
                // Unicode-safe single-char step.
                i += src[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            }
        }
    }
    None
}

/// Pre-process source to handle template bodies containing `}` (HTML, CSS, JS).
/// Extracts template bodies using balanced brace counting, replaces with safe placeholders,
/// and returns a mapping of placeholder -> actual body content.
/// Uses char_indices() for Unicode-safe byte positioning.
///
/// Наряд №835 (gh#1633): both searches (the `template` keyword and the
/// opening `{`) ride `next_code_occurrence` — comments and string
/// literals never host the keyword nor start the extraction. The
/// balanced brace scan over the body itself stays RAW (see the walker
/// doc — the body region is content, not code).
pub(super) fn preprocess_templates(source: &str) -> (String, HashMap<String, String>) {
    let mut result = source.to_string();
    let mut bodies = HashMap::new();
    let mut counter = 0u32;

    // Find template declarations and extract balanced brace bodies
    let mut search_from = 0;
    while search_from < result.len() {
        // Find "template" keyword OUTSIDE comments and string literals
        // (ASCII-only needle, find() is safe).
        //
        // Skip if this is part of a longer identifier — the №86 guard
        // (both neighbors) now lives inside the walker.
        //   - preceding char alphanumeric (e.g. "mytemplate")
        //   - following char alphanumeric or '_' (e.g. "template_render")
        // The keyword `template` in a declaration is ALWAYS followed
        // by whitespace then an IDENT (e.g. `template Card(...)`),
        // so a following `_` or alnum char means this is NOT the
        // keyword — it's an identifier like `template_render`.
        let Some(abs_start) = next_code_occurrence(&result, search_from, "template") else {
            break;
        };

        // Find the opening { of the template body (after type_name) —
        // ALSO outside comments and string literals (a `{` inside the
        // header comment must not start the extraction).
        // '{' is ASCII, so the walker's ASCII-only comparisons apply.
        if let Some(abs_brace) = next_code_occurrence(&result, abs_start + "template".len(), "{") {
            // Find the matching closing } using balanced brace counting.
            // MUST use char_indices() to get correct BYTE offsets for Unicode-safe slicing.
            let mut depth = 0;
            let mut end_byte_pos = None;
            for (byte_offset, ch) in result[abs_brace..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_byte_pos = Some(abs_brace + byte_offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(close_pos) = end_byte_pos {
                // Extract the body between the braces (byte offsets are char-boundary-safe)
                let body = result[abs_brace + 1..close_pos].to_string();
                let placeholder = format!("__TEMPLATE_BODY_{}__", counter);
                counter += 1;

                // Replace the body content with a safe placeholder (no })
                let replacement = format!("{{{}}}", placeholder);
                result.replace_range(abs_brace..=close_pos, &replacement);
                bodies.insert(placeholder, body);

                search_from = abs_brace + replacement.len();
                continue;
            }
        }
        search_from = abs_start + 1;
    }

    (result, bodies)
}

// ── DB (Phase 6.3) ─────────────────────────────────────

/// Process escape sequences in a string literal.
///
/// Contract (Narjad №493, anchor 7): the input is the raw STRING_LITERAL
/// lexeme — i.e. it includes the outer double quotes. The function strips
/// them. The docstring previously said "without outer quotes", which
/// contradicted the body and the way every caller in the parser feeds the
/// raw lexeme; the contradiction panicked on one-char literals such as
/// `"p"` whenever a caller pre-stripped the quotes (see parse_test_decl).
/// The body now guards short inputs so a misuse cannot slice out of bounds.
pub(super) fn unescape_string(s: &str) -> String {
    // Strip the outer double quotes when present; tolerate inputs that
    // arrive already-stripped (defensive — the function is a single
    // source of truth for the parser's escape handling).
    let trimmed = if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else if s.len() == 2 && s == "\"\"" {
        ""
    } else {
        s
    };
    let mut result = String::with_capacity(trimmed.len());
    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\"') => {
                    result.push('\"');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some('n') => {
                    result.push('\n');
                    chars.next();
                }
                Some('t') => {
                    result.push('\t');
                    chars.next();
                }
                Some('r') => {
                    result.push('\r');
                    chars.next();
                }
                Some('u') => {
                    chars.next(); // consume 'u'
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code_point) {
                            result.push(ch);
                        } else {
                            result.push_str("\\u");
                            result.push_str(&hex);
                        }
                    } else {
                        result.push_str("\\u");
                        result.push_str(&hex);
                    }
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a literal pair (STRING_LITERAL, FLOAT_LITERAL, or IDENT) to an Expr.
pub(super) fn parse_literal_to_expr(pair: &Pair<Rule>) -> Result<Expr, ParseError> {
    let inner =
        pair.clone().into_inner().next().ok_or_else(|| {
            pair_error(pair, "GRAMMAR INVARIANT: literal must have inner content")
        })?;
    match inner.as_rule() {
        Rule::STRING_LITERAL => Ok(Expr::StringLit {
            value: unescape_string(inner.as_str()),
            span: Span::unknown(),
        }),
        Rule::FLOAT_LITERAL => Ok(Expr::FloatLit {
            value: inner.as_str().parse().unwrap_or(0.0),
            span: Span::unknown(),
        }),
        Rule::IDENT => Ok(Expr::Ident {
            name: inner.as_str().to_string(),
            span: Span::unknown(),
        }),
        _ => Ok(Expr::StringLit {
            value: pair.as_str().to_string(),
            span: Span::unknown(),
        }),
    }
}

// ── Entity: struct instance ─────────────────────────────────────────
