# Upgrade Guide

The migration recipes between the language versions. The truth lives in
the code: every claim below is pinned by a machine check — the refusal
texts in `src/capability.rs` / `src/approval.rs`, the roster constants,
and `tests/naryad_865_migration_path.rs` (the 0.34.1-shaped program run
unmigrated and migrated on BOTH backends). The repeat-gate
`scripts/ci/breaking_change_gate.py` keeps the future deny-path
additions honest: a roster that grows without a `### Changed` BREAKING
row in the CHANGELOG reddens CI.

## 0.34 → 0.38

> **The version line:** `v0.37` was never tagged — the 0.37-numbered
> train (the wave-74 «Portable Refusal Algebra» lane) shipped its
> criteria INSIDE `v0.38.0` (the R38-era subphase→minor binding
> produced a numbered skip; recorded in the [0.38.0] release notes).
> From 0.39.0 the numbering is machine-enforced: the release-train
> gate (№899, the ADR-0189 amendment) refuses a minor gap.

Two runtime postures landed between 0.34 and 0.38, and both refuse
BEFORE the action (fail-closed). A 0.34-era program that calls an
affected carrier without the matching surface refuses at the first
call. The migration is small: one declaration line per carrier class.

### What changed and where it bites

| Since | Rule | The refusal | The carriers |
|---|---|---|---|
| 0.36.0 (№845, ADR-0203 §2.4) | the capability gate (`src/capability.rs::authorize_call`) | `IRREVERSIBLE_NO_GRANT` before the run | `exec`, `exec_argv`, `delete_file`, `git_push`, `http_post`, `http_post_multipart`, `send_message`, `send_photo`, `send_document`, `smtp_send`, `smtp_send_html`, `tts_send` (the `PROFILE_PERMIT_SHARP` roster) |
| 0.38.0 (№861, ADR-0205 §2.6) | the approval gate (`src/approval.rs::authorize_overseen_call`, composed into the same decision surface) | `IRREVERSIBLE_NO_GRANT` before the run | `cron_run` (the `HUMAN_OVERSIGHT` roster) |

Honest boundaries of the table:

- `profile legacy` does NOT bypass either rule — the transitional
  backend flag is not a capability waiver.
- The named `standard` baseline posture (`BASELINE_STANDARD`) permits
  the runtime-state and own-channel classes — `print`, `respond*`,
  `db_insert`, `db_execute` (non-destructive), `cron_add`, `memory_*`,
  `sleep` and the rest of the machine roster run unchanged. Neither
  the sharp set nor `cron_run` is in it.
- `memory_forget_cascade` and `forget` (`GRANT_ALWAYS`) always demanded
  a valid grant argument at the call itself — that contract did not
  change in this range and is not part of this migration.

### Step 1 — the inventory

List the affected calls in your project:

```bash
grep -rEn "\b(exec|exec_argv|delete_file|git_push|http_post|http_post_multipart|send_message|send_photo|send_document|smtp_send|smtp_send_html|tts_send|cron_run)\(" .
```

Every hit is a call site that needs ONE of the paths below. The runtime
refusal is the second inventory: run the program on 0.38.0 and every
uncovered call refuses with the carrier NAMED and the resolution paths
verbatim in the message — the capability refusal names the carrier, the
`builtin:<name>` grant scope, the `allow_carrier` profile line and the
exec host permit (`src/capability.rs::refusal`); the approval refusal
names the `approval_issue("actor", "carrier:cron_run", ttl)` recipe
(`src/approval.rs::approval_refusal`). You never have to guess what
`IRREVERSIBLE_NO_GRANT` wants.

`mlog check` stays static analysis: it does NOT run the gate — the
capability decision is a runtime decision before the action. An
unmigrated program passes `mlog check` and refuses at `mlog run`; a
migrated program passes both. That split is by design, not a gap.

### Step 2 — the minimal migration: the profile permit

Declare the permit at the top of the entry file:

```mlog
profile capability { allow_carrier: "http_post" allow_carrier: "send_message" }
```

The permit authorizes the named carrier for the program — the honest
minimum, appropriate for a program whose delivery targets are part of
its reviewed code. No TTL, no quota: the profile is code, reviewed like
code. Unknown carrier names in `allow_carrier` are refused loudly by
the semantic lane (a typo never becomes a silent no-op).

### Step 3 — the strict migration: the capability grants

The grant path meters the carrier with TTL and quota. The ledger is
the SSOT, and a valid token CONSUMES one use per authorized call at
the gate (ADR-0203 §2.2):

```mlog
pattern T(_t: String) -> String {
  let cap = grant_issue("builtin:http_post", 3600, "n", 10)
  // ... every http_post call within the window consumes one use;
  // the eleventh refuses IRREVERSIBLE_NO_GRANT
  let r = http_post("https://hooks.example.invalid/ingest", payload)
  return type_of(r)
}
```

- the scope `builtin:<name>` covers exactly the named carrier (a wider
  scope covers more — issue the narrowest that fits);
- `ttl_seconds` bounds the window (`0` = no expiry — prefer a real TTL);
- the class tail: `"once"` (single use), `"n", <k>` (k uses), or
  `"unlimited"` within the TTL.

`capability_state(cap)` is the live ledger view (remaining uses,
revocation state); `grant_revoke(cap)` flips the decision fail-closed —
the next call refuses BEFORE the action.

### Step 4 — the cron_run mandate

`cron_run` demands a LIVE approval mandate — the typed record of the
human's decision (№861, ADR-0205 §2.6):

```mlog
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "NightlyArchive")
  let m = approval_issue("owner", "carrier:cron_run", 600)
  let r = cron_run(job.id)
  return r.status
}
```

- the target is the CARRIER — `"carrier:cron_run"` — not the job id and
  not a call digest (the wave-75 binding; a finer per-call binding is a
  future registry transaction);
- the mandate expires after `ttl_seconds`; `approval_revoke("carrier:cron_run")`
  marks the decision withdrawn — the next trigger refuses;
- the mandate is verified AT CALL TIME against the live store — a
  stale cached value cannot cover the call.

### Step 5 — exec: the host permit

`exec` / `exec_argv` accept a THIRD resolution path — the named host
permit (№97):

```bash
METALOGOS_ALLOW_EXEC=1 ./mlog run app.mlog          # the process context
METALOGOS_SERVE_ALLOW_EXEC=1 ./mlog serve app.mlog  # the serve-route context
```

The two flags are context-dependent (the №253 semantics): a
serve-route body opens ONLY through the serve flag, the process flag
does not reach route bodies. The profile and grant paths authorize
`exec` the same way as any other carrier — pick one mechanism, do not
stack three.

### Step 6 — models and the passport (0.35)

0.35 landed the License-Typed Model Layer (№837/№838, ADR-0202): the
backend registry entries carry the ADR-0163 license class, the
`license_note`, the version and the SHA-256 pins. If your program uses
local weights:

- the weights download verifies EVERY file against the registry pin
  (hash AND byte count — `src/backends_weights.rs::verify_pinned_bytes`);
  a mismatch refuses loudly and the file is not written. Nothing to
  migrate — do not bypass the pin;
- check the license class of the model you load is one your deployment
  may use: the passport record is the SSOT, and the machine fact
  `LICENSED_FLOOR` (10000 bp — `docs/metrics.md`) holds the registry
  fully licensed-and-annotated;
- a passport carries a verification date; the freshness window is 180
  days (`PASSPORT_FRESHNESS_DAYS`) — a stale verification is a
  re-verification task, not a silent pass.

## 0.38 → next (№898 — the LLM loud posture, BREAKING)

`call_llm` without ANY LLM configuration now REFUSES at the first call.
The silent local-first mock default (№879) is rolled back (the owner
decision 04.11; ADR-0208 §2 п.2 is amended — the Accepted text): a value
a program builds on is never a mock marker mistaken for a model answer.

**The refusal** names every configuration surface:

```
LLM is not configured — `call_llm` refuses (№898: no LLM env var and no
`llm {}` declaration is set, and the silent mock default is rolled back).
Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for
anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }`
block in the program — or opt in EXPLICITLY to the deterministic offline
mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung
`llm { providers: [{alias: mock, provider: local}] }`).
```

**The migration** — pick ONE per program:

| You had | You do |
|---|---|
| relied on the silent mock answer (the `mock-llm:<hex>` marker) | add `METALOGOS_MOCK_LLM=1` to the environment, or declare the explicit local rung in the program: `llm { providers: [{alias: mock, provider: local}] }` |
| called a real provider through env | set `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai) or `METALOGOS_OPENAI_BASE_URL` — unchanged from №454 |
| routed through an `llm { providers: [...] }` block | nothing — the declared lane is untouched (the SmartRouter composition never read the rolled-back default) |

**The operator surfaces** (serve): the startup prints
`[serve] LLM UNCONFIGURED — every LLM call will refuse at the first call`
when nothing is declared; the built-in `/health` carries the
`x-llm-mode` header — `mock` / `declared` / `unconfigured` (the body
stays exactly `ok`, the №511 probe pin is untouched).

**What did NOT change:** a DECLARED provider without its key still fails
loudly naming `METALOGOS_API_KEY` (№454, on the declared lane); the
SmartRouter `llm {}` semantics, the failover/circuit-breaker contract and
the marker format (`[mock-llm:<hex>]`, №487) are byte-identical.

### The machine that keeps this honest

- `scripts/ci/breaking_change_gate.py` (the blocking
  `breaking-change-gate` CI job, №865): a tag range that adds a name to
  `GRANT_ALWAYS` / `PROFILE_PERMIT_SHARP` / `HUMAN_OVERSIGHT` without a
  `### Changed` BREAKING row naming it in the pending CHANGELOG section
  is RED. The next deny-path lands with its migration row or does not
  land. №898 extends the same gate to the POSTURE-DEFAULT sites (the
  composition sites whose default branch decides refuse-vs-serve —
  `create_llm_backend` is the first): a tag range that adds, removes or
  flips a `posture-default (<site>): serve|refuse` marker without a
  BREAKING row naming the site is RED — a silent default flip cannot
  ship undocumented (the №879 lesson).
- `tests/naryad_865_migration_path.rs`: the 0.34.1-shaped program
  (`http_post` + `send_message` + `cron_run`) — unmigrated it refuses
  with the named carrier and paths; migrated per this page it passes
  both gates on both backends.

## 0.39.0 → 0.39.1 (the security patch — three BREAKING sharp edges)

The unplanned security slice (ADR-0189 §2.6): the audit 12.11's P0 fix
line merged in one day (the dispatch gh#1935 — №948, №953, №958, №954,
№955; the audit probes live in
`docs/projects/metalogos/audit_2026-11-12/`). Three edges cut programs
that compiled (and misbehaved) on v0.39.0 — each refusal is the fix
working, and each has an explicit, loud migration path.

### 1. `mlog test` / `mlog resume` behind the Category-A gates (№953)

On v0.39.0 `mlog test` ran test bodies with NO gates — a "passing" test
run could write the leak file. On 0.39.1 a leak-class program refuses at
the gates (exit ≠ 0) BEFORE any execution; `check --root` is static (it
executes nothing — the import graph resolves through the №523 static
form).

**Migration:** a test body that genuinely needs the ungated execution —
the honest runs pass `--allow-unchecked` explicitly (a loud banner fires
on EVERY such run: the opt-out must never be silent):

```bash
mlog test tests/ --allow-unchecked
```

A program the gates refuse could never have "passed" honestly — the
previously "passing" run was the leak.

### 2. The serve/mcp-serve 500 body is the generic id-form (№955)

On v0.39.0 the 500 body carried the FULL handler error text — including
any string VALUE the text quoted (the audit's H-3: `to_int("TOPSECRET…")`
reached the client). On 0.39.1 the client receives
`Internal error (id: err-N)` (the MCP tools/call: `Error (id: err-N)`);
the full text — masked by the redact policy (string values become
`string(len=N)`) — rides the `[ROUTE_HANDLER_ERROR]` stderr line keyed
by the SAME id.

**Migration:** operators/monitoring that parsed the 500 body must read
the id from the body and the text from the server log (the correlation
is the id). Local development opt-out — `METALOGOS_SERVE_VERBOSE_ERRORS=1`
— restores the old full-text body and fires a loud startup banner; it
must never face untrusted clients.

### 3. Capability permits come from the ENTRY program only (№954)

On v0.39.0 the VM compiler unioned an IMPORTED module's `profile
capability { allow_carrier: … }` into the permit roster — any dependency
could grant itself a carrier (the audit's H-2, probe c03). On 0.39.1 an
imported capability profile refuses loudly
(`[CAPABILITY_PROFILE_IN_IMPORT]`) at check/run/VM-compile/serve/mcp-serve.

**Migration:** move the `profile capability` declaration into the root
(ENTRY) program — the dependency's intent stays, the grant decision
returns to the program owner. For one release window the ENTRY may take

```text
profile legacy { capability_imports: warn }
```

which downgrades the refusal to the loud stderr warning — the roster
STILL never extends and the irreversible sinks still refuse with
`IRREVERSIBLE_NO_GRANT` (fail-closed either way).

The honest boundary the patch does NOT close: the inter-pattern label
tracking (the boundary table B1–B9 in `docs/threat-model.md`) is the
0.40.0 train (№950–№952, after the owner's «принято» on the №949 ADR
amendments) — the advisory `docs/security/advisory-2026-11-12.md` keeps
describing that gap.

## 0.39 → 0.40.0 (№950 — the pattern call boundary, BREAKING)

A program that passed `mlog check` on ≤ 0.39.0 while carrying a secret
or an untrusted value **across a call into a user pattern** now refuses
at compile time. The program was leaking; the check was not looking
(ADR-0154 §12, the contract written by №949, the gap tabulated by №948
as the threat-model rows B1–B4 and proved on the published release
binary by the audit of 12.11).

### What changed

| Through 0.39.0 | From 0.40.0 |
|---|---|
| a call to a user pattern took the join of its ARGUMENTS; the called body was never consulted | the call's label also joins the callee's `return_internal` — the sources evaluated INSIDE its body (§12.2 (а)) |
| an unannotated parameter entered its body as `bottom` (the «open world» of ADR-0154 Appendix A) | each parameter carries the clearance of the sinks it reaches, and the CALL SITE re-decides that clearance with the ACTUAL argument label (§12.2 (б)) |
| only one hop, in one lane (the `TAINT_INTERP` param→return LLM lane) | any call depth: the summaries are computed in reverse topological order, and call cycles take the conservative top (§12.3) |

The diagnostic classes are the existing ones — no new `check_id` was
minted for the boundary (§12.4). A refusal that comes from the boundary
names the chain, so the call site that chose the value is where the fix
belongs:

```
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in
pattern P → Show(…) → print (parameter 0 reaches the sink) carries
label 'private, trusted'; sinks require public; failed threshold:
private-egress
```

### The migration — pick ONE per refusal

| The refusal says | You do |
|---|---|
| the callee's return carries a source (B1/B3/B4 shape: `print(G("a"))`, `respond_html(Gen(q))`, `exec(Sh(c))`) | redact or sanitize INSIDE the callee before returning (`redact(...)` for the private axis, `render`/`template_render` for the HTML axis, an allowlist match for the exec axis) — the sanitizer lowers the label at the point where the knowledge lives |
| a parameter of the callee reaches a sink (B2 shape: `Show(env("SP"))`) | pass a value that clears the sink (not the secret), or sanitize INSIDE the callee before the sink, or sanitize at the call site, or make the requirement EXPLICIT by annotating the parameter — `pattern Show(v: String<public>)` turns the refusal into the honest signature mismatch at the callee's own boundary |
| a recursive pattern refuses and you believe the flow is safe | the cycle's summary is the conservative top by design (§12.3): split the recursion so the sink-bearing part is not inside the cycle, or sanitize before the recursive call |
| you need time | declare the transition window (below) — for ONE release window |

### Sanitizing inside the callee works, and it works PER AXIS

The two axes of the label — confidentiality and integrity — are tracked
across the boundary separately, so a sanitizer inside the callee is
credited for the axis it actually repairs:

```
pattern Store(raw: String) -> String {
  let fp = redact(raw, "hash_only")   // restores the INTEGRITY axis
  kv_set("k-" + fp, "ok")             // the persisted key is now trusted
  return "stored"
}
route "/s" method=GET { return respond(Store(query_param("id"))) }
```

The argument is untrusted but public, the callee repairs the integrity,
and the call site clears. The same parameter handed a SECRET still
refuses, because `hash_only` repairs the integrity and not the
confidentiality — for that axis the downward move is
`redact(x, "secrets")` / `"all"` (one-way) or not sending the secret at
all. The repository's own benchmark fixture
(`benches/fixtures/production_workload.mlog`) is migrated exactly this
way and is worth reading as the worked example: it had been carrying
request fields and model output into `kv_set` through three patterns.

### The transition window

```
profile legacy { interproc: warn }
```

The summaries still run and the findings still appear in the audit
report (`mlog audit`), naming the window; nothing blocks compilation.
It is NARROW by construction: it downgrades only the findings the
summaries ADDED — an intraprocedural refusal in the same program keeps
blocking, and it is not a second `egress: permissive_with_audit`. The
key is removed one release window after 0.40.0 (§12.5), and a program
still relying on it then stops compiling.

### What did NOT change

- The intraprocedural refusals: identical classes, identical texts.
- The `TAINT_INTERP` lane (№292) and its `METALOGOS_TAINT_DEPTH`
  bound — a different mechanism, exactly as narrow as before.
- The analysis cost stays inside the dispatch threshold: +8.5% on the
  largest `examples/` programs, +3.4% on the 2300-line benchmark
  workload, +37.2% on the self-hosted parser (the worst case) and
  +36.2% on a synthetic 200-pattern call chain (§12.3's +50% bound).
- All 475 `.mlog` files of the repository: `mlog check` changed on the
  four B1–B4 pin files and nowhere else.
- Recursion: a mutually recursive program that leaks nothing stays
  clean (the self-hosted parser is the pin), while a leak through a
  recursive pattern refuses.

### Still open after this migration

The intraprocedural label losses (threat-model rows B5–B8 — the `each`
accumulator, the secret in an `http_get` URL, the flow output, the
`Secret` entity read back via `to_string`) are №951, and dynamic
dispatch beyond the static lattice (row B12) is №952. Until those land,
an annotated parameter and a redact before the sink remain the honest
defence on those shapes.

### The in-pattern losses (№951 — the same 0.40.0 window)

Three more shapes that compiled on ≤ 0.39.1 now refuse. All three were
losses, not permissions: the label existed and the gate did not see it.

| The refusal says | What happened | You do |
|---|---|---|
| a label survives an `each` loop it used to lose | the loop body's raises now fold back into the loop-exit environment, as `while` already did | sanitize inside the loop (`redact`, `escape_html`), or build the accumulator from values that clear the sink |
| a secret in a request URL or query | the ADDRESS/QUERY family (`http_get`, `http_download`, `web_search`, `geo_ip`, `weather*`, `series_pull`, `*_fetch_weights`) gates its arguments on confidentiality | put the secret in a HEADER through the headers argument of a sink builtin rather than the URL, or `redact(x, "secrets")` if a fingerprint is what the endpoint needs |
| an entity you declared `Secret` or annotated `<private>` | the entity now seeds from its annotation, its declared type and its initializer — the annotation used to be ignored outright | this is the annotation finally working: either stop materializing the value, or `redact` it at the point of use |

**What did NOT change here:** sending natural-language text to a model.
The CONTENT family (`call_llm`, `call_claude`, `whisper_transcribe`,
`tts_generate`, `mcp_call`, the mail/calendar/card readers) is not gated
at its argument position, because the confidentiality clearance there
collides with the literal-marker rule of ADR-0161 §3 — a prompt that
merely mentions «private» or «confidential» would refuse. That lane stays
with the `TAINT_INTERP` contour, which reports prompt egress without
blocking the ordinary call.

**The source vocabulary.** The registry's `Label::Secret` and
`Label::Network` source classes now carry their class into the lattice:
`env_or` and `generate_key` arrive private, and `web_search`,
`http_download`, `weather`, `imap_read`, `cal_read`, `card_read`,
`geo_ip`, `whisper_transcribe`, `mcp_call`, `series_pull` and the rest of
the network class arrive UNTRUSTED — their answer is remote data. A
program that printed or persisted such an answer directly now refuses, and
the fix is the ordinary one: `escape_html`/`render` for an HTML sink,
validation before a decision, or not persisting remote content.

**Still open in this window:** the flow output (threat-model row B7) is
held by three owner decisions, and the `Label::Internal` source class (94
names) goes to a name-by-name pass of its own. Until both land, a flow
that returns a source value and a read-back from the program's own store
remain unlabelled — stated here so the gap is not mistaken for a
guarantee.

### Pinned by

- `tests/naryad_950_interproc_summaries.rs` — both halves at depths
  1…20, the recursive leak and the benign recursion, the
  no-false-positive cases, the per-axis sanitizer credit, tool
  methods, the three rule families, the transition window (including
  that it does NOT downgrade an intraprocedural refusal), and the
  frozen `check_id` vocabulary.
- `tests/naryad_948_boundary_pins.rs` — the B1–B4 pins (№950) and the
  B5/B6/B8 pins (№951), flipped from «check passes today» into blocking
  leak-suite negatives.
- `tests/naryad_951_in_pattern_losses.rs` — the loop fold in both `each`
  forms and the iterant staying scope-local, the request-URL refusals,
  the deliberate NON-refusal of a prompt literal that mentions a
  confidentiality word, the three entity seeds, the derived registry
  source classes, and the two exclusions with their reasons
  (`world_state`, the `Internal` class).
