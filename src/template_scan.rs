// ── №685 (gh#1252, В44): the shared template tokenizer + the raw-insert
//    scan — the UNGATED home ──────────────────────────────────────────
//
// WHY A SEPARATE MODULE: `render()` is an ungated builtin (builtins/
// http.rs), the template DECLARATIONS parse regardless of the feature
// set, and the S-1 raw-insert gates must hold on every configuration —
// while `builtins::template` (the template_render engine) lives behind
// `feature = "template"` (the №111 capability gate). The ONE tokenizer
// therefore lives HERE, ungated; the gated engine imports it (a single
// source, no drift), and so do the three raw-insert consumers:
// `semantic::raw_insert_not_html_violations` (the declaration gate),
// the audit taint layer (via html_label's seeded map) and the
// html_label sink walk.

/// `{{ ... }}` — content already trimmed. `raw=true` for `{{{ ... }}}`.
#[derive(Debug, Clone)]
pub(crate) enum Token {
    Text(String),
    Tag { content: String, raw: bool },
}

/// Split the source into Text and Tag tokens.
/// `{{{`/`}}}` (triple) take priority over `{{`/`}}` (double).
/// №685 (gh#1252, В44): pub(crate) — the ONE tokenizer every raw-insert
/// consumer shares. A SECOND parser would drift from the render-time
/// behavior, and the drift is exactly how S-1 happened.
pub(crate) fn tokenize(src: &str) -> Vec<Token> {
    let bytes: Vec<char> = src.chars().collect();
    let n = bytes.len();
    let mut i = 0;
    let mut out = Vec::new();
    let mut buf = String::new();

    while i < n {
        if i + 2 < n && bytes[i] == '{' && bytes[i + 1] == '{' && bytes[i + 2] == '{' {
            // Triple-brace open — find matching `}}}`
            if !buf.is_empty() {
                out.push(Token::Text(std::mem::take(&mut buf)));
            }
            let start = i + 3;
            let mut j = start;
            while j + 2 < n {
                if bytes[j] == '}' && bytes[j + 1] == '}' && bytes[j + 2] == '}' {
                    break;
                }
                j += 1;
            }
            if j + 2 >= n {
                // No closing `}}}` — treat the rest as literal text.
                buf.push_str(&src[i..]);
                break;
            }
            let content: String = bytes[start..j].iter().collect();
            out.push(Token::Tag {
                content: content.trim().to_string(),
                raw: true,
            });
            i = j + 3;
        } else if i + 1 < n && bytes[i] == '{' && bytes[i + 1] == '{' {
            // Double-brace open — find matching `}}`
            if !buf.is_empty() {
                out.push(Token::Text(std::mem::take(&mut buf)));
            }
            let start = i + 2;
            let mut j = start;
            while j + 1 < n {
                if bytes[j] == '}' && bytes[j + 1] == '}' {
                    break;
                }
                j += 1;
            }
            if j + 1 >= n {
                buf.push_str(&src[i..]);
                break;
            }
            let content: String = bytes[start..j].iter().collect();
            out.push(Token::Tag {
                content: content.trim().to_string(),
                raw: false,
            });
            i = j + 2;
        } else {
            buf.push(bytes[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        out.push(Token::Text(buf));
    }
    out
}

/// №685 (gh#1252, В44): the shared raw-insert scan — the param INDICES
/// whose names appear in `{{{ ... }}}` (triple-brace) positions of the
/// template body. The construct forms (`{{#if}}` / `{{else}}` /
/// `{{/if}}`) are NOT var placeholders (the same guard
/// `template_body_has_filter_pipe` applies); a raw reference that is not
/// a declared param name is not a param use (render() can never
/// substitute it — the flat key replace — and template_render's path
/// behavior is its own contract).
///
/// Consumers: the declaration gate, the audit taint layer (render() =
/// sanitizer for escaped positions / conductor for raw positions) and
/// the html_label sink walk. ONE scan, THREE consumers — the mirror
/// discipline of html_label (№568) applied to the tokenizer.
pub(crate) fn raw_param_positions(body: &str, params: &[String]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for tok in tokenize(body) {
        let Token::Tag { content, raw } = tok else {
            continue;
        };
        if !raw {
            continue;
        }
        let inner = content.trim();
        // The construct forms are NOT var placeholders (their grammar
        // may carry '#', '/' or the else word — see
        // template_body_has_filter_pipe).
        if inner.starts_with('#') || inner.starts_with('/') || inner == "else" {
            continue;
        }
        if let Some(idx) = params.iter().position(|p| p == inner) {
            if !out.contains(&idx) {
                out.push(idx);
            }
        }
    }
    out
}
