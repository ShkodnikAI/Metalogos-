#!/usr/bin/env python3
"""
LLM Proxy with direct Telegram integration for Fosved Office v2.
Runs locally in Docker container alongside Metalogos mlog serve.

KEY DESIGN: Proxy calls LLM and sends response DIRECTLY to Telegram.
Metalogos only sends a short request to proxy, gets "OK" back immediately.
This avoids Metalogos http_post timeout issues with long LLM calls.

Endpoints:
  POST /route               - Route text to dept, call LLM, send result to Telegram
  POST /chat                - Yana general, send result to Telegram
  POST /dept/<name>         - Direct dept call, send result to Telegram
  POST /status              - Send status message to Telegram (for webhook handler)
  GET/POST /health          - Provider key status
  POST /ping                - Health ping

Request format (POST /route or /chat):
  JSON body: {"chat_id": "12345", "text": "user message here"}
  Or plain text with chat_id in X-Chat-Id header (for backward compat)

Response format:
  "OK" if request accepted
  "ERROR: <reason>" if failed

Priority: GLM 4.6 -> GLM 5.1 -> DeepSeek -> Groq -> Claude
"""

import json
import os
import threading
from urllib.request import urlopen, Request
from urllib.error import HTTPError
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

TIMEOUT = 30
PROXY_ACCEPT_TIMEOUT = 2  # How long to wait before returning "processing" to Metalogos

PROVIDERS = [
    {
        "name": "GLM-4.6",
        "base_url": "https://open.bigmodel.cn/api/paas/v4",
        "model": os.environ.get("GLM_46_MODEL", "glm-4-plus"),
        "key_env": "GLM_46_API_KEY",
        "type": "openai",
    },
    {
        "name": "GLM-5.1",
        "base_url": "https://open.bigmodel.cn/api/paas/v4",
        "model": os.environ.get("GLM_51_MODEL", "glm-z1-plus"),
        "key_env": "GLM_51_API_KEY",
        "type": "openai",
    },
    {
        "name": "DeepSeek",
        "base_url": "https://api.deepseek.com/v1",
        "model": os.environ.get("DEEPSEEK_MODEL", "deepseek-chat"),
        "key_env": "DEEPSEEK_API_KEY",
        "type": "openai",
    },
    {
        "name": "Groq",
        "base_url": "https://api.groq.com/openai/v1",
        "model": os.environ.get("GROQ_MODEL", "llama-3.3-70b-versatile"),
        "key_env": "GROQ_API_KEY",
        "type": "openai",
    },
    {
        "name": "Claude",
        "base_url": "https://api.anthropic.com/v1/messages",
        "model": os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-20250514"),
        "key_env": "ANTHROPIC_API_KEY",
        "type": "anthropic",
    },
]

LANGUAGE_RULE = (
    "\n\n[LANGUAGE RULE - MANDATORY]\n"
    "You MUST respond ENTIRELY in Russian language using Cyrillic alphabet.\n"
    "All text output must be in proper Russian: analysis, reports, summaries, "
    "headers, labels, recommendations, status messages.\n"
    "Never use transliteration of Russian words into Latin letters - "
    "always write real Cyrillic Russian.\n"
    "Technical terms, command names, code snippets and proper names may remain in Latin.\n"
    "This rule has highest priority - always apply it regardless of "
    "what language the user query is written in."
)

log_prefix = "[LLM-PROXY]"


def log(msg):
    print(f"{log_prefix} {msg}", flush=True)


# ---- Command -> Dept mapping ----
COMMAND_DEPT = {}

for cmd in [
    "/analyze", "/quickanalyze", "/wargame", "/osp-deep", "/osp-full",
    "/watch", "/unwatch", "/score", "/redteam", "/profile",
    "/regime", "/recalibrate", "/bet", "/verify", "/archive",
]:
    COMMAND_DEPT[cmd] = "osp"

for cmd in [
    "/scan", "/disrupt", "/inflection", "/hypecycle", "/cross-domain",
    "/lab-watch", "/lab-archive", "/lab-score", "/lab-recalibrate",
]:
    COMMAND_DEPT[cmd] = "lz"

for cmd in [
    "/expert", "/quickexpert", "/deepexpert",
    "/expert-update", "/expert-followup", "/expert-debrief", "/expert-archive",
]:
    COMMAND_DEPT[cmd] = "expert"

COMMAND_DEPT["/dev"] = "dev"
COMMAND_DEPT["/design"] = "design"
COMMAND_DEPT["/qa"] = "qa"

for cmd in ["/calc", "/eng-score", "/eng-archive"]:
    COMMAND_DEPT[cmd] = "engineering"

for cmd in [
    "/market-research", "/segment", "/competitors", "/campaign",
    "/marketing-score", "/marketing-archive",
]:
    COMMAND_DEPT[cmd] = "marketing"

COMMAND_DEPT["/finance"] = "finance"
COMMAND_DEPT["/legal"] = "legal"
COMMAND_DEPT["/visual"] = "visual"

for cmd in [
    "/recruit", "/train", "/polygon", "/debrief",
    "/forge-status", "/assess", "/recalibrate-dept", "/forge",
]:
    COMMAND_DEPT[cmd] = "kavalnya"


def detect_dept(text):
    """Detect department from command prefix."""
    if not text or not text.startswith("/"):
        return None, None
    parts = text.split(None, 1)
    if not parts:
        return None, None
    cmd = parts[0]
    if cmd in COMMAND_DEPT:
        return COMMAND_DEPT[cmd], cmd
    for known_cmd, dept in COMMAND_DEPT.items():
        if cmd.startswith(known_cmd) or cmd.startswith(known_cmd + " "):
            return dept, known_cmd
    return None, None


# ---- Library prompt loading ----

DEPT_LIBRARY_FILES = {
    "osp": "osp.md", "lz": "lz.md", "expert": "expert.md", "dev": "dev.md",
    "design": "design.md", "qa": "qa.md", "engineering": "engineering.md",
    "marketing": "marketing.md", "finance": "finance.md", "legal": "legal.md",
    "visual": "visual.md", "kavalnya": "kavalnya.md", "yana": "yana.md",
}

CONDENSED_PROMPTS = {
    "yana": (
        "You are Yana, the AI office manager of Fosved Office v2. "
        "You coordinate 12 departments: OSP, LZ, Expert, Dev, Design, QA, "
        "Engineering, Marketing, Finance, Legal, Visual, Kavalnya. "
        "Understand the user request, classify which department(s) can help, "
        "and provide a concise useful response or route to the right department. "
        "Be concise - max 4 sentences."
    ),
    "osp": (
        "You are the Strategic Planning Department (OSP). "
        "5-level topology analysis, actor potential P=A×R×S×L×E×E_modifier, "
        "two-paths synthesis, ACH with 5+ hypotheses. "
        "Calibrated probabilities (Sherman Kent), premortem, source triangulation. "
        "Mark statements: [FACT], [INTERPRETATION], [SPECULATION]."
    ),
    "lz": (
        "You are the Knowledge Lab (LZ). 24/7 technology scanner. "
        "Hype Cycle mapping, Disruption Probability Scoring, Inflection Point Detection. "
        "10 domains: AI, Biotech, Energy, Materials, Military, Transport, Comms, FinTech, Robotics, Climate. "
        "Min 3 independent confirmations for alerts."
    ),
    "expert": (
        "You are the Expert Department. Meeting briefing preparation. "
        "Adaptive depth L1-L3. Combat questions, due diligence, bullshit detection. "
        "Format: Executive Summary, Key Findings, Risks, Questions, Bottom Line."
    ),
    "dev": "You are the Development Department. Code, architecture, deployment. Practical technical advice.",
    "design": "You are the Design Department. UI/UX, wireframes, design systems. Specific recommendations.",
    "qa": "You are the QA Department. Testing, security, quality. Structured test plans.",
    "engineering": "You are the Engineering Department. Calculations, structural analysis. Include formulas and units.",
    "marketing": "You are the Marketing Department. Market research, segmentation, campaigns. Data-driven insights.",
    "finance": "You are the Finance Department. Budget, cashflow, modeling. Specific numbers and scenarios.",
    "legal": "You are the Legal Department. Contracts, NDA, compliance. Risk mitigation recommendations.",
    "visual": "You are the Visual Department. Infographics, data visualization. Design recommendations.",
    "kavalnya": "You are the Kavalnya (Forge) Department. Recruitment, training, assessment. Skill gap analysis.",
}

library_prompts = {}


def load_library_prompts():
    lib_dir = Path("/office/library")
    if not lib_dir.exists():
        log(f"Library dir not found: {lib_dir}")
        return
    for dept, filename in DEPT_LIBRARY_FILES.items():
        filepath = lib_dir / filename
        if filepath.exists():
            try:
                content = filepath.read_text(encoding="utf-8")
                if content.startswith("---"):
                    end = content.find("---", 3)
                    if end > 0:
                        content = content[end + 3:].strip()
                library_prompts[dept] = content
                log(f"Loaded {dept} ({len(content)} chars)")
            except Exception as e:
                log(f"Failed {filename}: {e}")
                library_prompts[dept] = CONDENSED_PROMPTS.get(dept, "")
        else:
            library_prompts[dept] = CONDENSED_PROMPTS.get(dept, "")


def get_dept_prompt(dept):
    prompt = library_prompts.get(dept, "")
    if not prompt:
        prompt = CONDENSED_PROMPTS.get(dept, "You are a helpful Fosved Office assistant.")
    return prompt + LANGUAGE_RULE


# ---- Telegram API ----

def send_telegram_message(chat_id, text):
    """Send message directly to Telegram API."""
    token = os.environ.get("TELEGRAM_BOT_TOKEN", "")
    if not token or not chat_id:
        log("Cannot send to Telegram: missing token or chat_id")
        return False
    if not text:
        log("Cannot send to Telegram: empty text")
        return False

    body_data = {"chat_id": chat_id, "text": text}
    body = json.dumps(body_data, ensure_ascii=False).encode("utf-8")

    url = f"https://api.telegram.org/bot{token}/sendMessage"
    req = Request(url, data=body, method="POST")
    req.add_header("Content-Type", "application/json; charset=utf-8")

    try:
        with urlopen(req, timeout=10) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            if result.get("ok"):
                log(f"Telegram sent OK to {chat_id} ({len(text)} chars)")
                return True
            else:
                log(f"Telegram API error: {result}")
                return False
    except Exception as e:
        log(f"Telegram send failed: {e}")
        return False


def send_telegram_direct(chat_id, text):
    """Send LLM response directly to Telegram, splitting long messages."""
    if not text:
        return False

    # Telegram limit is 4096 chars
    MAX_LEN = 4000
    if len(text) <= MAX_LEN:
        return send_telegram_message(chat_id, text)

    # Split by paragraph (double newline) to avoid breaking mid-sentence
    chunks = []
    current = ""
    for paragraph in text.split("\n\n"):
        if len(current) + len(paragraph) + 2 > MAX_LEN and current:
            chunks.append(current)
            current = paragraph
        else:
            current = current + "\n\n" + paragraph if current else paragraph
    if current:
        chunks.append(current)

    # If a single paragraph is still too long, split by lines
    final_chunks = []
    for chunk in chunks:
        if len(chunk) <= MAX_LEN:
            final_chunks.append(chunk)
        else:
            lines = chunk.split("\n")
            sub = ""
            for line in lines:
                if len(sub) + len(line) + 1 > MAX_LEN and sub:
                    final_chunks.append(sub)
                    sub = line
                else:
                    sub = sub + "\n" + line if sub else line
            if sub:
                final_chunks.append(sub)

    ok = True
    for i, chunk in enumerate(final_chunks[:10]):  # max 10 chunks
        if not send_telegram_message(chat_id, chunk):
            ok = False
    return ok


def send_telegram_report(chat_id, text, server_url):
    """Send report: short responses directly, long as link + stored file."""
    MAX_DIRECT = 4000

    if len(text) <= MAX_DIRECT:
        return send_telegram_direct(chat_id, text)

    # For very long responses, store file and send link
    import time
    ts = str(int(time.time() * 1000))
    doc_id = f"{chat_id}-{ts}"

    try:
        reports_dir = Path("/office/data/reports")
        reports_dir.mkdir(parents=True, exist_ok=True)
        report_file = reports_dir / f"{doc_id}.txt"
        report_file.write_text(text, encoding="utf-8")
    except Exception as e:
        log(f"Failed to save report: {e}")

    base_url = server_url if len(server_url) > 4 else "https://fosved-office-v2.onrender.com"
    link = f"{base_url}/report?id={doc_id}"
    send_telegram_direct(chat_id, f"[Report]({link})")


# ---- LLM Provider calls ----

last_llm_result = {"result": "", "error": "", "provider": "", "length": 0}


def try_openai_provider(provider, messages, max_tokens=2048):
    api_key = os.environ.get(provider["key_env"], "")
    if not api_key:
        return None, "no key"
    url = provider["base_url"] + "/chat/completions"
    body_data = {"model": provider["model"], "messages": messages, "max_tokens": max_tokens}
    req_body = json.dumps(body_data, ensure_ascii=False).encode("utf-8")
    req = Request(url, data=req_body, method="POST")
    req.add_header("Content-Type", "application/json; charset=utf-8")
    req.add_header("Authorization", f"Bearer {api_key}")
    try:
        with urlopen(req, timeout=TIMEOUT) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            content = result.get("choices", [{}])[0].get("message", {}).get("content", "")
            if content:
                return content, None
            return None, "empty content"
    except HTTPError as e:
        err_body = ""
        try:
            err_body = e.read().decode("utf-8", errors="replace")
        except Exception:
            pass
        return None, f"HTTP {e.code} {err_body[:200]}"
    except Exception as e:
        return None, f"{type(e).__name__}: {str(e)[:200]}"


def try_anthropic_provider(provider, messages, max_tokens=2048):
    api_key = os.environ.get(provider["key_env"], "")
    if not api_key:
        return None, "no key"
    url = provider["base_url"]
    system_msg = ""
    anth_messages = []
    for msg in messages:
        if msg.get("role") == "system":
            system_msg = msg.get("content", "")
        else:
            anth_messages.append({"role": msg["role"], "content": msg["content"]})
    anth_body = {"model": provider["model"], "system": system_msg, "messages": anth_messages, "max_tokens": max_tokens}
    req_body = json.dumps(anth_body, ensure_ascii=False).encode("utf-8")
    req = Request(url, data=req_body, method="POST")
    req.add_header("Content-Type", "application/json; charset=utf-8")
    req.add_header("x-api-key", api_key)
    req.add_header("anthropic-version", "2023-06-01")
    try:
        with urlopen(req, timeout=TIMEOUT) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            text = "".join(b.get("text", "") for b in result.get("content", []))
            if text:
                return text, None
            return None, "empty content"
    except HTTPError as e:
        err_body = ""
        try:
            err_body = e.read().decode("utf-8", errors="replace")
        except Exception:
            pass
        return None, f"HTTP {e.code} {err_body[:200]}"
    except Exception as e:
        return None, f"{type(e).__name__}: {str(e)[:200]}"


def call_llm_with_fallback(messages, max_tokens=2048):
    global last_llm_result
    errors = []
    for provider in PROVIDERS:
        name = provider["name"]
        if provider["type"] == "openai":
            content, err = try_openai_provider(provider, messages, max_tokens)
        elif provider["type"] == "anthropic":
            content, err = try_anthropic_provider(provider, messages, max_tokens)
        else:
            continue
        if content is not None:
            log(f"OK {name} ({len(content)} chars)")
            last_llm_result = {"result": content[:200], "error": "", "provider": name, "length": len(content)}
            return content, None
        else:
            errors.append(f"{name}: {err}")
            log(f"FAIL {name}: {err}")
    all_errors = " | ".join(errors)
    log(f"ALL FAILED: {all_errors}")
    last_llm_result = {"result": "", "error": all_errors[:200], "provider": "none", "length": 0}
    return None, all_errors


# ---- Process LLM request in background thread ----

def process_llm_request(user_text, chat_id, server_url):
    """Call LLM and send result directly to Telegram. Runs in background thread."""
    try:
        dept, cmd = detect_dept(user_text)
        if dept:
            system_prompt = get_dept_prompt(dept)
        else:
            system_prompt = get_dept_prompt("yana")

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_text},
        ]
        dept_label = dept if dept else "yana"
        log(f"BG LLM: dept={dept_label} chat_id={chat_id} text_len={len(user_text)}")

        result, err = call_llm_with_fallback(messages)

        if result is not None:
            send_telegram_report(chat_id, result, server_url)
        else:
            send_telegram_direct(chat_id, f"LLM providers unavailable: {err[:200]}")
    except Exception as e:
        log(f"BG LLM error: {e}")
        try:
            send_telegram_direct(chat_id, "Internal error processing request")
        except Exception:
            pass


# ---- HTTP Handler ----

class ProxyHandler(BaseHTTPRequestHandler):

    def _parse_path(self):
        path = self.path.split("?")[0]
        query = ""
        if "?" in self.path:
            query = self.path.split("?", 1)[1]
        return path, query

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        raw_body = self.rfile.read(content_length) if content_length > 0 else b""

        path, query = self._parse_path()

        if path == "/ping":
            self._send_text(200, "PONG")
            return

        if path in ("/route", "/chat", "/status"):
            # Accept request, parse, process in background, return OK immediately
            chat_id = ""

            # Extract chat_id from query param: /route?chat_id=12345
            if query:
                for param in query.split("&"):
                    if param.startswith("chat_id="):
                        chat_id = param[8:]
                        break

            # Fallback: try JSON body
            if not chat_id:
                try:
                    data = json.loads(raw_body.decode("utf-8"))
                    chat_id = str(data.get("chat_id", ""))
                    user_text = data.get("text", "")
                    if user_text:
                        raw_body = user_text.encode("utf-8")
                except (json.JSONDecodeError, UnicodeDecodeError):
                    pass

            user_text = raw_body.decode("utf-8", errors="replace") if raw_body else ""

            if not chat_id:
                self._send_text(400, "ERROR: missing chat_id")
                return

            server_url = os.environ.get("RENDER_SERVICE_URL", "")

            thread = threading.Thread(
                target=process_llm_request,
                args=(user_text, chat_id, server_url),
                daemon=True,
            )
            thread.start()

            self._send_text(200, "OK")
            return

        if path.startswith("/dept/"):
            dept = path[6:]
            if dept in DEPT_LIBRARY_FILES or dept in CONDENSED_PROMPTS:
                chat_id = ""
                user_text = ""
                try:
                    data = json.loads(raw_body.decode("utf-8"))
                    chat_id = str(data.get("chat_id", ""))
                    user_text = data.get("text", "")
                except (json.JSONDecodeError, UnicodeDecodeError):
                    user_text = raw_body.decode("utf-8", errors="replace")
                    chat_id = self.headers.get("X-Chat-Id", "")
                if not chat_id:
                    self._send_text(400, "ERROR: missing chat_id")
                    return
                server_url = os.environ.get("RENDER_SERVICE_URL", "")
                thread = threading.Thread(
                    target=process_llm_request,
                    args=(user_text, chat_id, server_url),
                    daemon=True,
                )
                thread.start()
                self._send_text(200, "OK")
                return
            else:
                self._send_text(404, f"Unknown dept: {dept}")
                return

        if path == "/v1/chat/completions":
            try:
                body_data = json.loads(raw_body.decode("utf-8"))
            except json.JSONDecodeError as e:
                self._send_text(400, f"Invalid JSON: {e}")
                return
            messages = body_data.get("messages", [])
            max_tokens = body_data.get("max_tokens", 2048)
            result, err = call_llm_with_fallback(messages, max_tokens)
            if result is not None:
                self._send_text(200, result)
            else:
                self._send_text(502, f"LLM_ERROR: {err}")
            return

        if path == "/health":
            self._send_health()
            return

        self._send_text(404, "Not found")

    def do_GET(self):
        path, _ = self._parse_path()
        if path == "/health":
            self._send_health()
        elif path == "/ping":
            self._send_text(200, "PONG")
        else:
            self._send_text(404, "Not found")

    def _send_health(self):
        status_parts = []
        for p in PROVIDERS:
            key = os.environ.get(p["key_env"], "")
            key_len = len(key) if key else 0
            status_parts.append(f"{p['name']}: key={'SET(' + str(key_len) + ')' if key else 'NO_KEY'}")
        loaded = len(library_prompts)
        lr = last_llm_result
        self._send_text(200, f"LLM_PROXY_OK | library={loaded} | last={lr['provider']}({lr['length']}) | " + " | ".join(status_parts))

    def _send_text(self, code, text):
        data = text.encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_PUT(self):
        self.do_POST()

    def log_message(self, format, *args):
        log(f"HTTP {args[0] if args else format}")


def main():
    port = int(os.environ.get("LLM_PROXY_PORT", "4000"))
    host = os.environ.get("LLM_PROXY_HOST", "127.0.0.1")
    log(f"Starting on {host}:{port}")

    load_library_prompts()
    log(f"Loaded {len(library_prompts)} dept prompts")

    for p in PROVIDERS:
        key = os.environ.get(p["key_env"], "")
        log(f"  {p['name']}: key={'SET(' + str(len(key)) + ')' if key else 'NOT_SET'} model={p['model']}")

    server = HTTPServer((host, port), ProxyHandler)
    log(f"Ready on port {port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log("Shutting down")
        server.shutdown()


if __name__ == "__main__":
    main()
