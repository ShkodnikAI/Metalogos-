# Metalogos W3b (№975) — Black-Box Adversarial Security Pass (Report of Record)

## 1. Header

- **Binary**: `/home/z/my-project/metalogos-work/redteam-w3b/candidate-mlog`, version string `mlog mlog 0.40.0 / grammar rev: 1`.
- **SHA256 (re-verified at run time)**: `4be5d0db9eaa66d78a655282b0653b08831cf08336f6ba11e84b203d2f852ed9` — matches `candidate.sha256`.
- **Date**: 2026-12-01 (session). **Scope**: W3b teacher-registry promises 1–7 (PROMISES.md, №975 / ADR-0200 §9).
- **Probes**: 5 authored probe files (p01–p05) + the recon pass behind them (dozens of grammar probes recorded in `scratch/` and `pocs/recon_00_env.txt`).
- **Black-box statement**: nothing under `/home/z/my-project/metalogos-work/repo/` was opened, grepped, or catted. All knowledge comes from the binary's own help/error output and PROMISES.md. Offline only; the binary was never modified; no host tool touched the repo.

## 2. Executive summary

**1 CAUGHT / 0 BYPASS / 4 INCONCLUSIVE** (formal probes; the recon pass is evidence attached to the INCONCLUSIVE items).

The headline result is a *reachability* finding, not a registry bypass: the black-box grammar hunt reconstructed large parts of the surface (top-level `test / spec / mlogserver / flow / tool / llm / reflex / db / type / import`; spec lines `pattern:/prompt:/distill_after:/min_accuracy:/margin:/fallback_if:` in COLON syntax; tool methods `name(args) -> T { ... }`; reflex blocks `input/layers/labels/seed`) — but **neither the `flow_pipeline` body grammar nor the learnable-pattern declaration could be started black-box**. Every flow body starter (identifiers, strings, numbers, 27 keywords, all printable ASCII punctuation, 27 multi-char operators) refused with `expected flow_pipeline` at the first token; every attempt to declare a "learnable pattern" failed the №523 semantic check `learnable pattern 'p' not found`. Because of that, **no probe could reach the teach→distill execution path at all**, so the gate (promise 1), sink-parity (2), certificate honesty (4), verify check (5), positive lane (6) and audit trail (7) are unreachable this session — each is INCONCLUSIVE with the exact blocker and what would settle it recorded below. The one axis that WAS reachable — program-side registry enumeration (promise 3) — was CAUGHT: all nine plausible registry builtin names refuse `[UNDEFINED_FUNCTION]` at compile time, in both the default and the mock lane.

**Honest limits**: zero bytes of the distill path executed; no certificate was ever produced; the tamper axis (cert_json in SQLite) was planned but had no DB to tamper with. This is a partial pass: the machine held everywhere it was actually attacked, but the richest surfaces (promises 1/2/4/5/6/7) were not attacked at all.

## 3. The probe table

| Probe | Promise attacked | Verdict | Classification |
|---|---|---|---|
| p01_env_mock_values | 6 (env axis / mock lane selection) | INCONCLUSIVE | compile-stage refusals identical under all env values; runtime unreachable |
| p02_registry_enumeration | 3 (registry not program-reachable) | CAUGHT | 9 builtin names refuse `[UNDEFINED_FUNCTION]`, VM and mock lane |
| p03_flow_grammar_hunt | 1/6 (reachability of distill path) | INCONCLUSIVE | `expected flow_pipeline` on every body starter; first-token grammar not discoverable |
| p04_spec_learnable_hunt | 6 (positive lane entry) | INCONCLUSIVE | №523 semantic refusal; learnable-pattern declaration form not discoverable |
| p05_reflex_model_decl | recon (model decl surface) | CAUGHT (parses clean) | reflex blocks accepted: `input/layers/labels/seed`, no commas |
| p06 recon_00_env | binary identity | CAUGHT (identity verified) | SHA256 match, version recorded |

## 4. Evidence for every BYPASS

**None. No BYPASS was achieved.** The machine refused every attack that actually reached it (p02: `[UNDEFINED_FUNCTION] compile: undefined function: teacher_list` — and identically for `teachers`, `teacher_record`, `teacher_get`, `teacher_all`, `registry`, plus the rejected `teach`, `distill`, `llm_call`, `teacher_all`, on both lanes; p05: clean parse of the model declaration). Nothing program-side could read, enumerate, extend, or influence the teacher registry — which is direct *support* for promise 3, though only for the builtin-name axis, not for program-side state manipulation (unreachable, see §5).

## 5. The INCONCLUSIVE list

1. **p01 / env axis (promises 6, and gate selection)** — Tried: one spec program run under `METALOGOS_MOCK_LLM` ∈ {(unset), 0, 1, true, TRUE, garbage}. Observed: identical compile-stage refusal (`learnable pattern 'p' not found`) under every value; env handling is not observable before the unreachable runtime stage. Would settle: any working teach program; then compare cert/driver behavior per value.
2. **p03 / flow body (promises 1, 2)** — Tried: ~80 first-token shapes inside `flow NAME { ... }` (identifiers, literals, keywords `let/use/seq/stage/step/teach/learn/...`, ASCII punctuation, operators `-> => |> >> :: ...`). Observed: uniform `expected flow_pipeline` at the first body token — no starting rule discoverable black-box. The gate refusal ordering (LLM-call refusal vs `DISTILL_TEACHER_UNLICENSED`) therefore never executed. Would settle: one reference `.mlog` showing a minimal flow pipeline (or the grammar EBNF); then run it without the mock lane and record which refusal fires first.
3. **p04 / learnable-pattern declaration (promise 6)** — Tried: `learnable pattern p {}`, `pattern p {}`, `learnable p {}`, tool-with-`pattern()`-method (closest accepted), spec-first/comma-separated orders. Observed: №523 `spec 'k': learnable pattern 'p' not found` in every combination. Would settle: the declaration form (likely a keyword or annotation); then the full mock-lane distill → `reflex_save` chain.
4. **Downstream axis chain (promises 1, 2, 4, 5, 7: gate refusal + parity on `--reference`/serve, certificate honesty incl. `teacher_reason`, tamper axis a–g incl. license_class/osi/backend/flag flips/deleted teacher, verify refusal `[ATTESTATION_TEACHER_MISMATCH]`, ledger `distill.teacher-refused`)** — Tried: planned and prepared (host python3/sqlite3 tooling ready; `attest verify` and `ledger verify` CLI help read). Blocked at the head of the chain: no teach program → no reflex_save → no certificate DB. Would settle: the same single missing grammar piece; everything downstream is then mechanical.

## 6. Findings for the owner

- **Wording gap (sheet vs machine)**: PROMISES.md promise 6 calls the mock lane "the explicit `METALOGOS_MOCK_LLM=1` lane", but nothing in the binary's own output surfaces what a *program* does to trigger a teach — the language surface for teaching was the single hardest thing to find black-box. If the intended surface is inside `flow` bodies, the flow_pipeline grammar has zero discoverable start rule via error messages — the parser's `expected flow_pipeline` hint names a nonterminal, not a token set, unlike the very informative `spec`/`reflex`/tool errors (which name exact fields). Recommend a parse error that lists acceptable first tokens for flow bodies.
- **Refusal quality (good)**: all observable refusals are loud, tagged (`[UNDEFINED_FUNCTION]`, №523 semantic block), and lane-independent (identical with/without mock). Compile-time checks demonstrably precede any LLM/gate logic — consistent with promise 1's "refuses BEFORE training executes" ordering, but that ordering itself is UNVERIFIED.
- **Anomalies**: (a) `flow` accepts `flow NAME {` and then rejects *every* first token including `}` (empty body refused) — an empty-flow program cannot exist, so either the nonterminal is misnamed or the rule is dead weight; (b) spec lines are comma-separated (`pattern: p,`) while reflex fields are newline-separated — inconsistent separator discipline across sibling constructs; (c) the semantic error cites a line number of the *spec*, not of the missing pattern's would-be declaration.
- **Self-report axis**: unreachable this session; the planned probe (teach under mock, flip provider env before save, compare cert claim) remains the highest-value single follow-up.

## 7. The artifact list (lab dir `/home/z/my-project/metalogos-work/redteam-w3b/`)

- `PROMISES.md` (input, unchanged), `candidate.sha256`, `candidate-mlog` (unchanged; hash re-verified)
- `pocs/recon_00_env.txt` — binary identity + version transcript
- `pocs/p01_env_mock_values.mlog` + `p01_env_mock_values_transcript.txt` (6 env values × same refusal)
- `pocs/p02_registry_enumeration.mlog` + `p02_registry_enumeration_transcript.txt` (both lanes)
- `pocs/p03_flow_grammar_hunt.mlog` + `p03_flow_grammar_hunt_transcript.txt` (representative attempts)
- `pocs/p04_spec_learnable_hunt.mlog` + `p04_spec_learnable_hunt_transcript.txt` (both lanes)
- `pocs/p05_reflex_model_decl.mlog` + `p05_reflex_model_decl_transcript.txt` (clean parse, `check` OK)
- `scratch/recon-notes.txt` — the full grammar-recon record backing §5
- `report.md` — this report
- Mirror copy of the lab artifacts under `/home/z/my-project/download/redteam-w3b/`
