#!/bin/sh
# Fosved Office v2 — Metalogos runtime entrypoint
# Наряд №12 Phase 4: Removed llm_proxy.py, Yana uses built-in call_llm()
# LLM provider configured via env vars:
#   METALOGOS_LLM_PROVIDER  = anthropic | openai | ollama (default: anthropic)
#   METALOGOS_LLM_MODEL     = model name (default: provider default)
#   METALOGOS_API_KEY       = API key for the provider
#   METALOGOS_OPENAI_BASE_URL = custom base URL (optional, for proxies/self-hosted)

echo "[entrypoint] Starting Metalogos server (no LLM proxy needed)"
echo "[entrypoint] LLM provider: ${METALOGOS_LLM_PROVIDER:-anthropic}"
echo "[entrypoint] LLM model: ${METALOGOS_LLM_MODEL:-default}"
echo "[entrypoint] Custom base URL: ${METALOGOS_OPENAI_BASE_URL:-none}"

# Start Metalogos server
exec mlog serve app.mlog
