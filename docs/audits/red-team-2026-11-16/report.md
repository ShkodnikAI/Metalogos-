# Metalogos `mlog` v0.40.0 — Independent Black-Box Red-Team Report

Date: 2026-11-16. Target: `mlog-v0.40.0` (SHA256 verified `bedb6d29…68576b`). Inputs: binary + PROMISES.md only. Probes: `probes/pNN_*.mlog`, full check/run transcripts in `probes/check_results.txt`.

## 1. Executive summary

I ran **46 probes** (p01–p45 plus 5 discovery probes p00*): 43 target the promise text directly, 3 are parse-grammar discovery. Verdicts: **1 BYPASS**, **38 CAUGHT** (refused at check or run exactly where the promises say), **5 INCONCLUSIVE/unresolved** (grammar or signature dead-ends, listed separately). The bypass is **p29**: LLM output (`call_llm`, label `public, untrusted`) piped through a flow pipeline to the `-> output` flow terminator reaches stdout unsanitized while `mlog check` reports `OK: no issues found` — the same taint is correctly refused at `respond_html`, so the flow terminator is an egress hole in an otherwise consistent label lattice. Most interesting secondary finding: the approval gate accepts a **ttl=0 mandate** (`approval_issue("alice","carrier:cron_run",0)` then `cron_run` executes with output identical to the blessed path) — I classify it INCONCLUSIVE-to-likely-bypass because a zero-second TTL should not count as a "live" mandate, but the runtime also refuses negative TTLs, so ttl=0 may be defined as "never expires" rather than "already expired". Overall the label tracker held through structs, lists, `each`/`while`/`if` control flow, helper-pattern indirection (both directions), string transforms and concatenation, and `join`; secrets and untrusted data were refused at every sink class probed (network, exec, file, memory, VCS, public output).

## 2. Probe table

Verdicts: BYPASS = promise text broken by observed behavior. CAUGHT = refused with the named error class (check or run). INCONCL. = no verdict (parse/signature dead-end or ambiguous semantics). "check P" = check passed.

| Probe | Promise | Check verdict | Run verdict | Final verdict |
|---|---|---|---|---|
| p00a–e (discovery: syntax, flow model, signatures) | — | various | various | (discovery) |
| p01_secret_print | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT | — | CAUGHT |
| p02_secret_struct | 1 | PII_EGRESS_OUTPUT (label survives struct field) | — | CAUGHT |
| p03_secret_list | 1 | PII_EGRESS_OUTPUT (label survives list elem) | — | CAUGHT |
| p04_secret_each | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT (label survives each-accumulator) | — | CAUGHT |
| p05_secret_while | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT (label survives while-accumulator) | — | CAUGHT |
| p06_secret_if | 1 | PII_EGRESS_OUTPUT (label survives if-branch merge) | — | CAUGHT |
| p07_secret_helper_return | 1 | PII_EGRESS_OUTPUT (helper pattern return keeps label) | — | CAUGHT |
| p08_secret_param | 1 | PII_EGRESS_OUTPUT (param flows into callee sink, cross-pattern trace shown) | — | CAUGHT |
| p09_secret_transform | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT (upper/replace/concat keep label) | — | CAUGHT |
| p10_secret_redact | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT (redact does NOT clear private) | — | CAUGHT |
| p11_secret_httppost | 1 | SECRET_LEAK + SECRET_EGRESS_NETWORK | — | CAUGHT |
| p12_secret_exec | 1 | SECRET_TO_EXEC | — | CAUGHT |
| p13_secret_writefile | 1 | SECRET_LEAK ×2 (file sink covered) | — | CAUGHT |
| p14_secret_sendmessage | 1 | PII_EGRESS_NETWORK | — | CAUGHT |
| p15_secret_memset | 1 | TAINT_PERSISTENCE (memory sink covered) | — | CAUGHT |
| p16_secret_gitpush (v2, secret in URL) | 1 | SECRET_EGRESS_VCS | — | CAUGHT |
| p17_secret_flowout (env through flow input → output) | 1 | PII_EGRESS_OUTPUT (flow terminator checked for secrets) | — | CAUGHT |
| p18_secret_escapehtml | 1 | SECRET_LEAK + PII_EGRESS_OUTPUT (escape_html does NOT clear private) | — | CAUGHT |
| p19_sql_concat | 2 | SQL_DYNAMIC | — | CAUGHT |
| p20_sql_grant | 2 | SQL_DYNAMIC (also under db_execute_with_grant) | — | CAUGHT |
| p21_llm_html | 3 | HTML_INJECTION ×2 + TAINT_INTERP | — | CAUGHT |
| p22_llm_escape_ok | 3 | check P (blessed path works) | not run (avoids real LLM call) | CAUGHT (as designed) |
| p23_llm_redact | 3 | refused (redact does NOT clear untrusted — stricter than promise text allows) | — | CAUGHT (over-strict, safe side) |
| p24_escape_rewrap (escaped + raw concat) | 3 | HTML_INJECTION ×3 (re-wrap + concat after sanitize still tracked) | — | CAUGHT |
| p25_llm_struct_each | 3 | HTML_INJECTION (struct→list→each preserved) | — | CAUGHT |
| p26_llm_exec | 3 | UNTRUSTED_EXEC_DECISION | — | CAUGHT |
| p27_queryparam_respond | 3 | HTML_INJECTION | — | CAUGHT |
| p28_readfile_html | 3 | HTML_INJECTION | — | CAUGHT |
| p29_llm_flowout (call_llm via flow input → output) | 3 | **check P** | not run (avoid real LLM host; check-pass is the bypass criterion) | **BYPASS** |
| p30_llm_join | 3 | HTML_INJECTION (join keeps taint) | — | CAUGHT |
| p31_cron_nomandate | 4 | check P | IRREVERSIBLE_NO_GRANT, refusal BEFORE action | CAUGHT |
| p32_cron_wrongcarrier | 4 | check P | IRREVERSIBLE_NO_GRANT (mandate for carrier:other does not cover) | CAUGHT |
| p33_cron_expired (ttl=0) | 4 | check P | **executed ("done", exit 0), identical to blessed path** | INCONCL. (likely bypass; ttl=0 semantics ambiguous) |
| p34_cron_ok (ttl=3600) | 4 | check P | executed ("done") — blessed path | CAUGHT (as designed) |
| p35_cron_wrongactor ("mallory") | 4 | check P | executed ("done") — no actor verification (not promised by text) | CAUGHT (observation) |
| p36_exec_nogrant | 5 | check P | IRREVERSIBLE_NO_GRANT (capability gate) | CAUGHT |
| p37_profile_capability (allow_carrier exec) | 5 | check P | EXEC_NOT_PERMITTED (profile does not enable exec; stricter than promise) | CAUGHT |
| p38_profile_legacy (egress permissive_with_audit) | 1/5 | SECRET_LEAK (legacy profile does not weaken secret rule) | — | CAUGHT |
| p39_serve_env / p40_serve_dotenv / p43_serve_route2 | 6 | parse error (mlogserver route grammar unresolved) | — | INCONCL. |
| p41_secret_websearch | 1 | PII_EGRESS_NETWORK | — | CAUGHT |
| p42_secret_httpget | 1 | PII_EGRESS_NETWORK | — | CAUGHT |
| p44_cron_negttl (ttl=-1) | 4 | check P | refused: ttl must be non-negative whole Number | CAUGHT |
| p45_cron_revoked | 4 | check P | run errored: approval_revoke() expects String, got Struct — revocation angle unresolved | INCONCL. |

Counts: 46 total; **BYPASS 1** (p29); CAUGHT 38 (incl. as-designed passes p22/p34); INCONCLUSIVE/unresolved 5 (p33, p39, p40, p43, p45) plus 3 grammar-discovery probes (p00d, p39-family). Parse-refusal note: the `mlogserver` block could not be parsed after 3 syntax attempts (route body grammar unknown), so Promise 6 was only attackable at parse level — the refusals are parse errors, not security refusals, hence not verdicts.

## 3. BYPASS evidence

### p29 — Promise 3 broken at check level: untrusted LLM output flows unsanitized to the public flow terminator `-> output`

Probe source (`probes/p29_llm_flowout.mlog`):

```
pattern echo(x: String) -> output {
  x
}

flow Main { input: String = call_llm("say hi") -> echo -> output }
```

Command output:

```
$ ./mlog-v0.40.0 check probes/p29_llm_flowout.mlog
OK: no issues found.
--exit:0
```

Mechanism (one sentence): `call_llm` output carries the `public, untrusted` label (proven by p21/p26 refusals) and the `-> output` flow terminator prints the piped value to stdout at runtime (verified with a benign string in p00e), yet the checker applies the untrusted-egress threshold only to builtin sinks (`respond_html`/`print`) and not to the flow terminator, so unsanitized LLM text reaches a public output with check passing — exactly the shape Promise 3 says must refuse.

Notes for the security report: (a) run was not executed for p29 because `call_llm` would contact a real LLM provider and the lab forbids real external hosts; per PROMISES.md, "a promise says refuses but check PASSES" is already a BYPASS. (b) The checker is not blind to flows — p17 shows a *secret* through the same flow terminator IS refused (`PII_EGRESS_OUTPUT ... in flow Main`); only the *untrusted* (public,untrusted) label class slips through, i.e. the flow-terminator rule covers private-egress but omits untrusted-egress.

### p33 — boundary observation, Promise 4 (classified INCONCLUSIVE, likely bypass)

Probe source (`probes/p33_cron_expired.mlog`):

```
pattern go(x: String) -> output {
  let a = approval_issue("alice", "carrier:cron_run", 0)
  let r = cron_run("job1")
  "done"
}

flow Main { input: String = "x" -> go -> output }
```

Command output (compare p34, the blessed path, which printed the same `done`/exit 0):

```
$ ./mlog-v0.40.0 run probes/p33_cron_expired.mlog
done
--exit:0
```

Mechanism: a mandate with `ttl_seconds = 0` is accepted by the approval gate and `cron_run` proceeds exactly as with a live 3600 s mandate; a zero-second TTL is arguably not a "live approval mandate" (Promise 4), but the runtime also refuses negative TTLs (`approval_issue: ttl_seconds must be a non-negative whole Number`), so ttl=0 may be defined as "never expires" — hence INCONCLUSIVE rather than confirmed BYPASS. Not run-verified for the actual cron side effect ("job1" does not exist, so the carrier action itself was a no-op in both p33 and p34).

## 4. Hypotheses not confirmed (what held up)

- **Label survival everywhere probed**: secret (`private, trusted`) and untrusted (`public, untrusted`) labels survived struct field storage (p02), list elements (p03), `each` and `while` accumulators (p04, p05), `if` branch merges (p06), helper-pattern returns (p07), cross-pattern parameter passing (p08, error even names the path `main → leak(…) → print (parameter 0 reaches the sink)`), string transforms (`upper`/`replace`, p09), concatenation after sanitization (p24), and `join` (p30).
- **Sanitizer honesty**: `redact` does NOT launder a `private` label (p10) nor an `untrusted` label (p23 — refused, i.e. stricter than the promise text which lists redact as an acceptable wrapper); `escape_html` does not launder secrets (p18) and laundering-by-rewrap fails (p24); only the blessed `escape_html` path into `respond_html` passes (p22).
- **Promise 1 sink coverage**: all sink families refused secrets — network (`http_post` URL+body, `http_get`, `web_search`, `send_message`), exec (`SECRET_TO_EXEC`), files (`write_file`), memory (`mem_set`, class TAINT_PERSISTENCE), VCS (`git_push`, class SECRET_EGRESS_VCS), public output (`print`, also via flow terminator p17).
- **Promise 2**: `db_execute` and `db_execute_with_grant` both refuse non-literal SQL built from `query_param` (`SQL_DYNAMIC`) — parameterization gap closed at compile time.
- **Promise 4 fail-closed**: no mandate and a wrong-carrier mandate both refuse `cron_run` BEFORE the action with `IRREVERSIBLE_NO_GRANT`; wrong-actor mandates are accepted (p35) but the promise text only requires a live mandate for `carrier:cron_run`, so that is an observation, not a break.
- **Promise 5 fail-closed**: bare `exec` refuses without grant; a `profile capability { allow_carrier: "exec" }` permit does NOT enable exec at run time (EXEC_NOT_PERMITTED unless `METALOGOS_ALLOW_EXEC=1`) — stricter than the promise; `profile legacy { egress: permissive_with_audit }` does not weaken the secret rule (p38).
- **Unresolved (not verdicts)**: `match`/`try` statement grammar and the `mlogserver` route-block grammar could not be discovered black-box within budget (parse errors at the brace), so match-branch taint merging, try/catch error-object laundering, and Promise 6 serve strictness were probed only at parse level; `approval_revoke` signature (String expected) left the revoked-mandate angle untested (p45).
