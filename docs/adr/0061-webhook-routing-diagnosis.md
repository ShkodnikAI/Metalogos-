# ADR-0061: Webhook Routing Diagnosis — No Language Gap, Architectural Root Cause

**Date:** 2026-07-12
**Status:** Accepted
**Context:** Наряд METALOGOS_4_PRIMITIVES v2, Problem D

## Diagnosis

### Question 1: Does `Hook` (ADR-0045) handle HTTP events?

**No.** `Hook` is an AOP-style interceptor for pattern invocations (`before_pattern` / `after_pattern`). It injects variables `pattern_name`, `args`, `result`, `confidence` into the hook body. It has no relationship to HTTP requests, webhooks, or Telegram callbacks. The `Hook` mechanism is fully implemented and works correctly for its designed purpose.

### Question 2: Does Metalogos have an HTTP routing mechanism?

**Yes — and it is sufficient.** The `route` declaration inside `server { ... }` provides:

```mlog
route "/webhook/telegram" method=POST {
  let data = parse_json(request_body())
  let text = json_get(data, "message.text")
  // ... handle update
}
```

This is Axum-based (server.rs), supports method dispatch (`GET`, `POST`, `PUT`, `DELETE`), role-based access (`requires=[admin]`), and full statement bodies including `parse_json`, `json_get`, and all builtins. Nested JSON access (e.g., `json_get(data, "message.web_app_data.data")`) works correctly.

### Question 3: Why doesn't the mlog route receive Telegram traffic?

**Root cause is NOT a language deficiency.** FOSVED-office-v2 uses `reverse_proxy.py` which explicitly routes ALL `/webhook/*` traffic to the Python process (llm_proxy.py, port 4000). The Metalogos `route "/webhook/telegram"` in app.mlog is syntactically correct, technically functional, but physically unreachable because the proxy never sends traffic to the mlog server (port 10001) for that path.

Evidence from production logs (2026-07-11): all Telegram events appear as `[WEBHOOK]` from the Python logger. Zero logs from the mlog server for the `/webhook/telegram` path.

The Python side (llm_proxy.py `telegram_webhook()`, ~line 4409) implements independent duplicate logic: its own response generation, its own HTML rendering, its own date injection. This is duplicated business logic between two runtimes, caused by a historical routing decision, not a language gap.

## Decision

**No new language primitive is needed.** The existing `route` declaration is the correct and sufficient mechanism for handling Telegram webhook events in Metalogos.

The fix for the actual production bug is an **architectural decision** about routing ownership (which runtime handles `/webhook/telegram`), not a language change. This decision belongs to the FOSVED-office-v2 project owner, not the Metalogos language team.

## Verification

A golden test (`examples/telegram_webhook_route.mlog`) verifies that `parse_json` + `json_get` correctly extract data from a mock Telegram update payload, including nested `web_app_data`. This proves the language mechanism is sound — the routing issue is purely infrastructure-level.

## Future Consideration

If a new primitive is needed for capabilities that `route` cannot provide (e.g., WebSocket for live updates, Server-Sent Events), that would be a separate future ADR outside the scope of this diagnosis.