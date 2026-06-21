---
name: local-model-deployment
description: Deploying LLMs locally — Ollama, llama.cpp, model selection by hardware, integration with applications. When local makes sense (privacy, cost at scale, offline) and when it doesn't (latest model needed, complex tasks, low volume). Practical guide for production local deployment.
---

# Local Model Deployment — LLMs Without the Cloud

Cloud LLMs (Claude, GPT) are powerful but expensive at scale and require network. Local models run on your hardware. This skill is when and how to deploy them.

## Prerequisites

- Use case evaluated against cloud cost
- Hardware available or budgeted
- Acceptance of capability gap vs frontier models

## Core principle

> Local models are a tool, not an ideology. They make sense when privacy is critical, volume is high, or offline operation is required. They don't make sense when you just want to avoid paying API costs for low-volume tasks — the engineering effort exceeds the savings.

## When local makes sense

**Yes, local:**
- Sensitive data can't leave premises (regulatory, contractual)
- Volume so high that API costs > hardware costs
- Offline operation required (no internet)
- Latency must be very low (LAN < cloud round-trip)
- Cost predictability needed (no per-request charges)
- Experimentation/training (custom fine-tuning)

**No, cloud:**
- Best quality required (frontier models always cloud)
- Low volume (< few thousand requests/month)
- Complex tasks (Opus beats local at reasoning)
- Multimodal needed (vision/audio mostly cloud)
- Team without ML ops expertise
- Tight timeline (cloud is faster to set up)

## Hardware sizing

Model size determines hardware:

| Model size | VRAM needed | Hardware example |
|-----------|-------------|------------------|
| 7B (Llama 3 7B, Mistral 7B) | 6-8 GB | RTX 4060 (8GB), Mac M2/M3 16GB unified |
| 13B (Llama 2 13B) | 10-12 GB | RTX 4070 Ti, Mac M3 Pro 32GB |
| 30B-34B (Llama 3 33B) | 20-24 GB | RTX 4090 (24GB), Mac M3 Max 48GB |
| 70B (Llama 3 70B) | 40-50 GB | 2x RTX 4090, Mac M3 Max 96GB+ |
| 405B (Llama 3 405B) | 250+ GB | server-grade GPUs (A100/H100) |

**Quantization** reduces memory at quality cost:
- FP16 (full precision): largest, best quality
- Q8 (8-bit): half size, minimal quality loss
- Q4 (4-bit): quarter size, noticeable but usable quality loss
- Q2-Q3: heavy quality loss, only for memory-constrained

**Practical defaults 2026:**
- Personal dev machine: Llama 3 8B Q4 (runs anywhere)
- Workstation: Llama 3 70B Q4 (RTX 4090)
- Server: Llama 3 70B Q8 or 405B Q4 (multi-GPU)

## Tool: Ollama (recommended default)

Ollama is the easiest path to local LLM. Wraps llama.cpp with friendly API.

**Install:**
```bash
# macOS
brew install ollama

# Linux
curl -fsSL https://ollama.com/install.sh | sh

# Windows
# Download from ollama.com
```

**Pull and run model:**
```bash
ollama pull llama3.3:70b    # download
ollama run llama3.3:70b     # interactive chat
```

**Available via HTTP API:**
```bash
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.3:70b",
  "prompt": "What is the capital of France?"
}'
```

**Integration with Fosved-style provider abstraction:**
```typescript
// lib/llm/ollama.ts
export class OllamaProvider implements LLMProvider {
  name = 'ollama';

  async call(messages, options) {
    const response = await fetch('http://localhost:11434/api/chat', {
      method: 'POST',
      body: JSON.stringify({
        model: options.model || 'llama3.3:70b',
        messages,
        stream: false,
        options: {
          num_predict: options.maxTokens || 4096,
          temperature: options.temperature || 0.7
        }
      })
    });
    const data = await response.json();
    return {
      text: data.message.content,
      usage: {
        inputTokens: data.prompt_eval_count || 0,
        outputTokens: data.eval_count || 0
      },
      model: data.model,
      finishReason: data.done_reason || 'stop'
    };
  }
}
```

## Tool: llama.cpp (lower-level, more control)

For fine control or non-standard hardware:

```bash
# Build llama.cpp
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make

# Download model (GGUF format)
# Then run:
./main -m models/llama-3-70b.Q4_K_M.gguf -p "Hello" -n 256
```

**When to use llama.cpp over Ollama:**
- Need specific quantization options
- Need fine-grained performance tuning
- Custom embedding generation
- Embedded in C/C++ application
- Server runs in environment Ollama doesn't support

For most cases: Ollama. llama.cpp is power-user tool.

## Model selection

**Llama 3.x family (Meta):** general-purpose, strong reasoning, multilingual. Default choice.

**Mistral / Mixtral (Mistral AI):** efficient (good performance per parameter). Use when hardware-constrained but want quality.

**Qwen 2.5 (Alibaba):** strong reasoning, good code. Multilingual including Chinese excellent.

**DeepSeek R1 (DeepSeek):** reasoning specialist (chain-of-thought baked in). Use for complex reasoning tasks.

**Phi-3 / Phi-4 (Microsoft):** small (3-14B), surprisingly capable. Good for edge.

**Code-specific:** Qwen Coder, DeepSeek Coder, CodeLlama — for code completion tasks.

**Embedding models (different family):**
- nomic-embed-text (general)
- mxbai-embed-large (high quality)

Tech radar reviews quarterly; specific recommendations evolve.

## Hosting options

**Personal dev machine:** Ollama, single user. Fine for development.

**Workstation as server:** Ollama bound to LAN, multiple users via LAN. Up to 5-10 concurrent users typical.

**Dedicated server (on-prem):** vLLM or similar inference server. Production-grade. Tens to hundreds of concurrent users.

**Cloud GPU (rented):** RunPod, Vast.ai, Lambda Labs. Pay per hour. Use for: bursty workloads, no local hardware.

**For Fosved owner's setup:** local server (mentioned in interest). When ready, Ollama on the server with LAN access from fosved-bot.

## Integration with fosved-bot

The provider abstraction makes local easy:

```typescript
// .env
LLM_PRIMARY_PROVIDER=anthropic
LLM_FALLBACK_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.3:70b

// Configuration determines provider
const provider = process.env.LLM_PRIMARY_PROVIDER === 'ollama'
  ? new OllamaProvider(process.env.OLLAMA_HOST)
  : new AnthropicProvider(process.env.ANTHROPIC_API_KEY);
```

Fallback chain:
```
Primary (Claude) → if fails → Fallback (Grok) → if fails → Local (Ollama)
```

Or invert for privacy-sensitive ops:
```
Primary (Ollama for sensitive analysis) → Fallback (Claude with sanitized input)
```

## Performance tuning

**Cold start:** first request to Ollama loads model from disk. Can take 30-60s for large models. Subsequent requests fast (~seconds).

**Keep alive:** `OLLAMA_KEEP_ALIVE=5m` keeps model in memory for 5 min after last request. Reduce cold starts.

**Concurrent requests:** Ollama processes sequentially by default. For concurrency, set `OLLAMA_NUM_PARALLEL=2` (depending on GPU memory).

**Context length:** longer context = more VRAM. Trim to needed.

**Speculative decoding:** smaller model "drafts" tokens, large model verifies. 2-3x speedup on some hardware.

## Cost analysis

**Cloud cost (Claude Opus):**
- $15/million input tokens, $75/million output
- Typical OSP analysis: 50k tokens → $1-2 per analysis
- At 100 analyses/month: $100-200

**Local cost (Llama 3 70B on RTX 4090):**
- Hardware: $1500 (one-time)
- Electricity: ~$10-20/month (depending on rates and usage)
- Payback: 10-15 months at the volume above

Plus quality difference: Opus dramatically outperforms Llama 3 70B on complex reasoning. So cost saving comes with quality cost.

**Realistic Fosved use cases for local:**
- Bulk processing (summarization, classification) where Llama is good enough
- Privacy-sensitive (sanitize before sending to cloud)
- High-volume routine tasks
- Embedding generation (much cheaper local)

Not for: OSP V2 deep analyses (need Opus), Expert briefings (need quality).

## Embeddings

Often-missed cost: embedding generation. Cloud charges per token; local is free after hardware.

For RAG systems with large corpora:
```typescript
// Local embedding via Ollama
const response = await fetch('http://localhost:11434/api/embed', {
  method: 'POST',
  body: JSON.stringify({
    model: 'nomic-embed-text',
    input: text
  })
});
const { embedding } = await response.json();
// 768-dim vector, free to compute locally
```

Embed 1M documents: cloud = $50-100, local = ~$2 electricity.

## Quality testing

Before switching production to local, run AI evals (see `qa/ai-evals-framework`):
- Golden dataset of 50+ representative inputs
- Compare outputs: cloud vs local
- Measure quality drop
- Accept or reject based on use case tolerance

Typical findings:
- Simple summarization: local matches cloud (good)
- Complex reasoning: local 60-80% of cloud quality (varies)
- Multilingual: depends heavily on model

## Operational considerations

**Updates:** Ollama models update via `ollama pull`. Pin versions in production.

**Monitoring:** track latency, errors, GPU memory. Standard observability applies.

**Backup:** model files are large (40GB+). Backup is annoying. Re-download is usually fine (publicly available).

**Security:** Ollama default bind is localhost. For LAN access:
```bash
OLLAMA_HOST=0.0.0.0 ollama serve
```
Add firewall rules. Don't expose to public internet.

## Anti-patterns

- **Local for everything.** Replacing frontier model with local across the board. Quality cliff. Use selectively.
- **Underprovisioning hardware.** "Llama 3 70B on a laptop". Will swap and be slow. Match hardware to model.
- **Ignoring quantization.** "Full FP16 only". Q8 or Q4 usually fine and frees memory.
- **No fallback.** Local server down = nothing works. Cloud fallback essential.
- **No monitoring.** GPU OOM, model crashes silently.
- **Same provider abstraction without quality check.** Code works but quality dropped — silent regression.
- **Over-quantizing.** Q2/Q3 on small model = useless output.
- **Inference server in same process as app.** Server crashes, app crashes. Run as separate service.
- **No version pinning.** Model updates change behavior unexpectedly.

## Migration path for Fosved

When owner has local server ready:

**Phase 1:** Deploy Ollama with Llama 3 70B on local server.

**Phase 2:** Add OllamaProvider to fosved-bot's `lib/llm/`.

**Phase 3:** Route low-stakes tasks to local (summarization, classification, embeddings).

**Phase 4:** AI eval comparison on high-stakes tasks (OSP analyses) — measure quality.

**Phase 5:** If quality acceptable, route appropriate tasks. Keep cloud for complex.

**Phase 6:** Consider fine-tuning local model on Fosved-specific tasks (see `model-fine-tuning`).

## Integration

- `llm-integration` provides provider abstraction (Ollama is one provider)
- `agent-architecture` uses LLMs — can use local for some agents
- `model-fine-tuning` specializes local models
- `qa/ai-evals-framework` validates quality before adoption
- `devops-deployment` covers operating the local server
