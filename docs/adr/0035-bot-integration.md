# ADR-0035: Bot Integration via Webhook Routes

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.6 — Bot Integration

## Context

Chat bots (Telegram, Discord, Slack) are a natural interface for Metalogos AI capabilities. Users should be able to interact with Metalogos entities, rules, and adaptive patterns through bot commands.

Rather than building bot clients into the interpreter, Metalogos should receive bot events via webhooks and respond by calling external Bot APIs. This keeps the interpreter lightweight and platform-agnostic.

## Decision

Bot integration uses **webhook routes** that receive HTTP POST payloads from bot platforms, with builtins for sending responses and parsing JSON.

### Webhook Routes

```mlog
mlogserver {
    route POST "/webhook/telegram" => {
        let body = json_body()                              // Parse incoming JSON
        let message = body.get("message.text")             // Extract fields
        let chat_id = body.get("message.chat.id")

        let reply = handle_message(message)                 // Call a learnable pattern
        send_message("telegram", chat_id, reply)
    }
}
```

### Builtins

| Builtin | Purpose |
|---------|---------|
| `json_body()` | Parse request body as JSON, returns opaque `Json` value |
| `send_message(platform, target, text)` | POST to Bot API (Telegram/Discord) with `Secret` token |
| `json.get(path)` | Extract nested field from `Json` value (e.g., `"data.user.name"`) |

### AI Handlers via Learnable Patterns

```mlog
learnable pattern handle_message(text) {
    // Learnable pattern classifies input and generates response
    // ~20 lines of .mlog code replaces hundreds of lines of traditional handler logic
}
```

Bot tokens loaded via `env("TELEGRAM_BOT_TOKEN")` → `Secret` type (ADR-0031). Never logged.

## Prior Art

- **Telegram Bot API:** Webhook-based message delivery, HTTP POST for responses.
- **Discord Interactions:** JSON payloads via webhook, `Authorization: Bot <token>` header.
- **Slack Bolt:** Event subscriptions via URL verification, structured JSON payloads.

## Consequences

- **Positive:** Bots are first-class citizens in Metalogos — a complete bot handler is ~20 lines of `.mlog`.
- **Positive:** Learnable patterns enable AI classification of bot messages with zero external ML dependencies.
- **Positive:** Platform-agnostic: the same `send_message()` builtin abstracts Telegram, Discord, and Slack APIs.
- **Neutral:** Webhook delivery requires a publicly accessible URL (ngrok for development, reverse proxy for production).
- **Negative:** Rate limiting and queue management for high-traffic bots require external infrastructure (message queues).
