# ADR-0194: The query-parity decision package — the db_ops error paths and the query result contract (PROPOSED; the owner's verdict pending)

- Status: **Proposed** (the draft, №711; BOTH directions — the error-path
  posture and the result contract — belong to the OWNER's verbatim: the
  language gate, the gh#1300 verbatim №538. Nothing here self-captures:
  the §5 OD-дисциплина, the №659-b precedent)
- Date: 2026-10-17 (the draft, №711, Волна 49)
- Deciders: the executor (the draft); the OWNER fixes the direction
- Naryad: №711 (issue #1318, Волна 49)
- Predecessor: [ADR-0171 — the VM production readiness](0171-vm-production-readiness.md)
  (the serve lane is the VM by default); the №474 class (issue #722)
- The finding source: gh#1300 (№702 — the verbatim refusal table: «the
  EXECUTION result type depends on the backend»); the pinned candidate
  `db_ops.rs:690-698` (the №480/№540 pair comment)

## §1. Context — what is divergent in `query()`

One builtin name, two lanes, TWO distinct forks:

1. **The RESULT CONTRACT — reachable and observable.** A mutation
   through `query()` — `query("INSERT/UPDATE/DELETE/CREATE…")` —
   returns the affected-row count as a **String** on the TW lane
   (`db_ops.rs:200-204`, the `else` branch of `query_tw`), while the
   VM lane builds rows UNCONDITIONALLY and returns an **empty List**
   (`db_ops.rs:699+`, `query_vm` — sqlite steps the statement to
   completion, no rows come back, the affected count is discarded).
   SELECT/PRAGMA is in parity (both lanes produce the List of Row
   structs).
2. **The ERROR PATHS — verified practically unreachable.** The TW lane
   silently SKIPS an erroring row (`query_map` + `filter_map(r.ok())`,
   a cell error maps to `Unit`), the VM lane fails LOUDLY (row/column
   errors name the column). The №480/№540 pair comment
   (`db_ops.rs:690-698`) called this "a parity-fix candidate (the №474
   class), not pinned here" — THIS ADR is the pin.

The result-contract fork is the SAME class the №474/№553 line closed
for `db_execute` (the affected count as a String on BOTH backends —
the ONE contract, pinned by `tests/naryad_474_db_parity.rs`);
`query()` mutations kept the legacy asymmetry.

## §2. The live probe record (№711 task 1, the draft worktree)

The probe ran on the lib (both backends, the n2-style harness; the
programs are the probe record, not shipped pins — the №476 rule keeps
SQL classes out of the fuzzer corpus):

| probe | TW | VM | verdict |
|---|---|---|---|
| `query("INSERT INTO t VALUES ('x'),('y'),('z')")` | `n=3` (String) | `n=[]` (empty List) | **the divergence is LIVE** |
| `query("SELECT a FROM t")` (control) | `[Row {a: x}, Row {a: y}]` | identical | SELECT parity holds |
| `query("SELECT a, a FROM t")` (duplicate columns) | `[Row {a: x}]` | identical | HashMap collapse, NO error path through the name shape |

The unreachability reasoning (why the error paths cannot be reached by
live program input over embedded sqlite):

- `row.get_ref(i)` errs ONLY on `InvalidColumnIndex` — the index
  enumerates `0..col_names.len()` taken from the SAME statement's
  `column_names()` → in-bounds by construction. `get_ref` performs NO
  type conversion (`ValueRef` is typeless) → `InvalidColumnType` is
  impossible on this call shape.
- The TW cell branch `Err(_) => Value::Unit` (`db_ops.rs:186`) is
  therefore dead, and the mapping closure cannot fail (always `Ok`) →
  the `filter_map(|r| r.ok())` (`db_ops.rs:196`) never drops a row.
- The VM row error (`rows.next()` → the sqlite step error) requires an
  OS/IO fault (a full disk, a corrupt file) — not constructible by a
  program input on an embedded connection.
- Blob/NUL oddities do not error on either lane (`from_utf8_lossy` /
  hex encode) — the SELECT control confirms the shapes.

**VERDICT:** the "practically unreachable" claim (`db_ops.rs:695`) is
CONFIRMED — the наряд's refutation trigger (a reachable error path →
STOP verified-wait) did NOT fire. The error-path fork is a dead-code
semantic fork; the result-contract fork is LIVE and observable.

## §3. The dependency inventory (№711 task 2)

- `examples/` with `query(` calls: `p99_sql_param_type_error.mlog`,
  `v05_final_integration.mlog`, `compat/p7_query_readable.mlog`,
  `compat/p6_full_app.mlog`, `dept_schema.mlog`, `w385_try_sql.mlog` —
  all SELECT-shaped; none relies on the mutation-through-`query()` form
  (mutations ride `db_execute` in the examples).
- Tests: `naryad_474_db_parity.rs` (the `db_execute` String-both pin),
  `n553_sql_params_parity.rs`, `crosscheck_backends.rs` — none pins
  the `query()` mutation shape (the divergence is unpinned today).
- The fuzzer/golden corpus: the SQL state domain is REFUSED at pin
  time (№476 — SQL classes are never pinnable as known) — this ADR §2
  carries the probe record instead.
- The Fosved-контур (№329/№345): the office programs consume `query()`
  SELECT lists; the office mutation pattern rides `db_execute`.

## §4. Decision package A — the error-path direction (the owner picks one)

- **(a) lift TW to loud** (the VM semantics): a cell error becomes a
  loud Err naming the column (the parity with `db_ops.rs:739`), the
  row error propagates. The dead branches become live-but-unreachable
  — zero observable change on reachable inputs; the code loses the
  silent `Unit` lie.
- **(b) drop VM to TW-skip**: the VM catches row/column errors and
  skips silently — a SILENT skip on a fault class; NOT recommended
  (the №604 class-4 «тихо-неверное поведение» shape; the fail-closed
  posture №523/№531 points the other way).
- **(c) pin loud-fail on BOTH lanes by contract**: today equivalent to
  (a) (the VM is already loud) — the wording makes the fail-closed
  reaction the CONTRACT, not an accident of each lane's history.

**Recommendation: (c)** — pin the loud-fail contract on both lanes
(the wording of (c), the mechanics of (a)).

## §5. Decision package B — the query result contract (the owner picks one)

- **(i) String-count on both**: the mutation branch mirrors the №474
  ONE contract — the VM gains the mutation detection (`query("UPDATE…")`
  returns `"3"` on both; SELECT keeps the rows). **Recommended:** it
  matches `db_execute`, removes the observable divergence, and keeps
  the least-surprise reading («query = the SQL executor: SELECT → rows,
  else → the affected count»).
- **(ii) rows on both**: the TW mutation would return an empty List —
  inventing a rows contract for statements that produce no rows; NOT
  recommended.
- **(iii) tagged result** (`Struct {kind: "rows"|"count", …}`): the
  honest-but-breaking option — every consumer changes, a MINOR+ (the
  borderline MAJOR) semantic needing its own migration наряд.

**Recommendation: (i).**

## §6. The ratchets while Proposed

- The limitations row (the TW/VM Divergences section) documents the
  live result-contract fork with the anchors; NOTHING pins it as a
  fuzzer-known class (№476).
- No behavior changes until the owner's verbatim: the error-path dead
  branches stay byte-identical, the mutation shapes stay as they are.
  The №465 fuzzer, the leak-suite, the goldens, the mirrors 6/6,
  dups 0, the debt-gate — untouched (the CI blocking set on this PR).

## §7. What lands with this ADR (the №711 mechanics)

- The candidate comment (`db_ops.rs:690-698`) → **PINNED**: the tail
  now names №711 + this ADR (Proposed; the owner's verbatim pending).
- The limitations row (the honest-limits entry — the №651 лекало).
- No divergence-pinning test (the №476 rule) — §2 is the record.
