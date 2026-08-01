# ADR-0071: Integration Test Triage (Block 3)

## Status
Accepted

## Context
After Block 2 fixed the golden runner (collect-all-failures pattern),
the CI integration test suite was fully analysed: 311 tests, 219 passed, 92 failed.

## Decision
Classify all 92 failures into 8 categories with actionable remediation:

### Category Summary

| Category | Count | Root Cause | Action |
|---|---|---|---|
| Runtime Error | 41 | Missing builtins, immutable vars, constraints | Fix or defer |
| Parse Error | 18 | Eval harness, memory persist, schema | Investigate |
| Server Dependency | 11 | HTTP server / webhook processing | Isolate |
| Semantic / Arity | 8 | BUILTIN_REGISTRY missing entries | Fix |
| VM Unimplemented | 5 | list_len, pop, make_list, startswith | Implement |
| DB / Sandbox | 4 | SQLite, filesystem access | Mock |
| Missing Builtin | 1 | Nonexistent pattern error check | Fix |
| Golden Test Failure | 4 | Known broken examples | Tracked |

### Key Findings

1. **Largest group (41)** is a mix of missing builtins (Phase 23 features
   registered via `funcs.insert()` but not in `BUILTIN_REGISTRY`) and
   test code issues (let mut needed in phase17).
2. **8 arity failures** all stem from BUILTIN_REGISTRY missing 8
   Telegram/Voice/LLM builtins (send_message, edit_message_text,
   session_logout, tts_send, whisper_transcribe, etc.).
3. **5 VM failures** are unimplemented bytecode instructions — the VM
   backend lacks list_len, pop, make_list, startswith.
4. **11 server-dependent tests** require mlogserver HTTP endpoint —
   these should be gated behind a cfg or cargo feature.
5. **18 parse errors** are split between eval harness (9), memory persist
   e2e (7), and schema (3). Need individual investigation.

## Consequences
- Block 3 triage complete — full failure map documented
- Quick wins identified: BUILTIN_REGISTRY entries, cfg gates, let mut fixes
- Remaining work split across Blocks 4-6 of Наряд №31

## References
- ADR-0069: slice() builtin for lists
- ADR-0070: Parser returns Result instead of abort()
- CHANGELOG.md: [0.12.0] Известные ограничения
