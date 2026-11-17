# The promise sheet — the public security claims (extracted verbatim, the black-box contract)

The promises below are the PUBLIC security contract of the tool under
attack (version: the candidate under review, the label-lattice line).
The adversary attacks these TEXTS with observed behavior — nothing
else.

## The flow output edge (the boundary under review)

- **P1.** A flow whose pipeline legitimately ends in model, remote or
  own-file data (`call_llm`, `mcp_call`, `read_file`, html/xml
  extraction) and whose `-> output` terminator is NOT annotated REFUSES
  `mlog check` with `[HTML_INJECTION]` when the outgoing edge carries
  untrusted data. The refusal names the idiom form.
- **P2.** A flow whose outgoing edge carries private/secret data (e.g.
  `env(...)` read inside a pipeline step) REFUSES with
  `[PII_EGRESS_OUTPUT]`.
- **P3.** Declaring `-> output<public, untrusted>` on a flow whose
  computed edge actually carries `public, untrusted` PASSES `mlog
  check` (the display of model/remote output is the intended idiom).
- **P4.** A declaration WEAKER than the computed edge — e.g.
  `-> output<public, trusted>` over an untrusted edge — REFUSES with
  the annotation-mismatch error. The annotation cannot silence what the
  program computes.
- **P5.** A declaration whose confidentiality word is anything other
  than `public` REFUSES.
- **P6.** A private edge REFUSES even when the terminator is annotated
  — no annotation waives the confidentiality half.
- **P7.** The annotation grammar is strictly additive: a program with
  no terminator annotations parses exactly as before (no new parse
  errors on previously-valid programs).
- **P8.** An unparseable annotation (`-> output<bogus>`) is a loud
  label error, never silence.

## The general refusal classes (the standing README claims)

- **P9.** `env()` into `respond()`/`http_post`/`exec`/VCS refuses
  (`SECRET_LEAK` / `SECRET_EGRESS_NETWORK` / `SECRET_TO_EXEC` /
  `SECRET_EGRESS_VCS`).
- **P10.** Unsanitized LLM / request output into HTML refuses
  (`HTML_INJECTION`).
- **P11.** Untrusted data steering `exec` refuses
  (`UNTRUSTED_EXEC_DECISION`).
- **P12.** Quarantine absorbs every value: a poisoned value has no
  legal sink except `quarantine_write`.

## The verdict discipline

A **BYPASS** means the promise TEXT is broken by OBSERVED behavior of
the binary (`mlog check` / `mlog run` transcripts in `pocs/`). An
inconclusive probe is not a bypass. Every probe is
`pNN_<shape>.mlog` + its transcript; the report table lists probe →
promise → verdict.
