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

// ── №695 (ADR-0188 phase 2): the pipe-stage classification ─────────────
//
// The v1 pipe grammar is ONE stage, `safe`, in the DOUBLE-brace
// interpolated form: `{{ param | safe }}`. The classification lives HERE
// — the ONE tokenizer every template consumer already shares (the №685
// discipline: a second parser would drift from the render-time behavior,
// and the drift is exactly how S-1 happened).
//
// The pest grammar is NOT touched: template bodies are raw by design
// (the balanced-brace extraction of grammar.pest:104-105 — HTML/CSS/JS
// bodies), so the in-body placeholder grammar has always lived in this
// module (the raw-insert form landed here in №685 the same way).

/// The classification of one tag's content against the v1 pipe grammar.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PipeForm {
    /// No pipe in the tag — the plain placeholder (escaped default).
    Plain,
    /// The recognized v1 form: `{{ param | safe }}` — a single `|`, the
    /// stage is exactly `safe`, and `param` names a declared parameter
    /// (render() substitutes by param name; an expression language is
    /// NOT part of v1).
    SafeParam(usize),
    /// A pipe the v1 grammar does not recognize: an unknown stage
    /// (`{{ x | upper }}`), multiple pipes, an empty side, a pipe in the
    /// raw triple-brace form, or an expr that is not a declared param.
    /// Every consumer must refuse these LOUDLY — the N9 lesson (gh#1219):
    /// a silent mismatch between the syntax and the runtime is forbidden.
    Unsupported,
}

/// №695: classify a tag's content against the v1 pipe grammar.
/// `content` is the tokenizer's trimmed tag content; `raw` distinguishes
/// the triple-brace form. Construct tags (`{{#if}}` / `{{else}}` /
/// `{{/if}}`) are NOT var placeholders — they classify as `Plain` here
/// (a pipe inside a condition string literal, `{{#if x == "a|b"}}`, is
/// the N9-5-legal construct grammar, not a filter).
pub(crate) fn classify_pipe(content: &str, raw: bool, params: &[String]) -> PipeForm {
    let inner = content.trim();
    if inner.starts_with('#') || inner.starts_with('/') || inner == "else" {
        return PipeForm::Plain;
    }
    if !inner.contains('|') {
        return PipeForm::Plain;
    }
    // The raw triple-brace form is already the raw insert (№685) — a pipe
    // there is not the v1 grammar (the raw form needs no stage).
    if raw {
        return PipeForm::Unsupported;
    }
    let Some((expr, stage)) = inner.split_once('|') else {
        return PipeForm::Unsupported;
    };
    let expr = expr.trim();
    let stage = stage.trim();
    if expr.is_empty() || stage.is_empty() || stage.contains('|') || expr.contains('|') {
        return PipeForm::Unsupported;
    }
    if stage != "safe" {
        return PipeForm::Unsupported;
    }
    match params.iter().position(|p| p == expr) {
        Some(idx) => PipeForm::SafeParam(idx),
        // The v1 substitution is by param name — an expr that is not a
        // declared param can never be substituted, so the form is not
        // part of the grammar (render() would leave it verbatim).
        None => PipeForm::Unsupported,
    }
}

/// №695: the shared pipe scan over a template body — every tag lands in
/// exactly one bucket: the recognized `{{ param | safe }}` positions
/// (the param INDICES) and the unsupported pipe forms (the verbatim tag
/// contents, for the refusal messages).
///
/// Consumers (the ONE scan, FOUR consumers — the №685 mirror discipline):
/// the declaration gate (`semantic::safe_pipe_not_html_violations`),
/// the registration backstop (`register_template`), the audit taint
/// layer via `html_label::seed_render_raw` (a safe position is a RAW
/// position — the taint conducts, never sanitizes) and
/// `render_template_body` (the substitution contract).
pub(crate) fn pipe_scan(body: &str, params: &[String]) -> (Vec<usize>, Vec<String>) {
    let mut safe: Vec<usize> = Vec::new();
    let mut unsupported: Vec<String> = Vec::new();
    for tok in tokenize(body) {
        let Token::Tag { content, raw } = tok else {
            continue;
        };
        match classify_pipe(&content, raw, params) {
            PipeForm::Plain => {}
            PipeForm::SafeParam(idx) => {
                if !safe.contains(&idx) {
                    safe.push(idx);
                }
            }
            PipeForm::Unsupported => {
                if !unsupported.contains(&content) {
                    unsupported.push(content);
                }
            }
        }
    }
    (safe, unsupported)
}
