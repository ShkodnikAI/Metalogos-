# Metalogos W3b (№975) — Black-Box Adversarial Security Pass (Report of Record)

## 1. Header

- **Binary**: `/home/z/my-project/metalogos-work/redteam-w3b/candidate-mlog`, version string `mlog mlog 0.40.0 / grammar rev: 1`.
- **SHA256 (re-verified at run time, both phases)**: phase 1 `4be5d0db9eaa66d78a655282b0653b08831cf08336f6ba11e84b203d2f852ed9`; **phase 2 (rebuilt from PR head) `3c8dfa13498a7f7839b5bf06e9bed0e98a2ba62f0822c3c3d0333d50907f56dc`** — matches `candidate.sha256`.
- **Dates**: phase 1 session 2026-11-18; **phase 2 session 2026-12-01**. Scope: W3b teacher-registry promises 1–7 (PROMISES.md, №975 / ADR-0200 §9).
- **Probes**: phase 1 = p01–p05 + recon; **phase 2 = p06, p06b, p07, p07b, p08** (p06b/p07b are the sub-vehicles of p06/p07).
- **Phase-2 unlock**: the orchestrator granted ONE public reference fact (the minimal teach+distill program), unblocking the flow-body grammar and the `learnable pattern` declaration that phase 1 could not find black-box. Everything downstream was then executed against the live binary.
- **Black-box statement**: nothing under `/home/z/my-project/metalogos-work/repo/` was opened, grepped, or catted in either phase. Knowledge = the binary's own output + PROMISES.md + the granted reference program. Offline only; the binary was never modified. Host python3/sqlite3 was used ONLY to inspect and patch the attestation DB the machine itself produced (the sanctioned tamper axis).

## 2. Executive summary

**Phase-2 formal probes: 2 CAUGHT / 1 BYPASS / 0 INCONCLUSIVE. Combined, both phases: 7 CAUGHT / 1 BYPASS (2 bypass axes) / 4 residual INCONCLUSIVE surfaces (recorded as sub-items, each with what would settle it).**

Phase 2 executed the full chain phase 1 could not start: teach under the mock lane (promise 6 — CONFIRMED end-to-end: the saved certificate carries `dataset.provenance:["runtime-teaching"], rows:10` plus a trained weights blob, and `reflex_save` persists it), certificate honesty (promise 4 — the `teacher{}` block carries exactly `backend / weights_id / license_class / training_on_output_allowed`, verbatim `{"backend":"mock","license_class":"osi","training_on_output_allowed":true,"weights_id":"in-tree-mock-teacher"}`), and the verify/tamper axis (promise 5). The gate refusal text `DISTILL_TEACHER_UNLICENSED` was **never observed in any lane**: with the env unset the №898 LLM-configuration refusal fires first (that ordering is the p06 evidence); every non-mock teacher lane dies earlier at №898 or at the provider API-key check; the only lane that reaches training is the permissive mock. Sink-parity was verified on the `--reference` (TW) backend for the verify route (identical verdicts and refusal text on both backends); the serve/hub-attached lane was not reached.

**The headline phase-2 result is a BYPASS with two independent axes**: (c) a certificate whose `teacher.weights_id` is forged (`"restrictive-teacher-x"`) while class/flag stay honest verifies **ok:true SILENTLY** — `weights_id` is one of the four promised teacher fields but is cross-checked against nothing; and (g1) a certificate with the **entire teacher block deleted** from a distilled model (whose spec proves a distill lineage) also verifies **ok:true SILENTLY** — the null-with-`teacher_reason` form is accepted, but nothing *demands* it. Everything the verify route does check is loud and precise: unknown class word (a), unknown backend (b), class/flag disagreement with the registry record (d, e2, e3) all refuse with `[ATTESTATION_TEACHER_MISMATCH]` and name the teacher, the claim and the registry value. The env axis (p08) is airtight: exactly {1, true, TRUE} engage the mock; {0, False, garbage, blank, 00, unset} all refuse loudly with identical №898 text — no silent fall-through anywhere.

**Honest limits**: the honest-restrictive verify case (promise 5's "honest non-permissive teacher is NOT refused") is unreachable offline — the only observable registry record is the mock's (osi/true), so no cert can honestly claim a restrictive record; approximations all refuse on the registry disagreement (which itself supports the check). `METALOGOS_ATTEST_STRICT=1` did not refuse in-program despite being advertised by the binary's own warning text. Promise 7's `distill.teacher-refused` ledger record is unobservable (no gate refusal ever fires). The serve/hub lane was not attacked.

## 3. The probe table

| Probe | Promise attacked | Verdict | Classification |
|---|---|---|---|
| p01_env_mock_values (phase 1) | 6 (env axis) | **RECLASSIFIED → SETTLED by p08: CAUGHT** | compile-stage identity in phase 1 was the unreachable-runtime artifact; p08 settled the real behavior (see p08 row) |
| p02_registry_enumeration (phase 1) | 3 | CAUGHT (unchanged) | 9 builtin names refuse `[UNDEFINED_FUNCTION]`, VM and mock lane |
| p03_flow_grammar_hunt (phase 1) | 1/6 reachability | **RECLASSIFIED → SETTLED: CAUGHT (reachability)** | flow grammar granted by orchestrator and works verbatim; the black-box discoverability gap stands as a wording finding, not an open blocker |
| p04_spec_learnable_hunt (phase 1) | 6 | **RECLASSIFIED → SETTLED: CAUGHT** | declaration form is `learnable pattern NAME(args) -> T { prompt: / distill_to: / distill_after: }` (newline fields, no commas) — accepted verbatim |
| p05_reflex_model_decl (phase 1) | recon | CAUGHT (unchanged) | reflex blocks parse clean |
| p06 recon_00_env (phase 1) | binary identity | CAUGHT (unchanged) | phase-1 hash recorded; phase-2 hash re-verified |
| **p06_gate_ordering** (phase 2) | 1 (ordering), 6 (positive lane) | **CAUGHT** | mock lane distills end-to-end (cert rows:10, trained weights); unset lane: №898 LLM-config refusal fires FIRST; `DISTILL_TEACHER_UNLICENSED` never appears in any lane (llm-rung dead-ends recorded in the transcript) |
| **p06b_llm_rung** (phase 2, p06 sub-vehicle) | 1 (gate reach) | CAUGHT (holds) | in-program `llm { providers: [...] }` cannot produce a working non-permissive teacher offline: №898 persists with the block, provider words fall through to Anthropic and die on the API key |
| **p07_cert_axis** (phase 2) | 4, 5 | **BYPASS (2 axes)** | (a/b/d/e2/e3) refuse loudly `[ATTESTATION_TEACHER_MISMATCH]`; (f) honest permissive silent ok; **(c) forged weights_id → silent ok:true; (g1) teacher block deleted → silent ok:true**; (e-true honest-restrictive) INCONCLUSIVE sub-item; strict-mode in-program refusal advertised but not observed |
| **p07b_builtin_verify** (phase 2, p07 sub-vehicle) | 5 (in-program parity) | CAUGHT (holds, with a caveat) | `attest_verify("DecideHead")` warns loudly (anchor/unsigned) and verifies; `METALOGOS_ATTEST_STRICT=1` did not refuse in-program |
| **p08_env_axis** (phase 2) | 6, 1-adjacent | **CAUGHT** | {1,true,TRUE} engage the mock; {0,False,garbage," ",00,unset} all refuse №898 loudly, EXIT=1; no silent path |

## 4. Evidence for every BYPASS

**p07, axis (c) — forged `teacher.weights_id` verifies silently.** Baseline cert (produced by the machine itself, `scratch/cert.db`, row `DecideHead`): `teacher = {"backend":"mock","license_class":"osi","training_on_output_allowed":true,"weights_id":"in-tree-mock-teacher"}`. Tampered copy `scratch/tamper/c_osi_unknown_teacher.db` changes ONLY `weights_id` → `"restrictive-teacher-x"` (patched via python3 sqlite3 on `cert_json`). `mlog attest verify ../pocs/p07_cert_axis.mlog --model DecideHead` → `ok: true`, `level: unsigned`, EXIT=0, and the reason line contains **no teacher-related text** (only the ledger-anchor note and the informational `[ATTESTATION_DATA_CHANGED]`). The machine accepted a teacher identity (which weights the teacher used) it has no record of and did not check. Per the phase sheet, (c) was expected to refuse with `[ATTESTATION_TEACHER_MISMATCH]`; it did not.

**p07, axis (g1) — teacher block strippable.** Tampered copy `scratch/tamper/g_teacher_deleted.db` deletes the `teacher` key from `cert_json` entirely (the model's `spec_json` still proves the distill lineage: `pattern: Decide, distill_after: 1, reflex: DecideHead`). Same verify command → `ok: true`, EXIT=0, silent on the teacher axis. The "null WITH sibling `teacher_reason`" form (g2) is accepted as the sheet predicts — but nothing requires the reason or the block. A certificate laundered of its teacher story verifies clean.

**What the same route DID catch (completeness of the record)**: (a) `license_class "proprietary"` → `[ATTESTATION_TEACHER_MISMATCH] teacher license_class 'proprietary' is not the ADR-0163 vocabulary (osi | non-osi | restrictive)` EXIT=1; (b) `backend "skynet"` → `... has no teacher-registry record — the certificate names a teacher the machine does not know` EXIT=1; (d) flag flipped → `teacher 'mock' certificate claims training_on_output_allowed = false, the registry record carries true — the provenance story disagrees with the machine` EXIT=1; (e2) `restrictive` on mock → identical mismatch form, EXIT=1; (e3) `non-osi` on mock → same, EXIT=1. All refusal texts are loud, tagged, and name teacher + claim + registry value.

## 5. The INCONCLUSIVE list (residual, phase 2)

1. **The gate refusal itself (promise 1 text)** — `DISTILL_TEACHER_UNLICENSED` was never emitted in any lane tried: unset (№898 fires first), in-program `llm{}` rung (№898 persists), `METALOGOS_LLM_PROVIDER=local|mock` without key (dies after 3 retries on the Anthropic API-key check), mock lane (permissive — no refusal expected). Offline there is no working non-permissive teacher. Would settle: a program-visible way to attach a real (non-mock) provider offline, or a registry seeded with a restrictive teacher record for one of the provider words.
2. **The honest-restrictive verify case (promise 5, positive half)** — the only observable registry record is the mock's (osi/true); no cert can honestly carry a restrictive record offline. Approximations (e, e2, e3) all refused on registry disagreement — supporting the check but not proving the "ok:true + loud reason" branch. Would settle: any restrictive-record teacher reachable offline.
3. **Serve/hub-attached lane (promise 2)** — not reached this session; only VM vs TW (`--reference`) parity was tested (verify route: identical verdicts and refusal texts on both; teach route: identical output, louder memory banners). Would settle: an `mlogserver` program with a learnable pattern on the serve path.
4. **Audit trail (promise 7)** — `distill.teacher-refused` unobservable (the gate never refuses, above); no ledger export flag exists on `run` (ledger is process-local; `ledger verify` reads exported files, none exported). Would settle: a lane that triggers the gate refusal (then inspect the exported ledger).
5. **`METALOGOS_ATTEST_STRICT=1` in-program** — advertised by the binary's own `[reflex]` warning ("METALOGOS_ATTEST_STRICT=1 refuses"), but the same program under the env verified silently with no warnings on the observed run. Would settle: a re-run campaign on the strict env (single-observation anomaly, not pushed further this session).

## 6. Findings for the owner

- **BYPASS, teacher identity (highest value)**: `teacher.weights_id` is carried in the certificate but verified against nothing (c), and the whole `teacher` block can be deleted without complaint (g1) — both silent `ok:true`. Every other teacher field is checked loudly and precisely. Recommend: cross-check `weights_id` against the registry record (or at least against the model's own distill lineage), and require the `teacher` block (or an explicit `teacher_reason`) whenever `spec_json` proves a distill lineage — exactly the fail-closed shape the `--model` flag already uses for multi-reflex files.
- **Wording gap (machine contradicts itself)**: the №898 refusal text suggests `llm { providers: [{ alias: "mock", provider: "local" }] }` with quoted string values — that snippet does NOT parse (`expected IDENT` at the quoted value); only unquoted idents parse. A refusal that recommends unparseable syntax is a trap for the language's own users.
- **Env fall-through**: `METALOGOS_LLM_PROVIDER=local` (and `mock`) silently select the Anthropic driver and fail after 3 retries on the missing API key — the provider word the error text itself names does not select a local driver; loud failure, but misleading.
- **Positive lane is real and silent-quiet (promise 6, good)**: teach → distill → trained weights → certificate with `runtime-teaching` provenance all executed under `METALOGOS_MOCK_LLM=1` with EXIT=0; the only loud traces are the mock tag on outputs and the save-time `[reflex]` warnings. Note the cert is `level: unsigned` with `ledger_seq: 0` (no ledger key configured at save) — consistent, and the warning says strict mode would refuse.
- **Refusal quality (good)**: every refusal observed in phase 2 is loud, tagged, and names the offending value (№898, Naryad #523 arity errors, `[TYPE_MISMATCH]`, `[ATTESTATION_TEACHER_MISMATCH]` with claim-vs-registry wording). The env axis has zero silent paths.
- **Phase-1 anomalies, status update**: the `expected flow_pipeline` first-token gap is now explained — the body grammar starts with `input: T = <expr> -> CALL -> output`, which no black-box starter guess could reach (the granted reference was needed); the learnable-pattern form is `learnable pattern` (a two-keyword head phase 1 did not try). Both phase-1 INCONCLUSIVE items were reachability artifacts, not machine defects.

## 7. The artifact list (lab dir `/home/z/my-project/metalogos-work/redteam-w3b/`)

Phase 1 (unchanged):
- `PROMISES.md` (input), `candidate.sha256` (updated to the phase-2 hash), `candidate-mlog` (rebuilt binary, hash re-verified)
- `pocs/recon_00_env.txt` — binary identity + version transcript
- `pocs/p01_env_mock_values.mlog` + `p01_env_mock_values_transcript.txt`
- `pocs/p02_registry_enumeration.mlog` + `p02_registry_enumeration_transcript.txt`
- `pocs/p03_flow_grammar_hunt.mlog` + `p03_flow_grammar_hunt_transcript.txt`
- `pocs/p04_spec_learnable_hunt.mlog` + `p04_spec_learnable_hunt_transcript.txt`
- `pocs/p05_reflex_model_decl.mlog` + `p05_reflex_model_decl_transcript.txt`
- `scratch/recon-notes.txt` — the phase-1 grammar-recon record

Phase 2 (new):
- `pocs/p06_gate_ordering.mlog` + `p06_gate_ordering_transcript.txt` — positive lane + №898-first ordering + gate-reach dead-ends (llm-rung, provider env)
- `pocs/p06b_llm_rung.mlog` — the in-program `llm{}` gate-reach vehicle
- `pocs/p07_cert_axis.mlog` + `p07_cert_axis_transcript.txt` — save-surface discovery, cert materialization (verbatim teacher/dataset blocks), baseline verify, tamper verdicts a–g, strict-mode note, TW parity
- `pocs/p07b_builtin_verify.mlog` — in-program `attest_verify` parity vehicle
- `pocs/p08_env_axis.mlog` + `p08_env_axis_transcript.txt` — nine env values, all-loud discipline
- `scratch/inspect_db.py`, `scratch/tamper_axis.py` — host sqlite3 tooling (attestation axis only)
- `scratch/cert.db` — the honest machine-produced certificate DB (distilled DecideHead); `scratch/model.db` — the no-lineage save (no attestation row)
- `scratch/tamper/` — the nine tampered DB copies: `a_proprietary, b_unknown_backend, c_osi_unknown_teacher, d_flag_false, e_honest_restrictive, e2_restrictive_word, e3_nonosi_word, f_honest_permissive, g_teacher_deleted, g_teacher_null_reason` (.db each)
- `report.md` — this report
- Mirror copy of the lab artifacts under `/home/z/my-project/download/redteam-w3b/`
