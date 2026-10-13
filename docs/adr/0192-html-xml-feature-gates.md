# ADR-0192: The `html` / `xml` capability features — html5ever / xml5ever behind off-by-default flags

- Status: **Accepted** (the owner's dispatch #1222 — the PE-2 wave "PE-2
  (N5-N9, feature-флаги/ADR)"; the stack verdict N5-pre, issue #1214,
  confirmed by the cargo-deny probe 2026-10-12)
- Date: 2026-10-13
- Deciders: the owner (the dispatch), the executor (the stack probe)
- Naryad: N5 (#1215), N6 (#1216); prerequisite N5-pre (#1214)
- Predecessor: [ADR-0122 — vision off-by-default](0122-vision-feature-gate.md)
  (the off-by-default capability-gate rhythm), №518 (the license
  allow-list and the blocking cargo-deny gate)

## §1. Context

The tender agent needs DOM extraction from HTML (the tender pages:
tables, lots, deadlines) and feed/sitemap parsing (XML). The obvious
Rust stack (`scraper` + `selectors` + `cssparser`) is MPL-2.0 — the №518
license policy forbids copyleft in the dependency tree, and the blocking
cargo-deny gate enforces it on every run. The N5-pre audit (#1214)
verified the alternative on the repo's own `deny.toml`:

- `scraper = "0.20"` → licenses **FAILED** (MPL-2.0: `selectors`,
  `cssparser`, `cssparser-macros`, `dtoa-short`; plus LGPL-2.1-or-later
  `r-efi`, ISC `scraper`/`ego-tree`).
- `html5ever = "0.27"` + `markup5ever_rcdom = "0.3"` → licenses **ok**
  (39 crates, ONLY MIT / Apache-2.0 / Unicode-3.0, zero copyleft).
  Bonus: `markup5ever_rcdom` pulls `xml5ever = "0.18.1"` (MIT /
  Apache-2.0) — ONE stack covers both the `html` (N5) and the `xml`
  (N6) features.

## §2. Decision

1. **The parser stack**: `html5ever` (parsing) + `markup5ever_rcdom`
   (the DOM) + `xml5ever` (N6, the XML feeds). NO `selectors`/`cssparser`
   — the CSS selector support N5 needs (tag, `.class`, `#id`, `[attr]`,
   `[attr=value]`, descendant, direct child, comma) is small enough for
   the in-house minimal matcher (~150 lines, no regex, no transitive
   deps).
2. **The feature gates**: `html = ["dep:html5ever", "dep:markup5ever_rcdom", "dep:url"]`
   and `xml = ["dep:xml5ever", "html"]` — off-by-default, NOT in
   `default`/`full` (the ADR-0122 rhythm: vision/voice/video precedents;
   the CI guard forbids the promotion). `xml` reuses the html ops' shared
   text/limit helpers.
3. **URL resolution**: the `url` crate (MIT/Apache-2.0, the WHATWG URL
   standard implementation, servo lineage — the same vendors as
   html5ever) for `html_links` relative→absolute resolution. Hand-rolling
   RFC 3986 resolution is high-risk for zero benefit; the crate is
   feature-gated so the default build never links it.
4. **The loud limits** (the repo's fail-loud discipline): input ≤ 5 MiB
   per call (beyond — loud `HTML_INPUT_TOO_LARGE`), the result ≤ 10 000
   nodes per call (beyond — loud `HTML_RESULT_TOO_LARGE`). Never a
   silent truncate.
5. **Taint**: every extracted string (tag/text/attrs/html/href) carries
   the untrusted label — the builtins are Sources in the audit
   vocabulary (the request_header N2 precedent), the classification
   rationale names the lane.

## §3. Consequences

- The default build (`cargo test`, the goldens, the coverage) links NO
  html/xml code and NO url crate — the surface stays identical for the
  existing consumers.
- The feature-on CI job (`html-tests`, blocking) guards the off-by-default
  contract (grep), builds with the feature, runs the lib tests, the
  golden contract test, and clippy on the gated code — the vision-tests
  job shape (ADR-0122).
- The golden examples that use the html builtins are excluded from the
  default golden suite (the `reflex_seq_*` / `p88_html_render_success`
  skip-list precedent) and exercised by the feature-on contract test.
- The tender agent unblocks: N5 is its blocker (the dispatch wording).

## §4. Alternatives considered

- **`scraper` (+ its selector engine)** — rejected: MPL-2.0 (the №518
  policy), the N5-pre probe FAILED the deny gate.
- **Regex-based HTML extraction** — rejected: broken-HTML tolerance and
  the malformed-document recovery are exactly what html5ever implements
  (the HTML5 spec algorithm); regex HTML parsing is a correctness hazard.
- ** Putting html5ever in the default features** — rejected: the wave's
  discipline is capability gates (the dispatch: "PE-2 (N5-N9,
  feature-флаги/ADR)"); the default build must stay lean.
