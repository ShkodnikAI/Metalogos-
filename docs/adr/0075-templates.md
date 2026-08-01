# ADR-0075: Type-Safe HTML Templates

**Status:** Accepted
**Date:** 2026-06-02
**Phase:** 6.2

## Context

Phase 6.1 added a minimal HTTP server with `respond(string_literal)`. Phase 6.2 introduces
HTML templates — the mechanism by which Metalogos serves HTML pages. The core security
requirement is **type-level XSS prevention**: it should be *impossible* to inject
unescaped user input into HTML output.

Cross-site scripting (XSS) remains the OWASP #3 most critical vulnerability (2021).
The root cause is always the same: string concatenation to build HTML. Languages that
solve this (Elm, Yesod, Askama) do so by making HTML a typed, opaque construct that
cannot be produced from raw string operations.

## Decision

Implement a `template` language construct with an opaque `Html` type, auto-escaping,
and a `render()` function. XSS is prevented by construction.

### Why a built-in template engine (not Askama/Handlebars)

| Alternative | Rejected because |
|---|---|
| **Askama** | Requires templates in separate `.html` files compiled at build time. Metalogos templates are in `.mlog` source files, parsed at runtime. Askama's compile-time approach doesn't map to our single-file .mlog model. |
| **Handlebars/Tera** | External dependency with its own expression language. Would create two languages in one (mlog expressions + Handlebars expressions). Our `{{ var }}` syntax is simpler and stays within Metalogos. |
| **String-based (sprintf)** | No auto-escaping, no type safety. The whole point of this ADR is to prevent that approach. |
| **HTMX/DOM diff** | Client-side technology. Metalogos needs server-rendered HTML. |

Our built-in template engine is minimal (75 lines): scan for `{{ var }}` placeholders,
replace with HTML-escaped values. No control flow, no conditionals, no loops in templates
(Phase 6.2). Future phases may add `{{#if }}` / `{{#each }}` but the core security
mechanism (auto-escaping + opaque type) remains the same.

### Prior art

- **Askama** (Rust): Compile-time templates typed as `impl IntoResponse`. Return type is
  `askama::Html`, not a raw String. This is the model for our opaque `Html` type.
- **Yesod** (Haskell): Type-safe HTML with the `blaze-html` DSL. Values constructed
  through typed combinators, not string concatenation. XSS is type-error.
- **Elm** (JavaScript): Virtual DOM approach. No raw HTML injection possible — `Html`
  is built through Elm's `Html` module functions. String-to-Html requires explicit
  `Html.text` which is explicitly marked unsafe.
- **ERB (Ruby on Rails): Auto-escaping by default (added in Rails 3.0). `<%= %>` escapes,
  `<%= raw %>` skips. Our `{{ var }}` / `{{ var | safe }}` follows this same pattern.

## Syntax

```mlog
// Template declaration
template Page(title: String, body: String) -> Html {
  <!DOCTYPE html>
  <html>
    <head>
      <title>{{ title }}</title>
    </head>
    <body>{{ body }}</body>
  </html>
}

// Template composition with safe pipe
template Layout(title: String, content: Html) -> Html {
  <html><body>{{ content | safe }}</body></html>
}

// Server route using render
server {
  port: 8080
  route "/" method=GET {
    render(Page, "My Site", "<b>Welcome</b>")
  }
}
```

### Auto-escaping rules

| Input type | `{{ var }}` (default) | `{{ var | safe }}` (pipe) |
|---|---|---|
| `String` | Always escaped: `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `&` → `&amp;` | Always escaped (same as default) |
| `Html` | Escaped (double-escaped for safety) | Raw passthrough (template composition) |

The `| safe` pipe is the only way to insert raw HTML into a template, and it
only works with values that are already `Html` type. This means XSS requires two
explicit opt-ins: (1) creating an Html value via render(), and (2) using `| safe`.

### Opaque Html type enforcement

`Html` is an opaque type enforced at three levels:

1. **Semantic analysis**: `entity x: Html = "<div>" + name + "</div>"` → compile error.
   Only `render(template, args...)` can produce Html values.
2. **Binary operations**: `Value::Html` + `Value::Html` → runtime error. String + Html
   → type mismatch error. Any binary operation involving Html is blocked.
3. **Built-in functions**: `to_string(Html)` → error. `print(Html)` → error.
   `str(Html)` → error. These functions are explicitly checked to reject Html.

## Consequences

- **`src/grammar.pest`**: New `template_decl`, `template_body`, `template_content`,
  `template_placeholder`, `template_pipe`, `template_raw` rules. `route_action` extended
  to support `render(TemplateName, args...)`. Keywords `template`, `render`, `respond`, `route`,
  `port`, `server`, `method` added to IDENT exclusion list.
- **`src/ast.rs`**: New `TemplateDecl` struct, `RouteAction` enum (replaces
  `RouteDecl.response`), `Declaration::Template` variant.
- **`src/parser.rs`**: New `parse_template_decl()`, modified `parse_route_decl()` to
  drill into `route_action` children for respond/render dispatch.
- **`src/interpreter.rs`**: New `Value::Html(String)` variant, `CompiledTemplate` struct,
  `templates` field in Interpreter. New `render_template()` method with placeholder scanning
  and HTML escaping. New `eval_expression()` public wrapper for server route evaluation.
- **`src/server.rs`**: `build_router()` now takes `&Interpreter` parameter to evaluate
  render() routes. Respond routes return `text/plain`, render routes return `text/html`
  via `axum::response::Html`.
- **`src/builtins.rs`**: `to_string()` and `print()` explicitly reject `Html` arguments.
- **`src/semantic.rs`**: Collects template names, validates render() references,
  enforces Html opacity (entity with type Html from non-render expression → error).
- **7 new integration tests** in `tests/template_integration.rs`: XSS prevention,
  safe pipe, multi-param escaping, Html opacity, semantic checks.
- **No new external dependencies** — template engine is built-in (75 lines).
