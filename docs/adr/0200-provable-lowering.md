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
