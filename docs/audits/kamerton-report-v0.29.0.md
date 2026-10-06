# Kamerton report: the R1–R6 verdicts on the fixed main, the SHA256 of v0.29.0

**Naryad №618 (gh#1076, wave 33), class [process].** The closure of the
external Kamerton cycle over the assignment `zadanie-metalogos-v0.28.2`:
both P0 defects (D1/Н1-03 — the schema-as-code `references` modifier
loss; D2/Н1-04 — the loop semantics: a bare call terminating the loop,
the cut tails, the vacuous greens) are fixed, the fixes are PUBLISHED in
**v0.29.0** (2026-10-06T14:17Z), the release assets are verified by a
fresh SHA256 re-run over the downloaded artifacts, and the R1–R6
verdicts come from real runs of the repro corpus on the fixed main. The
forwarding of this report to the external reviewer is the OWNER's gate
(the external channel), not the executor's — this document is the
evidence artifact the owner forwards.

**Report date:** 2026-10-06. **The binary of the runs:** `mlog 0.29.0`
(grammar rev: 1), built locally from the `n618-kamerton-report` branch —
the code is identical to main `e40ce8e` (the post-release sync commit of
v0.29.0; the branch adds only the repro corpus and this report, there is
no code delta between the branch and main at this commit), debug build
profile. The verdicts below are reproducible with any build from the tag
`v0.29.0` onward.

## 1. The SHA256 of the v0.29.0 assets — the fresh re-run over the downloaded artifacts

The four release assets were downloaded afresh from the GitHub release
404811694 (tag `v0.29.0`) on the report date; `sha256sum` was re-run over
the downloaded files and checked against the attached `SHA256SUMS` (the
digest list produced by the `release.yml` pipeline from the tagged
commit; the fourth asset pair — the two Sigstore attestations — binds to
the binary subject digest and is verified separately through the
attestations API):

| Asset | Size (bytes) | SHA256 | vs SHA256SUMS |
|---|---|---|---|
| `mlog-linux-x86_64` | 35118864 | `256bed0dd4a7ae08fe913b6da11cd20d7b52c5fd43adcd98dcb18144076a99f5` | **OK** |
| `mlog-linux-x86_64.cdx.json` | 373742 | `a3a89c292706f6e266522fe8c21e0e7355fcc76060b0d098a411251747d65620` | **OK** |
| `BUILD-INFO.txt` | 400 | `038b934e2902db7e1c57e750dd7ce208c844b18edfbcd9ec68549d9970631800` | **OK** |

`sha256sum -c SHA256SUMS` → all three lines report OK. `BUILD-INFO.txt`
pins the binary provenance end to end: **source commit
`4be0f729e13d77b234cc595589049926dbd4d909`** (the 0.29.0 version-lockstep
squash commit, PR gh#1073, the №583 pattern), rustc 1.99.0 (b940084d7,
2026-09-28), the pinned runner image ubuntu-24.04, and the build command
`cargo build --release --locked -p metalogos-server --bin mlog`. The
release pipeline run: 37477763021 — SUCCESS. Besides the digests, two
Sigstore attestations (the SLSA build provenance and the CycloneDX SBOM
attestation) are bound to the same binary subject and were checked
present via the attestations API by the binary digest.

## 2. The R1–R6 verdicts — real runs of the repro corpus on the fixed main

The corpus: `scripts/repro_kamerton/` — 8 self-contained `.mlog` files
(each with `db { url: "sqlite::memory:" }`, the repro declarations, and
a minimal entry point), the t92 reconstruction — see §6. Every verdict
in the table is reproducible with the command shown; nothing below is
taken from memory.

| Probe | Command | Expectation (the Kamerton matrix / the t92 reconstruction on `c694b6d`) | Obtained on v0.29.0 | Verdict |
|---|---|---|---|---|
| **R1** — `r1_references_compact.mlog`, the compact `references(parent.id)` | `mlog check` then `mlog run` | before the fix: the REFERENCES modifier lost **silently** (the parser kept `parent.id` as one token); after the fix: the applied DDL carries `REFERENCES parent(id)`, check green, run green | check → `OK: no issues found.` (rc 0); run → `ok` (rc 0) | **FIXED** |
| **R2** — `r2_references_spaced.mlog`, `references( parent.id )` | the same | before the fix: the same **silent** loss (see the correction §5.1 — the verbatim SQL_ERROR did NOT arise in this form); after: identical to R1 | check OK (rc 0); run → `ok` (rc 0) — identical to R1 | **FIXED** |
| **R2c** — `r2c_references_dot_spaced.mlog`, `references( parent . id )` | the same | before the fix: the only form with a loud trace — the broken clause `REFERENCES parent(.)` → SQL_ERROR at apply with a green check | check OK (rc 0 — the §611 DDL dry-run validates the rendered DDL through in-memory SQLite); run → `ok` (rc 0) | **FIXED** |
| **R3** — `r3_bare_call_pattern.mlog`, an `each` with a bare-call-only body in a PATTERN | `mlog run` | before the fix: **1 iteration of 3**, the pattern tail silently cut, the pattern returned the first `db_insert` id (`1`) | run → **`3`** (rc 0) — 3/3 inserts, the tail executes, the honest count (the TW↔VM parity e2e pins the VM's extent: the VM never fabricated the Return) | **FIXED** |
| **R3b** — `r3b_pattern_tail.mlog`, the marker INSERT after the loop | `mlog run` | before the fix: the marker table empty (0 rows), the counter `1`; the t92 correction §5.2 — the tail was NOT executed, contrary to their matrix | run → **`3`** (rc 0); the `tail_marker` table receives the `tail_ran` row | **FIXED** |
| **R4** — `r4_vacuous_test.mlog`, a TEST with a bare call in the `each` body | `mlog test` | before the fix: **green vacuously** — 1 of 3 inserts, the tail cut, the assert never ran | **✅ R4 — 1/1 tests passed** (rc 0); the assert executed against 3 rows | **FIXED** |
| **R4a** — `r4a_vacuous_mutation.mlog`, the mutation probe: a deliberately false assert | `mlog test` | before the fix: also **green** (the vacuousness proof); after the fix it MUST go red | **❌ `assert_eq failed: 3 != 999`, 0/1 tests passed, rc 1** — red, exactly as the contract demands | **FIXED** |
| **R6** — `r4b_tail_marker.mlog`, a `while` with the poisoning bare call EARLIER in the body + the tail | `mlog test` | before the fix: one iteration (`i` stays 1.0), the tail table never created, green vacuously | **✅ R6 — 1/1 tests passed** (rc 0): the full pass, 3 inserts, the tail row present, `i == 3.0` | **FIXED** |

The additional evidence for R1/R2/R2c (the DDL content, not just the exit
codes): the regression test `tests/naryad_611_schema_references.rs` pins
that the rendered DDL carries `REFERENCES parent(id)` and is accepted and
stored by SQLite (`sqlite_master`) for all three spacings, and that a
reserved-word identifier is caught loudly by `mlog check` (the §611 DDL
dry-run over the SSOT renderer `ast::schema_table_ddl`). R5 — the
assignment body with `let mut` — is the control row without regression:
the regression test `n612_r5_assignment_body_no_regression` is green
(60.0); it is not part of the corpus because the defect never reproduced
in that shape, even before the fix.

## 3. The D1/D2 → naryad/PR/commit map

| Kamerton defect | Naryad | Issue | PR | Squash commit | Published in |
|---|---|---|---|---|---|
| **D1** / Н1-03 — schema-as-code `references(t.f)`: the silent modifier loss and the broken `REFERENCES parent(.)` clause | №611 | gh#1060 | gh#1067 | `7b72f09` | v0.29.0 (`4be0f729`) |
| **D2** / Н1-04 — the loop semantics: a bare call terminated the loop, cut the tail, and left the tests green vacuously | №612 | gh#1061 | gh#1068 | `082a213` | v0.29.0 (`4be0f729`) |

Both fixes entered the v0.29.0 release through the version-lockstep PR
gh#1073 (the tag `v0.29.0` on `4be0f729`, the №583 pattern). The
CHANGELOG [0.29.0] § Fixed documents both defects with the mechanisms,
the regression tests, and the pre/post demonstration (the honest counter
1 → 3 on the fixed program).

## 4. The affected releases and the documentation

| Release | Status |
|---|---|
| v0.28.1 (2026-10-05) | **affected** — both defects present (this parser/execution surface had not changed since d5af542) |
| v0.28.2 (2026-10-06T04:14Z) | **affected** — published WITHOUT the fixes (recorded in gh#1061: v0.28.2 was published before the fix landed; both v0.28.1 and v0.28.2 are affected) |
| **v0.29.0 (2026-10-06T14:17Z)** | **the correcting release** — D1 and D2 closed; the update path from v0.28.x is the update to v0.29.0 |

There are no silent releases since t92: v0.29.0 is the first and only
release after the Kamerton cycle started. The documentation trail: the
CHANGELOG [0.29.0] § Fixed entries for №611 and №612 (with the
mechanisms, the regression tests, and the pre/post demonstration); the
REFERENCE and REGISTRY semantics rows need no changes — the fixes restore
the declared contract, no new language surfaces were introduced.

## 5. The honest corrections to the Kamerton protocol — verbatim from t92, no rephrasing

The two corrections below are quoted VERBATIM from the t92 office-side
verification record (the №618 naryad body, gh#1076, carries the same
verbatim text); the original phrasing is preserved deliberately:

1. «R2 в их формулировке (пробелы внутри скобок) даёт тихую потерю,
   дословный SQL_ERROR — форма с пробелами вокруг точки».
2. «хвост pattern-тела при D2 НЕ исполняется (их матрица: „выполняется"
   — неточна)».

In summary: correction 1 says the Kamerton matrix misattributed the loud
SQL_ERROR to the paren-spaced form, while that form actually lost the
modifier silently — the loud SQL_ERROR belonged to the dot-spaced form
only. Correction 2 says the Kamerton matrix claimed the pattern-body tail
executes under D2, while in fact it did not. Neither correction changes
the fixed-in-v0.29.0 verdicts: after №611 and №612 all forms behave
identically and honestly (the table §2).

## 6. The corpus reconstruction and the reproducibility

The corpus `scripts/repro_kamerton/` is the t92 reconstruction — the
office-side verification of step 0 recorded in gh#1061 (the repro run
plus the executor's code reading) — over the verbatim matrix programs of
gh#1060 and gh#1061 and the regression tests
`tests/naryad_611_schema_references.rs` and
`tests/naryad_612_loop_semantics.rs`. The files are self-describing: the
header of each file names the matrix row it reproduces and the
before/after behavior. The corpus lands in the repository in this same
PR — the naryad's fact line described the corpus as existing, while it
existed only as the t92 reconstruction and was NOT a repository artifact;
this is the honest note (the §16.0-D no-stubs rule).

The reproduction of every §2 verdict: the commands are in the table
itself; the binary — any build from the tag `v0.29.0` and later (the
`mlog-linux-x86_64` release asset or a local `cargo build --release -p
metalogos-server --bin mlog` from the tagged commit). The SHA256 evidence
is re-runnable with the three commands: download the assets from the
release, run `sha256sum -c SHA256SUMS`, compare with `BUILD-INFO.txt`.

The corpus files, one row each — the probe they reproduce and the
context the matrix distinguishes: `r1_references_compact.mlog` (R1, the
compact schema form, schema+flow program), `r2_references_spaced.mlog`
(R2, the paren-spaced schema form), `r2c_references_dot_spaced.mlog`
(R2c, the dot-spaced schema form — the verbatim SQL_ERROR row),
`r3_bare_call_pattern.mlog` (R3, the bare-call `each` body in a pattern
with a value tail), `r3b_pattern_tail.mlog` (R3b, the same plus the
marker INSERT after the loop — the tail-execution proof),
`r4_vacuous_test.mlog` (R4, the TEST-context probe whose assert must
actually run), `r4a_vacuous_mutation.mlog` (R4a, the mutation probe —
the deliberately false assert that must go red; this file is EXPECTED to
exit non-zero under `mlog test`, that is its contract),
`r4b_tail_marker.mlog` (R6, the `while` poisoning body plus the tail
marker). Each file names its row and the before/after behavior in its
header comment, so the corpus doubles as the executable documentation of
the matrix.
