# ADR-0029: Type-Safe HTML via Opaque Html Type

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.2 — Type-Safe HTML Templates

## Context

Web applications must render HTML from user-supplied data. The dominant vulnerability in web apps is XSS (Cross-Site Scripting), caused by injecting unescaped user input into HTML output.

Traditional approaches rely on developers remembering to escape every interpolation. This is unreliable — Metalogos should make XSS structurally impossible at the language level.

## Decision

Introduce an **opaque `Html` value variant** that cannot be constructed by concatenating raw strings. `Html` values can only be produced via the `render(template, data)` builtin, which auto-escapes all `{{ variable }}` interpolations.

```mlog
let name = request.get("name")          // String
let page = render("template.mlog", {    // Html
    name: name
})
// page is Html, not String
// String + Html is a compile error
// Html + Html concatenation is allowed (trusted HTML composition)
```

### Design Rules

1. `Html` is a distinct variant in `Value::Html(String)` — no `From<String>` conversion exists.
2. `render()` is the only builtin that produces `Html`. All `{{ expr }}` in templates are escaped via HTML entity encoding.
3. Raw HTML injection uses `{{ raw expr }}` — only allowed when `expr` is already of type `Html`.
4. String concatenation with `+` rejects `String + Html` and `Html + String` at runtime.

## Prior Art

- **Askama (Rust):** Typed templates, auto-escaping by default, `Html` wrapper type.
- **Yesod (Haskell):** `Html` is a newtype; cannot accidentally inject plain text.
- **Elm:** `Html` values built via the virtual DOM — raw HTML requires explicit `Html.text` or `Html.Attributes`.

## Consequences

- **Positive:** XSS is structurally impossible — unescaped user input cannot enter an HTML context.
- **Positive:** The type system guides developers toward safe patterns automatically.
- **Neutral:** Developers must use `{{ raw }}` for intentionally trusted HTML fragments, which is more verbose.
- **Negative:** Cannot serialize `Html` to a JSON API response without an explicit `to_string(html)` conversion, preventing accidental HTML in JSON bodies.
