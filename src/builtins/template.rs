// ── Наряд №86: Mini template engine (template_render) ────────────────
//
// A NEW, separate function — the existing `render()` in http.rs is left
// untouched. `template_render(template, data) -> Html` parses a small
// Mustache/Handlebars-like subset:
//
//   {{ var }}                — simple substitution (auto-escaped)
//   {{ obj.field }}          — one-level field access (max depth = 1)
//   {{{ var }}}              — raw substitution (NOT escaped)
//   {{#if cond}} ... {{/if}}
//   {{#if cond}} ... {{else}} ... {{/if}}
//   {{#each items}} ... {{/each}}  — `{{ this }}` / `{{ this.field }}` inside
//
// `cond` is either a variable path (truthy/falsy per Value::as_bool — Bool,
// non-empty String, non-zero Float) or a single `var == "literal"` /
// `var != "literal"` comparison. NOT a general expression language.
//
// Missing data fields → empty string (soft-failure). `{{#each}}` on a
// missing or non-List field → Err (structural). `{{#if}}` on a missing
// field → falsy (no error).
//
// Security: returns Value::Html (opaque). Substitution escapes via
// escape_html_chars (reused from string.rs). The template string itself
// is treated as trusted code (written by the .mlog programmer, not user
// input) — `template_render` is INTENTIONALLY NOT in SVG_AUTO_ESCAPE_BUILTINS
// (see report Block 7 for justification).

use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;

// ── AST ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum TemplateNode {
    /// Literal HTML text (between tags).
    Text(String),
    /// `{{ var }}` — auto-escaped substitution. `raw=true` for `{{{ var }}}`.
    Var { path: String, raw: bool },
    /// `{{#if cond}} ... {{else}} ... {{/if}}`
    If {
        cond: Cond,
        then: Vec<TemplateNode>,
        else_: Vec<TemplateNode>,
    },
    /// `{{#each items}} ... {{/each}}`
    Each {
        list_path: String,
        body: Vec<TemplateNode>,
    },
}

/// A condition for `{{#if}}`. Either a bare variable path (truthy/falsy)
/// or a single comparison `var == "literal"` / `var != "literal"`.
#[derive(Debug, Clone)]
enum Cond {
    /// `{{#if var}}` — truthy check via Value::as_bool.
    Truthy(String),
    /// `{{#if var == "x"}}` — string equality.
    Eq(String, String),
    /// `{{#if var != "x"}}` — string inequality.
    Ne(String, String),
}

// ── Tokenizer + parser ────────────────────────────────────────────────

/// Parse a template string into a Vec<TemplateNode>.
/// Errors on unmatched `{{#if}}`/`{{#each}}` (missing close tag).
fn parse_template(src: &str) -> Result<Vec<TemplateNode>, String> {
    let tokens = tokenize(src);
    let mut pos = 0;
    parse_nodes(&tokens, &mut pos, None)
}

#[derive(Debug, Clone)]
enum Token {
    Text(String),
    /// `{{ ... }}` — content already trimmed. `raw=true` for `{{{ ... }}}`.
    Tag {
        content: String,
        raw: bool,
    },
}

/// Split the source into Text and Tag tokens.
/// `{{{`/`}}}` (triple) take priority over `{{`/`}}` (double).
fn tokenize(src: &str) -> Vec<Token> {
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

/// Parse a sequence of nodes until either end-of-tokens or a closing tag
/// is encountered.
/// `close_with = None` for the top-level (parses to end).
/// `close_with = Some("/if")` or `Some("/each")` for nested blocks.
/// Stops (without consuming) when it sees its own close tag, OR —
/// when `close_with = Some("/if")` — when it sees `{{else}}` (the
/// caller decides whether to parse an else-body).
fn parse_nodes(
    tokens: &[Token],
    pos: &mut usize,
    close_with: Option<&str>,
) -> Result<Vec<TemplateNode>, String> {
    let mut nodes = Vec::new();
    while *pos < tokens.len() {
        let tok = &tokens[*pos];
        match tok {
            Token::Text(s) => {
                nodes.push(TemplateNode::Text(s.clone()));
                *pos += 1;
            }
            Token::Tag { content, raw } => {
                // Check if this is a closing tag we're looking for.
                // For {{#if}} blocks, `{{else}}` is ALSO a stop point
                // (the if-parser decides whether to parse an else body).
                if let Some(cw) = close_with {
                    if content == cw {
                        return Ok(nodes); // caller consumes the close tag
                    }
                    if cw == "/if" && content == "else" {
                        return Ok(nodes); // caller handles else
                    }
                }
                // Block-open tags
                if let Some(rest) = content.strip_prefix("#if ") {
                    *pos += 1;
                    let cond = parse_cond(rest.trim())?;
                    let then_body = parse_nodes(tokens, pos, Some("/if"))?;
                    // After then_body, we should be at /if or else.
                    if *pos >= tokens.len() {
                        return Err(format!(
                            "template_render: unmatched {{#if {}}} — missing {{/if}}",
                            rest.trim()
                        ));
                    }
                    // Peek: is this an {{else}}?
                    let mut else_body = Vec::new();
                    let mut consumed_else = false;
                    if let Token::Tag { content: c2, .. } = &tokens[*pos] {
                        if c2 == "else" {
                            *pos += 1; // consume else
                            else_body = parse_nodes(tokens, pos, Some("/if"))?;
                            consumed_else = true;
                        }
                    }
                    // Now must be at /if
                    if *pos >= tokens.len() {
                        return Err(format!(
                            "template_render: unmatched {{#if {}}} — missing {{/if}}{}",
                            rest.trim(),
                            if consumed_else {
                                " (after {{else}})"
                            } else {
                                ""
                            }
                        ));
                    }
                    match &tokens[*pos] {
                        Token::Tag { content: c3, .. } if c3 == "/if" => {
                            *pos += 1;
                        }
                        _ => {
                            return Err(format!(
                                "template_render: expected {{/if}} to close {{#if {}}}, found {:?}",
                                rest.trim(),
                                tokens[*pos]
                            ));
                        }
                    }
                    nodes.push(TemplateNode::If {
                        cond,
                        then: then_body,
                        else_: else_body,
                    });
                } else if let Some(rest) = content.strip_prefix("#each ") {
                    *pos += 1;
                    let list_path = rest.trim().to_string();
                    let body = parse_nodes(tokens, pos, Some("/each"))?;
                    if *pos >= tokens.len() {
                        return Err(format!(
                            "template_render: unmatched {{#each {}}} — missing {{/each}}",
                            list_path
                        ));
                    }
                    match &tokens[*pos] {
                        Token::Tag { content: c3, .. } if c3 == "/each" => {
                            *pos += 1;
                        }
                        _ => {
                            return Err(format!(
                                "template_render: expected {{/each}} to close {{#each {}}}, found {:?}",
                                list_path,
                                tokens[*pos]
                            ));
                        }
                    }
                    nodes.push(TemplateNode::Each { list_path, body });
                } else if content == "/if" || content == "/each" || content == "else" {
                    // A close tag we don't expect here is a structural error.
                    return Err(format!(
                        "template_render: unexpected {{{{{}}}}} — no matching opening block",
                        content
                    ));
                } else {
                    // Plain variable: `var` or `obj.field`
                    *pos += 1;
                    nodes.push(TemplateNode::Var {
                        path: content.clone(),
                        raw: *raw,
                    });
                }
            }
        }
    }
    if let Some(cw) = close_with {
        return Err(format!(
            "template_render: unmatched block — expected {{{{{}}}}} before end of template",
            cw
        ));
    }
    Ok(nodes)
}

/// Parse the condition part of `{{#if cond}}`.
/// `cond` is one of:
///   - `var` (truthy)
///   - `var == "literal"` (string equality)
///   - `var != "literal"` (string inequality)
fn parse_cond(src: &str) -> Result<Cond, String> {
    let s = src.trim();
    // Try `==` first
    if let Some(idx) = s.find("==") {
        let lhs = s[..idx].trim();
        let rhs = s[idx + 2..].trim();
        let lit = parse_string_literal(rhs)?;
        return Ok(Cond::Eq(lhs.to_string(), lit));
    }
    if let Some(idx) = s.find("!=") {
        let lhs = s[..idx].trim();
        let rhs = s[idx + 2..].trim();
        let lit = parse_string_literal(rhs)?;
        return Ok(Cond::Ne(lhs.to_string(), lit));
    }
    Ok(Cond::Truthy(s.to_string()))
}

/// Parse a double-quoted string literal `"..."` — supports `\"` escapes.
fn parse_string_literal(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return Err(format!(
            "template_render: comparison literal must be a double-quoted string, got {:?}",
            s
        ));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

// ── Renderer ──────────────────────────────────────────────────────────

/// Resolve a dotted path against `data`. Returns `&Value` if found.
/// Path can be:
///   - `var`            — direct field lookup
///   - `obj.field`      — one-level nested field lookup
///   - `this`           — the implicit `this` from `{{#each}}`
///     (the current item; caller passes item as `data`)
///   - `this.field`     — field of the current item
fn resolve_path<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    // `this` (alone) → the data value itself.
    if path == "this" {
        return Some(data);
    }
    // `this.field` → field of the data value (the current item).
    if let Some(tail) = path.strip_prefix("this.") {
        let tail = tail.trim();
        if tail.is_empty() {
            return None;
        }
        return match data {
            Value::Struct { fields, .. } => fields.get(tail),
            _ => None,
        };
    }
    // Generic `obj.field` — one-level nested field lookup.
    if let Some((head, tail)) = path.split_once('.') {
        match data {
            Value::Struct { fields, .. } => match fields.get(head) {
                Some(Value::Struct { fields: inner, .. }) => inner.get(tail),
                _ => None,
            },
            _ => None,
        }
    } else {
        match data {
            Value::Struct { fields, .. } => fields.get(path),
            _ => None,
        }
    }
}

/// Convert a Value to its string rendering (for substitution).
/// Mirrors the `Display` impl but returns an owned String.
fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Float(f) => format_float(*f),
        Value::Bool(b) => b.to_string(),
        Value::Unit => String::new(),
        Value::List(items) => {
            // Comma-joined, like Display impl
            let parts: Vec<String> = items.iter().map(value_to_string).collect();
            parts.join(", ")
        }
        Value::Struct { type_name, fields } => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, value_to_string(v)))
                .collect();
            format!("{} {{ {} }}", type_name, pairs.join(", "))
        }
        // Opaque types — print their type tag, not contents (matches Display).
        Value::Html(_) => "[Html]".to_string(),
        Value::Query(_) => "[Query]".to_string(),
        Value::Secret(_) => "[Secret]".to_string(),
        Value::Encrypted(_) => "[Encrypted]".to_string(),
        Value::Hash(_) => "[Hash]".to_string(),
        Value::Session(_) => "[Session]".to_string(),
        Value::HttpResponse { status, .. } => format!("[HttpResponse {}]", status),
        Value::Subgraph(snap) => format!(
            "[Subgraph {} nodes, {} edges]",
            snap.nodes.len(),
            snap.edges.len()
        ),
        Value::Fluid(variants) => {
            let best = variants.iter().max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            match best {
                Some(v) => value_to_string(&v.value),
                None => "()".to_string(),
            }
        }
        Value::Reflex(id) => format!("[Reflex#{}]", id.0),
    }
}

/// Format a float the same way `Display` does for `Value::Float` —
/// mirrors the `format!("{}", f)` used by the `Display` impl. Kept here
/// to ensure the renderer's output matches what crosscheck expects on
/// both backends (no surprising `1.0` vs `1` divergences).
fn format_float(f: f64) -> String {
    format!("{}", f)
}

/// Evaluate a Cond against `data`.
fn eval_cond(cond: &Cond, data: &Value) -> bool {
    match cond {
        Cond::Truthy(path) => match resolve_path(data, path) {
            Some(v) => v.as_bool().unwrap_or(false),
            None => false,
        },
        Cond::Eq(path, lit) => match resolve_path(data, path) {
            Some(v) => {
                let s = value_to_string(v);
                &s == lit
            }
            None => lit.is_empty(),
        },
        Cond::Ne(path, lit) => match resolve_path(data, path) {
            Some(v) => {
                let s = value_to_string(v);
                &s != lit
            }
            None => !lit.is_empty(),
        },
    }
}

/// Render a slice of nodes against `data`, appending to `out`.
fn render_nodes(nodes: &[TemplateNode], data: &Value, out: &mut String) -> Result<(), String> {
    for node in nodes {
        match node {
            TemplateNode::Text(s) => out.push_str(s),
            TemplateNode::Var { path, raw } => {
                let val = resolve_path(data, path);
                if let Some(v) = val {
                    let s = value_to_string(v);
                    if *raw {
                        out.push_str(&s);
                    } else {
                        out.push_str(&escape_html_chars(&s));
                    }
                }
                // Missing field → empty string (soft-failure per Block 5 decision)
            }
            TemplateNode::If { cond, then, else_ } => {
                if eval_cond(cond, data) {
                    render_nodes(then, data, out)?;
                } else {
                    render_nodes(else_, data, out)?;
                }
            }
            TemplateNode::Each { list_path, body } => {
                let list_val = resolve_path(data, list_path);
                match list_val {
                    Some(Value::List(items)) => {
                        for item in items {
                            // Inside the each block, `data` IS the current item.
                            // `{{ this }}` and `{{ this.field }}` resolve via
                            // resolve_path on the item directly.
                            render_nodes(body, item, out)?;
                        }
                    }
                    Some(other) => {
                        return Err(format!(
                            "template_render: {{#each {}}} expected List, got {}",
                            list_path,
                            other.type_name()
                        ));
                    }
                    None => {
                        return Err(format!(
                            "template_render: {{#each {}}} — path not found in data",
                            list_path
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

// ── Builtin entry point ──────────────────────────────────────────────

/// `template_render(template, data) -> Html`
///
/// - `template` — String containing Mustache/Handlebars-like template syntax
/// - `data`      — Struct (or any Value) used for variable resolution
///
/// Returns `Value::Html` (opaque — cannot be concatenated or printed).
pub fn builtin_template_render(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!(
            "template_render() requires 2 arguments (template, data), got {}",
            args.len()
        ));
    }
    let template = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "template_render: argument 1 (template) must be String, got {}",
                other.type_name()
            ));
        }
    };
    let data = &args[1];

    let nodes = parse_template(&template)?;
    let mut out = String::with_capacity(template.len());
    render_nodes(&nodes, data, &mut out)?;
    Ok(Value::Html(out))
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }

    fn struct_(pairs: Vec<(&str, Value)>) -> Value {
        let mut fields = HashMap::new();
        for (k, v) in pairs {
            fields.insert(k.to_string(), v);
        }
        Value::Struct {
            type_name: "Anon".to_string(),
            fields,
        }
    }

    #[test]
    fn simple_var_substitution_escapes() {
        let template = "<p>Hello, {{name}}!</p>";
        let data = struct_(vec![("name", s("<script>alert(1)</script>"))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => {
                assert_eq!(h, "<p>Hello, &lt;script&gt;alert(1)&lt;/script&gt;!</p>");
            }
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn triple_brace_does_not_escape() {
        let template = "<div>{{{trusted_html}}}</div>";
        let data = struct_(vec![("trusted_html", s("<b>bold</b>"))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "<div><b>bold</b></div>"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn missing_field_renders_empty() {
        let template = "[{{missing}}]";
        let data = struct_(vec![("present", s("yes"))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "[]"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn if_truthy_then_branch() {
        let template = "{{#if show}}visible{{/if}}";
        let data = struct_(vec![("show", Value::Bool(true))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "visible"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn if_falsy_else_branch() {
        let template = "{{#if show}}yes{{else}}no{{/if}}";
        let data = struct_(vec![("show", Value::Bool(false))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "no"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn if_eq_literal() {
        let template = "{{#if role == \"admin\"}}admin panel{{else}}user panel{{/if}}";
        let data = struct_(vec![("role", s("admin"))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "admin panel"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn each_over_list_of_structs() {
        let template = "{{#each items}}<li>{{this.name}}: {{this.qty}}</li>{{/each}}";
        let data = struct_(vec![(
            "items",
            Value::List(vec![
                struct_(vec![("name", s("A")), ("qty", Value::Float(1.0))]),
                struct_(vec![("name", s("B")), ("qty", Value::Float(2.0))]),
                struct_(vec![("name", s("C")), ("qty", Value::Float(3.0))]),
            ]),
        )]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => {
                assert_eq!(h, "<li>A: 1</li><li>B: 2</li><li>C: 3</li>");
            }
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn each_nested_inside_if() {
        let template = "{{#if show}}{{#each items}}{{this}},{{/each}}{{/if}}";
        let data = struct_(vec![
            ("show", Value::Bool(true)),
            ("items", Value::List(vec![s("x"), s("y"), s("z")])),
        ]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "x,y,z,"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn each_on_non_list_errors() {
        let template = "{{#each items}}x{{/each}}";
        let data = struct_(vec![("items", s("not a list"))]);
        let err = builtin_template_render(&[s(template), data]).unwrap_err();
        assert!(err.contains("expected List"), "got: {}", err);
    }

    #[test]
    fn unmatched_if_errors() {
        let template = "{{#if show}}hello";
        let data = struct_(vec![("show", Value::Bool(true))]);
        let err = builtin_template_render(&[s(template), data]).unwrap_err();
        assert!(err.contains("unmatched"), "got: {}", err);
        assert!(err.contains("{{/if}}"), "got: {}", err);
    }

    #[test]
    fn each_on_missing_path_errors() {
        let template = "{{#each missing}}x{{/each}}";
        let data = struct_(vec![]);
        let err = builtin_template_render(&[s(template), data]).unwrap_err();
        assert!(err.contains("not found"), "got: {}", err);
    }

    #[test]
    fn obj_field_access_one_level() {
        let template = "{{user.name}}";
        let data = struct_(vec![("user", struct_(vec![("name", s("Alice"))]))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "Alice"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn empty_each_renders_nothing() {
        let template = "[{{#each items}}x{{/each}}]";
        let data = struct_(vec![("items", Value::List(vec![]))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "[]"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn truthy_nonempty_string() {
        let template = "{{#if name}}has-name{{/if}}";
        let data = struct_(vec![("name", s("Bob"))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "has-name"),
            _ => panic!("expected Html"),
        }
    }

    #[test]
    fn falsy_empty_string() {
        let template = "{{#if name}}has-name{{else}}no-name{{/if}}";
        let data = struct_(vec![("name", s(""))]);
        let out = builtin_template_render(&[s(template), data]).unwrap();
        match out {
            Value::Html(h) => assert_eq!(h, "no-name"),
            _ => panic!("expected Html"),
        }
    }
}
