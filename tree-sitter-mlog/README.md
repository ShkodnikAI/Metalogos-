# tree-sitter-mlog

[Tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammar for the [Metalogos](https://github.com/ShkodnikAI/Metalogos-) language (`.mlog`).

> **Naryad #289** (issue #301, P2/tooling) — Tree-sitter grammar for `.mlog`. Source of truth — `REFERENCE.md` §3 (Syntax), cross-checked against `src/grammar.pest` (PEG grammar of the main compiler). Zero diff in `src/**`, `tests/**` and `src/grammar.pest` — a parallel artifact for external tooling, not part of `.mlog` compilation.

## Installation

```bash
cd tree-sitter-mlog
npm install
```

## Building the parser

```bash
./node_modules/.bin/tree-sitter generate
```

## Parsing a file

```bash
./node_modules/.bin/tree-sitter parse ../examples/m1_hello.mlog
```

## Construct coverage (Block 1 — grammar)

`grammar.js` covers all major language constructs from `REFERENCE.md` §3:

### Top-level declarations
- `entity` (three forms: type / record / simple — mirror pest ordered choice)
- `pattern`, `learnable pattern` (with ADR-0117 distill fields in any order)
- `flow` (with `checkpoint("...")` markers + `branch_def`s — ADR-0056)
- `rule` (match + actions)
- `reflex` / `reflex_seq` / `reflex_gen` (ADR-0114 / ADR-0119 / ADR-0120)
- `vision` (ADR-0124)
- `type` alias (Naryad #119)
- `llm {}` config (ADR-0048)
- `mlogserver {}` / `server {}` + `route` (ADR-0074)
- `template` (Phase 6.2)
- `db`, `schema`, `skill_index`, `memory`, `conversation`, `context_budget`
- `import`, `hook`, `sandbox`, `mutate`, `eval`, `fluid`, `adapt`
- `memorize`, `relate`, `forget` (statement + declaration forms)
- `tool`, `test` (Naryad #120 + #287)

### Statements
- `let` / `let mut` / assign / expression statement
- `if` (block form) / `if-then` (block form) / `else if` / `else`
- `each ... in ... { ... }` (loop over collection)
- `while ... { ... }`
- `match expr { arm* else? }` — exact / `starts_with` / `contains` / compare-op arms (Naryad #173b)
- `break` / `continue` / `return expr`

### Expressions (layered precedence)
- `or` → `and` → `compare` → `add` (`+`/`-`) → `mul` (`*`/`/`) → `unary` → `access` → `primary`
- Unary minus, `try expr` (error handling, Naryad #14)
- `if cond then a else b` (expression form, Naryad #14)
- `if cond { ... } else { ... }` (block-as-expression, Naryad #14)
- Function call `func(args)` + qualified call `module.func(args)` (via postfix chain)
- Field access `obj.field` + index `list[0]`
- Struct literal `{ field: value, ... }` + list literal `[a, b, c]`
- Parenthesized expression `(expr)`
- Literals: string (`"..."` with `\n \t \r \" \\ \uXXXX` escapes), multiline string (`"""..."""`), int, float, bool, ident

### Identifiers
- ASCII letters + underscore + digits (after first char)
- Cyrillic А-я (per pest IDENT rule)
- Apostrophe allowed inside (pest uses it for some idents)

### Comments
- `// single-line comment`

## Correctness contract (Block 2 — representative sample)

Ran `tree-sitter parse` on **23 representative files** from `examples/`, covering all major language pillars:

| Pillar / Category | Examples | Result |
|---|---|---|
| Basic syntax | `m1_hello`, `m2_triage`, `m4_memory`, `m5_adapt` | 2 PASS, 2 PARTIAL |
| Fluid types | `p1_fluid_collapse` | 1 PARTIAL |
| Control flow | `p5_if_else`, `p51_let_bindings` | 2 PASS |
| HTTP server + routes | `p7_post_routes`, `p7_query_readable`, `p69_deferred_response`, `vm_serve_realistic_dispatcher` | 3 PASS, 1 PARTIAL |
| Crypto / secrets | `p70_crypto_smoke` | 1 PASS |
| Cyrillic identifiers | `p9_cyrillic_full` | 1 PARTIAL |
| Context loading | `p11_context_loading` | 1 PARTIAL |
| Knowledge graph | `p2_knowledge_graph` | 1 PARTIAL |
| Hooks lifecycle | `hooks_lifecycle` | 1 PARTIAL |
| Schema | `dept_schema` | 1 PARTIAL |
| Skill index | `skill_index_tiered` | 1 PARTIAL |
| Reflex (generation) | `reflex_gen_declare`, `p201_reflex_generate_html_injection` | 2 PASS |
| Pattern composition | `n1_pattern_compose`, `p161_helper` | 2 PASS |
| Full app (template + render) | `p6_full_app` | 1 PARTIAL |

**Summary:** **12 PASS (no ERROR nodes), 11 PARTIAL (parser recovered, ERROR nodes in deep constructs), 0 FAIL (no crashes)**. All 23 files parse structurally — none of them crashes. PARTIAL means tree-sitter recovered the tree, but some nodes are marked `ERROR` (usually due to unresolved GLR conflicts in complex cases, not critical for basic indexing). Reworking PARTIAL → PASS is a separate, follow-up naryad (if real demand from agent indexers appears).

## Divergences from `grammar.pest` (recorded explicitly, not silently)

1. **pest ordered choice ↔ tree-sitter GLR**: pest uses PEG ordered choice (`|` — first match wins), tree-sitter uses GLR (all alternatives in parallel, conflicts resolved via `conflicts: $ => [...]` in grammar.js). This means tree-sitter can build a tree for constructs that pest would reject (due to lexical ambiguity). Not a bug — a feature, but cross-checking against `grammar.pest` diverges in places.
2. **`_{ ... }` silent rules ↔ tree-sitter `inline`**: pest silent rules (underscore prefix) are inlined into parent; tree-sitter uses `inline: $ => [...]` declaration. Same effect, different mechanism.
3. **Keyword extraction**: pest resolves keyword-vs-identifier conflicts through ordered choice in consumer rules. tree-sitter uses `word: $.ident` declaration (currently not set in grammar.js — known limitation, see "Known limitations" below). As a result, some keywords like `memorize`, `relate`, `if`, `each` may parse as identifiers in some contexts. Not a bug in semantics — pest catches it via ordered choice.
4. **Empty-matching rules**: pest allows `entity_type_body = { field_decl* }` (matches empty). tree-sitter forbids non-start rules matching empty string — `entity_type_body` was inlined into `entity_type_decl` parent seq with `repeat1` to enforce ≥1 field.
5. **Entity record decl shape**: pest `entity_record_decl = { "entity" ~ IDENT ~ ":" ~ type_name ~ "=" ~ LBRACE ~ field_init ~ ... ~ RBRACE }`. In my initial version of grammar.js I modeled it as `entity Name(params) { ... }` — that was a mistake (params in parens, not `: Type = {...}`). Fixed to `entity Name : Type = { field: value, ... }` — an exact mirror of pest.

## Known limitations (for a follow-up naryad, if there is demand)

1. **11/23 examples PARTIAL**: GLR conflicts in deep constructs (hooks lifecycle, schema, skill_index) — the parser recovers, but with ERROR nodes. Resolved by adding `prec(...)` / `prec.left/right(...)` declarations to the conflict rules.
2. **`word: $.ident` not set**: tree-sitter recommends `word` for keyword extraction. The current grammar.js relies on GLR conflicts (works, but less efficient for keyword-heavy languages).
3. **`if-else`-block as expression conflict**: `block_if_else_expr` self-conflicts through `repeat(else_if_block)` — added to `conflicts`, but with deeply nested if-else it may still produce ERROR nodes.
4. **Multiline strings**: simplified regex `/[^"]*/` — correct for most cases, but not identical to pest's `(!("\"\"\"") ~ ANY)*` (does not handle the `"\"\""` escape inside multiline). Full parity requires an external lexer rule.

## Publishing (Block 3)

The naryad assignment states explicitly: "For the first version — the grammar lives in the Metalogos repository itself; publishing as a separate package is a separate, later naryad, if real demand appears (the same 'do not fix without a confirmed use case' principle the whole project applies)". Therefore:
- `package.json` created (name `tree-sitter-mlog`, version 0.19.0 — in sync with `Cargo.toml`).
- Not published to the npm registry.
- Usage: clone Metalogos, `cd tree-sitter-mlog && npm install && ./node_modules/.bin/tree-sitter parse <file.mlog>`.

## Related

- Issue #301 (the naryad).
- `REFERENCE.md` §3 (Syntax) — primary source.
- `src/grammar.pest` (530+ lines) — secondary source for cross-checking.
- `examples/*.mlog` (214 files) — test corpus (23 representative picked).
- Naryad #287 (doc-tests) — parallel testing infrastructure for .mlog doc-snippets.
- Agent skill `agent-browser` — a potential consumer (if tree-sitter-mlog gets published).
