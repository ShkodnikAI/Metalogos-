# ADR-0194: The query-parity decision package — the db_ops error paths and the query result contract (ACCEPTED 2026-10-20; the owner's verbatim — packages A(c)+B(i))

- Status: **Accepted** (2026-10-20 — the owner's live verbatim «ADR-0194
  вербатим (пакеты A/B) давай сделаем с твоей рекомендацией и ссылкой
  что куда вносить» with the package-choice DELEGATION to the executor
  (the В49-final lekalo «по твоей рекомендации. Продолжай»); the
  packages fixed per the §4/§5 recommendations: **A = (c)** — the
  loud-fail contract on BOTH lanes (the wording of (c), the mechanics
  of (a)); **B = (i)** — the String-count mutation parity (the №474
  ONE contract); the landing — №734, issue gh#1364; the §8
  ratification record)
- Date: 2026-10-17 (the draft, №711, Волна 49); 2026-10-20 (the
  ratification, №734, Волна 51 tail)
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
   (`db_ops.rs:217-222`, the `else` branch of `query_tw`), while the
   VM lane builds rows UNCONDITIONALLY and returns an **empty List**
   (`db_ops.rs:721+`, `query_vm` — sqlite steps the statement to
   completion, no rows come back, the affected count is discarded).
   SELECT/PRAGMA is in parity (both lanes produce the List of Row
   structs). [Resolved 2026-10-20: the package B(i) landing — §8.]
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
  [Resolved 2026-10-20: the package A(c) landing — §8 — the cell error
  is the loud `Err` naming the column (`db_ops.rs:189-199`), the row
  error propagates (`db_ops.rs:209-215`).]
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
  loud Err naming the column (the parity with `db_ops.rs:773`), the
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

## §7. What landed with this ADR (the №711 mechanics)

- The candidate comment (`db_ops.rs:712-720` — the №690-698 anchor
  shifted by the landing edits) → **PINNED**: the tail now names №711
  + this ADR (Proposed; the owner's verbatim pending).
- The limitations row (the honest-limits entry — the №651 лекало).
- No divergence-pinning test (the №476 rule) — §2 is the record.

## §8. The ratification — 2026-10-20 (the owner's verbatim; the landing №734)

**The verbatim (дословно, 2026-10-20):** «ADR-0194 вербатим (пакеты
A/B) давай сделаем с твоей рекомендацией и ссылкой что куда вносить».

**The delegation:** the package choice is delegated to the executor
(лекало В49-финал «по твоей рекомендации. Продолжай»); the executor
fixed the packages per the §4/§5 recommendations of this ADR:

- **Package A = (c)** — pin loud-fail on BOTH lanes by contract: the
  TW cell error is the loud `Err` naming the column (the
  `db_ops.rs:773` mirror — `db_ops.rs:189-199`), the row error
  propagates (`db_ops.rs:209-215`); the fail-closed reaction is the
  CONTRACT (the №523/№531 posture), not an accident of each lane's
  history. Zero observable change on reachable inputs (§2 —
  `get_ref` in-bounds by construction); the batteries, not a
  dead-branch unit test, carry the verification.
- **Package B = (i)** — String-count on both: the VM mutation detect
  mirrors the TW byte-for-byte (`trim().to_uppercase()`; NOT
  SELECT/PRAGMA — `db_ops.rs:748-759`); `query("UPDATE…")` returns
  `"3"` on both lanes; SELECT keeps the rows. The №474 ONE contract,
  pinned by `tests/n194_query_parity.rs`.

**The rejected variants — why (one line each):**

- **(ii) rows on both** — invents a rows contract for statements that
  produce no rows; the TW's String-count shape IS the least-surprise
  reading standing since Наряд №7.
- **(iii) tagged result** — honest, but breaks every consumer (a
  MINOR+/borderline MAJOR semantic); possible as a SEPARATE future
  ADR with its own migration наряд — NOT here.

**The §6 ratchets — lifted by this ratification:** no behavior changes
until the owner's verbatim — LIFTED: the packages landed (№734, the
same wave tail); the limitations row moved to the resolved form; the
mutation shapes are pinned by the parity test. The §2 probe record
remains the error-path unreachability carrier (the №476 rule holds —
no SQL classes in the fuzzer corpus).
