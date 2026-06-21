---
name: performance-testing
description: Testing system performance under load — load tests, stress tests, soak tests, profiling. Verifying the system meets latency and throughput targets and finding the breaking point before users do. Distinct from dev-side performance-optimization — this is the QA verification.
---

# Performance Testing — Verify It Holds Under Load

A system that works for one user may collapse under a hundred. Performance testing finds the limits, verifies targets are met, and locates bottlenecks — before real load exposes them painfully.

## Prerequisites

- `test-strategy-design` understood
- Performance targets defined (latency, throughput)
- System deployed to test/staging environment

## Core principle

> Performance under load is invisible until you create load. A system tested only with single-user interactions tells you nothing about behavior at 100 or 1000 users. Performance testing creates the load deliberately, in a controlled environment, so the breaking point is a known number, not a production surprise.

## Types of performance tests

### Load test
System under expected load. Does it meet targets?

- Simulate realistic concurrent users
- Run for sustained period
- Measure: latency (p50/p95/p99), throughput, error rate
- Verdict: meets targets or not

### Stress test
Push beyond expected load. Where does it break?

- Gradually increase load until degradation/failure
- Find the breaking point
- Observe HOW it fails (graceful degradation vs catastrophic crash)
- Verdict: breaking point location + failure mode

### Spike test
Sudden load surge. Does it survive?

- Jump from low to very high load instantly
- Simulates: viral moment, traffic spike, thundering herd
- Verdict: survives spike or crashes

### Soak test (endurance)
Sustained load over long time. Memory leaks? Resource exhaustion?

- Moderate load for hours/days
- Watch for: memory growth, connection leaks, degradation over time
- Verdict: stable over time or degrades

### Scalability test
How does performance change as load scales?

- Measure at 10, 50, 100, 500 users
- Plot the curve
- Verdict: linear scaling, sub-linear, or cliff

## Targets to verify

From `performance-optimization` (Dev) — QA verifies these are met:

**Web app:**
- LCP < 2.5s, FID < 100ms, CLS < 0.1
- Page load < 3s on 3G

**API:**
- p50 < 100ms, p99 < 500ms (simple endpoints)
- Error rate < 0.1% under expected load

**AI office:**
- Routing < 500ms
- Response start < 2s
- Handles N concurrent conversations

If targets aren't documented — that's the first finding. "No targets" means no way to pass/fail.

## Tools

**k6** (recommended for Fosved — modern, scriptable):

```javascript
// load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 50 },   // ramp up to 50 users
    { duration: '5m', target: 50 },   // stay at 50
    { duration: '2m', target: 0 },    // ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% under 500ms
    http_req_failed: ['rate<0.01'],    // error rate under 1%
  },
};

export default function () {
  const res = http.get('https://staging.example.com/api/data');
  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time OK': (r) => r.timings.duration < 500,
  });
  sleep(1);
}
```

```bash
k6 run load-test.js
```

**Other tools:**
- **Artillery** — alternative load tester, YAML config
- **Lighthouse CI** — web vitals in CI
- **autocannon** — quick HTTP benchmarking
- **Apache Bench (ab)** — simple, classic

## Realistic load modeling

Bad load test: 100 users all hitting one endpoint simultaneously. Not realistic.

Good load test: model real usage:
- Mix of endpoints (read-heavy usually — 80% reads, 20% writes typical)
- Realistic think time between actions (users don't hammer)
- Realistic ramp-up (not instant 100 users)
- Realistic data variety (not all requesting same record — caching skews results)

```javascript
export default function () {
  // Realistic user journey, not endpoint hammering
  http.get(`${BASE}/api/dashboard`);
  sleep(randomBetween(2, 5));  // user reads dashboard

  http.get(`${BASE}/api/items?page=${randomPage()}`);
  sleep(randomBetween(1, 3));

  if (Math.random() < 0.2) {  // 20% of users create something
    http.post(`${BASE}/api/items`, JSON.stringify(randomItem()));
  }
}
```

## Test environment

Performance tests need an environment resembling production:

- **Same-ish hardware** — testing on a tiny instance tells you nothing about production capacity
- **Realistic data volume** — empty DB queries are fast; 1M-row DB is the real test
- **Isolated** — don't load-test production (you'll DoS yourself), use staging
- **Monitored** — observe CPU, memory, DB, network during test

If staging is much smaller than production: extrapolate carefully, or note the limitation.

## What to measure

During the test, capture:

**Application:**
- Latency percentiles (p50, p95, p99) — averages lie, percentiles tell truth
- Throughput (requests/sec)
- Error rate

**System:**
- CPU utilization
- Memory usage (and growth over time — leak detection)
- Disk I/O
- Network I/O

**Database:**
- Query latency
- Connection pool usage
- Slow query log
- Lock contention

**LLM systems:**
- Token throughput
- Provider latency
- Cost rate (tokens × price per second)
- Queue depth if requests queued

## Reading results

**p99 is the truth.** Average latency of 100ms sounds fine — but if p99 is 5s, 1% of users have a terrible experience. At scale, 1% is many people.

**Find the knee.** As load increases, latency stays flat then suddenly spikes — that knee is your capacity limit.

**Watch the failure mode.** When overloaded:
- Graceful: requests queue, latency rises, but no errors → acceptable
- Catastrophic: errors spike, crash, data corruption → must fix

**Memory over time.** In soak test, memory should plateau. Continuous growth = leak → will crash eventually.

## AI-specific performance testing

LLM systems have unique performance characteristics:

- **Provider rate limits:** at high load, you hit Anthropic/OpenAI rate limits. Test the fallback/retry behavior.
- **Cost under load:** 1000 concurrent analyses = significant $ per minute. Model the cost curve.
- **Latency variance:** LLM latency varies wildly. p99 may be 5x p50.
- **Local model throughput:** local models process sequentially (mostly). Concurrency limited by hardware.
- **Context window pressure:** under load, are contexts trimmed appropriately or do they bloat?

```javascript
// Load test for AI office
export default function () {
  const res = http.post(`${BASE}/api/analyze`, JSON.stringify({
    topic: randomTopic()
  }), { timeout: '60s' });  // LLM calls are slow

  check(res, {
    'completed': (r) => r.status === 200,
    'within latency budget': (r) => r.timings.duration < 30000,
    'no rate limit error': (r) => r.status !== 429,
  });
}
```

## Performance regression in CI

Catch performance regressions automatically:

```yaml
# CI step
- name: Performance smoke test
  run: |
    k6 run --quiet ci-perf-test.js
    # k6 thresholds fail the build if exceeded
```

Lighter than full load test — quick check that nothing regressed dramatically. Full load test runs less often (pre-release).

Bundle size as performance regression:
```yaml
- name: Bundle size check
  run: npx size-limit  # fails if bundle grew beyond limit
```

## When to performance test

- **Before launch** — establish baseline, verify targets
- **Before expected traffic increase** — marketing campaign, feature launch
- **After significant architecture change** — new caching, DB change, etc.
- **Periodically** — quarterly, catch gradual regressions
- **After performance complaints** — verify the fix

Not every project needs heavy performance testing. A low-traffic internal tool — basic check suffices. A public-facing app expecting growth — invest properly.

## Anti-patterns

- **No performance targets.** Can't pass/fail without defined targets.
- **Testing on toy environment.** Tiny instance, empty DB. Results meaningless for production.
- **Averages only.** p50 looks great, p99 is catastrophic, you don't see it.
- **Unrealistic load.** Hammering one endpoint with one record. Caching skews, not realistic.
- **Load testing production.** DoS yourself, anger real users.
- **No system monitoring.** See latency rise but not WHY (CPU? memory? DB?).
- **One-time test.** Tested at launch, never again. Gradual regressions accumulate.
- **Ignoring soak.** Memory leak only visible over hours. Short test misses it.
- **No failure mode observation.** Know the breaking point number but not how it breaks.
- **Premature performance testing.** Heavy load testing a prototype with 3 users. Wrong phase.
- **Ignoring AI cost under load.** Load test passes, then the bill arrives.

## Integration

- Verifies `performance-optimization` (Dev) targets are met
- `test-strategy-design` includes performance testing for relevant projects
- `observability-setup` (Dev) provides the monitoring during tests
- `failure-modes-mapping` — resource/load failures are mapped
- `chaos-engineering` — related: deliberately inducing failure
- `TestRun` records with `testType: performance`
- Findings → `DefectRecord` if targets missed
