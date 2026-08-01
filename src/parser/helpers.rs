use pest::iterators::Pair;
use std::collections::HashMap;

use super::*;

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a string from a pair (works for IDENT, type_name, COLON, etc.).
pub(super) fn pair_str(pair: &Pair<Rule>) -> String {
    pair.as_str().to_string()
}

/// Collect inner pairs into a Vec for easy index access.

/// Collect inner pairs into a Vec for easy index access.
pub(super) fn children_of<'a>(pair: &'a Pair<'a, Rule>) -> Vec<Pair<'a, Rule>> {
    pair.clone().into_inner().collect()
}

/// Find the first child matching a rule, return its string.

/// Find the first child matching a rule, return its string.
pub(super) fn find_child_str(children: &[Pair<Rule>], rule: Rule) -> Option<String> {
    children
        .iter()
        .find(|c| c.as_rule() == rule)
        .map(|c| pair_str(c))
}

/// Find the first child matching a rule and return it.

/// Find the first child matching a rule and return it.
pub(super) fn find_child<'a>(children: &'a [Pair<'a, Rule>], rule: Rule) -> Option<Pair<'a, Rule>> {
    children.iter().find(|c| c.as_rule() == rule).cloned()
}

// ── MlogServer (Phase 6.1) ─────────────────────────────────────

// ── Template (Phase 6.2) ─────────────────────────────────────

/// Pre-process source to handle template bodies containing `}` (HTML, CSS, JS).
/// Extracts template bodies using balanced brace counting, replaces with safe placeholders,
/// and returns a mapping of placeholder -> actual body content.
/// Uses char_indices() for Unicode-safe byte positioning.
pub(super) fn preprocess_templates(source: &str) -> (String, HashMap<String, String>) {
    let mut result = source.to_string();
    let mut bodies = HashMap::new();
    let mut counter = 0u32;

    // Find template declarations and extract balanced brace bodies
    let mut search_from = 0;
    while search_from < result.len() {
        // Find "template" keyword (ASCII-only, find() is safe)
        if let Some(start) = result[search_from..].find("template") {
            let abs_start = search_from + start;
            // Skip if this is part of a longer identifier (check preceding char)
            if abs_start > 0
                && result
                    .as_bytes()
                    .get(abs_start - 1)
                    .map(|&b| b.is_ascii_alphanumeric())
                    .unwrap_or(false)
            {
                search_from = abs_start + 1;
                continue;
            }

            // Find the opening { of the template body (after type_name)
            // '{' is ASCII, find() on ASCII patterns is char-boundary-safe
            if let Some(brace_pos) = result[abs_start..].find('{') {
                let abs_brace = abs_start + brace_pos;
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
        } else {
            break;
        }
    }

    (result, bodies)
}

/// Parse a template declaration, restoring the pre-processed body.

/// Extract content between balanced braces from a string like "{ content } }".
/// Handles nested braces by counting depth.
pub(super) fn extract_balanced_braces(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() || chars[0] != '{' {
        return String::new();
    }
    let mut depth = 0;
    let mut end = 0;
    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end > 0 {
        chars[1..end].iter().collect()
    } else {
        String::new()
    }
}

// ── DB (Phase 6.3) ─────────────────────────────────────

/// Process escape sequences in a string literal (without outer quotes).
pub(super) fn unescape_string(s: &str) -> String {
    let trimmed = &s[1..s.len() - 1]; // strip outer quotes
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

/// Convert a literal pair (STRING_LITERAL, FLOAT_LITERAL, or IDENT) to an Expr.
pub(super) fn parse_literal_to_expr(pair: &Pair<Rule>) -> Result<Expr, ParseError> {
    let inner =
        pair.clone().into_inner().next().ok_or_else(|| {
            pair_error(pair, "GRAMMAR INVARIANT: literal must have inner content")
        })?;
    match inner.as_rule() {
        Rule::STRING_LITERAL => Ok(Expr::StringLit(unescape_string(inner.as_str()))),
        Rule::FLOAT_LITERAL => Ok(Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0))),
        Rule::IDENT => Ok(Expr::Ident(inner.as_str().to_string())),
        _ => Ok(Expr::StringLit(pair.as_str().to_string())),
    }
}

// ── Entity: struct instance ─────────────────────────────────────────
