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
