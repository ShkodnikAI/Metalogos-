# Topic 3 — Errors and the try result

**Status: normative (the №645 protocol).** Every norm below was probed on
BOTH backends before it was written; the conformance pairs prove the
record on every CI run. Where the probes found a cross-backend
divergence, there is NO norm — the finding lives in «Honest limits» and
needs its own repair naryad. The topic-3 probe found NO cross-backend
divergence: the result SHAPE is built by the shared `try_result_struct`
on both backends (parity by construction), and the code classification
is the ONE shared classifier — divergence here is a bug the
cross-backend pairs catch on every run.

The stable refusal codes are the FROZEN contract (№385/ADR-0169): a norm
REFERENCES the codes (`STATUS: error` + `CODE:`), never redefines or
renames them.

### S-ERR-001 — try is an expression returning the TryResult struct

`try(expr)` evaluates the inner expression and returns the struct
`TryResult { ok: Bool, value, error }`; on success `ok` is `true`, `value`
is the inner value EXACTLY, and `error` is `Unit`. try never refuses
itself on the success path.
**Anchors:** TW the `Expr::Try` arm (src/interpreter/execution.rs:1684);
VM `Instruction::TryEval` (src/vm.rs:1527, compiled at src/compiler.rs:1369)
— the shape is built by the SHARED `try_result_struct`
(src/interpreter/values.rs:469), TW and VM cannot diverge structurally.
**Conformance:** `tests/conformance/serr_001_try_success.mlog`.

### S-ERR-002 — a runtime refusal inside try is caught; execution continues

A refusal raised while evaluating the inner expression (a builtin domain
gate, division by zero, a sandbox denial) does NOT abort the program:
`try` catches it, `ok` is `false` and `value` is `Unit`, and the
statements after the try run normally (try is an expression — the caught
refusal is a value, never a program stop).
**Anchors:** TW the `Err` arm of `Expr::Try`
(src/interpreter/execution.rs:1692); VM the `Err` arm of
`Instruction::TryEval` (src/vm.rs:1536).
**Conformance:** `tests/conformance/serr_002_try_catches_runtime.mlog`.

### S-ERR-003 — on a caught refusal, error is the TryError struct {code, message}

On a caught refusal `error` is the struct `TryError { code: String,
message: String }`: the `message` carries the FULL original refusal text
(its wording is deliberately NOT pinned — only its existence and its
String-ness are), and the `code` is the stable classification of S-ERR-004.
**Anchors:** TW/VM the shared `try_result_struct`
(src/interpreter/values.rs:469 — the `TryError` arm); the classification
feed `stable_try_error_code` (src/interpreter/values.rs:817).
**Conformance:** `tests/conformance/serr_003_error_struct_stamped.mlog`.

### S-ERR-004 — the code is the whitelisted origin stamp, else RUNTIME_ERROR

The `error.code` is classified by the ONE shared classifier (№385/
ADR-0169 — the frozen contract): a refusal whose message starts with a
WHITELISTED origin stamp (`[<CODE>] ` at position 0 — the
`ORIGIN_STAMPED_CODES` set, src/interpreter/values.rs:705) carries that
code; EVERY other refusal — unstamped, or stamped with a code outside the
whitelist — is honestly `RUNTIME_ERROR`. Classification is never by
message prose: a `[CODE]`-looking marker MID-message is content and
cannot forge a classification (the position-0 rule).
**Anchors:** TW/VM the ONE classification point `stable_try_error_code`
(src/interpreter/values.rs:817) over `split_origin_stamp`
(src/interpreter/values.rs:796) — called by BOTH backends' try arms, so
the codes agree by construction.
**Conformance:** `tests/conformance/serr_004_code_unstamped_fallback.mlog`,
`serr_005_code_nonwhitelisted_stamp.mlog`.

### S-ERR-005 — try does not catch semantic-blocking findings

An undefined name inside a try is a SEMANTIC-blocking finding: the
program refuses at the shared check gate BEFORE execution on BOTH
backends (the №523 blocking shape) with the stable code — try wraps
runtime evaluation, it never relaxes the compile-time contract.
**Anchors:** TW/VM the shared `semantic::check_program` blocking gate on
the production paths (the conformance runner shape, tests/conformance.rs;
the `UNDEFINED_VARIABLE` stamp at the origin — src/interpreter/values.rs:507).
**Conformance:** `tests/conformance/serr_006_try_not_catch_semantic.mlog`.

### S-ERR-006 — an inner try result is a first-class value

The result of an inner `try` is an ordinary value: an outer `try` around
an inner `try` SUCCEEDS (`ok` is `true`) with the inner TryResult struct
as its `value` — a caught inner refusal is DATA the program can branch
on, not a signal the outer try re-raises.
**Anchors:** TW the `Ok` arm of `Expr::Try`
(src/interpreter/execution.rs:1690 — the inner result is just a value);
VM the `Ok` arm of `Instruction::TryEval` (src/vm.rs:1533).
**Conformance:** `tests/conformance/serr_007_try_nesting.mlog`.

## Honest limits (the probe findings this topic records WITHOUT norms)

The topic-3 probe (the №645 protocol: every case run through the
production paths of BOTH backends) found NO cross-backend divergence —
the shape parity is by construction (the shared `try_result_struct`) and
the code parity is by construction (the ONE shared classifier). The
edges below are recorded honestly as SCOPE notes, not findings:

- **The whitelist is the frozen №385 surface.** The pairs above pin the
  classification MECHANISM (position-0 whitelisted stamp, else
  `RUNTIME_ERROR`) with two representative codes (SANDBOX_VIOLATION —
  whitelisted; IO_ERROR-stamped and unstamped text — the honest
  fallback). The OTHER whitelisted families (LLM_TIMEOUT,
  MEDIA_SEALED_EGRESS, HTTP_*, DATE_INVALID, FORECAST_TAINTED, ...) are
  covered by `tests/naryad_385_try_codes.rs` (the deterministic fault
  seam) and their own contour tests — this topic does not duplicate
  them; adding a code to the whitelist is a №385-contract change (an
  ADR), never a spec edit.
- **The `[try] caught error` stderr line is a diagnostic side-channel,
  not program output.** The runner pins the stdout OUT only; the
  side-channel wording is not a contract.
- **The compile-time vs runtime boundary follows the semantic checker.**
  What the shared `check_program` blocks is refused regardless of try
  (S-ERR-005); what it passes reaches runtime, where try catches. The
  boundary MOVES only when the checker's blocking set changes — that is
  a semantics-change naryad (the owner's gate), never a silent fit.
