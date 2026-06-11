#!/usr/bin/env node
// LLM Relay Proxy — bridges mlog http_post (no auth headers) to LLM APIs (with auth)
// FOSVED Office v2 — Narad 7

import { createServer } from 'http';

const PROVIDERS = {
  zai:      { base: 'https://api.z.ai/api/coding/paas/v4',                envKey: 'ZAI_API_KEY' },
  deepseek: { base: 'https://api.deepseek.com/v1',                        envKey: 'DEEPSEEK_API_KEY' },
  groq:     { base: 'https://api.groq.com/openai/v1',                    envKey: 'GROQ_API_KEY' },
  grok:     { base: 'https://api.x.ai/v1',                              envKey: 'GROK_API_KEY' },
  anthropic:{ base: 'https://api.anthropic.com/v1',                      envKey: 'ANTHROPIC_API_KEY' },
  openai:   { base: 'https://api.openai.com/v1',                         envKey: 'OPENAI_API_KEY' },
};

const server = createServer(async (req, res) => {
  if (req.method === 'GET' && req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', providers: Object.keys(PROVIDERS) }));
    return;
  }

  if (req.method !== 'POST') {
    res.writeHead(405);
    res.end('Method not allowed');
    return;
  }

  let body = '';
  for await (const chunk of req) body += chunk;

  try {
    const payload = JSON.parse(body);
    const { provider, model, system, message } = payload;

    if (!provider || !model || !message) {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Missing provider, model, or message' }));
      return;
    }

    const cfg = PROVIDERS[provider];
    if (!cfg) {
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: `Unknown provider: ${provider}` }));
      return;
    }

    const apiKey = process.env[cfg.envKey];
    if (!apiKey) {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: `Provider not configured: ${provider}. Set ${cfg.envKey} env var.` }));
      return;
    }

    let url, headers, reqBody;

    if (provider === 'anthropic') {
      url = `${cfg.base}/messages`;
      headers = {
        'Content-Type': 'application/json',
        'x-api-key': apiKey,
        'anthropic-version': '2023-06-01',
        'anthropic-dangerous-direct-browser-access': 'true',
      };
      reqBody = JSON.stringify({
        model,
        max_tokens: 1024,
        system: system || 'You are a helpful assistant.',
        messages: [{ role: 'user', content: message }],
      });
    } else {
      url = `${cfg.base}/chat/completions`;
      headers = {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
      };
      reqBody = JSON.stringify({
        model,
        max_tokens: 1024,
        messages: [
          { role: 'system', content: system || 'You are a helpful assistant.' },
          { role: 'user', content: message },
        ],
      });
    }

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 55000);

    try {
      const apiRes = await fetch(url, {
        method: 'POST',
        headers,
        body: reqBody,
        signal: controller.signal,
      });

      const apiBody = await apiRes.text();
      clearTimeout(timeout);

      if (apiRes.ok) {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(apiBody);
      } else {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: `LLM API ${apiRes.status}: ${apiBody.substring(0, 200)}` }));
      }
    } catch (fetchErr) {
      clearTimeout(timeout);
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: `Fetch failed: ${fetchErr.message}` }));
    }
  } catch (parseErr) {
    res.writeHead(400, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: `Invalid JSON: ${parseErr.message}` }));
  }
});

const PORT = process.env.RELAY_PORT || 10001;
server.listen(PORT, () => {
  console.log(`LLM Relay listening on 0.0.0.0:${PORT}`);
});
