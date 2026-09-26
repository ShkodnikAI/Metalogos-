# Key Features

## Key Features

### Security by Design

The compiler enforces structural security invariants — these are errors, not warnings:

```mlog
// doc-test: skip
// SQL injection — non-literal query() is rejected by Category A
let user = query("SELECT * FROM users WHERE id = $1", [id])

// Secret leak — env() to respond()/print() is rejected by Category A
entity token: Secret = env("API_KEY")
respond(token)   // [SECRET_LEAK] via mlog audit / Category A checks
print(token)     // runtime error: print() refused: Secret values cannot be printed

// XSS via LLM — unsanitized LLM output to respond() is rejected by Category A
let reply = call_llm(prompt)
respond(reply)   // [HTML_INJECTION] — use render() or escape_html()

// Templates: render(Name, args...) substitutes {{ var }} with HTML-escaped values (naryad 115)
// {{{ var }}} skips escaping — trusted author code only, not caught by audit
```

### OWASP Top 10 Coverage

| OWASP Item | Mechanism | Boundary |
|---|---|---|
| A01 Broken Access Control | `requires=[role]` on routes, `require` assertions in patterns | — |
| A02 Cryptographic Failures | `Secret`/`Encrypted`/`Hash` opaque types; AES-256-GCM | `Secret` printable only in tests; `env()` to sink caught by Category A |
| A03 Injection | `Query` opaque type (parameterized only); `Html` type; template `{{ }}` auto-escapes | `{{{ raw }}}` skips escaping; interprocedural taint not tracked |
| A04 Insecure Design | Security-first language design; opaque types; static audit | See Known boundaries (section 2 above) |
| A05 Security Misconfiguration | CSP/HSTS/X-Frame-Options headers set by default | Developer can override headers |
| A06 Vulnerable Components | Minimal dependency tree; `cargo-audit` in CI; Dependabot | Transitive deps may have unpatched CVEs (see RUSTSEC ignores) |
| A07 Auth Failures | `hash_password`/`verify_password`; HMAC-SHA256 signed sessions | — |
| A08 Data Integrity Failures | CSRF double-submit on mutating routes; signed cookies | CSRF not needed for token-authenticated APIs |
| A09 Logging Failures | Audit log for `require` failures, `adapt` mutations, `unsafe_html` | — |
| A10 SSRF | LLM sandbox `forbidden: [network]`; outbound HTTP restricted | Sandbox is opt-in; `http_get`/`http_post` available outside sandbox |

### Two Execution Backends

- **Tree-walking interpreter** — full feature support, used for `mlog run` and `mlog serve`
- **Bytecode VM** — 46 instructions, stack-based, used for `mlog compile` + `mlog run file.mbc`
- **JIT** — experimental scaffold, not part of the build (see ADR-0073)

### 430 Built-in Functions

String ops, math, collections, type conversion, LLM/AI, HTTP, JSON, file I/O, KV store, session memory, encryption, authentication, HTTP server, templates, databases, Telegram/Discord bots, time/date/calendar, geolocation, weather, reminders, cron, goals, todos, memory tree, preferences, approval workflows, fuzzy matching, hashline editing, context compaction, budget awareness, replay logging, policy enforcement, PDF processing (classify, extract, OCR), typed semantic memory (FTS5 BM25 + cosine RRF), SMTP/IMAP email, CalDAV/CardDAV calendar and contacts, native SVG graphics, and more. See [REFERENCE.md](REFERENCE.md) for the full list.

### Span-Aware Error Messages (ADR-0111)

Every AST node carries source position (`start_line:start_col–end_line:end_col`).
Semantic errors include line numbers for easy debugging:

```mlog
entity User { name: String }
entity User { age: Int }   // line 2: duplicate entity type: User
```

Parser fills `Span` from `pest::Span` via `Span::from_pest()`;
`Expr::span()` and `Declaration::span()` provide uniform access.
Tuple-style variants (e.g. `StringLit(String)`) converted to struct
variants with named fields + `span` — zero change in execution logic.

### Cargo feature gates

Optional compile-time gates (`svg`, `chart`, `diagram`, `template`, `llm`, `server`;
default = all). Measured impact of building without the server stack: ~7% smaller
release binary — see [ADR-0104](docs/adr/0104-feature-gating-measured-impact.md).

### Native SVG/Graphics Subsystem

44 builtins, hand-rolled in pure Rust — zero external SVG/charting/
rendering dependencies. Every text-carrying function is covered by a
dedicated static security lint (`SVG_AUTO_ESCAPE_BUILTINS` /
`SVG_NO_ESCAPE_BUILTINS`), the same "unsafe by construction" discipline
as the rest of the language:

```mlog
// doc-test: skip
let style = color_palette("energy", "dark")
let chart = chart_bar(revenue_data, style)
let flow  = diagram_flowchart(nodes, edges, style)
let poster = InfographicPoster("Q3 Revenue", "energy", stats, narrative)
```

- **Primitives** — `svg_rect`, `svg_circle`, `svg_path`, `svg_text`,
  `svg_group`, `svg_canvas` (+ named presets: `doc_inline`,
  `slide_16x9`, `social_og`, print A4), `svg_icon` (9 glyphs)
- **8 chart types** — bar, donut, line, scatter, area, heatmap, radar
  (multi-series), boxplot (real quartile math)
- **21 diagram types** — flowchart (topological layering, cycle
  detection), tree, org chart, sequence, timeline, Gantt, state machine,
  ER, swimlane, Venn (2/3-circle), quadrant, pyramid, and more
- **`color_palette`** — HSL-cascade generator, 5 intents × 2 modes
- **`template_render`** — `{{ var }}`, `{{#if}}/{{else}}`, `{{#each}}`,
  hand-written recursive parser, no templating crate
- **Anti-overlap engine** — iterative label-collision resolution,
  wired into `diagram_timeline`
- **`infographic_qa`** — advisory contrast/saturation/density checks
- **`html_render`** — headless-browser screenshot, hardened `exec()`
  underneath (real timeout + kill, file audit log, no shell
  interpretation); network isolation is a documented caller
  responsibility, not an OS-level guarantee
- **`std/infographic.mlog`** — `InfographicPoster`, `InfographicDashboard`,
  `InfographicComparison`, `InfographicTimeline`

Delivered across naryads №77–92 (see [CHANGELOG.md](CHANGELOG.md)).
Found and fixed one critical, previously-invisible bug along the way:
the bytecode VM discarded `try`'s result on the success path since
naryad №14 — masked for the project's entire history because every
existing `try`-using test only checked the error path.

### Human Intelligence Layer

Persona system with memory trees, mood tracking, and human-like response generation — inspired by [OpenHuman](https://github.com/tinyhumansai/OpenHuman):

```mlog
human_create("Alice", "friendly, professional, curious")
human_remember("Alice", "project", "building AI assistant in Metalogos", 0.8)
human_mood("Alice", "excited", 0.9)
let reply = human_respond("Alice", "How is my project going?")
```

### Cron Scheduler

Fuzzy matching (Jaro-Winkler), content-verified hashline editing (CRC32), context compaction, budget awareness, replay logging, shell policy enforcement:

```mlog
let code = "1:3f|fn main() {"
let text = "alpha\nbeta\ngamma"
let messages = ["a", "b", "c", "d", "e", "f"]
let events = ["e1", "e2", "e3", "e4", "e5"]
fuzzy_match("metalogos", "metalogus")           // 0.96
fuzzy_find_best("Mikhail", ["Michele", "Mikael"])  // FuzzyMatch{index:1, candidate:"Mikael", score:0.82}
hashline_read(code)                                  // "1:3f|fn main() {"
hashline_edit(text, [{op:"set_line", ref:"1:6a", content:"..."}])
compact_list(messages, 2, 4)                          // protect head/tail, compress middle
budget_check(8, 10)                                   // BudgetStatus{level:"warning", pct_remaining:20}
policy_check("vim file.txt")                          // PolicyResult{allowed:false, reason:"blocked: interactive..."}
replay_snapshot(events)                              // ReplaySnapshot{seq:0, count:5, snapshot:"..."}
```

Cron scheduler, recurring and one-shot jobs, dispatches both builtins and user patterns:

```mlog
cron_run("*/30 * * * *", "HealthCheck")     // every 30 min
cron_run("0 9 * * 1-5", "MorningReport")     // weekdays 09:00
cron_list()   // list all jobs
```

### Memory Tree

Three-level hierarchical memory: L0 (raw entries) → L1 (chunk summaries) → L2 (global summary). Admission-gated storage, keyword-relevance retrieval with scoring, and automatic compression.

### Typed Memory with Hybrid Search (ADR-0072, ADR-0073)

Memory entries carry a type tag (`persona`, `episodic`, `instruction`, `fact`) for differentiated recall. SQLite-backed persistence with FTS5 BM25 keyword index + cosine similarity, merged via Reciprocal Rank Fusion (k=60). Top-K recall with type filtering:

```mlog
memorize("user likes spicy food", 0.9, "persona")
let results = recall_top_k("food preferences", 5, "persona")
```

### Goals & Todos

Built-in goal tracking with deadlines and todo management with priorities — all persisted in KV store.

### Reflex — Local Neural Models (ADR-0112, ADR-0114, ADR-0117)

The Reflex pillar trains, predicts, persists, and distills local neural models — the LLM acts as a teacher, the local head as a student. Models are declared as first-class language constructs (`reflex Name { ... }`), opaque to `Value` (only a `ReflexId` handle enters the value system — weights never leak).

**Classification, not generation** — per ADR-0117 §3, `reflex` classifies into a closed-set label list (`labels: ["a", "b", ...]`). Free-form text generation is explicitly out of scope. This is the same boundary that applies to `reflex_seq` (sequence models) — symmetric ADR-0117 enforcement.

```mlog
// doc-test: skip
// 1. Declare a classifier — input dim, dense layers, closed label set.
reflex SentimentClassifier {
  input: embedding(2)
  layers: [dense(8, relu), dense(2, softmax)]
  labels: ["positive", "negative"]
  seed: 42
}

// 2. Train on labeled data — returns Struct {loss, accuracy, metric, threshold_met}.
let result = reflex_train(SentimentClassifier, [
  [0.1, 0.2, 0.0],   // features + class_idx (last element)
  [0.8, 0.9, 1.0],
  // ... ≥10 samples for 80/20 holdout (ADR-0115)
], 200.0, "accuracy", 0.85)

// 3. Predict on new input — returns Fluid with label variants, sorted by confidence.
let prediction = reflex_predict(SentimentClassifier, [0.15, 0.25])
// prediction → Fluid{ "positive" (0.92), "negative" (0.08) }
```

**Distillation** — a `learnable pattern` can `distill_to` a reflex model: the LLM is called during the *teaching* phase, then the local head replaces it once confidence exceeds the `fallback_if` threshold.

```mlog
// doc-test: skip
learnable pattern Classify(text: String) -> String {
  distill_to: SentimentClassifier
  fallback_if: confidence < 0.85
  call_llm(system_prompt, text)
}
```

**Persistence** — `reflex_save`/`reflex_load` serialize trained weights to the SQLite database configured by `memory { persist: "path.db" }` (ADR-0116). Shape mismatches between saved and current declarations are explicit errors, never silent corruption.

**Architecture blocks (ADR-0118, ADR-0119)** — `reflex_seq` declares sequence models for transformer-family layers: `attention` (multi-head with RoPE, Naryad #183), `rms_norm` / `swiglu` / `transformer_block` (Naryad #184). These require the optional `candle` feature (`cargo build --features candle`), not unconditional — when the feature is off, `reflex_seq` declarations produce a clean error naming the missing feature. Sequence models classify the *whole* sequence into one label (mean pooling + Dense head), not token-by-token generation (ADR-0117 §3 boundary, symmetric).

**Grouped-Query Attention (GQA, Naryad #188)** — `attention` accepts an optional third parameter for the number of KV heads:

```mlog
// doc-test: skip
reflex_seq GqaModel {
  input: embedding(64)
  seq_len: 16
  layers: [attention(8, 64, 2)]   // 8 query heads, dim 64, 2 KV heads (GQA)
  labels: ["signal", "noise"]
  seed: 42
}
```

When the third parameter is omitted (`attention(8, 64)`), behaviour is identical to standard multi-head attention (Naryad #183) — `n_kv_heads` defaults to `n_heads`. When `n_kv_heads < n_heads`, K and V weights are smaller (`[dim, kv_dim]` instead of `[dim, dim]`) and repeated along the head axis during the attention computation (Llama 2/3 architecture). Constraints: `n_kv_heads > 0`, `n_kv_heads ≤ n_heads`, `n_heads % n_kv_heads == 0`.

**Stacked transformer blocks (Naryad #190)** — multiple `transformer_block` entries can be chained in the `layers` list. Each block gets its own independent, deterministically different weights (via `VarMap` prefixing — not identical copies):

```mlog
// doc-test: skip
reflex_seq StackedTransformer {
  input: embedding(8)
  seq_len: 4
  layers: [
    transformer_block(2, 8, 16),
    transformer_block(2, 8, 16)   // different weights from block 0
  ]
  labels: ["a", "b"]
  seed: 42
}
```

Each layer receives `seed.wrapping_add(layer_index)` for deterministic weight initialization, and registers its parameters under unique `VarMap` names (`block0_attn_w_q`, `block1_attn_w_q`, etc.) so that `backward()` populates gradients for all blocks — gradients flow through the entire stack, not just the last layer.

### Vision — Generative Media (ADR-0122, ADR-0125)

**Weights run parked — no production PNG yet.** The Vision pillar (images, ADR-0122) is feature-gated (`--features vision`, which implies `candle`) and ships compiler-level provenance and supply-chain gates (ADR-0125); the real-weights run (runbook №237) has not been executed, so no production image has been generated. **Naryad #294 (2026-09-14) — formal No-Go**: the container preflight failed (4 GB RAM vs 64 required; 10 GB disk vs 40 required; no GPU). Revisit date — when hardware is allocated. Report: `docs/research/naryad-294-vision-realw-no-go.md`. The Parked status remains (No-Go → not lifted).

### Voice — Speech Synthesis & Cloning (ADR-0143–0146)

The Voice pillar (speech synthesis, zero-shot voice cloning, voice design) is feature-gated (`--features voice`, implies `candle`) and off-by-default. Four ADRs define the scope: [ADR-0143](docs/adr/0143-voice-scope.md) (scope — TTS, cloning, voice-design; non-scope: pre-training, singing, streaming, voice conversion), [ADR-0144](docs/adr/0144-voice-value-registry.md) (opaque `Value::Audio`/`Value::Voice` handles + `VoiceRegistry` with encrypted-at-rest voiceprints), [ADR-0145](docs/adr/0145-voice-security-gates.md) (5 security gates: consent, provenance, privacy, taint, shared `MODEL_WEIGHTS_UNSAFE`), [ADR-0146](docs/adr/0146-voice-wedge.md) (wedge: Chatterbox Multilingual V3 MIT/MIT 500M primary, Kokoro-82M Apache 82M warm-up). Skeleton built (Naryad #302): `src/voice/mod.rs` with `VoiceId`/`AudioId`, `VoiceRegistry`, `KNOWN_VOICE_MODELS` SSOT, 6 stub builtins. Speaker encoder contract (Naryad #303): 192-dim L2-normalized embeddings, `VoiceStore` (SQLite, encrypted BLOB), consent ledger (GDPR Art. 9). Real ECAPA encoder + AES-256-GCM encryption deferred to phase A4.

### Always-on Agent Runtime — Session, Typed Memory, Duplex (ADR-0172–0175)

The registry Phase 4 lane ("Always-on, память, забывание") is landed and real (wave 9, naryads №348/№349/№350/№351/№352/№426):

- **Session model** (№348/[ADR-0172](docs/adr/0172-session-model.md)) — `session_login`/`session_logout` over a live registry, `session_wake` (keyword|event|schedule), `session_take_interrupt` with the typed priority ladder `low < normal < high < critical`, the duty profile (`profile duty { materialization: denied; surfaces: local_only }` — a compile-time contour, №349); every transition is an Action-Ledger record.
- **Typed Memory<K>** (№350) — label-typed containers (`public`/`private`), private storage is consent-gated (№335) and AES-256-GCM encrypted at rest under a per-subject key, reads return `Secret` (the lattice/redact contract is the only egress), every operation is a ledger record.
- **The derived-from graph and cascading forgetting** (№351/[ADR-0173](docs/adr/0173-memory-derived-graph-cascade.md)) — `memory_forget_cascade` deletes the full descendant closure; the delete is a GRANTED linear action (ADR-0155: scope `memory:forget:<container>`, `GRANT_*` refusal matrix, the post-success `irreversible.memory_forget` ledger record); `memory_retain` pins a subtree — a retained node VETOES the whole forget (provenance integrity by construction, fuzz-pinned against an independent model); the dry-run preview (`memory_cascade_preview`) is the №280 discipline.
- **Persistent typed memory** (№351/[ADR-0175](docs/adr/0175-tick-context.md) §3.5) — the env anchor `METALOGOS_MEMORY_DB` makes a bundled-rusqlite file DB the authoritative store (additive-only DDL, write-through, load-on-open); restart-stable decryption via `METALOGOS_MEMORY_MASTER`.
- **Directed audio effects and the duplex channel** (№352/[ADR-0174](docs/adr/0174-directed-audio-effects-duplex.md)) — `listen`/`speak` are typed EFFECT WORDS in the №324 trail vocabulary: a pattern using an undeclared direction is a COMPILE error; the duplex channel binds a live session and implements barge-in over the session priority ladder (the preempted stream ends typed, every transition is a ledger record).
- **The tick context** (№426/[ADR-0175](docs/adr/0175-tick-context.md)) — cron ticks execute in the PROGRAM context (the same db/schema/patterns stor-set as routes) on a blocking thread while the scheduler holds no lock; the schema-as-code DDL replays into every context (declaration order does not matter); `sqlite::memory:` is unified across contexts. Verified nightly by the serve-soak (11/0 GREEN).

### Video — I2V Pipeline & Provenance (ADR-0147–0151)

The Video pillar is feature-gated (`--features video`, implies `candle`) and off-by-default. Since №309 ([ADR-0151](docs/adr/0151-video-i2v-pipeline.md)) the pipeline builtins are **real implementations** on the №310 tiny seeded tensors (CPU, milliseconds, seed-deterministic — the no-stubs template; production-weights inference stays a documented №294-class No-Go):

- `video_render(decl, prompt[, ref_first[, ref_last]])` — T2V (2 args) / I2V first-frame anchor (3) / two-anchor first–last contract (4). Seed = `sha256(model | prompt)`; reference hashes are recorded in the `VideoManifest`; anchors are pinned exactly in the final latent after every Euler step ([ADR-0151 D1](docs/adr/0151-video-i2v-pipeline.md)).
- `frame_interp(handle, factor)` — RIFE-class latent interpolation (2x/4x) with exact endpoint preservation (ADR-0151 D2); `video_extend(handle, extra)` — clip continuation anchored on the source's last latent frame (ADR-0151 D3).
- `av_mux(video, audio)` — deterministic `.mlgv.av` sidecar container pairing VideoId ↔ AudioId (Voice pillar) with frame-aligned timestamps and recorded A/V drift (ADR-0151 D4).
- `video_export(handle, path)` — signed-by-construction `.mlgv` container (manifest + watermark embedded); **unsigned export does not exist** — the runtime `VIDEO_UNSIGNED_EXPORT` gate refuses manifest-less artifacts (ADR-0151 D5).
- Security: `UNTRUSTED_FRAME` advisory taint in `audit.rs` — an I2V reference from user input / http / file is flagged (Warning in `mlog audit`; loud compile error on the check path) until the screen+consent path lands in V6 (ADR-0149 D5, ADR-0151 D6).

### Cross-Pillar Composition (ADR-0151)

The three generative pillars compose across modalities through opaque handles and shared provenance: a Vision-class reference frame (pixels) feeds `video_render(kind: i2v)` (Video), and the composed clip is muxed with a Voice-pillar `AudioId` via `av_mux` — one `VideoManifest` carries the whole chain (`ref_hash`, `source_sha`, `audio_ref`). The E2E "voiced scene" (frame → video → interp → extend → mux with AudioId → export with manifest) runs seed-deterministic on tiny weights in CI (Naryad #309). Full LikenessToken consent mechanics across the pillars land in phase V6 (ADR-0149 D6).

---
