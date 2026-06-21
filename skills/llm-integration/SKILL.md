---
name: llm-integration
description: Low-level patterns for integrating LLMs (Claude, Grok, OpenAI, local models) into applications. Provider abstraction, streaming, retries, fallback providers, cost tracking, prompt management. The plumbing that makes LLM-powered apps production-ready.
---

# LLM Integration — Production-Ready LLM Plumbing

Calling an LLM API in a tutorial is one line. Doing it reliably in production requires retries, fallbacks, streaming handling, cost tracking, secret management, and provider abstraction. This skill is the plumbing.

## Prerequisites

- LLM provider API keys available
- Use case: simple LLM call, not full agent (use `agent-architecture` for agents)

## Core principle

> Don't couple your code to one LLM provider. Today it's Claude, tomorrow it might be Mistral or a local model. The abstraction layer is cheap to build now and expensive to retrofit later. Plus you get fallback resilience for free.

## Provider abstraction layer

Build minimal abstraction:

```typescript
// lib/llm/types.ts
export interface LLMMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface LLMCallOptions {
  model?: string;
  maxTokens?: number;
  temperature?: number;
  systemPrompt?: string;
  tools?: ToolDefinition[];
}

export interface LLMResponse {
  text: string;
  toolCalls?: ToolCall[];
  usage: { inputTokens: number; outputTokens: number };
  model: string;
  finishReason: 'stop' | 'length' | 'tool_use' | 'error';
}

// lib/llm/provider.ts
export interface LLMProvider {
  name: string;
  call(messages: LLMMessage[], options: LLMCallOptions): Promise<LLMResponse>;
  callStreaming(messages: LLMMessage[], options: LLMCallOptions): AsyncIterable<LLMChunk>;
}
```

Implement per provider:

```typescript
// lib/llm/anthropic.ts
import Anthropic from '@anthropic-ai/sdk';

export class AnthropicProvider implements LLMProvider {
  name = 'anthropic';
  private client: Anthropic;

  constructor(apiKey: string) {
    this.client = new Anthropic({ apiKey });
  }

  async call(messages, options) {
    const response = await this.client.messages.create({
      model: options.model || 'claude-opus-4-7',
      max_tokens: options.maxTokens || 4096,
      system: options.systemPrompt,
      messages: messages.map(m => ({ role: m.role, content: m.content })),
      tools: options.tools,
      temperature: options.temperature
    });

    return {
      text: response.content.find(c => c.type === 'text')?.text || '',
      toolCalls: response.content.filter(c => c.type === 'tool_use'),
      usage: {
        inputTokens: response.usage.input_tokens,
        outputTokens: response.usage.output_tokens
      },
      model: response.model,
      finishReason: response.stop_reason as any
    };
  }
}
```

Use through abstraction:

```typescript
const llm = getLLMProvider();  // returns currently configured
const response = await llm.call(messages, { maxTokens: 2000 });
```

## Retries

Network calls fail. Always retry transient errors:

```typescript
async function callWithRetry(provider, messages, options, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await provider.call(messages, options);
    } catch (error) {
      if (!isRetryable(error) || attempt === maxRetries) throw error;
      const delay = Math.min(1000 * Math.pow(2, attempt), 30000);  // exponential backoff
      await sleep(delay);
    }
  }
}

function isRetryable(error) {
  if (error.status === 429) return true;  // rate limit
  if (error.status >= 500) return true;   // server error
  if (error.code === 'ECONNRESET') return true;
  if (error.code === 'ETIMEDOUT') return true;
  return false;
}
```

**Don't retry:**
- 400 Bad Request (your fault, won't change)
- 401/403 (auth issues)
- 422 (validation, your input wrong)
- Content moderation errors (provider refused)

## Fallback providers

If primary fails after retries, fall back to secondary:

```typescript
async function callWithFallback(messages, options) {
  const providers = [
    new AnthropicProvider(process.env.ANTHROPIC_API_KEY),
    new GrokProvider(process.env.GROK_API_KEY),
    new OpenAIProvider(process.env.OPENAI_API_KEY)
  ];

  let lastError;
  for (const provider of providers) {
    try {
      return await callWithRetry(provider, messages, options);
    } catch (error) {
      console.warn(`[llm] ${provider.name} failed, trying next:`, error.message);
      lastError = error;
    }
  }
  throw lastError;  // all providers failed
}
```

**Fallback considerations:**
- Different providers have different capabilities — fallback may not support tools/vision
- Quality varies — log when fallback used so you know
- Cost varies — fallback may be more expensive
- Output format differs — may need to normalize

For Fosved-bot existing pattern: 4-provider STT chain (Groq → MiniMax → Deepgram → OpenAI). Same principle works for LLM.

## Streaming

For UX (chat interfaces, long responses), stream tokens as generated:

```typescript
async function* streamCompletion(provider, messages, options) {
  const stream = await provider.callStreaming(messages, options);
  for await (const chunk of stream) {
    yield chunk.text;
  }
}

// Usage in API route:
const stream = streamCompletion(provider, messages, options);
for await (const chunk of stream) {
  response.write(chunk);
}
```

**Server-Sent Events (SSE)** for browser clients:

```typescript
// Next.js route handler
export async function POST(request) {
  const { messages } = await request.json();
  const encoder = new TextEncoder();

  const stream = new ReadableStream({
    async start(controller) {
      const llmStream = streamCompletion(provider, messages, {});
      for await (const chunk of llmStream) {
        controller.enqueue(encoder.encode(`data: ${JSON.stringify({ text: chunk })}\n\n`));
      }
      controller.enqueue(encoder.encode('data: [DONE]\n\n'));
      controller.close();
    }
  });

  return new Response(stream, {
    headers: { 'Content-Type': 'text/event-stream' }
  });
}
```

**Client consumption:**

```typescript
const response = await fetch('/api/chat', { method: 'POST', body: ... });
const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  const text = decoder.decode(value);
  // Parse SSE format
  const lines = text.split('\n').filter(l => l.startsWith('data: '));
  for (const line of lines) {
    const data = line.slice(6);
    if (data === '[DONE]') return;
    const { text: chunk } = JSON.parse(data);
    appendToUI(chunk);
  }
}
```

## Cost tracking

Per-call:
```typescript
async function trackedCall(provider, messages, options, context) {
  const response = await provider.call(messages, options);
  await prisma.llmCall.create({
    data: {
      provider: provider.name,
      model: response.model,
      inputTokens: response.usage.inputTokens,
      outputTokens: response.usage.outputTokens,
      costUsd: computeCost(provider, response.usage, response.model),
      context: context, // 'osp_analysis' | 'expert_briefing' | etc.
      success: true
    }
  });
  return response;
}
```

Cost computation per provider:
```typescript
const PRICING = {
  'claude-opus-4-7':    { inputPer1M: 15, outputPer1M: 75 },
  'claude-sonnet-4-6':  { inputPer1M: 3,  outputPer1M: 15 },
  'claude-haiku-4-5':   { inputPer1M: 0.8, outputPer1M: 4 },
  'gpt-4o':             { inputPer1M: 2.5, outputPer1M: 10 },
  'grok-2':             { inputPer1M: 5,  outputPer1M: 15 }
};
```

Aggregate monthly:
```typescript
const monthly = await prisma.llmCall.aggregate({
  where: { createdAt: { gte: startOfMonth } },
  _sum: { costUsd: true, inputTokens: true, outputTokens: true }
});
```

Budget alerts via scheduler.

## Prompt management

**Don't inline prompts in code.** Use prompt templates:

```typescript
// lib/prompts/osp_analysis.ts
export const OSP_ANALYSIS_PROMPT = `
<role>
You are OSP — Strategic Planning Department specialist...
</role>

<topic>
{{topic}}
</topic>

<protocol>
Apply 5-level topology...
</protocol>
`;

export function buildOspPrompt(topic: string) {
  return OSP_ANALYSIS_PROMPT.replace('{{topic}}', topic);
}
```

**Version your prompts.** When prompt changes:
- Save old version
- Test new vs old on golden dataset (see `qa/ai-evals-framework`)
- Document why changed
- Reference version in archive

## Model selection per task

Don't use Opus for everything — wasteful:

```typescript
function selectModel(task) {
  switch (task.complexity) {
    case 'simple':       return 'claude-haiku-4-5';   // quick lookups, summarization
    case 'medium':       return 'claude-sonnet-4-6';  // most tasks
    case 'complex':      return 'claude-opus-4-7';    // analysis, reasoning, long context
    case 'experimental': return 'claude-opus-4-7';    // when quality matters most
  }
}
```

Cost savings: Haiku is ~20x cheaper than Opus. For simple tasks, the difference is enormous.

## Caching

Anthropic prompt caching for repeated system prompts:

```typescript
const response = await anthropic.messages.create({
  model: 'claude-opus-4-7',
  system: [
    {
      type: 'text',
      text: longSystemPrompt,
      cache_control: { type: 'ephemeral' }  // cache this
    }
  ],
  messages,
  ...
});
```

When system prompt cached:
- First call: full cost
- Subsequent calls (within TTL): 90% cheaper for system prompt portion

Use for: agent system prompts, RAG context, long instructions.

Don't cache: user-specific content, time-sensitive data.

## Secret management

API keys never in code, never in repo:

```typescript
// ✗ Bad
const anthropic = new Anthropic({ apiKey: 'sk-ant-...' });

// ✓ Good
const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
```

`.env.example` in repo:
```
ANTHROPIC_API_KEY=
GROK_API_KEY=
OPENAI_API_KEY=
```

`.env` in `.gitignore`. Real values in:
- Local: `.env`
- Production: Render env vars (encrypted at rest)
- CI: GitHub Secrets

**Rotate keys periodically.** Anthropic dashboard allows revocation. Rotate every 6-12 months or immediately if compromised.

## Anti-patterns

- **Hardcoded provider.** Switching providers requires code surgery. Use abstraction.
- **No retry.** First network blip kills your feature.
- **No fallback.** Provider has outage, your app is down.
- **Inline prompts.** Every prompt change is a code change. Hard to version, hard to test.
- **One model for everything.** Opus on summarization is wasteful.
- **No cost tracking.** Bill arrives, you don't know what consumed it.
- **No streaming where UX needs it.** User waits 30s for full response when they could see start in 500ms.
- **Streaming everywhere.** Streaming for backend-to-backend calls is overcomplication.
- **Caching user-specific data.** Cache hit shows wrong user's info. Security disaster.
- **No timeout.** Hung LLM call hangs your request forever. Always set timeout (60-120s typical).
- **Logs include prompts/responses with PII.** Privacy violation. Sanitize or redact.

## Configuration pattern

Single config object, env-driven:

```typescript
export const LLM_CONFIG = {
  primary: {
    provider: 'anthropic',
    model: process.env.LLM_PRIMARY_MODEL || 'claude-opus-4-7',
    apiKey: process.env.ANTHROPIC_API_KEY
  },
  fallback: {
    provider: 'grok',
    model: process.env.LLM_FALLBACK_MODEL || 'grok-2',
    apiKey: process.env.GROK_API_KEY
  },
  defaults: {
    maxTokens: 4096,
    temperature: 0.3,
    timeoutMs: 60000,
    maxRetries: 3
  }
};
```

## Integration

- `agent-architecture` builds on `llm-integration` for the call layer
- `multi-agent-orchestration` uses provider abstraction across all agents
- `local-model-deployment` provides one of the providers (Ollama)
- `qa/ai-evals-framework` tests prompts and outputs
- `observability-setup` logs all LLM calls with context
