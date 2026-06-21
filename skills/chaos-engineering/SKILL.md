---
name: chaos-engineering
description: Deliberately injecting failures into systems to verify resilience — killing services, adding latency, exhausting resources, simulating network partitions. Tests whether the system degrades gracefully or collapses catastrophically. For production-grade systems where uptime matters.
---

# Chaos Engineering — Break It On Purpose, Before It Breaks Itself

Every system fails eventually — a service crashes, a network blips, a disk fills, a dependency goes down. The question is not "if" but "what happens when". Chaos engineering answers it deliberately: induce the failure in controlled conditions, observe, verify graceful handling.

## Prerequisites

- `failure-modes-mapping` understood
- System is production-grade (uptime matters)
- Staging environment OR mature production monitoring
- Resilience mechanisms exist to test (retries, fallbacks, circuit breakers)

## Core principle

> You don't know if your system survives failure until you cause failure. Documentation says "it has retry logic" — chaos engineering proves it by killing the dependency and watching. The discipline: the failure you induce in a controlled test is the failure you don't fear in production.

## When chaos engineering applies

**Use chaos engineering when:**
- System has uptime requirements
- System has resilience mechanisms (retries, fallbacks, circuit breakers) that need verification
- System has multiple components that can fail independently
- Production incidents have happened (learn from them, prevent recurrence)

**Don't bother when:**
- Prototype / experiment
- Single-user tool with no uptime requirement
- No resilience mechanisms exist yet (build them first, then test them)
- Very simple system with one component

For Fosved: applies to fosved-bot (production AI office) once it's mature. Not for throwaway scripts.

## The chaos experiment structure

A chaos experiment is a hypothesis test:

```
1. Steady state: define what "normal" looks like (metrics)
2. Hypothesis: "if X fails, the system will still [behavior]"
3. Inject the failure
4. Observe: did the hypothesis hold?
5. Conclude: resilient (hypothesis held) or vulnerability found (didn't)
6. If vulnerability: fix it, re-test
```

Example:
```
Steady state: bot responds to /analyze in < 30s, 99% success
Hypothesis: if Anthropic API goes down, bot falls back to Grok and still responds
Inject: block Anthropic API endpoint
Observe: does bot fall back? does it still respond? latency?
Conclude: fallback works (good) OR bot just errors (vulnerability — fix fallback)
```

## Failure injection types

### Service failure
Kill a dependency. Database down, API unreachable, microservice crashed.

```
- Stop the database container
- Block the LLM provider endpoint (firewall rule)
- Kill a dependent service
```

Verify: graceful degradation, clear errors, recovery when service returns.

### Latency injection
Make a dependency slow, not dead.

```
- Add 5s delay to database responses
- Throttle LLM API to 30s responses
- Simulate slow network
```

Verify: timeouts work, system doesn't hang forever, user gets feedback.

### Resource exhaustion
Starve the system.

```
- Fill the disk
- Consume available memory
- Exhaust the connection pool
- Max out CPU
```

Verify: graceful handling, no data corruption, recovery.

### Network failures
```
- Network partition (component can't reach another)
- Packet loss
- DNS failure
- Connection drops mid-request
```

Verify: retries, reconnection, no stuck states.

### Error injection
Make a dependency return errors.

```
- API returns 500s
- API returns malformed responses
- API returns 429 rate limits
- Database returns errors
```

Verify: error handling, retries on retryable errors, no crash on bad data.

### Time manipulation
```
- Clock skew between components
- Sudden time jumps
- Token expiry during operation
```

Verify: time-dependent logic survives.

## Chaos for AI offices specifically

fosved-bot-style systems have AI-specific chaos scenarios:

**LLM provider failure:**
- Anthropic API down → does Grok fallback engage? (test the fallback chain from `llm-integration`)
- All providers down → graceful error, work not lost?

**LLM returns garbage:**
- Provider returns malformed JSON → parser handles it?
- Provider returns empty response → handled?
- Provider returns refusal → handled?

**Slow LLM:**
- LLM takes 60s → timeout? user feedback during wait?

**Cost runaway:**
- Inject a scenario causing repeated LLM calls → does cost limiting kick in?

**Database failure mid-operation:**
- Expensive analysis completes, DB write fails → is work recoverable? or lost?

**Specialist failure in multi-agent:**
- One specialist (OSP) errors → does Yana handle gracefully? do other specialists still work?

These map directly to the AI office's failure modes.

## Tools

**Manual chaos (start here):**
- Stop containers: `docker stop <service>`
- Block endpoints: firewall rules, `/etc/hosts` manipulation
- Resource limits: `stress-ng`, `ulimit`
- Network: `tc` (traffic control) for latency/loss

**Frameworks:**
- **Chaos Toolkit** — open-source, declarative experiments
- **Toxiproxy** — programmable network failure injection (great for latency, connection drops)
- **Gremlin** — commercial chaos platform
- **LitmusChaos** — Kubernetes-native

For Fosved: start with manual chaos + Toxiproxy for network scenarios. Frameworks if it scales.

```javascript
// Toxiproxy example — inject latency to database
const toxiproxy = require('toxiproxy-node-client');
const proxy = await toxiproxy.createProxy({
  name: 'postgres',
  listen: 'localhost:5433',
  upstream: 'localhost:5432'
});
// Add 3s latency
await proxy.addToxic({
  type: 'latency',
  attributes: { latency: 3000 }
});
// Now run tests against the slow database
```

## Start small, controlled

**Don't start chaos in production.** Progression:

1. **Local/staging first** — inject failures in safe environment
2. **Limited blast radius** — small experiments, one failure at a time
3. **Off-peak** — if testing production-like, low-traffic times
4. **Monitored** — watch closely, ready to abort
5. **Gradual** — only expand to production after confidence built

**Always have an abort button** — ability to stop the experiment and restore immediately.

## The game day

A "game day" is a scheduled chaos session:

- Team gathers (for Fosved: owner + Claude Code)
- Run a series of chaos experiments
- Observe, document
- Fix vulnerabilities found
- Repeat periodically (quarterly)

Game day mindset: deliberately, calmly breaking things to learn — not firefighting.

## Steady-state metrics

Before any experiment, define "normal":

- Request success rate (e.g., 99.5%)
- Latency p99 (e.g., < 2s)
- Error rate (e.g., < 0.5%)
- For AI office: response completion rate, cost rate

During experiment: are these maintained (degraded acceptably) or violated (catastrophically)?

## What you're verifying

Chaos experiments verify resilience mechanisms actually work:

- **Retries** — do they trigger on transient failure? exponential backoff? give up eventually?
- **Fallbacks** — does the fallback path engage? (LLM provider fallback chain)
- **Circuit breakers** — does the system stop hammering a dead dependency?
- **Timeouts** — do operations give up rather than hang forever?
- **Graceful degradation** — does partial failure mean partial service, not total collapse?
- **Recovery** — when the failure ends, does the system recover automatically?
- **Data integrity** — does failure mid-operation corrupt data, or stay consistent?

If a mechanism is documented but chaos shows it doesn't work — that's a critical finding.

## Documenting findings

Each experiment recorded:

```
Experiment: Anthropic API outage
Hypothesis: Bot falls back to Grok, continues serving
Steady state: 99% /analyze success, p99 25s
Injection: blocked api.anthropic.com
Observed: 
  - Fallback to Grok engaged after 2 retries (~8s added latency)
  - 97% success maintained (acceptable degradation)
  - 3% failures were requests that also exhausted Grok retries
Verdict: RESILIENT — fallback works
Follow-up: investigate the 3% — should fall through to third provider
```

Findings that reveal vulnerabilities → `DefectRecord`, fix, re-test.

## Anti-patterns

- **Chaos in production without preparation.** Real users, real damage. Build confidence in staging first.
- **No steady state defined.** Can't tell if the experiment degraded things — no baseline.
- **No abort plan.** Experiment goes wrong, can't stop it quickly.
- **Testing without resilience mechanisms.** Chaos on a system with no retries/fallbacks just confirms it breaks. Build resilience first, then verify with chaos.
- **One giant experiment.** Inject 5 failures at once. When something breaks, unclear which caused it.
- **No documentation.** Run chaos, learn things, forget them. No compounding.
- **Chaos as stunt.** "We do chaos engineering" as a badge, without acting on findings.
- **Ignoring findings.** Vulnerability found, noted, never fixed.
- **No follow-up testing.** Fixed a vulnerability, never re-ran chaos to confirm the fix.

## Realistic scope for Fosved

For fosved-bot once mature:

**Quarterly game day, manual chaos:**
- Kill the database → verify graceful errors, recovery
- Block Anthropic → verify Grok fallback
- Block all LLM providers → verify graceful failure, no data loss
- Slow the database (Toxiproxy) → verify timeouts
- Exhaust connection pool → verify handling
- Kill bot mid-analysis → verify recovery, no corrupt archive

Document, fix what breaks, re-test. Don't over-engineer — a handful of well-chosen experiments covers the main risks.

## Integration

- `failure-modes-mapping` — chaos experiments target the mapped failure modes
- `performance-testing` — related: load is one form of stress
- `observability-setup` (Dev) — essential, provides the metrics during experiments
- `llm-integration` (Dev) — the fallback chains chaos verifies
- `defect-discipline` — vulnerabilities found become defects
- `TestRun` with `testType: chaos` (or special handling)
- Quarterly game days for production-grade systems
