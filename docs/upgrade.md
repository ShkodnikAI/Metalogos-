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

### The machine that keeps this honest

- `scripts/ci/breaking_change_gate.py` (the blocking
  `breaking-change-gate` CI job, №865): a tag range that adds a name to
  `GRANT_ALWAYS` / `PROFILE_PERMIT_SHARP` / `HUMAN_OVERSIGHT` without a
  `### Changed` BREAKING row naming it in the pending CHANGELOG section
  is RED. The next deny-path lands with its migration row or does not
  land.
- `tests/naryad_865_migration_path.rs`: the 0.34.1-shaped program
  (`http_post` + `send_message` + `cron_run`) — unmigrated it refuses
  with the named carrier and paths; migrated per this page it passes
  both gates on both backends.
