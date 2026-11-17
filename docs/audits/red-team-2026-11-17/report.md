# Red-team report — flow output edge (P1–P12), black-box pass

Binary under attack: `/tmp/redteam-967/mlog` (`mlog check <file>`, `mlog run <file>`, plus `compile`, `--reference`, `audit` used as divergence surfaces). Knowledge base: binary behavior + `pocs/PROMISES.md` only. No repository sources, tests, docs or git were read. All transcripts are verbatim in `pocs/transcripts.md`; every probe program is a `pNN_<shape>.mlog` in `pocs/` (import probes in `pocs/import_probe/`, bytecode probe `p45_clean_ann.mbc`).

## Executive summary

The pass attacked the twelve promise texts with 54 recorded probes concentrated on the flow output edge (P1–P8). The attack surface was enumerated black-box: the label vocabulary (`public|consented|private|poisoned`), the annotation grammar (label parts, `consent(...)` parts, duplicate checks), the sanitizer set (`escape_html`, `redact`), the sink set (`print`, `respond`, `http_post`, `exec`, `git_push`, `quarantine_write`), and every container the binary's parse errors leaked (`tool`, `mlogserver`, `branch`, `template`, `learnable`, `sandbox`, `hook`, `spec`, `memory`, imports). **No bypass of any promise text P1–P12 was found.** Every untrusted source tried (`call_llm`, `mcp_call`, `http_get`+`html_text`/`html_select`, `read_file`) refuses an unannotated output edge with `[HTML_INJECTION]` naming the idiom form; the private edge refuses with `[PII_EGRESS_OUTPUT]` unannotated, annotated, redacted, escaped, consent-decorated, loop-accumulated, chain-relayed and control-flow-merged; weaker or non-public annotations refuse with `[LABEL_ANNOTATION_MISMATCH]`; unknown, partial, duplicated, unicode-look-alike and misplaced annotations are all loud. `mlog run` re-enforces the same invariants (no check/run divergence found on any probe), `mlog compile` refuses Category A findings, and the `--reference` oracle backend agrees.

The strongest adverse results are two adjacent gaps rather than promise breaks. First, `exec` used as a flow step passes `mlog check` with **no annotation** and, once the two runtime gates are opened (`profile capability { allow_carrier: "exec" }` plus `METALOGOS_ALLOW_EXEC=1`), the command's stdout is emitted on the display edge verbatim (`p38_exec_step_stdout_display`) — command output is classic untrusted data, but `exec` is not among P1's enumerated sources (`call_llm`, `mcp_call`, `read_file`, html/xml extraction), so P1's text is not broken; it is an untrusted channel the display-edge rule does not cover. Second, the error diagnostics for a private edge are self-contradictory (`p08_env_annotated_priv_trust_deadend`): the PII refusal advises `-> output<private, trusted>`, and declaring exactly that is rejected as "a weaker label than the computed edge 'private, trusted'" — the suggestion re-appears verbatim, so the display path for private data is a diagnostic dead end (refusal is guaranteed, which is why this is a soundness/usability bug, not a bypass).

Three vectors could not be closed black-box and are recorded as INCONCLUSIVE, not bypasses: (1) **flow branch definitions** — `branch { }` parses as an empty no-op and ~80 body shapes (pipelines, calls, conditions, keywords) were all rejected, so the "branch definitions and branch returns" smuggling vector could not even be constructed; (2) **mlogserver routes** — the `route_decl` rule exists but no spelling of a route (`route "/" { }`, `route: "/"`, method-prefixed, ident-led) parsed in ~15 shapes; (3) **P7's "parses exactly as before"** — with only one binary, pre-annotation behavior is unobservable; two grammar quirks were found that at least plausibly changed with the feature (a pattern named `output` used mid-pipeline cannot parse; a plain string containing `{{x}}` mangles the surrounding program text) — both fail loudly, so no silence was produced. P9–P12 held: `SECRET_LEAK`, `SECRET_TO_EXEC`, `SECRET_EGRESS_VCS`, `UNTRUSTED_EXEC_DECISION`, `HTML_INJECTION` and the `quarantine_write` audit event were all observed as promised, with one cosmetic deviation (env→`http_post` reports `SECRET_LEAK` + `PII_EGRESS_NETWORK`, not the promised `SECRET_EGRESS_NETWORK` class string).

**Verdict tally: 54 probes — 0 BYPASS, 47 CAUGHT, 7 INCONCLUSIVE.**

## Probe table

| id | shape (few words) | promise | `mlog check` | `mlog run` | verdict |
|---|---|---|---|---|---|
| p01 | baseline clean flow, no annotation | P7 | clean (exit 0) | ran, printed "doc" | CAUGHT |
| p02 | call_llm → output unannotated | P1 | refuse `[HTML_INJECTION]`, names idiom | refuse (exit 1) | CAUGHT |
| p03 | call_llm → output<public, untrusted> | P3 | OK (exit 0) | runs with mock LLM | CAUGHT |
| p04 | output<public, trusted> over untrusted edge | P4 | refuse `[LABEL_ANNOTATION_MISMATCH]` | refuse | CAUGHT |
| p05 | env() edge, unannotated | P2 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p06 | env edge + output<public, untrusted> | P6 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p07 | env edge + output<public, trusted> | P6 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p08 | env edge + output<private, trusted> (identical) | P5 | refuse MISMATCH (dead-end, see F1) | refuse | CAUGHT |
| p09 | output<private, untrusted> over clean literal edge | P5 | refuse MISMATCH | refuse | CAUGHT |
| p10 | env() in the flow INPUT binding | P2 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p11 | env + redact(x, "email") | P2 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p12 | env + escape_html | P2/P6 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p13 | http_get + html_text → output unannotated | P1 | refuse `[HTML_INJECTION]` | refuse | CAUGHT |
| p14 | read_file → output unannotated | P1 | refuse `[HTML_INJECTION]` | refuse | CAUGHT |
| p15 | mcp_call (4 args) → output unannotated | P1 | refuse `[HTML_INJECTION]` | refuse | CAUGHT |
| p16 | escape_html(call_llm) unannotated (sanitizer) | P1 | OK (edge no longer untrusted) | escapes for real at runtime | CAUGHT |
| p17 | env edge + consent(a) third part | P6 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p18 | each-loop accumulator of html_select items | P1 | refuse `[HTML_INJECTION]` | refuse | CAUGHT |
| p19 | 3-pattern relay of env value | P2 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p20 | env in one `if` arm, "safe" in other | P2 | refuse `[PII_EGRESS_OUTPUT]` | refuse | CAUGHT |
| p21 | output<public, untrusted> over trusted literal edge | P4 | OK (conservative claim) | ran | CAUGHT |
| p22 | output<consented, untrusted> over untrusted edge | P5 | refuse MISMATCH | refuse | CAUGHT |
| p23 | output<poisoned, untrusted> over untrusted edge | P5 | refuse MISMATCH | refuse | CAUGHT |
| p24 | pattern named `output` used mid-pipeline | P7 | parse error | parse error | INCONCLUSIVE (F6) |
| p25 | string literal containing `{{x}}` in pattern | P7 | parse error (mangled source) | parse error | INCONCLUSIVE (F5) |
| p26 | output\<bogus\> | P8 | loud `unknown label word 'bogus'` | refuse | CAUGHT |
| p27 | output\<untrusted\> (no conf word) | P8/P5 | loud "no confidentiality word" | refuse | CAUGHT |
| p28 | output\<public\> over untrusted edge | P4/P8 | refuse MISMATCH (weaker claim) | refuse | CAUGHT |
| p29 | Cyrillic look-alike `рublic` in annotation | P8 | loud parse error | parse error | CAUGHT |
| p30 | duplicated annotation `<...><...>` | P8 | loud parse error | parse error | CAUGHT |
| p31 | annotation on a pipeline step, not terminator | P8 | loud parse error | parse error | CAUGHT |
| p32 | `<public,untrusted>` no spaces | P3/P7 | OK (same meaning) | ran | CAUGHT |
| p32b | newline inside annotation | P3/P7 | OK (same meaning) | ran | CAUGHT |
| p33 | consent(a) third part over llm edge | P3 | OK | ran | CAUGHT |
| p34 | annotation on wrong (clean) flow only | P1/P4 | dirty flow refuses | refuse | CAUGHT |
| p35 | duplicate flow name `Main` | P4 | duplicate error + still refuses | refuse | CAUGHT |
| p36 | call_llm directly in input binding, unannotated | P1 | refuse `[HTML_INJECTION]` | refuse | CAUGHT |
| p37 | respond as flow step with env input | P2/P9 | refuse (output-edge PII; see F4) | refuse | CAUGHT |
| p38 | exec as flow step, stdout → display | P1/P10 adj. | OK (no annotation needed) | executes w/ grants, stdout displayed | INCONCLUSIVE (F2) |
| p39 | import boundary (dirty imported pattern) | P2 | no-root: unknown step; `--root`: PII refuse | resolves import, refuses | CAUGHT (F7) |
| p40 | local clean pattern shadows dirty import | P2 | OK | runs clean local pattern | CAUGHT (no divergence) |
| p41 | `branch { }` empty in flow | vector | OK (no-op) | ran | INCONCLUSIVE (F8) |
| p42 | branch body = pipeline | vector | parse error | parse error | INCONCLUSIVE (F8) |
| p43 | mlogserver route_decl hunt | vector | parse error | parse error | INCONCLUSIVE (F8) |
| p44 | `mlog compile` on check-refused source | P1 | — | compile refuses, exit 1 | CAUGHT |
| p45 | .mbc bytecode execution | divergence | compile OK | runs without re-analysis | INCONCLUSIVE (F9) |
| p46 | env → respond() in pattern | P9 | refuse `[SECRET_LEAK]` + PII | refuse | CAUGHT |
| p47 | env → http_post body | P9 | refuse `[SECRET_LEAK]` + `[PII_EGRESS_NETWORK]` (class deviation, F3) | refuse | CAUGHT |
| p48 | env → exec() | P9 | refuse `[SECRET_TO_EXEC]` | refuse | CAUGHT |
| p49 | env → git_push() | P9 | refuse `[SECRET_EGRESS_VCS]` | refuse | CAUGHT |
| p50 | call_llm → respond() | P10 | refuse `[HTML_INJECTION]` + `[TAINT_INTERP]` | refuse | CAUGHT |
| p51 | call_llm → exec() | P11 | refuse `[UNTRUSTED_EXEC_DECISION]` | refuse | CAUGHT |
| p52 | "ls " + call_llm → exec() | P11 | refuse `[UNTRUSTED_EXEC_DECISION]` | refuse | CAUGHT |
| p53 | env → quarantine_write (return → output) | P12 | OK with `[CONSENT]` audit event | legal egress, no secret in output | CAUGHT (partial, see F10) |

## Findings (non-bypass)

**F1 — self-contradictory dead-end diagnostic for private edges (p08).** The private-edge refusal says `declare -> output<private, trusted> if the display is intended`; the program declaring exactly that gets `[LABEL_ANNOTATION_MISMATCH] ... claims a weaker label than the computed edge 'private, trusted' ... declare -> output<private, trusted> (the computed one) if the display is intended` — the same declaration, called "weaker" than itself. Refusal is invariant (P5/P6 hold), so the security posture is intact; the diagnostic loop is a soundness bug in the error text. Related: over a clean literal edge, `<public>` (integrity omitted) passes, while over an untrusted edge the same annotation is "weaker" — the omitted-integrity default is trusted, consistently applied.

**F2 — exec stdout reaches the display edge unannotated (p38).** `flow Main { input: String = "echo SECRET_MARKER_XYZ" -> exec -> output }` passes `mlog check` with no annotation; with `profile capability { allow_carrier: "exec" }` and `METALOGOS_ALLOW_EXEC=1` the command runs and its stdout is printed on the output edge. Neither P1's source list nor P10/P11 cover displaying exec output, so no promise text is broken — but command output is a textbook untrusted channel and the display-edge rule does not require `output<public, untrusted>` for it. The runtime gates themselves are fail-closed (no grant, no env var, no execution).

**F3 — http_post refusal class deviates from P9's text (p47).** P9 promises `SECRET_EGRESS_NETWORK` for env→http_post; observed: `[SECRET_LEAK] secret may be leaked — env() value passed as http_post body` plus `[PII_EGRESS_NETWORK]`. The refusal fires; the promised class string does not appear verbatim for this sink. (`git_push` did produce the promised `SECRET_EGRESS_VCS`; `exec` produced `SECRET_TO_EXEC`.)

**F4 — respond-as-flow-step skips the SECRET_LEAK rule (p37).** `input: String = env("HOME") -> respond -> output` refuses only through the output-edge check (`[PII_EGRESS_OUTPUT]`); the `[SECRET_LEAK]` rule that fires for `respond(env(...))` inside a pattern (p46) did not fire for the step form. Refusal still occurs, so P2 holds; the P9 class coverage for env→respond is context-dependent.

**F5 — `{{...}}` in a plain string literal corrupts the surrounding parse (p25).** `template("Hello {{x}}", call_llm("key", x))` fails with the flow body rendered as `flow Main {__TEMPLATE_BODY_0__}` — template preprocessing rewrote program text across statement boundaries. Loud, deterministic; provenance relative to the annotation feature unknowable black-box (P7 datum).

**F6 — a pattern named `output` cannot be used as a mid-pipeline step (p24).** `-> output -> Ask -> output` is a parse error (`expected IDENT, LT, or RBRACE` at the arrow after `output`); `output` is reserved to the terminator position. Whether this was parseable before the annotation feature is unobservable black-box (P7 datum).

**F7 — check/run import asymmetry (p39/p40).** Without `--root`, `mlog check` cannot resolve imports (reports the step as unknown) while `mlog run` resolves and fully analyzes them; with `--root` both analyze the taint and refuse. Both outcomes refuse, and local patterns shadow imports identically in check and run (p40), so no divergence was obtainable — but the blind spot of check-without-`--root` is a hardening note.

**F8 — branch/route/tool smuggling vectors could not be constructed (p41–p43).** `branch { }` parses as an empty no-op in a flow; ~80 body shapes (flow-style pipelines, pattern calls, `branch_condition` guesses, keywords) were all rejected, and no named branch definition form was found anywhere (top level rejects `branch`, tool bodies reject it). `tool` methods parse (`tool T { m(x: String) -> String { ... } }`) but cannot be flow steps. mlogserver leaks its rule names (`mlogserver_port|host|middleware|rate_limit|redact_mode|route_decl`) but no route spelling parsed (~15 shapes). The mission's "branch definitions / branch returns / tool-route-template containers" vectors are therefore INCONCLUSIVE, not bypasses.

**F9 — bytecode path trusts compiled artifacts (p45).** `mlog compile` enforces the Category A invariant (p44 refuses), and `mlog run file.mbc` executes without re-analysis; no divergence was obtainable black-box (the annotation is not stored as plaintext in the 317-byte `.mbc`; no checksum was tested successfully).

**F10 — P12 tested only from the private side (p53).** `quarantine_write(env(...))` is legal with an audit event and its return value reaches the output edge clean with no runtime secret emission — the "legal sink" half of P12 held. No constructor of a *poisoned* value was discoverable black-box (`poison`, `poison_value`, `quarantine`, `mark_poisoned`, `taint` all undefined), so "a poisoned value has no legal sink except quarantine_write" could not be exercised end-to-end; recorded as partial.

## Promise-by-promise verdicts

- **P1 — HELD.** All enumerated sources (`call_llm` p02, `mcp_call` p15, `read_file` p14, html/xml extraction p13, plus `http_get` in the input binding p36) refuse unannotated with `[HTML_INJECTION]` and the error names the idiom form (`declare -> output<public, untrusted>`). `escape_html` clears the edge only because it is a real sanitizer (p16: runtime output shows `&lt;script&gt;` escaping). Adjacent gap outside the enumerated sources: exec stdout (F2).
- **P2 — HELD.** env edges refuse with `[PII_EGRESS_OUTPUT]` from every construction tried: direct (p05), via input binding (p10), redacted (p11), escaped (p12), 3-hop relays (p19), if-arm merges (p20), respond-as-step (p37).
- **P3 — HELD.** `output<public, untrusted>` over a computed public-untrusted edge passes (p03, p32, p32b, p33).
- **P4 — HELD.** Weaker declarations refuse with `[LABEL_ANNOTATION_MISMATCH]` (p04, p28, p22, p23, p09); the conservative direction (claiming untrusted over trusted, p21) passes — a coherent lattice, no annotation silences what the program computes.
- **P5 — HELD.** `private` (p09), `consented` (p22), `poisoned` (p23) all refuse; p08 documents the dead-end wording (F1).
- **P6 — HELD.** Annotated private edges still refuse: `<public, untrusted>` (p06), `<public, trusted>` (p07), with `consent(a)` (p17). No annotation waives confidentiality.
- **P7 — INCONCLUSIVE (no positive evidence of breakage).** Ordinary previously-plausible programs (p01, p32, p32b) parse cleanly with no annotations; but "exactly as before" is unprovable with a single binary, and two grammar quirks (F5, F6) could plausibly be feature-introduced. No new parse error was *observed* on any program that did not involve the reserved `output` token or `{{...}}` strings.
- **P8 — HELD.** `<bogus>` (p26), missing confidentiality word (p27), unknown third words, uppercase words, unicode look-alikes (p29), double annotations (p30), step-position annotations (p31), trailing/leading junk — all loud errors; no silent acceptance found.
- **P9 — HELD with one class-name deviation.** respond (p46: `SECRET_LEAK`), exec (p48: `SECRET_TO_EXEC`), VCS via `git_push` (p49: `SECRET_EGRESS_VCS`) as promised; http_post (p47) refuses but reports `SECRET_LEAK` + `PII_EGRESS_NETWORK` instead of the promised `SECRET_EGRESS_NETWORK` (F3). respond-as-step nuances (F4).
- **P10 — HELD.** Unsanitized LLM output into the HTML sink `respond()` refuses (`HTML_INJECTION`, p50), also with a merged-state `TAINT_INTERP` line.
- **P11 — HELD.** Untrusted steering of exec refuses (`UNTRUSTED_EXEC_DECISION`), direct (p51) and through string concat (p52).
- **P12 — HELD (partial, F10).** `quarantine_write` is the legal sink with audit (p53); private values have no other legal sink among the probed set (print, respond, http_post, exec, git_push all refuse); the poisoned-value production itself was not constructible black-box.

## Honesty notes

- Verdicts use observed transcripts only (`pocs/transcripts.md`, true exit codes captured without pipes). `INCONCLUSIVE` marks probes where the promise text could not be tested to a conclusion black-box (P7's "as before", the branch/route vector, the bytecode tamper path, exec-vs-P1 applicability, poisoned-value construction) — none of them is claimed as a bypass.
- Several probes pass `mlog check` and fail `mlog run` for environmental reasons only (LLM not configured); those runs were re-executed with `METALOGOS_MOCK_LLM=1` where the runtime behavior mattered (p03, p16, p45).
- The probe programs were shaped entirely from the promise sheet's syntax hints plus black-box grammar feedback (parse-error messages); nothing outside `/tmp/redteam-967/` was read.
