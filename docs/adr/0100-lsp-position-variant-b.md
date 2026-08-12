# ADR-0100: LSP Position Resolution via Text Search (Variant B)

**Status:** Accepted
**Date:** 2026-08-10
**Narad:** #68

## Context

When `mlog-lsp` was excluded from the workspace (narad #59), the root cause was
missing methods on `Declaration` in `ast.rs`: `name()`, `kind_str()`,
`type_info()`, `span()`. Two architectural options existed for providing
source positions to the LSP server:

- **Variant A:** Track real spans in the parser/AST. Every AST node carries
  `(start_line, start_col, end_line, end_col)` from the parse phase. LSP reads
  them directly.

- **Variant B:** No span tracking in the parser. LSP resolves positions
  post-hoc by searching the source text for `keyword <name>` patterns.

## Decision

**Variant B** — text-based position resolution inside `mlog-lsp`.

### Rationale

1. **Parser untouched.** The pest-based parser in `parser.rs` works correctly
   for all 27 declaration types. Adding span tracking would require changes to
   every production rule and struct construction — high risk of regressions for
   no functional benefit outside LSP.

2. **Separation of concerns.** Position resolution is an LSP-specific concern.
   The interpreter, VM, and semantic analyzer do not need source positions.
   Keeping span logic in `mlog-lsp` avoids polluting the core AST with
   LSP-specific data.

3. **Adequate accuracy.** Text search for `keyword <name>` at line start
   (after whitespace) handles all current declaration syntaxes. For the rare
   case of a non-standard pattern, a fallback whole-word name search covers
   entity records and similar declarations.

4. **Low maintenance.** When new declaration types are added, only
   `declaration_keyword()` and `kind_str()` need a new entry — both are
   exhaustive matches on `Declaration` variants, so the compiler enforces
   completeness.

## Implementation

- `Declaration::span()` always returns `Span::unknown()` — a zero-span
  placeholder.
- `mlog-lsp::find_declaration_span()` searches for `keyword <name>` patterns
  in source text to produce actual positions.
- `mlog-lsp::build_symbols_with_text()` combines AST declarations with
  text-based position resolution.
- `ast.rs` gains `Span` struct, `name()`, `kind_str()`, `type_info()` methods
  on `impl Declaration` (27-variant exhaustive match).

## Consequences

- LSP positions are approximate (line-level, keyword-aligned) rather than
  token-precise. This is acceptable for go-to-definition and hover.
- If a future need arises for precise token spans in the core (e.g., error
  reporting with exact positions), Variant A can be adopted incrementally
  by adding span tracking to the parser without changing the LSP layer.
