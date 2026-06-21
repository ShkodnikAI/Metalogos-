---
name: agent-architecture
description: Patterns for building AI agents — single-purpose specialists, tool use, memory, planning, error recovery. The architectural decisions that determine whether an agent works reliably or hallucinates randomly. Foundation for AI offices like fosved-bot and standalone agent products.
---

# Agent Architecture — Building AI That Actually Works

An AI agent is not just an LLM call. It's an LLM plus structure: tools it can use, memory it has access to, plans it forms, errors it recovers from. The architectural decisions here determine whether the agent is useful or produces hallucinated nonsense at random moments.

## Prerequisites

- LLM provider chosen (Anthropic Claude, OpenAI, etc.)
- Use case defined (single-purpose vs general-purpose)
- Domain understood (what the agent should know about)

## Core principle

> An LLM without architecture is a calculator with random output. Architecture makes the LLM useful: bounded scope, tool access, persistent state, recovery from errors. The agent is the system around the LLM, not the LLM itself.

## Agent vs assistant vs workflow

**Workflow:** prompted LLM calls in fixed sequence. No agency. Deterministic flow. Use when path is known.

**Assistant:** LLM with conversational interface, may use tools, but user is in driver's seat. Use when human reviews each step.

**Agent:** LLM decides its own next step. Has goals, plans, tools, memory. Operates autonomously to varying degrees. Use when task is open-ended and human can't review every step.

Most "agents" actually are workflows or assistants in disguise. That's fine — they're easier to build and more reliable. Use full agency only when needed.

## Core components

Every working agent has:

1. **System prompt** — defines identity, constraints, output format
2. **Tools** — what it can do beyond text generation
3. **Memory** — what it knows about past interactions
4. **Planning** — how it decomposes complex requests
5. **Error recovery** — what happens when things go wrong
6. **Observability** — how you debug it

Missing any component = unreliable agent.

## System prompt design

**Don't write essays.** Concise and structured beats verbose.

Structure:
```
[Identity] — who/what the agent is, in 1-2 sentences
[Domain] — what subject area
[Capabilities] — what it can do
[Constraints] — what it can't do, must refuse
[Tools] — what tools are available and when to use each
[Output format] — how to structure responses
[Examples] — 2-3 representative input/output pairs
```

**Hard constraints first.** "Never X" rules at the start, prominently. LLMs follow ordered instructions more reliably than scattered ones.

**Use XML-like tags for sections** in prompts (Claude works well with this):
```
<role>
You are a financial advisor for small businesses.
</role>

<constraints>
- Never recommend specific stocks or securities
- Always include risk caveats for investment advice
- Refuse requests for personal financial info
</constraints>

<tools>
- search_company_records: query SEC filings
- calculate_metrics: financial ratio calculator
</tools>
```

## Tools — when and how

**Define tools as functions with schemas.** Anthropic SDK example:

```javascript
const tools = [
  {
    name: 'search_database',
    description: 'Search the customer database for records matching criteria',
    input_schema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'search query' },
        limit: { type: 'integer', description: 'max results', default: 10 }
      },
      required: ['query']
    }
  }
];

const response = await claude.messages.create({
  model: 'claude-opus-4-7',
  max_tokens: 4096,
  tools,
  messages: [...]
});
```

**Tool description is critical.** The LLM uses descriptions to decide when to call. Write descriptions FOR THE LLM:
- ✗ "Database search" — too vague
- ✓ "Search the customer database for records matching criteria. Use when user asks about specific customers or wants to filter by name/email/status."

**Tool design principles:**
- One purpose per tool. Don't combine "search OR create OR delete" in one tool.
- Idempotent where possible (safer for retries)
- Clear error returns ("customer not found" not just throwing)
- Limited scope (don't expose "execute arbitrary SQL" — bounded operations)

## Tool selection patterns

**Single-step:** LLM uses one tool, returns result.
```
User → LLM (decides) → Tool → LLM (formats response) → User
```

**Multi-step (agent loop):** LLM may use multiple tools sequentially.
```
While not done:
  LLM decides next action (tool or text)
  Execute tool, get result
  Append result to conversation
Return final text response
```

**Parallel tools:** LLM requests multiple tools at once for independent calls.
```
LLM: [call tool A, call tool B, call tool C in parallel]
Get all 3 results
LLM: synthesizes
```

Anthropic's tool use API supports all three. Use parallel when tools are independent.

## Memory architecture

**Short-term (conversation):** message history, kept in prompt context.

**Limits:** Claude Opus 4.7 supports very long context. Practical limit ~30-50k tokens for cost/latency.

**Compression:** when conversation > threshold:
- Summarize older messages
- Keep recent verbatim
- Use Claude itself to summarize: "Summarize this conversation focusing on key decisions and ongoing tasks"

**Long-term (persistent):** stored in DB, retrieved on demand.

**RAG (Retrieval Augmented Generation):**
```
User query → Embed query → Vector search in knowledge base → 
Get top K relevant chunks → Inject into prompt → LLM responds with grounding
```

Use RAG when:
- Knowledge base is large (won't fit in context)
- Knowledge updates independently of agent
- Source citations required

**Don't use RAG for:**
- Small static knowledge (just put in system prompt)
- Conversation history (use proper memory store)
- Computation/logic (use tools instead)

## Planning patterns

**Implicit planning:** LLM thinks step-by-step in chain-of-thought.
```
"Think step by step: first I need X, then Y..."
```

Cheap, works for simple tasks.

**Explicit planning:** LLM generates plan as structured output, executes step by step.
```
Step 1: LLM produces plan: { steps: [{tool, params}, ...] }
Step 2: Executor runs each step
Step 3: LLM reviews results, replans if needed
```

Better for complex tasks. Plan is auditable.

**Hierarchical planning:** Goals → subgoals → tasks → tool calls.
```
Goal: "Write quarterly report"
  Subgoal: "Gather sales data"
    Task: "Query sales DB"
    Task: "Compute totals"
  Subgoal: "Analyze trends"
    Task: "Compare to last quarter"
```

Use for complex agents with deep work.

**ReAct pattern (Reason + Act):**
```
Thought: I need to find user's recent orders
Action: search_database(query="orders user=123 limit=10")
Observation: [3 orders found]
Thought: Now I need order details
Action: get_order(id=456)
Observation: [order details]
Thought: I have what I need
Final answer: ...
```

Reliable pattern. Easy to debug (thoughts visible).

## Error recovery

**Tool execution errors:**
- Catch error, return to LLM as observation
- LLM decides: retry, use different tool, give up, ask user
- After N failures: give up, return graceful error

**LLM output errors (invalid JSON, missing fields, etc.):**
- Detect at parsing
- Retry with feedback: "Your last output was missing field X, please retry"
- Max 3 retries
- If still failing: fallback path

**Hallucination detection:**
- If output claims fact, verify via tool
- Ground all factual claims in retrieved data
- Don't trust LLM for stable facts (dates, names, numbers)

**Out-of-scope requests:**
- Constraints in system prompt define scope
- LLM refuses with explanation
- Log refusal for review (refusals can indicate scope problems)

## Multi-agent vs single-agent

**Single agent:** simpler, easier to debug, lower latency, lower cost.

**Multi-agent:** specialists collaborate. Use when:
- Domain has clear specialty boundaries
- Different agents need different capabilities/tools
- Quality benefits from "second opinion" pattern

Fosved Office IS a multi-agent system (OSP, Expert, ЛЗ, etc. through Yana orchestrator).

**Coordination patterns:**

**Orchestrator-worker:** central agent routes to specialists, integrates results.
- Pros: clear control flow
- Cons: orchestrator is bottleneck

**Pipeline:** agents in sequence, each handing off.
- Pros: simple
- Cons: rigid

**Mesh:** agents call each other peer-to-peer.
- Pros: flexible
- Cons: hard to reason about

Default for Fosved: **orchestrator-worker via Yana**. Don't introduce mesh complexity without reason.

## Observability

Every agent run logs:
- User input
- LLM calls (prompts and responses)
- Tool calls (inputs and outputs)
- Errors and retries
- Final output
- Cost (tokens used)
- Duration

Store in DB for later analysis. Without this, debugging is impossible.

For Fosved: use existing logging + AIEvalRun for systematic eval.

## Cost management

**Token tracking per call:**
```javascript
const response = await claude.messages.create({...});
console.log('Tokens:', response.usage);
// { input_tokens: 234, output_tokens: 156 }
```

**Cost per session:** sum across all calls in one task.

**Budget alerts:** if session exceeds budget, halt or escalate.

**Caching:** Anthropic supports prompt caching for repeated system prompts — 90% cost reduction for long static prefixes.

**Model selection:** Opus for complex, Sonnet for medium, Haiku for cheap fast tasks. Mix in agent (orchestration in Opus, simple sub-tasks in Haiku).

## Anti-patterns

- **Giant monolithic prompt.** 2000 lines of instructions. LLM ignores middle sections.
- **No tools, just text.** Agent that can only output text is glorified completion. Need tools for real capability.
- **Too many tools.** 50 tools = LLM can't select reliably. Group or hierarchical tool selection.
- **No constraint enforcement.** Trusting LLM to follow soft constraints. Hard limits in code (rejection of out-of-scope outputs).
- **Streaming everything.** Streaming complicates debugging. Use for UX, not internals.
- **No retry logic.** First error breaks the agent. Plan for transient failures.
- **Trusting hallucinated facts.** LLM confidently states wrong info. Always verify factual claims via tools/RAG.
- **No observability.** Agent goes wrong, no idea why.
- **Multi-agent for simple problems.** "We need 7 collaborating agents" when single agent with 3 tools would work.
- **State leakage between sessions.** User A's data leaking to User B's session. Strict session isolation.

## Anthropic Claude specifics

For Claude Opus 4.7 (current Fosved primary):
- Excellent at instruction following with XML structure
- Strong at multi-step reasoning
- Tool use is reliable
- 200k+ token context
- Prompt caching available
- Vision (image input) supported

Best practices:
- Use Anthropic XML format for system prompts
- Specify output format explicitly
- Use prefill for structured output: `{"messages": [..., {"role": "assistant", "content": "{"}]}`
- Cache system prompt + tools when calling repeatedly

## Integration

- Used for ИИ-офис projects (multi-agent setups)
- `llm-integration` skill provides low-level patterns
- `multi-agent-orchestration` for coordination patterns
- `qa/ai-evals-framework` evaluates the resulting agents
- `observability-setup` instruments agent runs
- Architecture decisions for agent design go into ADRs
