---
name: observability-setup
description: Instrumentation patterns for production services — structured logging, metrics, traces, alerts. The 3 pillars (logs, metrics, traces) practically applied. When production breaks, the difference between 5-minute fix and 5-hour fix is observability quality.
---

# Observability Setup — See What Production Is Doing

A black-box service is unfixable when it breaks. You see "something failed" but not what. Observability turns the black box into a glass box: structured data about every important event, queryable when investigating.

## Prerequisites

- Service deployed (real or staging)
- Errors will occur (they always do)
- Owner cares about uptime

## Core principle

> Don't add observability after something breaks — by then you're guessing what to add. Instrument from day one. Cost is small; value compounds with every incident.

## The three pillars

**Logs:** discrete events, contextual.
- "User 123 logged in at 14:32:01"
- "API call to Anthropic took 4.2s"
- "Database query failed: timeout"

**Metrics:** numeric measurements, aggregatable.
- Request rate (requests/sec)
- Error rate (errors/sec)
- Latency percentiles (p50, p95, p99)
- Resource usage (CPU, memory)

**Traces:** end-to-end request flow, hierarchical.
- Request A → entered HTTP handler → called DB (50ms) → called Anthropic (3200ms) → returned response

Use all three for production. Logs for debugging specifics, metrics for trends, traces for understanding flow.

## Structured logging

**Don't** log unstructured strings:
```typescript
// ✗ Bad
console.log(`User ${userId} did action ${action} at ${date}`);
// Difficult to parse, search, aggregate
```

**Do** log structured JSON:
```typescript
// ✓ Good
logger.info('user.action', {
  userId,
  action,
  timestamp: new Date().toISOString(),
  metadata: { ... }
});
```

Structured logs are queryable: "show me all `user.action` events for userId=123".

**Logger library:** `pino` (fast, JSON output) for Node.js.

```typescript
// lib/logger.ts
import pino from 'pino';

export const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  formatters: {
    level: (label) => ({ level: label })
  },
  timestamp: pino.stdTimeFunctions.isoTime
});

// Usage
logger.info({ userId, action: 'login' }, 'user.action.login');
logger.error({ err: error, requestId }, 'request.error');
```

## Log levels

- **fatal:** service crashing, requires immediate action
- **error:** error occurred, request failed, but service running
- **warn:** unexpected but recoverable
- **info:** normal operations of interest (login, deploy, scheduled job ran)
- **debug:** verbose details, off in production
- **trace:** extremely verbose, off normally

In production: `info` level. `debug`/`trace` only temporarily for investigation.

Don't log everything as `info`. Be intentional — info should be events you'd want to see in audit.

## What to log

**Always log:**
- Service start/stop
- Authentication events (login, logout, failed login)
- Authorization failures
- Errors (with stack traces)
- External API calls (provider, latency, outcome)
- Database errors
- Scheduled job execution (start, end, outcome)
- Deploy events

**Often log:**
- Significant business events (order created, etc.)
- Configuration loaded (with secrets redacted)
- Rate limit hits
- Cache hits/misses (sampled)

**Don't log:**
- Sensitive data (passwords, full payment info, PII)
- Every single request (too much noise) — use metrics instead
- Internal function calls

## Request IDs (correlation)

Every request gets unique ID, propagated through all logs for that request:

```typescript
// Middleware
import { randomUUID } from 'crypto';

app.use((req, res, next) => {
  req.requestId = req.headers['x-request-id'] || randomUUID();
  res.setHeader('x-request-id', req.requestId);
  next();
});

// In handler
logger.info({ requestId: req.requestId, ... }, 'handler.start');
```

When user reports issue: they share requestId, you grep logs.

For LLM-heavy systems: also `sessionId` for conversation correlation.

## Error handling discipline

Every error has full context:

```typescript
try {
  await doSomething();
} catch (error) {
  logger.error({
    err: error,
    stack: error.stack,
    requestId: req.requestId,
    userId: req.userId,
    operation: 'doSomething',
    context: { ... }
  }, 'operation.failed');
  
  // Decide: retry, fallback, fail
  throw error;  // or graceful response
}
```

Bare `console.error(error)` is useless. Add context.

## Metrics

For Node.js services, use `prom-client` (Prometheus format):

```typescript
import { register, Counter, Histogram, Gauge } from 'prom-client';

// Counter: monotonically increasing
const requestCount = new Counter({
  name: 'http_requests_total',
  help: 'Total HTTP requests',
  labelNames: ['method', 'route', 'status']
});

// Histogram: distribution of values
const requestDuration = new Histogram({
  name: 'http_request_duration_seconds',
  help: 'HTTP request duration',
  labelNames: ['method', 'route'],
  buckets: [0.01, 0.05, 0.1, 0.5, 1, 2, 5, 10]
});

// Gauge: current value (can go up/down)
const activeConnections = new Gauge({
  name: 'active_connections',
  help: 'Currently active connections'
});

// In handler
const end = requestDuration.startTimer({ method: 'GET', route: '/users' });
requestCount.inc({ method: 'GET', route: '/users', status: '200' });
end();
```

Expose `/metrics` endpoint for Prometheus scraping.

For Fosved-scale: this may be overkill initially. Start with logs, add metrics when needed.

## What to measure

**Service health:**
- Uptime
- Memory usage trend
- CPU usage
- Event loop lag (Node-specific)

**Per endpoint:**
- Request rate
- Latency (p50, p95, p99)
- Error rate
- Status code distribution

**Business metrics:**
- Active users
- Operations per second
- LLM tokens consumed
- Cost per operation

**LLM-specific:**
- Token usage per request
- Latency per provider/model
- Error rate per provider
- Fallback frequency

## External service monitoring

**Render:** built-in metrics dashboard. Memory, CPU, response times.

**Sentry:** error tracking. Auto-captures unhandled errors. Free tier sufficient for small projects.
```typescript
import * as Sentry from '@sentry/node';

Sentry.init({
  dsn: process.env.SENTRY_DSN,
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.1  // sample 10% for performance traces
});

// Then errors caught automatically
// Manual capture:
Sentry.captureException(error, { extra: { context } });
```

**Datadog / Grafana:** full observability stack. Overkill for small projects but standard for serious production.

**For Fosved current scale:** Render logs + Sentry for errors + custom DB tables for business metrics. Skip Datadog until justified.

## Alerts

Without alerts, monitoring is useless — you only notice problems after they bite.

**Alert on:**
- Error rate >5% over 5 minutes
- Response time p99 >2x baseline
- Service down (no successful health checks)
- Resource near limit (memory >90%, disk >85%)
- External dependency failing (DB unreachable, LLM provider down)

**Alert delivery:**
- Telegram (for personal projects, fits Fosved style)
- Email (slower, less intrusive)
- PagerDuty / Opsgenie (production-grade)

**Alert hygiene:**
- Don't alert on every error (noise)
- Don't alert on warnings (too sensitive)
- Each alert is actionable — if you can't act, don't alert
- Track alert-to-action ratio. If >5% noise, refine

## Health checks

Every service has `/health` endpoint:

```typescript
app.get('/health', async (req, res) => {
  const checks = await Promise.all([
    checkDatabase(),
    checkExternalAPI(),
    checkDiskSpace()
  ]);

  const healthy = checks.every(c => c.ok);
  res.status(healthy ? 200 : 503).json({
    status: healthy ? 'ok' : 'degraded',
    checks
  });
});
```

External monitor (Render, UptimeRobot) hits `/health` every minute. If failing → alert.

## LLM-specific observability

For Fosved-style AI offices:

**Track per LLM call:**
- Provider, model
- Input tokens, output tokens
- Latency
- Cost (computed)
- Context (which specialist, which task)
- Success/failure
- Retry count

Store in `LlmCall` table or similar:

```typescript
await prisma.llmCall.create({
  data: {
    requestId,
    provider: 'anthropic',
    model: 'claude-opus-4-7',
    inputTokens: usage.input_tokens,
    outputTokens: usage.output_tokens,
    costUsd: computeCost(usage),
    latencyMs: duration,
    context: 'osp_analysis',
    success: true
  }
});
```

Aggregate for monthly reports: total cost, per-specialist cost, per-model usage.

## Tracing (advanced)

OpenTelemetry for distributed traces:

```typescript
import { trace } from '@opentelemetry/api';

const tracer = trace.getTracer('fosved-bot');

async function handleRequest(req) {
  const span = tracer.startSpan('handleRequest');
  try {
    const result = await processRequest(req);
    span.setStatus({ code: SpanStatusCode.OK });
    return result;
  } catch (error) {
    span.recordException(error);
    span.setStatus({ code: SpanStatusCode.ERROR });
    throw error;
  } finally {
    span.end();
  }
}
```

Spans nest. See full request flow.

For Fosved: skip tracing initially. Logs + LLM tracking sufficient until system is more complex.

## Cost discipline

Observability has cost (storage, processing). Be intentional:

- Logs: sample non-critical events (10% of debug logs in prod)
- Metrics: cardinality matters — don't label by user ID if millions of users
- Traces: sample (don't trace every request, sample)
- Retention: 30 days typical, 90+ days expensive

## Anti-patterns

- **No logs at all.** Black box. When breaks, no idea.
- **Unstructured logs.** `console.log(message)` everywhere. Unsearchable.
- **Logging secrets.** Passwords or tokens in logs. Security incident.
- **Logging everything as error.** Real errors lost in noise.
- **No requestId.** Can't correlate across services.
- **Alerts you ignore.** "It always alerts, I tune it out." Refine or remove.
- **No alerts.** Find out about issues from users.
- **No health check.** Don't know when service down until users complain.
- **Logging without rotation.** Logs fill disk, service crashes.
- **Verbose logs in tight loops.** Performance impact.
- **Only logging on success.** Need errors too. Both sides.
- **No baseline.** Don't know what "normal" looks like. Can't detect anomalies.

## Minimal viable observability for Fosved bot

1. `pino` logger with JSON output (lib/logger.ts)
2. Request ID middleware
3. Sentry for error capture (free tier)
4. LLM call tracking in DB table
5. `/health` endpoint
6. UptimeRobot hitting health endpoint
7. Telegram alert on critical errors via existing bot

This is achievable in 1-2 days of work. Massive improvement over zero observability.

## Integration

- Used by every Fosved project that deploys
- `lib/logger.ts` shared across services
- `devops-deployment` ensures logging in deployed code
- `security-hardening` validates no secrets in logs
- `qa/integration-testing-patterns` validates health endpoint
- LLM cost tracking informs monthly review
