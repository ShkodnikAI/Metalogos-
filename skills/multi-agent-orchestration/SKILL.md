---
name: multi-agent-orchestration
description: Coordinating multiple specialized AI agents to accomplish complex tasks. Routing logic, hand-off patterns, parallel execution, conflict resolution, error containment. The architectural backbone of AI offices like Fosved. Different from single-agent multi-step — this is about coordination between independent agents with different capabilities.
---

# Multi-Agent Orchestration — When Specialists Collaborate

A single agent can do many things poorly. Specialized agents do specific things well. Multi-agent orchestration is how you compose specialists into systems that handle complex tasks.

Fosved Office is itself a multi-agent system. This skill captures the patterns.

## Prerequisites

- `agent-architecture` understood
- `llm-integration` set up
- Use case has clear specialty boundaries

## Core principle

> Specialization beats generalization for complex domains. The right architecture: many narrow specialists + one orchestrator deciding who handles what. Wrong architecture: one giant generalist trying to do everything, or specialists calling each other freely without coordination.

## Architecture patterns

### Orchestrator-Worker (default for Fosved)

One agent (orchestrator) routes incoming requests to specialists. Specialists do work and return results. Orchestrator integrates and presents.

```
User request
     │
     ▼
[Orchestrator] ─── decides who handles
     │
     ├──> [Specialist A]
     ├──> [Specialist B]
     └──> [Specialist C]
     │
     │ ◄────── results return
     │
[Orchestrator] ─── integrates results
     │
     ▼
User response
```

**Pros:**
- Clear control flow
- One place to debug routing
- Easy to add new specialists
- Conflict resolution centralized

**Cons:**
- Orchestrator can be bottleneck
- Specialists can't directly collaborate

**Use when:** specialties are well-defined and orchestrator can identify them from input.

This is **Yana** in fosved-bot.

### Pipeline (sequential hand-off)

Agents in fixed sequence, each transforming output of previous.

```
Request → Agent A → Agent B → Agent C → Response
```

**Pros:**
- Simple to implement
- Each agent has predictable input shape
- Easy to test in isolation

**Cons:**
- Rigid — same flow regardless of need
- Slow — sequential
- Failure in middle breaks chain

**Use when:** task has clear sequential phases (Design → Dev → QA in DDQ block).

### Mesh (peer-to-peer)

Agents call each other freely based on need.

```
        ┌── Agent A ──┐
        │     ↕      │
        │   Agent B  │
        │     ↕      │
        └── Agent C ──┘
```

**Pros:**
- Maximum flexibility
- Specialists handle their domain

**Cons:**
- Complex control flow
- Loop potential (A calls B calls A...)
- Hard to reason about
- Hard to debug

**Use when:** rarely. Only if simpler patterns fail. Common cause: research labs experimenting.

For Fosved: avoid mesh. Stick to orchestrator-worker.

### Hierarchical (orchestrators have orchestrators)

Multiple levels of orchestration.

```
[Top Orchestrator]
       │
   ┌───┴───┐
   ▼       ▼
[Mid A] [Mid B]
   │       │
 ┌─┴─┐   ┌─┴─┐
 ▼   ▼   ▼   ▼
[Spec][Spec][Spec][Spec]
```

**Use when:** very large agent system (50+ specialists). Decompose into sub-systems.

Fosved doesn't need this yet (10 specialists). Maybe future.

## Routing logic in orchestrator

Orchestrator decides which specialist handles request. Approaches:

### Trigger-based routing (lightweight)

Match keywords/patterns:
```typescript
function routeRequest(text) {
  if (/анализ|проанализируй|сценарии/i.test(text)) return 'osp';
  if (/встреча.*через|due diligence/i.test(text)) return 'expert';
  if (/инфографик|визуализируй/i.test(text)) return 'visual';
  if (/тест|проверь баг/i.test(text)) return 'qa';
  ...
  return 'general';
}
```

**Pros:** fast, deterministic, no LLM cost.

**Cons:** brittle, misses nuance, requires manual updates as triggers evolve.

Used in fosved-bot Yana as first-pass routing.

### LLM-based routing

Ask LLM to classify:
```typescript
const routingPrompt = `
Given user request: "${userText}"
Which specialist should handle this?
Available: osp (strategic analysis), expert (meeting prep), 
visual (infographics), dev (code), design (UI), qa (testing), 
lz (technology tracking).
Return JSON: { specialist: "...", confidence: 0-1, rationale: "..." }
`;

const result = await llm.call([{ role: 'user', content: routingPrompt }]);
```

**Pros:** flexible, handles natural language.

**Cons:** LLM cost per request, latency, can be wrong.

Used as fallback when trigger-based fails.

### Hybrid (fosved-bot Yana approach)

1. Try trigger-based routing
2. If unambiguous match: route
3. If no match or multiple: invoke LLM-based
4. Log routing decision for review

This is how Yana works in fosved-bot.

## Hand-off patterns

When orchestrator routes to specialist, hand-off carries:

```typescript
interface SpecialistTask {
  taskId: string;
  fromOrchestrator: string;
  request: string;
  context: {
    conversationHistory?: Message[];
    relatedArtifacts?: ArtifactRef[];
    userPreferences?: any;
    urgency?: 'low' | 'medium' | 'high';
    expectedOutputFormat?: string;
  };
  deadline?: Date;
  budget?: { maxTokens: number; maxDurationSec: number };
}
```

**Hand-off principles:**
- Specialist gets enough context to act independently
- Specialist returns standardized response (so orchestrator can integrate)
- Hand-off is logged (for debugging and metrics)
- Hand-off is non-blocking (orchestrator doesn't wait synchronously for slow specialists)

## Parallel execution

When request needs multiple specialists with independent work:

```typescript
async function multiDepartmentRequest(request) {
  const [ospResult, lzResult, expertResult] = await Promise.all([
    routeToSpecialist('osp', request),
    routeToSpecialist('lz', request),
    routeToSpecialist('expert', request)
  ]);

  return integrateResults({ ospResult, lzResult, expertResult });
}
```

**When to parallelize:**
- Specialists work on different aspects
- No specialist needs another's output
- Latency matters

**When NOT to parallelize:**
- Sequential dependency exists
- Cost matters more than latency (parallel = simultaneous spend)
- One specialist's input could be informed by another's output

Example: Investment decision needs OSP (context), Expert (meeting prep), ЛЗ (technology status). All can run in parallel. Saves time over sequential.

## Conflict resolution

Specialists may produce conflicting results. Orchestrator decides:

**Strategy 1: First-priority wins.** Declared hierarchy: when OSP and Expert disagree on market direction, OSP wins (it's the strategic analysis department).

**Strategy 2: Highest-confidence wins.** Each specialist returns confidence score. Higher confidence prevails.

**Strategy 3: Synthesis.** Orchestrator combines: "OSP says X with high confidence. Expert says Y with medium confidence. The points of disagreement are..."

**Strategy 4: Escalate to user.** Present both views, ask user.

For Fosved: usually strategy 3 (synthesis). Yana presents nuanced view rather than picking one.

## Error containment

One specialist failing shouldn't bring down orchestrator:

```typescript
async function safeRouteToSpecialist(specialist, request) {
  try {
    return await routeToSpecialist(specialist, request);
  } catch (error) {
    console.error(`[orchestrator] ${specialist} failed:`, error.message);
    return {
      specialist,
      error: error.message,
      fallback: getFallbackResponse(specialist, request)
    };
  }
}
```

Patterns:
- Timeout per specialist call (60-120s typical)
- Retry once if transient error
- Fallback response if specialist unavailable
- Log all failures for review
- Don't propagate raw errors to user — graceful messages

## State management

**Stateless orchestrator (default):** orchestrator has no memory between requests. Each request fresh.

**Pros:** simple, scales horizontally, no consistency issues.
**Cons:** can't reference earlier interactions in conversation.

**Stateful orchestrator:** keeps conversation history, context across requests.

**Pros:** natural conversation, can build up complex tasks.
**Cons:** harder to scale, requires session management.

Fosved uses stateful (conversation history in Prisma `Conversation` table). State stored in DB, not in memory.

## Tool/specialist registration

For extensibility:

```typescript
// lib/specialists/registry.ts
class SpecialistRegistry {
  private specialists = new Map<string, SpecialistDef>();

  register(name: string, def: SpecialistDef) {
    this.specialists.set(name, def);
  }

  get(name: string) {
    return this.specialists.get(name);
  }

  list() {
    return Array.from(this.specialists.entries());
  }
}

interface SpecialistDef {
  name: string;
  description: string;  // for orchestrator routing
  triggers: RegExp[];
  call: (task: SpecialistTask) => Promise<SpecialistResponse>;
  cost: { typical: number; max: number };  // in tokens
}
```

Specialists self-register at bot startup. Adding new = just registering — no orchestrator code change.

## Observability

For each multi-agent request, log:
- Original request
- Routing decision and rationale
- Each specialist call (input, output, duration, cost)
- Conflict resolution if any
- Final integrated response
- Total duration and cost

Store in DB for debugging and metrics.

## Cost management

Multi-agent systems can be expensive — multiple LLM calls per request:

**Strategies:**
- **Cheap orchestrator:** Haiku/Sonnet for routing, Opus only for specialists
- **Conditional specialists:** don't call all specialists every time, only relevant
- **Caching:** specialist system prompts cached (Anthropic prompt cache)
- **Result reuse:** if similar request came recently, reuse partial results

Budget per request type, log when exceeded.

## Anti-patterns

- **Too many specialists.** 50 specialists when 5 would do. Orchestrator routing becomes noise.
- **Generic specialists.** "Helper agent" — not actually specialized. Just an extra LLM call.
- **Mesh without reason.** Specialists calling each other freely. Loops, infinite costs, debug nightmare.
- **No timeout.** Slow specialist hangs entire request.
- **Synchronous chain.** All specialists in series when parallel would work. Slow.
- **No state isolation.** Specialist A's session data leaks to Specialist B. Privacy/security risk.
- **No fallback.** One specialist down = whole system down.
- **No observability.** When something goes wrong, no idea which specialist.
- **Routing by LLM only.** Every request pays for routing LLM call. Use triggers when possible.
- **Specialist that uses other specialists.** Becomes mini-orchestrator. Use proper hierarchy.

## Yana as case study

Yana is the orchestrator in fosved-bot. Behaviors:
- Trigger-based routing first (regex match on keywords)
- LLM-based fallback when ambiguous
- Hand-off via direct function call (not API — in-process)
- Stateful (conversation history accessed)
- Tracks routing decisions for accuracy metric
- Multi-department parallel for investment decisions, briefings
- Sequential for design → dev → qa pipeline
- Conflict resolution via synthesis (presents nuanced view)

This is the reference implementation.

## Integration

- `agent-architecture` defines what specialists look like internally
- `llm-integration` provides the LLM layer specialists use
- Used by Yana in fosved-bot
- `qa/ai-evals-framework` tests orchestration correctness
- Observability via existing logging + AIEvalRun
