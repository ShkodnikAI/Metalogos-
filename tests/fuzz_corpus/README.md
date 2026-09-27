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

## Files

- `known_divergences.txt` — the class signatures (the ratchet);
- `seed_*.mlog` — the checked-in seed programs (both backends run them
  every fuzzer invocation);
- `last_report.txt` — the latest run's class census (written by the
  fuzzer; the KNOWN/NEW counts per class).
