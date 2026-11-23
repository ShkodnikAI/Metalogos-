# ADR-0200: The Provable Tier-Lowering Line — the Reflex Attestation Plan, the Owner Decisions 1–6, and the W1–W9 + E1–E4 Roadmap

- Status: **Accepted** (the owner decisions of 2026-10-27 — decisions 1–3
  via §13.12 of the strategy plan, decisions 4–6 and the merge rule
  verbatim in [issue #1599](https://github.com/ShkodnikAI/Metalogos-/issues/1599))
- Date: 2026-10-27
- Deciders: the OWNER (decisions 1–6 and the merge rule, verbatim)
- Naryad: №822 (issue #1599, Волна 68)
- Source: the owner's W1 naryad v2 (the office canon:
  `naryads/W1_reflex_attestation_and_roadmap_v2.md`, the revision after
  review); the line's load-bearing records: [ADR-0117](0117-reflex-distillation-semantics.md)
  (the distillation semantics), [ADR-0116](0116-reflex-weight-persistence.md) (the
  weights persist), [ADR-0121](0121-vm-reflex-parity.md) (the VM Reflex
  parity), [ADR-0167](0167-action-ledger-v1.md) (the ledger is
  tamper-evident, not tamper-proof), [ADR-0178](0178-generative-contour-boundary.md)
  (the generative contour does not grow)
- Related: №825 (W1 — the Reflex attestation, issued after this ADR),
  №826 (W2′ — the metric registry), №823 (E1–E4 — the measurements)

## §1. Context — why a trust line for lowered tiers is needed

The distillation line lowers program understanding from an LLM teacher
into local Reflex weights: `distill_hub.rs` trains in the background,
`reflex_train` / `reflex_predict` (tier `stable`) consume and produce
models, and `save_model_to_db` / `load_model_from_db` in
`src/nn/persist.rs` are the shared persistence point of both backends
(the hub itself does not write weights — "weights keep persisting
through `reflex_save`"). The `feature_signature` check (№504,
`persist.rs:391`) already refuses distilled answers after an embedding
drift — but only warns.

Nothing today answers the operator's real question: **do these weights
come from where the operator believes they come from, and do they still
match the pattern that produced them?** The line answers it with an
attestation certificate carried with the weights and verified at load —
and later (W3+) with a `spec` declaration the certificate can hash.

Two machine facts shape the design (verified at `f14b721`, not
re-opened):

1. **The ledger has no sign-arbitrary-bytes primitive.** The key is
   `METALOGOS_LEDGER_KEY`; without it a per-process random key is
   generated. Therefore the certificate is anchored **indirectly**: a
   ledger record `reflex.attest` carrying `cert_sha256` (the ledger's
   own Ed25519 signature over the chain). This is **tamper-evident, not
   tamper-proof** — exactly ADR-0167's boundary, and the documentation
   says so verbatim.
2. **The VM parity for Reflex save/load already holds**
   (`src/vm.rs:3296-3304` calls the same `reflex_save_dispatch` /
   `reflex_load_dispatch` as the TW; the "VM backend does not yet
   support Reflex" stubs in `builtins/reflex.rs` are the shadow
   fallback, the №748 precedent). The certificate attaches to
   `persist.rs` — the shared point — so both lanes attest identically.

## §2. The boundary — what the certificate is and is not

The certificate fixes **provenance and consistency**: which pattern,
which spec shape, which training data, which embedding signature, which
weight bytes, anchored into the ledger chain. It does **not** certify
quality. Until E3 measures calibration, every confidence value is
`uncalibrated`; nothing in the line may claim "calibrated" or
"proven-correct". The attestation is not a proof of correctness — it is
a checkable story of origin, and the honest-limits register carries
that sentence.

## §3. The owner decisions (verbatim, 2026-10-27)

**Decisions 1–3** (accepted via §13.12 of the strategy plan):
(1) the "provable tier lowering" plan is accepted; (2) the metrics —
the Rust registry only; (3) the teacher — open models with a permissive
license.

**Decision 4 — the transitional certificate mode (variant (б), the soft
transition).** Owner's verbatim: «вариант б». Weights without a
certificate (saved before W1) load with a loud warning and
`cert: none`; an `unsigned` certificate (no configured ledger) — a
loud warning; `METALOGOS_ATTEST_STRICT=1` makes both cases refusals;
**the default flips to strict one release window after W1 ships** (a
CHANGELOG Migration entry).

**Decision 5 — `attest_verify` on unfiltered data (variant (б), allowed
with an explicit mark).** Owner's verbatim: «б». `attest_verify` works
on models trained on unfiltered data (until W4); `level: "unfiltered"`
is mandatory in the builtin's response and in the CLI output;
`ok: true` means provenance consistency, **not** quality; the W6
enablement gate refuses `unfiltered` models.

**Decision 6 — the `Frozen` exit (variant (в), bounded auto-unfreeze).**
Owner's verbatim: «в». A model exits `Frozen` automatically when the
recovery checks pass, **but no more than once per 24 hours** (the
default parameter; the concrete recovery conditions and the parameter
surface are fixed in W5 and in the implementation record of this ADR).
Every auto-unfreeze writes a ledger record (the reason, the metrics
before/after). If the model re-freezes within the window, it stays
frozen until an explicit external command (CLI / an operator action,
also ledger-recorded). The implementation is W5; W1 is unaffected.

**The merge rule (variant (а)).** Owner's verbatim: «а». A PR is merged
by the **executor** after green CI on the merge commit (all required
checks). The owner's control points are ADR-level decisions and the
release publication (the machine never publishes, ADR-0186 §4.5). The
"merge done by the owner" line in the W1 canon text is replaced by this
rule when №825 is issued (the canon edit `785efb9` already carries it).

## §4. The certificate shape (W1 normative sketch)

Canonical JSON, keys sorted: `attestation_version`, `model`,
`pattern`, `spec_hash`, `data_hash`, `embed_signature`,
`weights_sha256`, `teacher{backend, weights_id, license_class,
training_on_output_allowed}` (until W3b — `null` with the reason),
`dataset{rows, filter, provenance[]}`, `eval{held_out_accuracy, metric,
calibration:"uncalibrated"}`, `rollback_if`,
`toolchain{reflex_version, mlog_version}`, `ledger_seq`. What is not
known is `null` **with the reason** — never invented. Until W4:
`dataset.filter = "none"`.

Three hashes, three refusal codes:

| Hash | Over | Violation | Verdict |
|---|---|---|---|
| `spec_hash` | the pattern signature (normalized prompt, labels, `distill_*`, metric, threshold; without comments/formatting/`cache*`) | `[ATTESTATION_STALE]` | refusal, **the changed field is named** |
| `data_hash` | the training-set manifest | `[ATTESTATION_DATA_CHANGED]` | informational, **not** a refusal |
| `embed_signature` | the embedding feature signature | `[ATTESTATION_EMBED_MISMATCH]` | hard refusal |

The history table `reflex_attestation(seq, model, spec_hash, data_hash,
embed_signature, weights_sha256, cert_json, ledger_seq, created_at,
superseded)` is append-only; the current certificate of a model is its
last non-superseded row. Ledger write failure at save does **not**
silently cancel the save — a loud error (this is an artifact, not a
side effect; the ADR-0167 asymmetry for actions does not apply here).

## §5. The roadmap W1–W9 + E1–E4 (verbatim dependencies)

| № | Naryad | Depends on | Note |
|---|---|---|---|
| E1–E4 | Experiments (round-2 plan, §9): the `embed` speed, the `reflex_train` time, the confidence calibration, the teacher's `baseline_agreement`. Results in `docs/research/` | — | Block W4–W6; can run in parallel with W1 (№823) |
| W1 | This line's first naryad (№825): the Reflex attestation certificate + `mlog attest verify` + the `attest_verify` builtin | An accepted ADR | The first visible result |
| W2′ | The metric registry: `macro_f1`, `min_class_recall` (the SSOT №545 scheme: the code in `metalogos-reflex/src/nn/metric.rs`, re-exported from `src/nn/metric.rs`; not in the ADR-0178 manifest — verified); the `mutate_rollback` rule fix (the metric name instead of the literal `accuracy`; the old form stays); the mandatory majority baseline for gating metrics | — | Instead of the old W2; grammar semantics, an ADR record is mandatory (№826) |
| W3 | The `spec` declaration (grammar, AST, semantic, VM compilation) | W1 | One new keyword |
| W3b | The teacher registry: LLM-backend records with the license class and `training_on_output_allowed`; the `DISTILL_TEACHER_UNLICENSED` rule | W3 | A new naryad absent from the original E plan |
| W4 | `mlog curriculum build`: propose-and-refute, held-out the teacher did not generate | W3, E1 | Built over `distill_hub.rs`; the quarantine — `reviewed`/`reason`/`quarantined` columns in `distill_samples`, no new table |
| W5 | The states, hysteresis, cooldown, quarantine, `--trace-distill` | W4, E2, E3 | The state in VM-owned state; decision 6 lands here |
| W6 | The differential enablement gate | W5, E4 | The №530 atomicity preserved; refuses `unfiltered` (decision 5) |
| W7 | The audit rules, the threat model, the limitations | W3b, W5 | The final check_id list lives here |
| W8 | The documentation and the indexes | cross-cutting | |
| W9 | The demo, the e2e, the benchmark | everything | Numbers measured only |

The cut order when the line grows: W9 first → part of W7 (keep
`DISTILL_TEACHER_UNLICENSED`, `ATTESTATION_STALE`) → the cooldown /
`Frozen` parts of W5. **Never cut:** the certificate signature and
verification (W1) and the held-out the teacher did not generate (W4).
The author's estimate "28–33 agent-days" is a lower bound — plan with
headroom.

## §6. The honest limits

1. **Not a quality proof.** The certificate and `attest_verify` attest
   provenance and consistency; `ok: true` never means "the model is
   good" (decision 5 makes this a response-field-level contract).
2. **`uncalibrated` everywhere until E3** — including the certificate's
   `eval.calibration` literal, which W1 writes as `uncalibrated`.
3. **Tamper-evident, not tamper-proof** (ADR-0167): the ledger anchor is
   `cert_sha256` in a signed chain; an attacker with the ledger key and
   the DB can forge a consistent history — the line's threat model
   (W7) says so, the documentation says so.
4. **No new `Value` variants**: the attestation is built from existing
   shapes (`Struct` fields, strings, integers); the grammar does not
   change until W3 (and then by exactly one keyword).
5. **The generative contour does not grow** (ADR-0178): none of
   `src/nn/persist.rs`, `src/distill_hub.rs`,
   `metalogos-reflex/src/nn/metric.rs` is in the stop-list manifest
   (verified), and the line adds no generative surface.
6. **The transitional window is loud, then strict** (decision 4): the
   soft default is explicitly time-boxed to one release window after
   W1, with the flip recorded in CHANGELOG Migration — the window is
   part of the contract, not an accident.

## §7. The implementation record (№825, 2026-10-28)

The W1 naryad landed; this section records the machine facts of the
implementation (the §4 sketch, realized — deviations are named, never
silent):

1. **The canonical spec form.** `spec_hash` covers the machine-built
   JSON of the pattern signature: pattern name, prompt, distill_after,
   fallback_if (the operator as the canonical word), min_accuracy,
   margin (the RESOLVED values), reflex name, input_size, labels, seed,
   the layer descriptors (`dense(in,out)` — type name + running dims),
   and the metric literal `accuracy` (the only registered gating metric
   until №826 widens the registry). Comments, formatting and `cache*`
   keys cannot enter — they are not part of the compiled pattern node.
   Activation kind is deliberately excluded (recoverable at deserialize;
   the persist Block 3 documents an activation swap as "the architecture
   didn't change"); weight BYTES are the `weights_sha256` lane.
2. **The changed-field name.** The `[ATTESTATION_STALE]` contract names
   the changed field — which requires the OLD canonical spec at verify
   time. The history table carries one extra column, `spec_json` (the
   exact canonical string per row); the certificate JSON itself keeps
   the §4 key set verbatim. The snapshot lives in the table, not in the
   cert — the cert shape is unchanged.
3. **The null-with-reason encoding.** `teacher`, `pattern` (when no
   learnable pattern distills into the model) and `rollback_if` are
   `null` with a sibling `*_reason` key carrying the machine-true
   reason. Nothing is invented.
4. **The level precedence.** The certificate's own `level` is
   `signed` (a configured `METALOGOS_LEDGER_KEY` at save — externally
   verifiable) or `unsigned` (the per-process random key, §1 fact 1).
   The verify OUTPUT level is the DATA level while every W1 dataset is
   unfiltered: `unfiltered` (decision 5 — mandatory in the builtin's
   response and the CLI output); `signed`/`unsigned` surface from W4
   on. `none` — no certificate row.
5. **The verify order in `reflex_load`.** The attestation gate runs
   BEFORE deserialize: the weights BYTES are the certificate's subject,
   and a tampered blob must produce the `[ATTESTATION_WEIGHTS_CHANGED]`
   refusal, not the serializer's magic error. Strict refusals
   (no certificate, `unsigned`) and every other not-ok verdict refuse
   the load; the no-certificate lenient path warns and continues
   (decision 4).
6. **The data manifest.** The caller's live teaching set (the examples
   the training consumed — the VM's and the interpreter's
   `distill_states`) takes precedence; without it, the persistent
   `distill_samples` rows for the pattern are the manifest (the hub
   lane's ledger); with neither, the manifest is empty and the
   provenance word says `none`. The provenance word is part of the
   stored `data_hash` (`"runtime-teaching:<sha>"` /
   `"distill_samples:<sha>"` / `"none:<sha>"`).
7. **The ledger-failure asymmetry (§4 verbatim).** The
   `reflex.attest` append happens INSIDE the save transaction; a
   failure rolls the weights row back and surfaces a loud
   `reflex_save` error — the attestation is the artifact, never a
   silent side effect.
8. **The measured bounds.** The full verify pass (open DB → hash the
   stored blob → rebuild the spec hash → compare) is microseconds on
   the naryad's fixture sizes — the < 200 ms contract holds with ~5
   orders of magnitude of headroom (the measurement rides the contract
   tests).

## §8. Revision — the anchor verification (№832, 2026-10-29; the wave-70 audit H-3)

A separate revision under §6.3 (the owner verdicts above are untouched —
this sharpens the tamper-evident boundary, it does not re-issue it):

**§6.3 (revised).** «Tamper-evident» holds ONLY where the anchor is
actually checked. Since №832 both verify paths (`reflex_load` /
`attest_verify` and `mlog attest verify`) locate the `reflex.attest`
ledger record by the certificate's `ledger_seq` and compare its
`args_hash` against the sha256 of the certificate's PROVISIONAL view
(the exact bytes the save hashed). Three outcomes: the anchor agrees;
`anchor not found` (the level claim demotes to `unsigned`, loudly,
strict refuses); `[ATTESTATION_ANCHOR_MISMATCH]` (positive tamper
evidence — a loud refusal under both policies). The pre-№832 shape
(a DB-only attacker rewriting the weights BLOB and the certificate row
passed verify without the ledger key) is closed; the H-3 tamper test
pins it red-before/green-after.

**The storage rule (the choice recorded per the №832 naryad).** The
runtime ledger stays process-local (ADR-0167 unchanged — no persistent
ledger in the W1 boundary): `signed` is verifiable only where the anchor
record lives; a cross-process verify honestly reports `anchor not
found`; the external audit rides the EXPORTED chain (`ledger_export` →
`mlog ledger verify --expect-key`). A persistent ledger (a path in
`memory { persist }` / env) remains the W-lane future work — it is a
boundary extension and does not ride a fix naryad.

**The honest level word.** `level: "signed"` means «the ledger key was
configured at save AND the anchor record verified at load» — never
merely «a key existed at save». The `level_for_output` data dominance
(decision 5: `unfiltered` while W1 datasets are unfiltered) is
unchanged; the anchor state rides the reason line and the strict gate.

## §9. The implementation record — W3b, the teacher registry (№975, 2026-11-18)

The W3b slot landed (the dispatch В98, №AB2); this section records the
machine facts (deviations are named, never silent):

1. **The registry form.** `TEACHER_REGISTRY` (src/backends.rs) — a
   static config-in-code overlay on the ADR-0163 taxonomy: records
   `TeacherEntry { backend, weights_id, license_class,
   training_on_output_allowed, note }`, the license-class vocabulary
   RE-USED (Osi | NonOsi | Restrictive — no new words), the §4 sketch's
   four certificate keys realized verbatim. No BACKEND_REGISTRY row was
   added; every class re-states the claim the tree already carries for
   that backend. Seed set: the in-tree mock lane (osi — the embodied-sim
   precedent), qwen25-vl-7b (osi — the tree's Apache-2.0 claim), and the
   three LLM provider defaults (claude/gpt-4o restrictive, llama3
   non-osi). There is NO permissive real-provider teacher in the seed —
   decision 3 is honored by default-deny until the owner adds one (a
   one-line edit, loudly diffable).
2. **The rule at the train boundary.** `DISTILL_TEACHER_UNLICENSED`
   (backends.rs, the SSOT gate fn) refuses the distill training BEFORE
   it executes, naming the teacher and the class, on three axes: no
   record (default-deny), a non-permissive class (decision 3's word:
   Osi), or `training_on_output_allowed = false`. One gate fn, THREE
   runtime sites — the TW distill trigger (`spawn_distill_training`),
   the VM trigger (`try_train_distilled_model`), the hub worker
   (`train_one`) — the №973 sink-parity form. The `#[cfg(test)]` unit
   seam (№489) is deliberately NOT gated: it is the honest test seam,
   not a distill run against a teacher. The refusal is audited and
   ledger-recorded (`distill.teacher-refused`).
3. **The teacher identity is the process's actual backend.**
   `current_teacher_backend()` resolves through the SAME predicates
   `create_llm_backend()` consults (the explicit mock lane → the
   in-tree mock; otherwise `Provider::default_model()` — exactly the
   three BACKEND_REGISTRY Llm names). The runtime does not know a finer
   model name at the call site today (the trace's `model: None` is the
   machine fact); the registry is keyed on what the machine can
   honestly name. A finer identity (the resolved model string per call)
   is future work — it must ride the llm surface, not an assumption.
4. **The certificate fills; the §4 key set is verbatim.** For a distill
   lineage (a pattern distilled into the model) with a recorded teacher
   the block carries the four keys; the no-teacher (direct
   reflex_train) and no-record cases stay `null` WITH the sibling
   `teacher_reason` (§7.3 — never invented). The canonical spec/hash
   SSOT is untouched (the teacher is NOT hashed into `spec_hash` — it
   is provenance, not spec).
5. **The verify check.** `check_teacher_block` (attest.rs, both the
   builtin and the CLI report routes): the class word must be the
   ADR-0163 vocabulary, the backend must resolve in the registry, and
   the class/flag must AGREE with the record — a disagreement is the
   loud `[ATTESTATION_TEACHER_MISMATCH]` refusal (a forged provenance
   story). An HONEST non-permissive teacher is NOT a mismatch: the
   story is consistent, `ok` stays true (decision 5's boundary —
   provenance, not policy), and the reason line names the teacher
   loudly so `ok: true` can never be read as "the teacher was fine".
   The policy enforcement of pre-W3b weights is W6's enablement gate.
6. **The honest boundary.** The registry is a DECLARATION, not a
   license fact-check — the machine cannot read license texts; the
   records are claims with named bases (the limitations.md №975-b row
   carries this verbatim). The cut-order record holds: the rule's
   check_id is kept first when the line is trimmed (§5).

## §10. The implementation record — W4, the curriculum build (№989, 2026-11-21)

The W4 slot landed (the dispatch wave В99 — W-AC, №AC2, after №988/№AC1
measured E1–E4). This section records the machine facts (deviations are
named, never silent):

1. **The propose-and-refute core is a store function, not a server.**
   `curriculum_build(conn, pattern, audit)` in `src/distill_hub.rs`
   (the file the ADR names) — propose: the teacher-generated units
   (`source = 'llm-response'`, not quarantined) are the candidates, one
   persisted row per unit; refute: (a) a teacher self-contradiction
   (the same input taught two outputs — the LATER row quarantines,
   first-teacher-wins) and (b) a held-out counterexample (a held-out
   row whose input matches a unit's input with a DIFFERENT output —
   the unit quarantines, the reason names the counterexample's rowid).
   The CLI `mlog curriculum build` rides it (the machine-readable gate
   line `{"gate":"curriculum-build",...}` is the verdict's last output
   line and the audit journal's line — the same bytes both ways).
2. **The quarantine is the three columns, no new table** (verbatim
   §5): `reviewed INTEGER DEFAULT 0`, `reason TEXT`,
   `quarantined INTEGER DEFAULT 0` ALTERed onto an existing
   `distill_samples` and present in the fresh-schema DDL — the honest
   migration keeps every pre-W4 row byte-identical. A quarantined row
   carries the №983-form machine id `dq-<rowid>-<hash8>` in the audit
   journal and a `distill.curriculum-quarantine` ledger record.
3. **The held-out provenance has a machine-false refusal.** A held-out
   row (source `held-out`) whose (input, output) pair EXACTLY
   duplicates a teacher unit — the «the teacher did not generate this»
   story is false for that row — the build REFUSES loudly and
   quarantines nothing. The held-out rows themselves never enter any
   training-set assembly point: the hub boot load (the WHERE
   exclusion + the loud `distill.held-out-excluded` count), the
   attestation manifest (`read_distill_samples` is W4-aware with a
   pragma guard — a pre-W4 schema reads all rows and says "none", the
   byte-compat form) and the filter word all exclude them.
4. **The filter word went live.** `dataset.filter` stops being the
   hardcoded "none": `curriculum_filter_word(conn, pattern)` — a PURE
   function of the pattern's store state — answers "none" until the
   store carries W4 state (a quarantined, held-out or reviewed row for
   the pattern), then `curriculum:<sha16>` over the SORTED surviving
   units (order-independent, reproducible). The save-side certificate
   writes the computed word (`build_certificate` reads the manifest's
   `filter` field); the verify side recomputes and — the `data_hash`
   form — a moved word is the INFORMATIONAL
   `[ATTESTATION_FILTER_CHANGED]` (never a refusal), naming both
   words. The stored pre-W4 certificates ("none") are untouched.
5. **The level precedence flipped exactly as §7.4 promised.** The
   verify output level is the certificate's OWN level
   (`signed`/`unsigned`) once the STORED word is live; `none` keeps
   the decision-5 shape (`unfiltered`). The verify code carried the
   mechanism since W1 (dormant behind the hardcoded "none") — W4 makes
   it reachable, the leak-suite pins both sides.
6. **The honest boundary.** The curriculum is NOT a quality guarantee
   (§6.1): the filter speaks provenance; E3's underconfidence
   measurement (ECE 0.6013, the limitations.md W4 row) is why the
   `eval.calibration` literal stays `uncalibrated` — the flip is
   W5/W6's. The E4 teacher-agreement rows for real teachers are null
   WITH the reason in this environment (the limitations row names the
   removal condition). W5/W6 material (states, hysteresis, cooldown,
   the differential gate) is NOT built here — the §5 boundary holds;
   the reviewed column is the W5 review workflow's landing spot, the
   build only writes 0.
7. **The leak-suite.** `tests/naryad_989_curriculum.rs` — 7 tests: the
   refutation positive (the machine id + the reason + the journal),
   the crossing refusal (a no-op on the store), the
   self-contradiction first-wins, the held-out manifest exclusion
   (positive + the pre-W4 compat), the filter word purity
   (order-independence, the state move, the recompute agreement), the
   informational filter code (ok stays true, the code names both
   words, the stored word decides the level) and the signed/unsigned
   surfacing (no key → `unsigned`; the configured key → `signed`).
   The mutation contract lives in `scripts/mutation_verify_989.sh`.

### The W5 record — the lifecycle machine (№990, 2026-11-21)

The W5 slot landed (the dispatch wave В100 — W-AD, one row; the
dependencies W4/E2/E3 closed by №989/№988). The machine facts
(deviations are named, never silent):

1. **The states machine is a store function over the persist DB, not a
   server.** The durable home is the additive `distill_states` table
   (`pattern` PRIMARY KEY, phase, entered_at, the violation ring, the
   auto-unfreeze clock anchor, the re-freeze lock, the operator lock) —
   CREATE IF NOT EXISTS in `ensure_distill_store`; a pre-W4/pre-W5
   database gains the table on the next ensure and keeps every row
   byte-for-byte. VMs are pooled and per-request state resets, so the
   lifecycle NEVER lives in a per-request container — the hub lane and
   the CLI lane share the same table. `Active`/`Frozen`: a Frozen
   pattern refuses distill MOVES (the hub worker's training jobs, the
   CLI `curriculum build`) with the machine reason
   `[DISTILL_FROZEN]` (audited + ledger-recorded
   `distill.frozen-refused`); serving continues.
2. **The hysteresis (the anti-flap band).** A single violation is
   refusal-only (audit + ledger via the existing sites); crossing the
   threshold INSIDE the window auto-freezes. The violation feed rides
   the hub worker's `DISTILL_TEACHER_UNLICENSED` refusal (№975's site —
   one gate, now feeding the lifecycle). The ring trims to the window
   on every write.
3. **The parameter surface is fixed HERE (decision 6's own mandate).**
   `METALOGOS_DISTILL_FREEZE_THRESHOLD` (default 2),
   `METALOGOS_DISTILL_FREEZE_WINDOW_SECS` (default 3600),
   `METALOGOS_DISTILL_AUTO_UNFREEZE_COOLDOWN_SECS` (default 86400 —
   decision 6's 24h). Read at call time; the machine fns take `now`
   explicitly (pure, testable).
4. **The bounded auto-unfreeze (decision 6 verbatim).** The lazy
   evaluation rides every distill-move attempt: the recovery checks —
   fixed HERE as `teacher-gate-ok` (the direct
   `DISTILL_TEACHER_UNLICENSED` cause) + `store-integrity-ok`
   (`ensure_distill_store`) — pass, the cooldown clock allows (the
   anchor is the LAST auto-exit), no re-freeze lock and no operator
   lock sit on the pattern → exit Frozen, ledger-record
   `distill.auto-unfreeze` (the reason + the metrics: the check results
   and the store counts — the transition trains nothing, so the counts
   do not move, the record names them once), trace loudly.
5. **The re-freeze lock (decision 6 verbatim):** a freeze inside the
   cooldown window after an auto-exit sets `refroze_in_window` — the
   pattern stays frozen until the explicit external command
   `mlog distill unfreeze` (ledger `distill.unfreeze`, the lock clears).
6. **The operator lock (the W5-fixed surface):** the explicit operator
   freeze does NOT auto-release — the recovery checks are machine-true,
   the operator's reason is not; only `mlog distill unfreeze` lifts it.
   The automation freezes (the hysteresis) remain subject to the
   bounded auto-exit; an operator freeze never does.
7. **`--trace-distill`** (the global CLI flag; it sets
   `METALOGOS_TRACE_DISTILL=1` — one SSOT): every transition,
   violation, recovery evaluation and withholding emits `[TRACE]`
   lines — the same bytes to stderr and the audit journal (the
   gate-line form). The CLI:
   `mlog distill {freeze,unfreeze,status} --pattern <P> --db <F>
   [--json]` — the gate line `{"gate":"distill-<op>",...}` is the
   verdict's last output line; `status` is read-only (the auto-exit
   fires on move attempts, not on reads — the eligibility is REPORTED).
8. **The honest deviations.** (a) `ATTESTATION_STALE` is NOT wired as
   an auto-freeze trigger — the verify lane keeps the decision-4/5
   shape; the operator path (`mlog distill freeze`) covers it; the
   wiring is a W7-audit-lane candidate. (b) The `rollback_if` capture
   (the mutate_rollback rules as stored state) is NOT here — it needs
   the session rules plumbed through the save path (W1 territory, the
   never-cut lane); it rides the W7 audit lane. The certificate is
   UNTOUCHED by W5: the lifecycle is operational state, not provenance
   (the attestation reason string at the §7.4 record stays as written).
   (c) The offline TW/interpreter lane without the hub has NO durable
   lifecycle (its per-process anti-retry throttle stays as it was) —
   the durable home is the hub/persist DB. (d) `eval.calibration`
   stays `uncalibrated` (E3's ECE 0.6013 — the flip is W6's).
9. **The leak-suite.** `tests/naryad_990_states.rs` — 8 tests: the
   pre-W5 byte-compat no-op, the explicit freeze → the move refusal →
   the explicit release, the hysteresis threshold inside the window,
   the violations outside the window, the bounded auto-exit (the first
   exit fires, ledger + trace), the re-freeze-within-window lock (the
   explicit command is the only exit), the cooldown clock (withheld
   inside 24h, fires beyond), the trace bytes + the gate-line JSON
   shapes. The mutation contract `scripts/mutation_verify_990.sh`:
   M1 the 24h bound dropped, M2 the re-freeze window ignored, M3 the
   operator lock ignored — each RED, the inverse-replace restores
   byte-identically.

### The W6 record — the differential enablement gate (№994, 2026-11-22)

The W6 slot landed (the dispatch wave В101 — W-AE, one row; the
dependencies W5/E4 closed by №990/№988). The gate facts (deviations are
named, never silent):

1. **The gate is the policy enforcer at the enablement boundary
   (decision 5's), and it is NEW — there was no enablement operation
   before W6.** `mlog distill enable-differential --pattern <P> --db <F>
   [--research-dir <D>] [--json]` (the default research dir is
   `docs/research`). The gate refuses the effective level `unfiltered`
   with the machine code
   `[ATTESTATION_ENABLEMENT_UNFILTERED]` — the reason line names the
   field (`level`) and the cause (decision 5's dominance: the data
   filter word `none` keeps the W1 `unfiltered` shape). A refusal
   writes NOTHING (the certificate row stays byte-identical), is
   ledger-recorded (`distill.enablement-refused`) and rides
   `--trace-distill`. The check_id registry itself is W7's (§5) — W6
   introduces the refusal, W7 fixes the registry.
2. **The level predicate is decision 5's dominance, read from the
   STORED certificate** (`effective_level_from_cert`: the data filter
   word `none` → `unfiltered`, else the certificate's own `level`). The
   verify path itself is UNTOUCHED — the W1 never-cut lane: `attest_verify`
   on unfiltered data stays allowed with the mandatory `unfiltered`
   marker (decision 5), `ok: true` keeps its semantics (provenance
   consistency, not quality).
3. **The №530 atomicity is preserved by construction.** The flip runs
   INSIDE one caller transaction: the new ledger anchor
   (`reflex.attest-flip`, the ledger-first order the save path uses —
   a ledger failure is a loud error and the flip is not applied) and
   the superseding `reflex_attestation` row land together — either both
   persist or neither does. The unchanged facts (the weights bytes, the
   spec hash, the data hash, the embed signature) are COPIED into the
   superseding row — the flip changes the eval record only; the
   `reflex_models` weights BLOB is never touched (the leak-suite pins
   the bytes).
4. **The inputs are ONLY the recorded E3/E4 artifacts** (`docs/research/`):
   E3's `ece` is the measured calibration value the flip writes; E4's
   real-teacher `agreement_with_majority_baseline` rides the verdict.
   Read without recomputation; every failure mode (the missing file,
   the unparseable JSON, the missing key) lands null-with-reason naming
   the path and the cause. The mock row's 0.0 agreement is NOT read as
   one (the E4 artifact's own reason says so verbatim); the real
   teachers' nulls stay null-with-reason. When E3 carries no measured
   number the flip is WITHHELD loudly (the certificate stays
   "uncalibrated", the verdict reports the missing input) — the gate
   still answers the policy question (the level), the measured step
   waits for the number.
5. **The certificate §4 form is unchanged, the value is honest about
   being measured.** The `eval` keys stay (`held_out_accuracy`,
   `metric`, `calibration`); the `calibration` value switches from the
   literal `"uncalibrated"` (a string) to the measured ECE (a number) —
   the §4 record is the flip's carrier, named HERE. The superseding row
   re-anchors the certificate in the ledger (the two-step the save path
   uses: the provisional cert → the sha → the anchor record → the seq
   into the cert).
6. **The lifecycle interlock composes (W5).** A Frozen pattern refuses
   the enablement like any other distill move (`[DISTILL_FROZEN]` via
   `lifecycle_gate` — the interlock runs BEFORE the policy gate); a
   pattern with no attestation certificate (saved before W1) refuses
   loudly with `[ATTESTATION_NONE]`.
7. **`--trace-distill`** — the inputs line, the refusal line and the
   flip line emit `[TRACE]` bytes (the same bytes to stderr and the
   audit journal, `trace_distill` — one SSOT); the CLI gate line
   `{"gate":"distill-enable-differential",...}` is the verdict's last
   output line (the JSON carries the null-with-reason inputs as JSON
   nulls, the reason lines name them).
8. **The honest deviations.** (a) The gate does not enforce the
   non-permissive-teacher policy (decision 3) — the teacher-lane
   reasons stay informational in verify (the W3b shape), and the W6
   gate reads only the data level; the teacher-policy enforcement is a
   W7-audit-lane candidate and is NOT silently dropped — it is
   deferred, named here. (b) The baseline_agreement fact rides the
   verdict and the records but does not GATE the flip (E4's real-teacher
   rows are null-with-reason at the baseline date — there is no measured
   number to gate on; the W7 registry decides the check-id set). (c) The
   mutation contract is NOT part of W6's checklist (the naryad's check
   list names the gate, the inputs, the trace, §10 and the merge rule) —
   the leak-suite pins the behaviors; the mutation sweep rides W7's
   audit lane.
9. **The leak-suite.** `tests/naryad_994_enablement.rs` — 9 tests: the
   unfiltered refusal (the code, the named field, zero writes), the
   filtered flip to the measured ECE (the superseding row, the anchor
   advance), the weights/facts untouched (the W1 lane pinned), the
   missing-artifacts null-with-reason (the loud no-op, the certificate
   stays "uncalibrated"), the mock 0.0 not read as one, the frozen
   interlock, the no-certificate refusal, the trace bytes on the audit
   sink, the gate-line JSON shape (the nulls carrying the
   null-with-reason fields).

## §11. The W7 record — the audit lane (№995, 2026-11-23)

The W7 slot landed (the dispatch wave В102 — W-AF; the dependencies
W3b/W5 closed by №975/№990). §5 names W7's deliverable: «The audit
rules, the threat model, the limitations; the final check_id list lives
here». This section IS that deliverable: the registry below is the
final check_id list of the line, machine-checked two-way against the
tree (`tests/naryad_995_audit.rs` — the census: every bracketed
`ATTESTATION_*`/`DISTILL_*` code literal in the tree lives here, every
machine-visible row here lives in the tree; the other subsystems'
bracketed event words ([AUDIT]/[TRACE]/… — the audit line's own
vocabulary) and the other subsystems' ledger/env words are OUT of this
registry's scope by design — §11.6(iii). The №971 rule: a new machine
word lands IN THE SAME PR). The deferred §10 items (a)–(d) are
executed, not verdicts-only — each with its honest boundary named.
Deviations are named, never silent.

### The W7 implementation record

1. **(a) The `rollback_if` capture — EXECUTED** (the §10.8(b) deferred
   item). The mutate session's clause is stored state: the canonical
   word `metric op threshold` (the word = the operator's canonical
   spelling — `CompareOp::canonical_word` on the TW, the same word set
   over `ConditionOp` on the VM; both lanes build byte-identical
   strings, the parity is the leak-suite's). The capture lives in
   `mutate_rollback_rules` maps on BOTH lanes (the session fact is the
   MUTATE session's, not the declaration's — the clause rides the
   `mutate` statement, so the compile-time `CompiledLearnableInfo`
   stays untouched and the `.mbc` compat is unthreatened; the VM's
   `reset_for_reuse` clears the map with the same per-request
   discipline as `distill_states`). The save path captures it BESIDE
   the certificate: the `rollback_json` column (additive — a pre-W7
   database gains the column and keeps every row) + the
   `reflex.rollback-rules` ledger record anchoring the rule's sha256
   (tamper-evident: the mirror cross-checks against the chain). The
   capture rides the artifact transaction — a ledger failure is the
   same loud save error class (§4: never a silent save). **The
   certificate §4 bytes are UNTOUCHED** — the rule does not enter
   `AttestSpecInput` or `spec_hash` (the never-cut lane; the leak-suite
   pins the content parity with and without the capture). The capture
   is the audit mirror, not an attested fact (the threat class TM-AT-5
   says so; LIM-AT-5). The certificate's own §4 key `rollback_if`
   (null-with-reason since W1/№826) stays as written — the bytes do not
   move; the capture rides beside.
2. **(b) The `ATTESTATION_STALE` drift signal feeds the lifecycle
   hysteresis — EXECUTED** (the §10.8(a) deferred candidate). The load
   lane (`reflex_load`) feeds the SAME violation ring the hub refusals
   feed (`distill_hub::note_attestation_stale` → `record_violation`,
   the code `ATTESTATION_STALE`) on BOTH paths that saw a stale
   verdict: the bypassed-ok load (allow_stale) and the refused load.
   The verify verdict and its policy are UNTOUCHED (decision 4/5
   shape: the loud marker, the strict refusal, the bypass — exactly as
   W1 built them; the leak-suite pins the reason bytes). The threshold
   (2, the W5 default) inside the window auto-freezes the pattern for
   the distill moves; serving continues. Best-effort by contract: a
   feed error prints loudly and never becomes a load failure. Patterns
   without a distill lineage are never fed (no pattern name — decision
   3 and the lifecycle are both vacuous without one).
3. **(c) The teacher-policy enforcement (decision 3) at the enablement
   gate — EXECUTED** (the W6 §8(a) deferred candidate). The gate runs
   AFTER the level gate, BEFORE the flip:
   `attest::teacher_policy_gate` wraps the SAME `check_teacher_block`
   the verify path runs (one machine truth, no second implementation).
   Three outcomes: the forged story refuses with the SAME
   `[ATTESTATION_TEACHER_MISMATCH]`; the honest non-permissive teacher
   (the block agrees with the registry, the class is not OSI or
   `training_on_output_allowed` is false) refuses the ENABLEMENT with
   the new `[ATTESTATION_ENABLEMENT_TEACHER_POLICY]` (the reason names
   the teacher, the class and the flag); a pattern with no teacher
   (the §7.3 null + the machine-true reason, or no lineage) passes —
   decision 3 is vacuous without a teacher, the E3/E4 discipline still
   applies. Zero writes on refusal; the refusal is ledger-recorded
   (`distill.enablement-refused`) and rides `--trace-distill`. The
   verify path keeps its marker shape (decision 5's boundary — the
   W1 never-cut lane).
4. **(d) The mutation sweep — EXECUTED** (the §10.8(c) deferred item;
   not weaker than the №990 form). `scripts/mutation_verify_995.sh`:
   M1 the teacher-policy gate dropped from the enablement → the
   leak-suite reds; M2 the STALE feed removed from the load lane → the
   leak-suite reds; M3 the capture write dropped from the save path →
   the leak-suite reds. Each mutation is the inverse-replace, each
   restores byte-identically (the script diffs the file after restore).
5. **The honest deviations.** (i) The enablement gate reads the STORED
   certificate without re-anchoring (the W6 shape) — a DB-level cert
   swap that keeps the anchor story consistent shows at the VERIFY lane
   (the anchor evidence), not at the enablement; the class is named
   (TM-AT-4, LIM-AT-6), the boundary is the verify's, and the gate's
   teacher check is the provenance CONSISTENCY check (the forged story
   refuses at the gate too). (ii) The capture's canonical word carries
   the threshold as Rust's `f64` Display — both lanes produce identical
   bytes by construction; the parity test pins it. (iii) The registry's
   scan scope is the bracketed `ATTESTATION_*`/`DISTILL_*` families
   over the WHOLE tree (no other subsystem uses these families) and
   the ledger/env words over THE LINE'S files (listed in §11.2) — the
   other subsystems' words are out of this registry's scope by design,
   the boundary is stated here.

### The check_id registry (§11.1)

Every refusal/gate/event code the line emits, its site, its policy,
its slot. `Scan ✓` — the census finds the literal in the tree (two-way
machine-checked); `—` — a vocabulary word documented for completeness,
not a bracketed code.

| check_id | Site (the emission fn) | Policy | Slot | Scan |
|---|---|---|---|---|
| ATTESTATION_STALE | attest.rs `verify_attestation` (the load gate, the CLI report) | the spec drift: the loud marker; strict refuses; allow_stale bypasses with the loud audit line + a ledger record every use; W7: the drift feeds the lifecycle ring (the §11.6 record, (b)) | W1 (the capture W7) | ✓ |
| ATTESTATION_DATA_CHANGED | attest.rs (the verify + the load note) | informational, never a refusal (the data fact, not a spec fact) | W1 | ✓ |
| ATTESTATION_EMBED_MISMATCH | attest.rs | hard refusal (the feature-extractor signature moved) | W1 | ✓ |
| ATTESTATION_WEIGHTS_CHANGED | attest.rs | always-loud refusal (the weights bytes moved) | W1 | ✓ |
| ATTESTATION_ANCHOR_MISMATCH | attest.rs | loud refusal, both policies — positive tamper evidence (the certificate rewritten under a live anchor) | №832 | ✓ |
| ATTESTATION_NONE | attest.rs (the load gate, the enablement gate) | the no-certificate level: warn + `cert: none` (lenient), the strict refusal (decision 4); the enablement refuses | W1 | ✓ |
| ATTESTATION_UNSIGNED | attest.rs | the unsigned level: the loud warning (lenient), the strict refusal (decision 4) | W1 | ✓ |
| ATTESTATION_TEACHER_MISMATCH | attest.rs `check_teacher_block` (the verify, the CLI, the W7 policy gate) | the forged provenance story: loud refusal (an unknown class word or backend, a class/flag/weights disagreement with the registry, a stripped or reason-less null block) | №975 (W3b) | ✓ |
| ATTESTATION_FILTER_CHANGED | attest.rs | informational (the dataset filter word moved — the curriculum state, never a spec fact) | №989 (W4) | ✓ |
| ATTESTATION_ENABLEMENT_UNFILTERED | attest.rs + distill_hub.rs `enable_differential` | the enablement refusal: the effective level `unfiltered` (decision 5's dominance), zero writes, ledger-recorded | №994 (W6) | ✓ |
| ATTESTATION_ENABLEMENT_TEACHER_POLICY | attest.rs `teacher_policy_gate` + distill_hub.rs | the enablement refusal: the honest non-permissive teacher (decision 3 enforced at the enablement boundary — W7), zero writes, ledger-recorded | №995 (W7) | ✓ |
| DISTILL_FROZEN | distill_hub.rs `lifecycle_gate` | the lifecycle interlock: a Frozen pattern refuses every distill move (train, curriculum, enablement); serving continues | №990 (W5) | ✓ |
| DISTILL_TEACHER_UNLICENSED | backends.rs `distill_teacher_gate_for` (the pure SSOT) + llm.rs + learnable.rs + vm.rs + distill_hub.rs | the training-run refusal: no registry record (default-deny, ADR-0163 §2.1), a non-permissive class, the flag false; cut-order: NEVER cut (§5) | №975 (W3b) | ✓ |
| DISTILL_SHADOW | distill_hub.rs | the event line (the shadow-distill report — the stderr posture, no refusal, no schema claims) | the shadow lane (№495) | ✓ |
| DISTILL_SOURCE_HELD_OUT | distill_hub.rs (the vocabulary word `held-out`) | the source vocabulary: the rows the teacher did not generate — the refutation instrument, never training data (the Never-cut invariant); not a bracketed code | №989 (W4) | — |

### The line's ledger records (§11.2)

The scan scope for this section: `src/nn/attest.rs`, `src/nn/persist.rs`,
`src/distill_hub.rs`, `src/interpreter/learnable.rs`,
`src/interpreter/hooks.rs`, `src/interpreter/reflex_builtin.rs`,
`src/backends.rs`, `src/llm.rs`, `src/builtins/attest.rs`,
`src/builtins/reflex.rs`, `src/vm.rs`, `metalogos-server/src/bin/mlog.rs`.

| record | Emitter | Slot |
|---|---|---|
| reflex.attest | attest.rs `write_attestation` (the save lane's anchor, the ledger-first order) | W1 |
| reflex.attest-flip | attest.rs `flip_calibration_tx` (the enablement flip's anchor) | №994 (W6) |
| reflex.rollback-rules | attest.rs `write_attestation` (the capture's sha anchor — the tamper-evident mirror) | №995 (W7) |
| distill.trace | distill_hub.rs (the `--trace-distill` journal half; the load lane's feed journals here) | №990 (W5) |
| distill.freeze | distill_hub.rs (the explicit freeze + the hysteresis auto-freeze) | №990 (W5) |
| distill.auto-unfreeze | distill_hub.rs (the bounded auto-exit) | №990 (W5) |
| distill.unfreeze | distill_hub.rs (the explicit command; the re-freeze lock clears) | №990 (W5) |
| distill.frozen-refused | distill_hub.rs `lifecycle_gate` | №990 (W5) |
| distill.teacher-refused | distill_hub.rs + llm.rs + learnable.rs + vm.rs | №975 (W3b) |
| distill.enablement-refused | distill_hub.rs `enable_differential` (the unfiltered refusal, the teacher-policy refusal, the no-certificate refusal) | №994 (W6) + №995 (W7) |
| distill.enablement | distill_hub.rs (the successful flip) | №994 (W6) |
| distill.rejected | distill_hub.rs + learnable.rs + vm.rs (the propose-and-refute rejects) | №989 (W4) |
| distill.training-started | distill_hub.rs + learnable.rs | the distill line (№205-era) |
| distill.training-finished | distill_hub.rs + learnable.rs | the distill line (№205-era) |
| distill.curriculum-quarantine | distill_hub.rs (the quarantine machine ids) | №989 (W4) |

### The line's environment variables (§11.3)

The scan scope: the same file list as §11.2.

| variable | Reader | Slot |
|---|---|---|
| METALOGOS_ATTEST_STRICT | attest.rs `AttestPolicy::from_env` (decision 4: the missing/unsigned certificate refuses) | W1 |
| METALOGOS_ATTEST_ALLOW_STALE | attest.rs (the serve-path stale bypass — the loud audit line + a ledger record every use) | W1 |
| METALOGOS_DISTILL_FREEZE_THRESHOLD | distill_hub.rs (the hysteresis threshold, default 2) | №990 (W5) |
| METALOGOS_DISTILL_FREEZE_WINDOW_SECS | distill_hub.rs (the hysteresis window, default 3600) | №990 (W5) |
| METALOGOS_DISTILL_AUTO_UNFREEZE_COOLDOWN_SECS | distill_hub.rs (the cooldown, default 86400 — the 24h) | №990 (W5) |
| METALOGOS_TRACE_DISTILL | distill_hub.rs `distill_trace_enabled` (the CLI flag sets it — one SSOT) | №990 (W5) |

### The limitations mirrors (§11.4)

Every ACTIVE honest limitation of the line carries its id HERE and its
mirror row in `docs/limitations.md` (the №674 form; IN SYNC by id —
machine-checked).

| id | The limitation (the ADR anchor) | Slot |
|---|---|---|
| LIM-AT-1 | Tamper-evident, not tamper-proof (§6.3): an attacker with the ledger key AND the DB forges a consistent history — the line's threat model says so, the documentation says so | W1 (§6.3) |
| LIM-AT-2 | The cross-process anchor: the runtime ledger is process-local (ADR-0167) — a cross-process verify honestly reports `anchor not found` and demotes the level to `unsigned`; the exported chain rides `ledger verify --expect-key` for the external audit | №832 |
| LIM-AT-3 | `uncalibrated` until the W6 flip: the pre-W6 saves and the withheld flips (the E3 number missing) carry the literal — the flip is per-pattern, gated on the recorded E3/E4 artifacts | W1→№994 (W6) |
| LIM-AT-4 | The offline lane without the hub has no durable lifecycle (the W5 record, 8(c)): the STALE feed needs the persist DB (the `distill_states` table); the pure in-memory lane keeps the per-process anti-retry throttle only | №990 (W5) + №995 (W7) |
| LIM-AT-5 | The rollback capture is the audit mirror, NOT an attested fact: it does not enter `spec_hash`, the certificate bytes do not cover it, the ledger sha anchor makes its tampering EVIDENT (not impossible); the pre-W7 rows and the clause-less sessions read the empty form | №995 (W7) |
| LIM-AT-6 | The enablement gate reads the STORED certificate only (no re-anchor at the gate — the W6 shape): a DB-level certificate swap that keeps the anchor story consistent shows at the VERIFY lane (the anchor evidence), not at the enablement; the teacher check at the gate is the provenance-consistency half | №994 (W6) + №995 (W7) |

### The threat-model classes (§11.5)

The line's dated threat classes (the №985 form: each row names its
date); the full dated table lives in `docs/threat-model.md` (the
section «The attestation line»), mirrored here by id.

| id | The class (the vector → the evidence half → the residual) | Slot |
|---|---|---|
| TM-AT-1 | The attestation table rewritten under a live anchor → the anchor comparison disagrees → `[ATTESTATION_ANCHOR_MISMATCH]`, loud refusal, both policies (the positive tamper evidence) | №832 |
| TM-AT-2 | The ledger-key+DB attacker forges a consistent history → the evidence half holds (the chain verifies) → the RESIDUAL: the consistent forgery is not detectable by the line — tamper-evident, not tamper-proof (§6.3, LIM-AT-1) | W1 |
| TM-AT-3 | The forged teacher provenance story → the registry disagreement → `[ATTESTATION_TEACHER_MISMATCH]` at verify AND at the enablement gate (W7); the honest non-permissive teacher is not a mismatch — it is decision 3's policy refusal at the enablement boundary | №975 + №995 (W7) |
| TM-AT-4 | The DB-level certificate swap (the anchor-consistent rewrite: the stored cert replaced wholesale) → the verify lane's anchor/spec evidence fires on the next verify; the enablement gate (the stored-cert read) refuses the forged-teacher form, the swap class is the verify's evidence boundary (LIM-AT-6) | №994 + №995 (W7) |
| TM-AT-5 | The rollback-capture mirror tampering (the `rollback_json` column edited in the DB) → the ledger sha anchor disagrees with the mirror → tamper-EVIDENT (the cross-check); the capture is the audit mirror, not an attested fact (LIM-AT-5) | №995 (W7) |
