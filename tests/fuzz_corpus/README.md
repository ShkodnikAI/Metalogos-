# The TW/VM Divergence Corpus (№465 fuzzer, №476 rule)

The corpus of the KNOWN TW/VM divergence classes — the ratchet of the
differential fuzzer (`tests/naryad_465_diff_fuzzer.rs`). A class
signature (outcome/errorclass + normalized sides) pins a systematic
backend gap; each class is fixed by a SEPARATE naryad; a class
disappearing from `known_divergences.txt` = the gap closed (remove the
line in the same PR as the fix); a NEW class failing CI is a regression
— investigate before pinning, never silently.

## The №476 BLOCKED-DOMAIN rule (blocking, enforced in CI)

Classes touching **SQL, filesystem, network, exec, labels, or secrets**
are BLOCKING correctness errors — pinning them as known is FORBIDDEN:

- every line in `known_divergences.txt` carries the `known-` prefix
  (a bare line is read as `known-` for back-compat);
- a `blocked-`-prefixed line in the corpus file FAILS CI with the line
  named — there is nothing to pin in a blocked domain, fix it;
- at run time, a divergence whose RAW error/outcome text carries a
  blocked-domain marker (database/sql/sqlite, read/write_file/fs_gate,
  http/socket, exec/shell, label/taint, secret/credential) NEVER
  matches as known — the run fails naming the class, even if someone
  pinned it;
- the corpus holds LANGUAGE-CORE classes only — never state domains
  (DB/FS/net/exec/labels/secrets).

Why: the audit 26.09 (§3.2/§3.4) found the known-list acting as a
SHELTER for correctness defects — the query_row SQL divergence was
pinned (and even unit-pinned at db_ops.rs) instead of repaired. The
repair (№474) closed the class; this rule makes the whole domain class
un-pinnable going forward.

## The №479 line format (readable and machine-checked)

Every signature line in `known_divergences.txt` is a coded, node-tagged
class:

    known-errorclass|tw=err:[TYPE_MISMATCH]@fn_call|vm=err:[UNDEFINED_FUNCTION]@fn_call

- the STABLE ERROR CODES (`[CODE]` origin stamps, ADR-0131) of both
  sides — a wording change can no longer move a class;
- `@<node>` — the AST node kind each error was born at (`fn_call`,
  `ident`, `binary_op`, `each`, `other`);
- `err:uncoded(...)` on a side is an HONEST GAP — the origin site lacks
  its stable code; assign the code first, then pin.

Each signature line MUST be preceded by a contiguous comment block:

    # class: <the human description of the systematic gap>
    # example: <a checked-in minimal .mlog in this dir>
    # status: open — <the repair lane> / closed by <naryad, PR, merge>

The corpus parse FAILS CI when a line loses its block. The example is
LOAD-BEARING: both backends run every corpus `.mlog` (examples included)
on every fuzzer invocation, and when an example stops reproducing its
class the run FAILS demanding the line and the example be removed in the
fix PR — the "class disappeared = the gap closed" ratchet now
self-enforces. New classes enter ONLY with the naryad report that
explains them; the minimized catch of the discovering run is written to
`target/fuzz_min/`.

## The №479 stateful generation

The generator emits, per seed: an in-memory-SQLite `db` declaration +
`query`/`db_execute` calls (the №474 ONE db_execute contract),
`call_llm` (the deterministic mock — default `METALOGOS_LLM_MOCK=on`,
no network), `try smtp_send` (the deterministic config-refusal — the
honest MockSmtp: no SMTP env in tests, so no connection is ever
attempted) and the memory group. The duplication zones the audit named
(DB, memory, mail, LLM) are now exercised by the fuzzer itself — a
backend divergence there is caught HERE, not only by unit tests. The
№476 blocked-domain rule applies at full strength to everything the
stateful group finds.

## Files

- `known_divergences.txt` — the coded class signatures with their
  `# class:/# example:/# status:` blocks (the ratchet);
- `class_example_*.mlog` — the load-bearing minimal programs, one per
  pinned class (reproduce the class every run; removal = the fix PR);
- `seed_*.mlog` — the checked-in seed programs (both backends run them
  every fuzzer invocation);
- `last_report.txt` — the latest run's class census (written by the
  fuzzer; the KNOWN/NEW counts per class, with the RAW side texts and
  the program's AST node-kind view a reviewer reads).
