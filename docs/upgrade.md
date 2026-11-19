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

## 0.39.1 → 0.40.0 (the value-label second line, №952)

**What changed:** the label now rides the VALUE at runtime on both
backends. A program whose runtime flow the static lattice missed REFUSES
at the sink with `[SINK_CLEARANCE_RUNTIME]` (exit ≠ 0) where 0.39.1
executed the leak silently.

**The migration recipe (in order):**
1. Run `mlog check` — the static line reports the audited shapes first
   (the №950/№951 machinery). Fix every reported site: annotate the
   parameter `String<private>`, or redact before the sink per the
   ADR-0154 §10 table (`redact(x, "all")` → public; `hash_keyed` →
   public; conservative policies stay private).
2. If `mlog check` is clean but `mlog run` refuses: the flow crossed a
   boundary the static lane cannot see (a kv round-trip, a dynamic
   dispatch, a flow edge). The refusal message names the sink, the
   argument position and the deny-reason class — the SAME class the
   static gate would use. Sanitize at the SOURCE (redact before the
   store/sink), never by widening the sink.
3. Escaping does NOT clear a secret: `escape_html(env("K"))` still
   refuses (the A2 rule — escaping restores integrity only). Use
   `redact` with a literal policy; unknown policy words are loud
   runtime errors.
4. `profile legacy { egress: permissive_with_audit }` (the №325
   window, ADR-0161) downgrades the runtime refusals to audit events
   for the ONE transition window — the events stay in the audit log,
   the roster never extends, and the window is the owner's to close.

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

**Still open in this window:** the flow output's integrity half
(threat-model row B13) is measured, not gated — the confidentiality half
closed with №965, the integrity package (the 16 idiom files and the three
candidate forms, ADR-0154 §13) waits for the owner's explicit choice of
the form, and the `Label::Internal` source class (94 names) goes to a
name-by-name pass of its own. Until both land, a flow that ends in
model/remote/own-file output stays unlabelled on the integrity axis —
stated here so the gap is not mistaken for a guarantee.

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

## 0.40.0 → next (the verifiable mandate actor, №957 — BREAKING)

**What changed and where it bites.** A program can no longer approve
itself. On ≤ 0.40.0 the program could issue its own human-oversight
mandate — `approval_issue("owner", "carrier:cron_run", 60)` — and the
`cron_run` call passed: the gate verified the window, the target digest
and the revocation state, but NOT the ORIGIN of the decision (the audit
12.11 probe `4_capability/c05`). Now the mandate record carries
`actor_verified`, the `approval_issue` builtin always records `false`
(a string the program typed is not a human), and the approval gate in
the default mode counts ONLY the VERIFIED class — mandates issued
OUTSIDE the program by the host CLI:

```
./target/release/mlog approve carrier:cron_run --actor owner --ttl 3600
```

The verified mandate lands in the host-owned persisted store
(`METALOGOS_MANDATE_STORE` or `~/.metalogos/mandates.json`) and
survives the restart. The store is on the №455 sensitive-path deny
vocabulary (`.metalogos/**`, `mandates.json`) — no `.mlog` program can
read or write it.

### The migration (one release window)

1. **The minimal (transition) path** — the program keeps working for
   ONE release window, loudly: declare the window key in the ENTRY
   program:

   ```
   profile legacy { approval: warn }
   ```

   A live program-issued mandate opens the human-oversight carrier with
   the `[APPROVAL_ACTOR_UNVERIFIED]` stderr warning. After the window
   the key is REMOVED from the language and a program still carrying it
   is a check error (the §12.5 form).
2. **The strict path** — do it now: remove the `approval_issue` call
   from the program, run the host ritual once per carrier
   (`mlog approve carrier:cron_run --actor <who> [--ttl S]`), and the
   program runs in the default mode with no key. The mandate persists
   across restarts; revoke by marking the store row revoked (the
   in-language `approval_revoke` covers the in-memory class; the
   verified store row is the host's to withdraw — the dedicated
   `mlog revoke` surface follows the owner's acceptance of ADR-0211).
3. The refusal text names the remedy verbatim: the frozen
   `IRREVERSIBLE_NO_GRANT` class (№385) with the `mlog approve` command
   spelled out — no diagnostic vocabulary changed.

### What keeps this honest

- `tests/naryad_957_verified_mandate.rs` — the c05 probe flip on BOTH
  lanes (the default mode refuses a program-issued mandate; the
  mutation that counts one reds this file), the transition window, the
  verified path, the marking visibility, and the store isolation (the
  program cannot read or write `.metalogos/mandates.json`).
- `metalogos-server/tests/naryad_957_cli_approve.rs` — the REAL binary
  e2e: refuse → `mlog approve` → pass on a fresh process (the restart
  half).
- `scripts/mutation_verify_957.sh` — M1 counts a program-issued
  mandate (the c05 pin reds); M2 the same mutation drops the gate_036
  machine fact and `gen_metrics --check` reds (the floor follows the
  fact).
- The APPROVAL_COVERAGE machine fact (gate_036) requires the
  verified-class gate site — the coverage is now a checked fact about
  HUMANS, not about strings; the attestation map carries the «actor
  verification» row.

## 0.40.0 → next (the flow output integrity gate, №967 — BREAKING)

**What changed.** A flow whose pipeline ends in model, remote or own-file
data (`call_llm`, `mcp_call`, the html/xml extraction family,
`read_file` — the language's primary idiom) now REFUSES `mlog check`
with `[HTML_INJECTION]` when its `-> output` terminator is NOT
annotated. The refusal names the form:

```
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main
carries label 'public, untrusted'; ... — the flow output carries
'public, untrusted'; declare `-> output<public, untrusted>` if the
display is intended (ADR-0154 §13.4 form (а))
```

**The migration is one line** — annotate the terminator where the
display is intended:

```metalogos
flow Main { input: String = "doc" -> Summarize -> output<public, untrusted> }
```

The annotation is a CLAIM, not a silencer (ADR-0154 §13.8 п.2, §12.2):

- the declared conf word must be `public` exactly — the confidentiality
  half is not declareable: a flow ending in `env(...)`, a `Secret`
  entity or any private-labeled value refuses with `[PII_EGRESS_OUTPUT]`
  with or without the annotation;
- a declared set WEAKER than the computed edge — `-> output<public,
  trusted>` over an untrusted edge — refuses with
  `[LABEL_ANNOTATION_MISMATCH]`; declare the label the edge actually
  carries, or drop the annotation;
- a dirtier declaration over a cleaner edge covers (honest conservatism
  passes): `-> output<public, untrusted>` on a `public, trusted` edge is
  fine;
- an unparseable annotation (`-> output<bogus>`) is the №322 label
  error before any clearance question.

**Who must migrate.** The corpus sweep on the landed tree (463 files)
found exactly 18 idiom files — the §13.2 measurement's 16 (`v05_call_llm`,
`p100_mcp_echo`, `n5_html_tender`, `n6_feed_parse`, `p76_http_download`,
`v05_file_io`, four `w385_*`, two `w72_*`, `self-host/lexer.mlog`,
`self-host/parser.mlog`, `serr_004/005`) plus `reflex_gen_declare` and
`reflex_gen_from_text` — migrated in the same commit (20 terminators,
every annotation a TRUE claim). FIVE already-refusing files gained the
flow-edge detector as an ADDITIONAL finding and keep refusing
(`l1_injection`, `leak/n33_duty_http_get`, `taint_chain_d3`,
`taint_chain_d4`, `examples/boundaries/b04_exec_via_pattern`) — two
independent detectors on one site is a signal, not a duplicate
(`docs/limitations.md`).

**What did NOT change.** The confidentiality half of the flow edge
(№965): a private edge refuses exactly as before. The annotation
grammar is strictly additive — a program without terminator annotations
parses identically to 0.40.0; only the unannotated idiom flows gained
the refusal. No transition window: the migration is one line per flow,
and the §13.4 form (в) (a `profile legacy` key) stays bridge-only by
decision — if a one-release bridge is ever wanted, it is a separate
naryad.

### Pinned by

- `tests/naryad_966_flow_integrity.rs` — the landed contract: the
  unannotated fixture refuses with `[HTML_INJECTION]` and the refusal
  names the form; the annotated positive passes; the weaker-claim and
  conf-word negatives refuse with `[LABEL_ANNOTATION_MISMATCH]`; the
  confidentiality-unwaivable control; the over-declaration; the №322
  bogus-word parse error; the docs gate (threat-model row B13 = FIXED,
  ADR-0154 §13.7/§13.8/§13.9 present, limitations.md clean); the
  agree-test (the `.expected` pin carries `status: LANDED`).
- `tests/naryad_948_boundary_pins.rs` — B13 flipped into
  `FLIPPED_967_PINS`: the unannotated fixture refuses with the frozen
  class and the form text.
- `scripts/mutation_verify_966.sh` — M1 the integrity arm dropped →
  the negatives red; M2 the whole gate dropped → the conf controls red;
  M3 the claim check skipped → the weaker-claim negative reds; M4 the
  confidentiality half declareable → the annotated control reds. All
  four RED before the push.

## 0.40.0 → next (№977 — the receipt table's echo names, BREAKING)

> **The version line:** the row lands after the 0.40.0 stable cut; the
> section number follows the release-train numbering at publication.

The sink-receipt table (`src/sink_receipt.rs`) claimed that
`mem_set`, `session_set` and `mem_delete` return «the key the value
was stored under, never the value». The claim was false — the handlers
return THE VALUE (`args[1]`, `args[2]`) and the value removed FROM THE
STORE respectively — so the receipt rule was a laundering path: a
program could store untrusted data with `mem_set`, read the return
back, and push it to a public channel past the integrity gate. On the
non-maskable probe (`query_param` — untrusted, not a secret) the
`HTML_INJECTION` refusal that fires on the bare
`query_param → respond` control vanished on all three paths; a
`mem_delete` with a literal key did not refuse at all.

### What changed and where it bites

| Since | Rule | The refusal | The paths |
|---|---|---|---|
| the №977 merge (the `mem_set`/`session_set`/`mem_delete` retraction) | the receipt exemption is RETRACTED — the return label is the join of the call's arguments again | the same classes the bare control gets: `HTML_INJECTION` (untrusted → public channel), on top of any class the call itself already carries (`TAINT_PERSISTENCE`, `UNTRUSTED_EGRESS_NETWORK`) | `mem_set(k, v) → …`, `session_set(s, k, v) → …`, `mem_delete(k) → …` where the RETURN reaches a sink |

**A program that passed `mlog check` on ≤ 0.40.0 while reading the
return of one of the three builtins into a public channel now
REFUSES — this is the fix, not a regression.** Programs that never
reuse the returns (the common shape: call for the effect, ignore the
value) change nothing, and the corpus sweep on the landing tree found
zero verdict changes across all `.mlog` files of the repository.

### The migration

1. **Do not echo the returns into a sink.** The return of a store
   builtin is store content — treat it like any other value: if it
   must reach a public channel, sanitize it first
   (`redact(v, "all")` for a value that must not carry its origin, or
   `render`/`escape_html` for a display context — the same recipe the
   bare `query_param → respond` control has always required).
2. **If the stored value is meant to be public**, label it at the
   store boundary, not at the read: store already-sanitized data
   (`mem_set("k", redact(x, "all"))`), and the return joins the
   sanitized label honestly.
3. **`kv_set` is unaffected** — it returns `Unit`, and its call-side
   persistence gate (`TAINT_PERSISTENCE`) is unchanged. The rest of
   the receipt table is re-verified against the handler bodies and
   stands (see the №977 PR description for the 21-name table).

### Pinned by

- `tests/naryad_977_echo_receipts.rs` — the control + the three echo
  paths on the non-maskable probe (`HTML_INJECTION` PRESENT on each),
  the `kv_set` correct-verdict control, the structural mutation guard
  (the three names out of the table AND still `Role::Sink`), the
  rest-of-the-table guard, the two-lane agreement.
- `src/builtins/memory.rs` `mod n977_actual_returns` — the four
  no-parsing return pins (mem_set → args[1], session_set → args[2],
  mem_delete → the removed store value, kv_set → Unit); the
  receipt-table docstring cannot drift from the handlers without
  breaking them.
- The mutation: re-adding any of the three names to
  `SINK_RECEIPT_NAMES` reds the suite (verified before the push — the
  pin answers with the pre-№977 verdict `["TAINT_PERSISTENCE"]`).

## 0.40.0 → next (№978 — the external sink returns, BREAKING)

> **The version line:** the row lands after the 0.40.0 stable cut; the
> section number follows the release-train numbering at publication.

A delivery sink's return is now what it always was at runtime: EXTERNAL
data. `http_post` returns the remote response body, `exec`/`exec_argv`/
`git_push` return host command stdout, the messaging family
(`send_message`, `send_photo`, `send_document`, `answer_callback_query`,
`edit_message_text`, `tts_send`) returns the remote API response body,
`human_respond` returns the human's typed answer. On ≤ 0.40.0 the static
line treated these returns as the join of the call's arguments — bottom
for literal arguments — so the SAME remote body `http_get` refused
(`Role::Source`, №951) reached a public channel trusted when it came
from `http_post` (the №316 one-direction-per-builtin asymmetry, the
audit 18.11 finding C).

### What changed and where it bites

| Since | Rule | The refusal | The paths |
|---|---|---|---|
| the №978 merge | the return of the 12 named delivery sinks carries `public, untrusted` (`sink_receipt::EXTERNAL_RETURN_NAMES` — one table, both label lanes) | the same classes the `http_get` control always got: `HTML_INJECTION` at a public sink or on an unannotated flow-output edge | `exec("ls") → respond`, `http_post(url, b) → respond`, `send_message(...) → print(...)`, a pattern returning any of them into a public sink or the flow terminator |

**A program that passed `mlog check` on ≤ 0.40.0 while reading one of
these returns into a public channel now REFUSES — this IS the fix.**
Programs that ignore the returns (call for the effect) change nothing;
the corpus sweep on the landing tree changed 3 files (two
already-refusing duty/leak fixtures gained an additional detector, one
demo example took the display-intent annotation), 0 healed, and the
§12.3 analysis budget is intact (parser 1612→1608ms, workload
5738→5751ms).

### The migration

1. **Declare the display intent** where the value is meant for display:
   the §13.4 form (а) annotation on the flow terminator
   (`-> output<public, untrusted>` — a CHECKABLE claim; a weaker claim
   is the loud `[LABEL_ANNOTATION_MISMATCH]`, the annotation cannot
   become a silencer).
2. **Sanitize before the echo** where the value feeds a sink argument:
   `escape_html(v)` / `render(v)` for a display context, `redact(v,
   "all")` for a value that must not carry its origin — the same recipe
   the `http_get → respond` control has always required.
3. **A secret in the arguments still refuses at the CALL** — the return
   rule never masks the argument gates; nothing about the call-side
   posture changed.

### Reviewed and deliberately NOT in the rule

The email family (`smtp_send`, `smtp_send_html`, `imap_mark_read`,
`imap_move`) returns the constant typed Struct `…Result{ok: true}`; the
media/model generation family returns GENERATED artifacts, not ingested
remote data; the store-content return class (`memory_export` and kin)
is the №978-a owner question in `docs/limitations.md`. Every exclusion
carries its verified body reason in the №978 PR description (the full
120-sink table).

### Pinned by

- `tests/naryad_978_external_returns.rs` — 9 tests: the `http_get`
  control (preserved), the three probe forms (the direct call, the
  pattern-return/flow-edge form, the git_push addition), the
  argument-gate non-masking, the exact-table structural guard, the
  reviewed-not-external guard, the two-lane agreement.
- The mutation: removing any name from `EXTERNAL_RETURN_NAMES` reds the
  corresponding pin (verified RED before the push — the pin answers
  with the pre-№978 PASS).

## 0.40.0 → next (№980 — the quarantine receipt carries the reason's label, BREAKING)

> **The version line:** the row lands after the 0.40.0 stable cut; the
> section number follows the release-train numbering at publication.

`quarantine_write` keeps its receipt claim (№964/№325: the PAYLOAD —
args[0], the quarantined value — must not ride the return, so
quarantining a secret and logging the receipt stays legal), but its
return is not a pure receipt: the handler returns the audit text
`«[QUARANTINE_EGRESS] … (reason: …)»` which interpolates args[1] — the
caller-supplied reason (`src/builtins/consent.rs`, read in the bounds
of the handler body). Until №980 the receipt rule blanked that echo to
`Label::bottom()`, so a value passed AS THE REASON arrived at every
display sink unlabelled: `quarantine_write(v, query_param("r")) →
respond(…)` PASSED where the bare `query_param → respond` control
refuses `HTML_INJECTION` (the audit's R2 probe).

### What changed and where it bites

| Since | Rule | The refusal | The paths |
|---|---|---|---|
| the №980 merge | the receipt's return carries `from=1` — EXACTLY the label of the reason it echoes, in all FOUR label lanes (the static lattice, the TW runtime, the VM runtime, the typed secret lane; one table, `sink_receipt::RECEIPT_ECHO_ARGS`) | the same class the echoed value would get on its own: `HTML_INJECTION` for an untrusted reason into a public channel, `SECRET_LEAK` for a secret reason into any sink — on top of any class the call itself already carries | `quarantine_write(v, reason) → <sink>` where the RETURN reaches a sink and the REASON carries a mark |

**A program that passed `mlog check` on ≤ 0.40.0 while printing or
forwarding a quarantine receipt built from a MARKED reason (an
`env()`/`query_param()` value passed as the reason) now REFUSES — this
is the fix, not a regression.** Programs that pass a benign literal
reason (the common shape, `examples/w1_consent_revoke.mlog` among them:
`quarantine_write(data, "gdpr revoked by patient")`) change nothing —
the corpus sweep on the landing tree found zero verdict changes across
all 683 `.mlog` files.

### The migration

1. **Do not pass a marked value as the reason.** The reason is
   caller-supplied text interpolated into the receipt — treat it like
   any other value that reaches a sink: pass a benign literal, or
   sanitize it first (`redact(r, "all")` for a reason that must not
   carry its origin).
2. **The reason belongs to the audit event, not the return.** The
   №335 audit event already records the quarantine with its reason —
   the program-visible receipt is the wrong channel for it. №983 moves
   the reason into the audit event entirely and replaces the echo with
   the machine quarantine id; after that naryad lands, this whole
   section applies to nothing (the hand is removed BY №983's PR — the
   removal condition is written in the code next to the table).
3. **The payload half is unchanged.** `quarantine_write(secret, "benign
   reason") → print(receipt)` keeps its №964 verdict (the receipt
   carries the REASON's label only); the №964 probes in
   `tests/naryad_964_sink_receipt.rs` are updated to that
   mechanically-true form.

### Pinned by

- `tests/naryad_980_receipt_echo.rs` — 7 tests: the control, the static
  R2 pin (the non-maskable probe refuses `HTML_INJECTION` again), the
  secret-reason end-to-end (`SECRET_LEAK`), the payload-stays-receipt
  pin, the 1-arg bottom pin, the typed-lane unit pin (the only pin that
  isolates the fourth lane's hand), the table grounding (a live
  `Role::Sink`, the receipt membership, the String return).
- `tests/naryad_980_receipt_echo_runtime.rs` — 4 tests: the TW and VM
  runtime refusals under `METALOGOS_STATIC_LABEL_GATE_OFF=1` (each pin
  names its lane's hand) plus the TW and VM payload controls.
- `scripts/mutation_verify_980.sh` — M1 semantic / M2 TW / M3 VM /
  M4 typed: each hand removed → exactly that lane's pin reddens,
  restore byte-identical (PASS before the push).

## 0.40.0 → next (№981 — the external-return rule reaches the executor, BREAKING)

> **The version line:** the row lands after the 0.40.0 stable cut; the
> section number follows the release-train numbering at publication.

№978 gave the rule its static half: a sink's value that carries
EXTERNAL data — the remote response body (`http_post`, `send_message`,
…), the host stdout (`exec`), a human's answer (`human_respond`), the
TTS carrier's response (`tts_send`) — arrives `public, untrusted`, so
`respond(http_post(…))` refuses `mlog check`. But the coverage was
measured at 2 of 4 label lanes: the VM (the executor, ADR-0197) and the
TW (the reference oracle) still took the argument join for those
returns — `bottom` for a literal URL — and the executor EXECUTED what
check refuses.

### What changed and where it bites

| Since | Rule | The refusal | The paths |
|---|---|---|---|
| the №981 merge | the external-return consult stands in all FOUR label lanes, the same branch order everywhere (`label_source` → private-field bridge → receipt → external → join); `git grep -c external_sink_return` = 1/1/1/1 | on the TW: the sink gate refuses at the call's arm head, BEFORE evaluation; on the VM: after evaluation — so for a NETWORK carrier the capability/SSRF layers refuse first (measured, pinned); for `exec` under the named permit the value is created and the gate refuses on its LABEL | `respond(http_post(…))`, `respond(exec(…))`, the same through variables and flow steps — anywhere the external return reaches a sink that requires trusted-or-public data |

**A program that EXECUTED `respond(http_post(…))` or `respond(exec(…))`
on ≤ 0.40.0 now REFUSES at runtime — this is the fix.** The static
verdicts are unchanged (the static hands are №978's; the corpus sweep
on the common 683 `.mlog` files found zero verdict changes).

### The migration

1. **Sanitize before you forward.** The remote response body is
   untrusted data: `escape_html`/`render` before an HTML sink,
   `redact` before anything else, or validate it first. The static
   refusal (№978) already told you where; the runtime now enforces the
   same rule when the program runs.
2. **The capability layer sits in front on the VM.** For a network
   carrier the VM's capability gate / SSRF guard refuse before the
   label line can (the stack machine evaluates the sink's arguments
   first — measured, pinned in
   `tests/naryad_981_external_returns_runtime.rs`). Grant the carrier a
   capability only when the egress is intended; the label rule still
   holds behind it for the value that gets created.
3. **`http_get` is unchanged.** Its №325 ingress source label already
   refused the body into public sinks before №981; the verdict is
   preserved (the controls pin it).

### Pinned by

- `tests/naryad_981_external_returns_runtime.rs` — 6 tests: the TW
  http_post and exec refusals (pre-evaluation), the VM exec label
  refusal under the named permit, the VM http_post layering pin (the
  refusal must come from the EARLIER layer — if the label line ever
  fires first, the gate order moved and the layering story needs
  re-measuring), the TW http_get preservation control, the VM http_get
  control (the refusal stays, any layer).
- `tests/crosscheck_backends.rs` — the exec return refuses IDENTICALLY
  on both backends (the divergence-free contract); the http_post return
  refuses on both (the TW by the label line, the VM by the earlier
  layer — documented).
- `scripts/mutation_verify_981.sh` — M1 TW / M2 VM: each hand removed →
  exactly that lane's pins redden, restore byte-identical.
