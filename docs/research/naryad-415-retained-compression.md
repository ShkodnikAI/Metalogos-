# Naryad №415 — retained-representation compression (path A, gh#527)

**Executor:** Super Z · **Date:** 2026-09-21 · **Base:** main @ `b8a5976` · **Issue:** gh#569
**Owner decision:** path A (comment 5751899756 in gh#527) — keep fighting the memory gate,
one-wave limit, then re-gate №3 under the SAME thresholds.

## 1. The diagnostic finding (before)

`examples/retained_diag.rs` (versioned) on the Stage-4 bench corpus
(`benches/fixtures/production_workload.mlog`, 2 344 lines, 14 routes):

| Probe (pre-№415) | Value |
|---|---|
| Pattern bodies INLINE in `main_code` (`RegisterPattern(CompiledFn)`) | **175 bodies, 9 243 instructions, 131 KiB serialized** |
| `Program::patterns` table | **0 entries — a dead field** (`compiler.rs:301`: `patterns: Vec::new(), // will be filled from pass1 data` — never filled) |
| №402 shared snapshot `pre_registered_patterns` | full CLONE of every inline body → **2 copies of the entire pattern mass in serve** |
| `size_of::<Instruction>()` | **160 B** — dictated by `FlowExec{FlowExpr(Value 96 B)+Vec+Vec}` ≈ 152, `RegisterLearnable` ≈ 88+, `Const(Value)` = 104 |
| In-RAM mass of one body copy | 9 243 × 160 B ≈ **1.48 MB** |
| Retained duplicate in serve (inline + snapshot) | ≈ **2.96 MB in-RAM** (2 × 1.48 MB), before compile-spike and clone-spike contributions |
| rules / deny_handlers / skill_indices / globals / schema_ddl / learnables | all **0 entries** on this corpus — their Arc-ing cannot move this gate |
| Route bodies | 14 routes, 135 instructions, ~2 KiB — **lazy route-body compilation is N/A for this gate** (the Stage-4 plan hits all 14 routes every round, so laziness moves no watermark) |
| `.mbc` wire size | 132 KiB (floor estimate of the wire mass) |

Why the RSS gate reads ×1.126–×1.129 while the retained delta is "only" ~4.3–4.6 MB:
the Stage-4 harness measures the **VmHWM of its own process**, which holds 5 ×
compile spikes (STARTUP_RUNS), 5 × (parse + compile + routes) spikes, 20 × DSL runs
that clone the whole `Program`, and the in-process server (interpreter AST + VM
bytecode). The representation mass participates in every spike — shrinking it lowers
both the retained baseline and the spike ceilings.

## 2. The levers implemented

1. **Pattern table, zero duplicate** — the compiler now FILLS `Program::patterns`
   (one canonical copy of every body, pass2 emission order = pass1 index order) and
   emits the new append-only `Instruction::RegisterPatternRef(u32)` (4-byte index)
   instead of the inline `RegisterPattern(CompiledFn)`. The №402 shared snapshot
   becomes a plain `Arc` increment over the table (`pre_registered_patterns()`);
   the legacy inline variant stays for old-.mbc deserialization (payload boxed),
   with the pre-№415 scan kept as the fallback.
2. **Instruction payload compaction** — fat payloads boxed:
   `Const(Box<Value>)`, `SinkCheck(Box<SinkCheckData>)`, `MakeStruct(Box<MakeStructData>)`,
   `FlowPipeline(Box<FlowPipelineData>)`, `FlowExec(Box<FlowExecData>)`,
   `Mutate(Box<MutateData>)`, `StoreAssignLocal(Box<StoreAssignLocalData>)`,
   `MatchTest(Box<MatchTest>)`, `RegisterLearnable(Box<CompiledLearnableInfo>)`,
   legacy `RegisterPattern(Box<CompiledFn>)`, `LabelJoin(Box<LabelJoinData>)`.
   `Box<T>` serializes exactly as `T` under bincode (serde deref impls), and every
   struct-variant payload became a struct with the SAME fields in the SAME order —
   the .mbc wire format is byte-identical for all boxed variants.
3. **`Arc<Vec<CompiledFn>>` in `Program`** — serde `rc` feature (wire-identical to
   `Vec`), enabling the zero-clone snapshot and a zero-clone `program.clone()` of
   the table across the harness's clone spikes.

## 3. Measured after (same corpus, same probe)

| Probe (post-№415) | Value | Δ |
|---|---|---|
| Inline bodies in `main_code` | **0** (main_code = 178 instructions, all top-level) | −175 bodies / −9 243 instructions |
| `Program::patterns` table | **175 entries** — the single canonical copy | dead field → canonical store |
| Serve duplicate mass | **~0** (snapshot = Arc increment) | −≈2.96 MB in-RAM retained |
| `size_of::<Instruction>()` | **≤ 32 B** (asserted by `n415_instruction_size_is_pointer_scale`) | −80% |
| `.mbc` wire size | 133 KiB | +1 KiB (the refs + table header) |

Projected effect on the CI gate: the VM-side peak loses the retained duplicate
(~2.96 MB) plus a proportional share of every compile/clone spike. Whether this
closes the ×1.126 → ×1.1 gap is decided ONLY by re-gate №3 (3 pinned runs on the
merged main, protocol №398, thresholds unchanged: p95 ≥ ×1.5 AND peak RSS ≤ ×1.1
on 3/3). The series record and the verdict package land in gh#527.

## 4. Compatibility

- Old binaries reading NEW bytecode: fail loudly at deserialize (unknown
  `RegisterPatternRef` variant index) — the №264 append-compat precedent.
- New binaries reading OLD .mbc: the legacy `RegisterPattern(Box<CompiledFn>)`
  variant deserializes identically (Box is wire-transparent); `load_program`
  rebuilds the table via the legacy scan fallback; the run() path keeps the
  №402 make_mut/CoW semantics for legacy bodies.
- New bytecode on the run() path: `RegisterPatternRef(idx)` is a validation
  no-op (the table already holds the body); an out-of-range index fails LOUDLY
  (`n415_out_of_range_ref_fails_loudly`) — the №264 backstop contract.

## 5. Test coverage (tests/naryad_415_retained.rs)

1. `n415_instruction_size_is_pointer_scale` — the ≤ 32 B size contract.
2. `n415_pattern_table_is_the_single_copy` — table filled, main_code carries refs.
3. `n415_snapshot_is_zero_clone_over_the_table` — `Arc::ptr_eq` snapshot ↔ table.
4. `n415_roundtrip_preserves_table_and_refs` — .mbc wire round-trip + run.
5. `n415_legacy_wire_still_loads_and_runs` — legacy shape loads + scan rebuild.
6. `n415_out_of_range_ref_fails_loudly` — the loud backstop.
7. `n415_serve_route_dispatches_table_pattern` — route → table → CallPattern parity.

Full suite: 739 lib tests + 206 integration targets green locally (batched runs;
the one local failure `naryad_335_consent::ledger_export_builtin_writes_sandboxed_file`
reproduces on clean main `b8a5976` via `git stash` — a local-environment artifact,
not a №415 regression; blocking CI is the arbiter).
