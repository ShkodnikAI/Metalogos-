# ADR-0102: Native SVG Graphics & Diagrams

**Status:** Accepted (MVP scope — Наряд №74)
**Date:** 2026-08-14
**Narad:** #74

## Context

Metalogos had **no native graphical primitives** — `svg_rect`, `svg_circle`,
`chart_bar`, `canvas_new` and similar all returned `unknown function`. Diagrams
for reports were produced either by external Python sidecars (LLM-driven,
non-deterministic, geometrically unreliable) or not at all.

Owner decision (Наряд №74): rendering belongs to `mlog`, not a proxy. Reasons:

1. **Determinism.** Diagrams built via parameterized functions produce
   byte-identical output for identical inputs. LLM-generated SVG text does
   not — coordinates drift, lines wobble, labels misalign on each call.
2. **Geometric correctness.** Straight lines, exact coordinates, and
   proportional sizing are guaranteed by arithmetic, not by model guesswork.
3. **Project direction.** Consistent with the ongoing migration of logic
   from Python sidecars into Metalogos native code.

This ADR covers the MVP scope: Level 1 (SVG primitives), Level 2 (design
tokens), Level 2.5 (wow-effects: sketchy filter, icons, callouts), and
the first Level 3 chart type (`chart_bar`). Remaining chart types
(timeline, pyramid, quadrant, loop, flowchart) are deferred to future
narads — each gets its own golden test.

## Decision

### Three-layer architecture

```
Level 1 (primitives)         Level 2 (tokens)        Level 3 (charts)
─────────────────────         ──────────────         ──────────────
svg_rect                      diagram_style          chart_bar
svg_circle                      └─> paper             (timeline, pyramid,
svg_line                        └─> ink                quadrant, loop,
svg_text                        └─> accent             flowchart — future)
svg_path                        └─> muted
svg_group                       └─> rule
svg_canvas
                              Level 2.5 (effects)
                              ──────────────
                              svg_sketchy_filter
                              svg_icon
                              svg_callout
```

Each layer builds on the previous one. Level 3 chart functions take a
`diagram_style` Struct as their `style` argument — they do not read
global state. This mirrors the SQL-function pattern (each takes explicit
parameters, no hidden context).

### Level 1: SVG primitives

Seven builtins returning XML fragments:

| Builtin | Arity | Returns |
|---------|-------|---------|
| `svg_rect(x, y, w, h, fill [, stroke])` | 5–6 | `<rect ... />` |
| `svg_circle(cx, cy, r, fill)` | 4 | `<circle ... />` |
| `svg_line(x1, y1, x2, y2, stroke [, width])` | 5–6 | `<line ... />` |
| `svg_text(x, y, content, font_size, fill [, anchor])` | 5–6 | `<text>...</text>` |
| `svg_path(d, fill [, stroke])` | 2–3 | `<path d="..." />` |
| `svg_group(children, transform)` | 1–2 | `<g [transform="..."]>...</g>` |
| `svg_canvas(w, h, viewbox, children)` | 4 | `<svg>...</svg>` (complete document) |

**Security invariant:** `svg_text` content and `svg_callout` text are
ALWAYS XML-escaped via `escape_html_chars` (the same function used by
the existing `escape_html` builtin). This makes `<script>` injection
impossible at the runtime level — `<` becomes `&lt;`, `>` becomes
`&gt;`, `"` becomes `&quot;`, `'` becomes `&#39;`, `&` becomes `&amp;`.

**Path data exception:** `svg_path`'s `d` argument is a path-data
mini-language (e.g. `M 10 10 L 100 100`). Escaping it would break the
syntax. Instead, `svg_path` rejects path data containing `<` or `>` —
this preserves XML structure without breaking valid paths.

### Level 2: Design Tokens

`diagram_style({paper, ink, accent, muted, rule})` returns a Struct
with type_name `"DiagramStyle"` and exactly 5 fields. All 5 are
required — missing any returns an error.

The 5 canonical tokens (sourced from the `cathrynlavery/diagram-design`
reference repo):

- `paper` — background color
- `ink` — primary text/element color
- `accent` — emphasis color (used for 1–2 elements per diagram max)
- `muted` — secondary text color
- `rule` — axis/divider color (typically a low-opacity variant of ink)

**Single-accent rule** (documentation, not runtime-enforced): `accent`
is intended for 1–2 focal elements per diagram. Using it on every
element defeats its purpose. Chart functions enforce this where
practical: `chart_bar` colors only the tallest bar with `accent`, all
others with `ink`.

### Level 2.5: Wow-Effects

Three techniques, all cheap to implement (no algorithmic complexity):

**`svg_sketchy_filter(id [, base_freq, octaves, scale, seed])`** —
Returns a `<filter>` element using standard SVG `feTurbulence` +
`feDisplacementMap`. Apply via `filter="url(#id)"` on a `<g>` of shapes.
Critical rule (documented in source): **never apply to text** — makes
labels unreadable. The runtime does not enforce this structurally;
chart functions are responsible for keeping `<text>` outside filtered
groups.

**`svg_icon(name, x, y, size, color)`** — Returns a `<svg>` fragment
containing a Tabler Icons (MIT) path scaled to `size`. Initial set:
`server, laptop, phone, database, cloud, arrow-right, check, warning,
user, document` (10 icons). Uses `stroke="currentColor"` — inherits
color from parent. Unknown names return an explicit error.

**`svg_callout(text, from_x, from_y, to_x, to_y [, intent])`** —
Editorial annotation: italic text + dashed Bezier curve + anchor dot.
Visually distinct from regular diagram connections (those are solid).
Intent: `"neutral"` (default), `"accent"`, `"muted"` — selects color
palette. Recommended limit: max 2 callouts per diagram (documentation,
not runtime-enforced — would complicate the signature).

### Level 3: `chart_bar` (first chart type)

`chart_bar(data: List<{label, value}>, style)` — bar chart with pure
parametric geometry (no graph-layout algorithm).

Layout constants:
- Canvas: 600×400
- Chart area: x=[80, 580], y=[40, 340] (500×300)
- Bar width: `chart_w / N - gap` (gap=20)
- Bar height: `(value / max_value) * chart_h`
- Tallest bar: accent color; others: ink color

**Determinism invariant:** identical inputs produce byte-identical
output. Verified by golden test `chart_bar_golden_3_bars_deterministic`
in `tests/svg_graphics_contract.rs` — runs the same input twice and
asserts equality.

**Limits:** 1–50 bars. Empty list or >50 bars returns an explicit error.

### Security: AST-level lint in `mlog check`

`mlog check` runs an additional pass: `svg_security_lint` in
`src/semantic.rs`. This is **defense-in-depth** — the primary barrier
is runtime escaping in `svg_text`/`svg_callout`, but the lint catches
cases where an attacker could bypass escaping by passing a payload to
a non-escaping argument.

Lint rules:

| Builtin | Arg type | Payload `<script>` | Payload `onX=` | `javascript:` URL |
|---|---|---|---|---|
| `svg_text` content | auto-escaped | WARNING | WARNING | ERROR |
| `svg_callout` text | auto-escaped | WARNING | WARNING | ERROR |
| `svg_path` d | NOT escaped | **ERROR** | **ERROR** | ERROR |
| `svg_canvas` viewbox | NOT escaped | **ERROR** | **ERROR** | ERROR |
| `svg_group` transform | NOT escaped | **ERROR** | **ERROR** | ERROR |
| `svg_sketchy_filter` id | NOT escaped | **ERROR** | **ERROR** | ERROR |

- **ERROR** = potential bypass; runtime cannot catch it.
- **WARNING** = suspicious but auto-escaped at runtime.

Also scans string literals inside `BinaryOp` (concatenation) — catches
`svg_path("M 10 10 " + "<script>", ...)` where the payload is hidden
in a concat expression.

**URL whitelist:** `https://fonts.googleapis.com/` and
`https://fonts.gstatic.com/` (Google Fonts) are the only permitted
external resources — matches the source repo's `self_check.py` rule.
`data:image/svg+xml` URIs are also allowed (common for inline icons).

## Consequences

### Positive

- **Native diagram rendering in Metalogos** — no Python sidecar, no
  LLM drift, deterministic output.
- **Security by construction** — runtime escaping + AST lint = two
  independent barriers against XSS in SVG output.
- **Composable** — `chart_bar` output is a String that wraps cleanly
  in HTML (verified by `chart_bar_output_is_embeddable_in_html` test).
- **Test coverage:** 18 unit tests (`src/builtins/svg.rs`) + 30
  integration tests (`tests/svg_graphics_contract.rs`) + 10 security
  lint tests (`tests/svg_security_lint.rs`) = **58 new tests**.

### Negative

- **Path data not escaped** — `svg_path`'s `d` argument is a
  mini-language that would break if escaped. Mitigated by rejecting
  `<`/`>` in path data at runtime, plus AST lint flagging `<script>`
  in path args as ERROR.
- **Single-accent rule not enforced** — `chart_bar` applies it
  implicitly (tallest bar = accent), but other chart types (when
  added) will need to enforce it themselves. Documentation only.
- **Callout count limit not enforced** — recommended max 2 callouts
  per diagram, but runtime does not count. Would complicate function
  signatures.
- **10 icons only** — initial set covers common office-diagram cases.
  Extending requires adding path data to `icon_path_data` match arm.

## Test Coverage

| Test file | Tests | Coverage |
|---|---:|---|
| `src/builtins/svg.rs` (unit) | 18 | All builtins, error paths, escaping |
| `tests/svg_graphics_contract.rs` (integration) | 30 | Full pipeline, golden test, composability |
| `tests/svg_security_lint.rs` | 10 | AST lint: script/onX/javascript: in all contexts |
| **Total** | **58** | |

## References

- Source repo: `cathrynlavery/diagram-design` (design tokens, callout
  pattern, sketchy filter approach, single-accent rule)
- Tabler Icons (MIT): https://tabler.io/icons — `svg_icon` path data
- W3C SVG 1.1 spec: `feTurbulence`, `feDisplacementMap` filter primitives
- ADR-0057: existing `audit_program` static-analysis pattern (this ADR
  extends it with SVG-specific security checks)
- ADR-0071: HTTP retry — same "contract → impl → test" pattern

## Future Work (out of MVP scope)

- `chart_timeline` — events on a time axis (pure geometry)
- `chart_pyramid` — funnel/trapezoid geometry
- `chart_quadrant` — 2D positioning (risk/benefit, importance/urgency)
- `diagram_loop` — circular layout with deterministic trigonometry
  (5–8 stations, 1 hub, ≤1 focal station)
- `diagram_flowchart` — requires Sugiyama layered layout algorithm
  (the only item needing graph theory, not just parametric geometry)
- Animation/motion — explicitly out of scope for PDF-style reports
