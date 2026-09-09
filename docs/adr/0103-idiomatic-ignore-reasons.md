# ADR 0103 — Idiomatic `#[ignore]` Reasons (Naryad #73 Block 3)

**Status**: Accepted
**Date**: 2026-08-14
**Milestone**: Naryad #73 — closing block 3 (audit `#[ignore]` without reason)

## Context

Naryad #73 Block 3 required auditing all `#[ignore]` markers in Metalogos's tests without a reason. The initial check showed **68** `#[ignore]` attributes in the code, split across two forms:

| Form | Count | cargo behavior |
|-------|--------|-----------------|
| `#[ignore = "reason"]` (idiomatic) | 2 | The reason is visible in `cargo test` output, in `--list`, in CI logs |
| `#[ignore] // reason` (non-idiomatic) | 66 | Cargo did not show the reason — it just printed `... ignored`. Finding the reason required a `git blame` |

No **fully bare** `#[ignore]` with no explanation anywhere in the code was found — all 66 non-idiomatic cases were accompanied by a `//` comment explaining the reason. However, that comment was invisible to tooling (cargo, CI aggregators, IDE plugins).

Example **before**:

```rust
#[test]
#[ignore] // TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)
fn jit_p5_golden_example() { ... }
```

Cargo output: `test jit_p5_golden_example ... ignored` — no explanation.

Example **after**:

```rust
#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
fn jit_p5_golden_example() { ... }
```

Cargo output: `test jit_p5_golden_example ... ignored, TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)` — the reason is visible in logs and in CI.

## Decision

Adopt the **idiomatic form `#[ignore = "reason"]`** as the only one allowed in the Metalogos codebase. All `#[ignore] // reason` occurrences were converted to `#[ignore = "reason"]`, preserving the original wording of the reason.

The conversion was performed by the script `/home/z/my-project/scripts/convert_ignore_to_idiomatic.py` (66 lines across 10 files). The script is idempotent: re-running it does not change already-converted lines.

### Reason categories (catalog)

All 68 `#[ignore]` instances are grouped into 11 categories:

| Category | Count | File(s) | What's needed to un-ignore |
|-----------|--------|---------|-------------------------|
| Legacy syntax (top-level stmts) | 21 | `tests/phase23_v084_v087_tests.rs` | Rewrite the tests for the `pattern`/`flow` syntax (top-level statements were removed in phase 23) |
| Flaky in sandboxed env | 9 | `tests/memory_persist_e2e.rs` | Isolate a temp dir for each test |
| VM compiler feature gap | 7 | `tests/phase18_compiler_statements.rs`, `tests/phase19_22_constraints.rs` | Implement in the VM compiler: match with string literal arms, match contains, match with compare ops, match with starts_with arms, Ne compare in rules, process-style declarations |
| Semantic/template feature gap | 7 | `tests/phase19_22_constraints.rs`, `tests/template_integration.rs` | Implement: opaque Html type constraint, undefined variable detection, Interpreter::render_template, unknown template detection |
| JIT not integrated | 6 | `tests/jit_golden.rs` | Integrate the JIT (ADR-0073) — `Vm::with_jit` is not yet available |
| Needs HTTP server setup | 6 | `tests/server_json_body.rs` | Configure the webhook server to run in test setup |
| Parallel-test race (needs #[serial]) | 4 | `tests/llm_cache_contract.rs` | Add `#[serial_test::serial]` (as in the naryad-71 fix) |
| External LLM API (manual run) | 3 | `src/llm.rs` | Not eligible for un-ignore — these are manual tests against the OpenAI/Anthropic/Ollama APIs. Run with: `METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=sk-xxx cargo test -- --ignored` |
| Secret type removed | 2 | `tests/phase19_22_constraints.rs` | Reimplement the Secret type (env() currently returns String, not Secret) |
| Semantic checker gap | 2 | `tests/phase19_22_constraints.rs` | Implement undefined variable detection in the semantic checker |
| Self-hosting lexer | 1 | `tests/self_host_lexer.rs` | Resolve the product decision (ADR-0023): Option A — continue, Option B — abandon, Option C — leave as is |

**Total: 68 `#[ignore]`, of which:**
- **3** — permanent (manual LLM API tests) — not eligible for un-ignore
- **1** — awaiting a product decision on self-hosting (ADR-0023)
- **64** — waiting for implementation work (listed in the table above)

## Consequences

### Positive

1. **CI logs are self-documenting**: the ignore reason is visible right in `cargo test` output, without a `git blame`.
2. **Search by reason**: one can run `grep -r 'ignore = "JIT' tests/` to find every JIT-blocked test.
3. **A ready-made roadmap for un-ignoring**: the catalog in this ADR is effectively a todo list for future naryads paying down this technical debt.
4. **Idempotency**: the `convert_ignore_to_idiomatic.py` script can be run in CI as a lint (not yet added — future work).

### Negative

1. **Long lines**: some `#[ignore = "..."]` are now longer than 100 characters. `cargo fmt` allows this (attributes are not wrapped by line width), but it is visually noisy. The alternative (a multi-line `#[ignore = "..."]`) is not supported by Rust — the attribute must be on one line.
2. **The conversion did not fix the underlying problem**: the tests are still ignored. This was an audit + normalization of the form, not a fix of the tests. Every `#[ignore]` remains in the code with exactly the same behavior.

### Neutral

1. **Reason wording was not edited**: left as it was in the `//` comments. Some contain a `TODO:` prefix — this was kept.
2. **2 already-idiomatic `#[ignore]` instances were left untouched**: in `tests/llm_cache_contract.rs:137` and `:173` — they were already in the correct form.

## Future work

1. ~~**CI guard**: add a step to `.github/workflows/ci.yml` that fails if a PR adds an `#[ignore]` without `= "..."`. The simplest implementation — `grep -rE '#\[ignore\s*\]' tests/ src/ | grep -v '#'` should be empty.~~
   **Done (Naryad #73 Block 3, 2026-08-14).** Implemented as a Rust integration test, `tests/ignore_reasons_lint.rs` (instead of a shell step in CI — more idiomatic for a codebase where all invariant checks are already done as cargo tests, see `registry_sync_check.rs`). The test scans every `.rs` file under `tests/`, `src/`, `examples/`, `self-host/`, `benches/`, and fails with a detailed list of violations if it finds a bare `#[ignore]`. It skips comments and string literals so it doesn't catch mentions of `#[ignore]` in docs/test data.
2. **Un-ignore by category**: future naryads can close entire categories at once. For example, "Naryad #N: closed parallel-test race category" — add `#[serial_test::serial]` to the 4 tests in `llm_cache_contract.rs` and remove `#[ignore]`.
   - **Subcategory 'Parallel-test race' closed (Naryad #75, 2026-08-14).** All 4 tests in `tests/llm_cache_contract.rs` are now under `#[serial_test::serial]`, `#[ignore]` removed. See PR `naryad-75-llm-cache-serial`.
3. **Quarterly audit**: recheck once a quarter whether any `#[ignore]` has become stale (e.g., if the JIT is integrated, remove all 6 ignores in `jit_golden.rs`).
4. ~~**Product decision on self-hosting**: the owner must choose Option A/B/C (see ADR-0023) — this will close the last category.~~
   **Done (Naryad #73 Block 3, 2026-08-14).** Decision: Option C (defer). See the updated ADR-0023 — the "Owner Decision" section. The test `self_host_lexer_tokenizes_m1_hello` remains under `#[ignore]` with a reason referencing ADR-0023 Option C. The category in the catalog above is closed.
