# Testing Evidence — Metalogos

Material for grant applications (NLnet/Restack — the Testing Evidence section):
how many and which properties are verified automatically, with numbers. Everything below
is run by CI on every PR (blocking) or on a schedule (trends).

## Property-based tests (naryad #277, proptest)

Fast, deterministic (seeds), green in the blocking CI:

| File | Properties | Numbers |
|---|---|---|
| `tests/property_builtin_nopanic.rs` | **No-panic across all pure builtins**: random `Value` arguments (including unicode, deep nesting up to 6 levels, boundary floats) into random functions → either a value or a Result error, NOT a panic. The registry (`BUILTIN_REGISTRY`) is enumerated at runtime — new builtins enter the sweep automatically | **162 pure builtins** covered directly (deterministic sweep + 128 proptest cases); 23 stub entries skipped honestly (a call is a loud error, pinned by other tests); side-effectful categories (bot/web/io/email/llm/db/voice/…) excluded and listed in the test output |
| `tests/property_json_roundtrip.rs` | `json_encode` of any nested `Value` → valid JSON; canonical stability (parse→encode is stable from the second round on); `json_get` returns exactly the leaves that were put in via the constructed dot-paths; the default on arbitrary paths — no panics. gh#580 killer extension (2026-09-22): `parse_json ∘ json_encode` full-cycle stability; `has_field` exact 1.0/0.0 answers (including the documented fields-only navigation); `dict_set`/`keys`/`values`/`has` keyed-store consistency + parse-back restore; `parse_json` on arbitrary text — value or loud error, never a panic | 256 cases × 7 properties |
| `tests/property_string_invariants.rs` | `reverse∘reverse = id` on arbitrary unicode; `len(s) == chars().count()` (documented semantics); `substring/char_at` = per-character slices at all boundaries; `escape_html` without raw angle brackets | 512 cases × 4 properties |
| `tests/property_tw_vm_parity.rs` | Programs generated from a conservative subset of the grammar (literals, arithmetic, concatenation, let, builtin calls, pattern calls) execute IDENTICALLY in TW and VM. Exceptions — the documented ADR-0105 boundary (`match`, `BlockIfElse`-as-value, memory/learnable/server/IO) | 192 cases × 2 program shapes |

**Found and fixed by the property tests right as they were written (#277):** `strip()`
panicked when both ends of the string consisted entirely of strip characters
(`strip("&", "Ⱥ&")` → slice panic `start > len-end`); fixed in
`builtin_strip`, the minimizer and the regular forms are pinned by the test
`regression_strip_overlap_ends_no_panic`. Documented (not a crash):
`json_encode` prints a float with a deviation of up to 1 ulp from the canonical
shortest serde representation (observation in the J2 comment) — a record of
the behavior, not a change.

## Mutational testing (naryad #277, cargo-mutants smoke)

- Target: `src/builtins/json.rs` — dense escaping/parsing/navigation logic.
- Killer: `tests/property_json_roundtrip` (the roundtrip properties catch
  most serialization mutations). gh#580 (2026-09-21 smoke failure): the
  killer covered only `json_encode`/`json_get` — mutants in the file's
  other five builtins (`parse_json`, `has_field`, `dict_set`, `dict_keys`,
  `dict_values`, `dict_has`) survived BY CONSTRUCTION. Fixed 2026-09-22
  by extending the killer to the whole module surface (J5–J8; mutation
  verification `scripts/mutation_verify_580.sh`, 2/2 VERIFIED per the
  №382 protocol).
- Run: weekly (Monday 06:00 UTC) + manual dispatch —
  `.github/workflows/mutants.yml`, NOT merge-blocking, trends only.
- Artifact: `mutants.out/` + `mut-score.txt` (mut-score = killed / (killed +
  missed + timeouts)) — published as a GitHub Actions artifact of each
  run.
- Why not `src/audit.rs` (3232 lines) and not `src/builtins/string.rs`
  (905 lines) from the original brief: an hour-scale weekly run with no gain in
  value for the smoke contract; the module choice is a deliberate deviation,
  recorded in the workflow header and the naryad's PR.

## Adjacent loops (already existed)

- **Fuzzing** (naryad #256): 3 cargo-fuzz targets (parser, bytecode, url_decode),
  a smoke run in CI (2 min/target), non-blocking.
- **Blocking set**: 15 check-runs per PR (lib/integration/crosscheck/
  candle/vision/registry-arity/llm-cache/minimal-build/fmt/clippy/ADR-numbering/
  module-size/vscode/cargo-audit/branch-freshness) — merge only when the set is
  fully green on the merge commit.
- **Crosscheck**: TW vs VM parity — a separate blocking test +
  the property extension from #277 (see above).
