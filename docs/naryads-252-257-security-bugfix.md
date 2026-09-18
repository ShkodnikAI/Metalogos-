# Naryads #252–257 — bugs and vulnerabilities (audit 2026-09-11)

> Source: external security and reliability audit dated 2026-09-11
> (repository `main` @ `0bbcc5f`, PR #249 merged). Every item
> is verified by evidence — a grep over the code and/or a reproducible
> scenario, not on the word of the documentation (AGENTS.md §1).
>
> Priorities: P0 — close before any public launch of servers;
> P1 — the current quarter; P2 — planned hygiene; P3 — as opportunity allows.
>
> The "done" contract — AGENTS.md §8: branch → PR → green blocking
> jobs on the merge commit; one commit per logical step; the report
> names assumptions and unresolved items explicitly.

---

## Naryad #252 (P0, security) — sandbox_path_ex: TOCTOU escape via symlink on write

**Fact.** `src/builtins/io.rs:247` — `sandbox_path_ex` after all three
protection layers returns the **original** path, not the canonicalized one:
`Ok(std::path::PathBuf::from(path))`. For `SandboxMode::ForWrite`
only the parent directory is canonicalized; the final path component
is checked before the file is opened. Between `canonicalize(parent)` and
`fs::write(&safe_path, ...)` there is a TOCTOU window: if `a.txt` is replaced
by a symlink to a path outside the sandbox (e.g. `exec("ln -s /etc/passwd a.txt")`,
even under `METALOGOS_ALLOW_EXEC=1`, or another process in a shared directory),
the write follows the symlink outside the sandbox. The three protection layers of #131
check the filesystem state at check time, but not at
use time.

**Task.**
1. For reads: return the canonicalized path (today the check runs against
   the canonicalized path but the original is opened — the same hole, one class lower).
2. For a write to an existing file: open the final component with
   `O_NOFOLLOW` (unix) / re-canonicalize the opened file and re-check
   the base prefix. For a write to a new file: `create_new(true)`;
   on failure — treat as an existing file and follow item 2.
3. Errors — loud, with the text "file I/O sandbox: ..." (not soft-failure).

**§3.** `src/builtins/io.rs` (+ unit/integration tests). No changes to
the semantics of ordinary paths — all existing io golden tests stay green.

**Done, when:** an escape-reproduction test (unix-only, `#[cfg(unix)]`)
fails before the fix and is green after; a test for a broken symlink; a test
for "a directory instead of a file"; CI blocking jobs green on the merge commit.

---

## Naryad #253 (P1, security + owner decision) — exec() in the serve context: the process-global gate is inherited by routes

**Fact.** `src/builtins/io.rs:376` — `exec()` is permitted if the **process**
has `METALOGOS_ALLOW_EXEC=1`. `mlog serve` with this variable gives every
route handler (route code is an arbitrary mlog program, often
written by another person or generated) a full `sh -c` on
behalf of the server user. Subprocess audit logging is in place
(`append_subprocess_audit`, `src/builtins/io.rs:425`), but that is
after-the-fact: a record does not prevent. Separately: `exec_restricted`
is used by `html_render` and the pdf pipeline — check the allowed-binaries
list for closure (no argument substitution from user
input: `src/builtins/pdf.rs:2399` — `wkhtmltopdf` receives a temporary
html file, the arguments do not come from the request body — confirm with a test).

**Task.**
1. Owner decision (AGENTS.md §3 — the course of the project is decided by the owner):
   which gate for `exec()` in the serve context? Option A — in serve mode
   route programs do not inherit the process flag; a separate
   `METALOGOS_SERVE_ALLOW_EXEC=1` is required. Option B — leave as is,
   document in the threat model with the line "exec in routes —
   owner-responsibility, enabled by a process flag".
2. Test contract for the chosen option: a serve program calling `exec()` with
   the gate off gets a loud error (stable code per
   ADR-0131, e.g. `EXEC_NOT_PERMITTED`); with the gate on — it works,
   and an audit record is written.
3. Closure test for `exec_restricted`: the binary is fixed, the arguments
   are formed by code, not by the request body.

**§3.** `src/builtins/io.rs` (or `src/server.rs` for a context gate),
`src/audit.rs` (if a new diagnostic code), `docs/threat-model.md`,
`SECURITY.md`.

**Done, when:** the option is chosen by the owner; the tests of items 2–3 are green;
the threat model contains a line matching the actual behavior (§1).

---

## Naryad #254 (P2, bug/ux) — read_file: soft-failure masks a sandbox violation

**Fact.** `src/builtins/io.rs` (`builtin_read_file`) — any `Err` from
`sandbox_path` turns into an empty string: a sandbox violation
(absolute path, `..`, an unresolvable path) is indistinguishable from "no such file".
For a missing file, soft-failure is deliberate semantics; for a sandbox
violation it is a programmer error, and silently swallowing it means
hiding a real program defect: code with `read_file("../secrets")`
behaves identically to `read_file("typo.txt")`.

**Task.** Split the outcomes: file missing / unreadable — an empty
string (as today); sandbox violation — a loud error with a
stable code per ADR-0131 (e.g. `SANDBOX_VIOLATION`). The same
treatment for write_file/append/remove_file, where soft-failure returns
`Ok(())`/`false` on any `Err`.

**§3.** `src/builtins/io.rs`, `src/audit.rs` (the code), `src/semantic.rs`
(if the codes are registered there too), the README section on io builtins.

**Done, when:** test: `read_file("../x")` → an error with a code,
`read_file("no_such.txt")` → an empty string; the existing soft-failure
tests are not broken (if they break — the scenarios are revisited
loudly, not silently, in the manner of naryad #250's truth-ups).

---

## Naryad #255 (P2, hardening) — server: an explicit request body limit instead of the implicit axum default

**Fact.** `src/server.rs:688` — `route_handler` takes
`body: bytes::Bytes`. There is no explicit `DefaultBodyLimit` in server.rs
(grep is empty). The behavior rests on the implicit axum 0.8 default (~2 MB) —
the source of truth on the limit lives in someone else's crate and will
change silently on an upgrade. The limit in bytes is documented nowhere; the threat model
does not answer "what is the maximum request size the server accepts".

**Task.** Pin `DefaultBodyLimit::max(N)` explicitly at router construction
(N — a deliberate constant, e.g. 2 MB, with a justifying comment),
test: a body of N+1 bytes → 413, a body of N-1 → goes through; a line in
`docs/threat-model.md` and `REFERENCE.md` with the actual limit.

**§3.** `src/server.rs`, `docs/threat-model.md`, `REFERENCE.md`.

**Done, when:** the oversized-body test is green; the documented
number matches the code (§1); CI is green.

---

## Naryad #256 (P2, robustness) — fuzz: real targets for .mbc and the URL decoder; a smoke run in CI

**Fact.** `fuzz/fuzz_targets/fuzz_target_1.rs` — the single target
covers `parser::parse(str)`; bytecode deserialization (bincode,
`src/main.rs:222` — writing, reading .mbc in the load path) and the manual
`url_decode_fallback` (`src/server.rs:701`) are not covered by fuzzing.
Naryad #250 touched backward compatibility of old .mbc — the class
"malformed file → panic instead of a loud error" is not closed systematically.
CI does not run fuzz (it is not in the blocking list).

**Task.**
1. Target: `metalogos::bytecode` — deserialization of arbitrary bytes +
   (if the API allows) a safe dispatch run on a small instruction budget;
   crashes = panics, not "Err".
2. Target: `url_decode_fallback(&[u8])` — invariants: no panic,
   no out-of-bounds reads, for a well-formed `encodeURIComponent(s)`
   a round-trip restores `s`.
3. A CI smoke job: 2–3 minutes of budget per target (the nightly crate or
   cargo-fuzz run with `-max_total_time=120`); found panics are fixed
   with loud errors carrying stable codes (ADR-0131: `BYTECODE_INVALID`).

**§3.** `fuzz/fuzz_targets/*`, `fuzz/Cargo.toml` (if the targets are registered
there), `.github/workflows/*`, `src/error.rs`/`semantic.rs` (codes).

**Done, when:** both targets build and run; the CI job is green;
all found panics are closed (the list in the naryad report — §8.4).

---

## Naryad #257 (P3, hygiene) — url_decode_fallback: RFC 3986 conformance; replacement with a vetted implementation on divergence

**Fact.** `src/server.rs:701` (`url_decode_fallback`) — a manual
percent-decoder of query parameters; no property tests for the edge cases
(`%ZZ`, a truncated `%` at the end of the string, `+` as space or not, multibyte
UTF-8 `%D0%B6`, double encoding). Hand-rolled decoders are a classic
source of divergence from RFC 3986, and the input comes from external users
of the server. The naryad overlaps with fuzz target #256.2, but looks not at panics
but at **correctness**.

**Task.** A table of expectations per RFC 3986 + tests for the listed
edge cases; on divergence — replace the manual decoder with
`percent-encoding` (or strictly document the chosen behavior,
if it is a deliberate deviation — e.g. `+` is not expanded to a space
in query — exactly as in the RFC appendix, which it is not; choose and record it).

**§3.** `src/server.rs` (+ tests), `Cargo.toml` (if a new dependency —
per the dependency-discipline rules), `REFERENCE.md` (the documented
behavior of query_param).

**Done, when:** the edge cases are covered by tests and the behavior
matches the documented one; the dependency is either not added or
justified.

---

## Order and dependencies

- #252 — isolated, start with it (P0).
- #253 — starts with a question to the owner; the test part is independent.
- #254 — the first consumer of the ADR-0131 stable codes; goes after
  #252 (the same file, to avoid colliding in rebase).
- #255, #257 — independent, small.
- #256 — closes systematically what #252/254 fix pointwise;
  there is no need to delay #252 for its sake.

## Explicit audit assumptions (§8.4)

- The audit is static (grep + code reading + a local run of
  `readme_consistency`); the full test run and CI are the source of truth
  on regressions, not this paper.
- The LLM key leak through errors was checked by reading
  (`src/llm.rs:543`, `:561` — reqwest does not include headers in
  the `Display` of errors, the provider response body is truncated to 500 characters);
  this is not a test contract — to close it systematically, if desired, add
  a test "error strings contain no api_key" to #253.
- `checkpoints.db`, `test_memory.db` in the repo root — local artifacts,
  not tracked by git, no naryad required.
