---
name: security-hardening
description: Defense-in-depth security practices for production systems. Authentication, authorization, secret management, input validation, OWASP top 10 mitigations, supply chain security. Applied to every production deploy, not just "security-critical" projects. Security failures are catastrophic and irreversible (data breach, credential leak, ransomware) — the discipline prevents them, not detection after.
---

# Security Hardening — Defense in Depth, Always

Security in software has an asymmetric risk profile: 99 secure releases plus 1 breach equals catastrophe. The discipline is not "make it secure" but "remove ways it can be insecure". Each layer of defense reduces blast radius when others fail.

This skill applies to every production-bound project. Not just "the ones with sensitive data" — every project, because every project might gain sensitive data later.

## Prerequisites

- Project in security-relevant context (production deployment, user data, authentication)
- `dependency-management` understood
- `qa/security-testing-protocol` available

## Core principle

> Assume the attacker is inside the perimeter. Assume the dependency is compromised. Assume the user is hostile. Build the system to remain safe under those assumptions. This is paranoid by design — but the alternative is catastrophic by default.

## The security layers

Production systems need ALL of these. Each layer reduces risk when others fail.

### Layer 1: Authentication (who are you?)

Identity verification.

**Bare minimum for any system with users:**
- Passwords: bcrypt/argon2 hashing (NEVER plain text, NEVER MD5/SHA1)
- Minimum password length: 12 characters
- Rate limiting on login attempts (5 fails → 5 min lockout)
- HTTPS only — never plain HTTP for auth

**Better practice:**
- OAuth/OIDC instead of password (delegate to Google, GitHub, etc.)
- 2FA for sensitive accounts
- Session tokens with secure cookies (httpOnly, secure, sameSite=strict)
- Session expiration (e.g., 30 days absolute, 24h idle)

**For Fosved Office:**
- Single-user setup: hardcoded OWNER_TELEGRAM_ID check, Telegram WebApp initData verification (HMAC signature)
- Token never stored client-side except via initData verification

### Layer 2: Authorization (what can you do?)

Permission enforcement.

**Principle of least privilege:**
- Each user/service has minimum permissions needed
- Default deny — explicit grant per resource
- Roles or permissions, never "admin: true" boolean

**For each API endpoint:**
- Who can access? (authentication check)
- Can they access THIS resource specifically? (ownership check)
- What operations? (read vs write vs delete)

```javascript
// Wrong (broken access control)
app.get('/api/users/:id', (req, res) => {
  const user = await db.user.findUnique({ where: { id: req.params.id } });
  res.json(user);
});

// Right (ownership check)
app.get('/api/users/:id', authMiddleware, (req, res) => {
  if (req.user.id !== req.params.id && !req.user.isAdmin) {
    return res.status(403).json({ error: 'Forbidden' });
  }
  const user = await db.user.findUnique({ where: { id: req.params.id } });
  res.json(user);
});
```

### Layer 3: Input validation (data hostility)

Every external input is hostile until validated.

**Validate at the boundary:**
- API endpoints: schema validation on body, query, params
- Form submissions: validate everything
- File uploads: size, type, content checks

```javascript
import { z } from 'zod';

const CreatePostSchema = z.object({
  title: z.string().min(1).max(200),
  body: z.string().min(1).max(10000),
  tags: z.array(z.string()).max(10)
});

app.post('/api/posts', authMiddleware, (req, res) => {
  const parsed = CreatePostSchema.safeParse(req.body);
  if (!parsed.success) {
    return res.status(400).json({ errors: parsed.error.errors });
  }
  // Now parsed.data is safe to use
});
```

**SQL injection prevention:**
- Use parameterized queries (Prisma does this by default)
- NEVER string concatenate user input into SQL
- ORMs help but don't make you immune

**XSS prevention:**
- Escape HTML output by default (React does this)
- Never `dangerouslySetInnerHTML` with user content
- Content Security Policy (CSP) header

**Command injection prevention:**
- Never `exec()` with user input
- If shell needed, use parameterized commands (`execFile` not `exec`)

### Layer 4: Secret management

Credentials must never be in code/repo.

**Never commit:**
- API keys
- Database passwords
- Encryption keys
- JWT signing secrets
- OAuth client secrets

**Storage:**
- Development: `.env` file (gitignored)
- Production: environment variables (Render env, Vercel env, etc.)
- Better: secret manager (AWS Secrets Manager, HashiCorp Vault) — for serious systems

**Detection:**
- `gitleaks` or `truffleHog` in CI — fails build if secret detected
- GitHub native secret scanning (free for public repos)
- Pre-commit hook to scan staged files

**If leaked:**
- Rotate immediately (don't think "no one saw it")
- Audit all systems using the leaked secret
- Document in incident log

### Layer 5: HTTPS / TLS

Encryption in transit.

**Everywhere:**
- All endpoints HTTPS (no HTTP redirect after — block plain HTTP)
- HSTS header to prevent downgrade
- Modern TLS (1.2+, 1.3 preferred)
- Strong cipher suites

**Free TLS certs:**
- Let's Encrypt (automated)
- Render/Vercel/Cloudflare provide automatic TLS

### Layer 6: Dependency security

External code is risk surface.

- Weekly `npm audit` (or quarterly minimum)
- Auto-update patch versions (Dependabot)
- Review major updates with eye to security implications
- Avoid abandoned packages
- See `dependency-management` skill for full discipline

### Layer 7: Logging without leaking

Logs are valuable for debugging but dangerous if they leak.

**Never log:**
- Passwords (obvious but happens)
- API keys
- Full credit card numbers
- Personal identifying information beyond minimal
- Session tokens

**Log redaction:**
```javascript
function redactSensitive(obj) {
  const SENSITIVE_KEYS = ['password', 'token', 'apiKey', 'creditCard', 'ssn'];
  // recursively redact matching keys
  return JSON.parse(JSON.stringify(obj, (key, value) => {
    if (SENSITIVE_KEYS.some(s => key.toLowerCase().includes(s))) return '[REDACTED]';
    return value;
  }));
}
```

### Layer 8: Rate limiting & DoS protection

Throttle abusive traffic.

- API rate limiting (e.g., 100 req/min per IP)
- Login attempt throttling
- Expensive operations rate-limited (DB-heavy queries, AI calls)

```javascript
import rateLimit from 'express-rate-limit';

const apiLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 100,
  message: 'Too many requests'
});
app.use('/api', apiLimiter);
```

For LLM-based apps, rate limit also AI token consumption (cost protection).

### Layer 9: Security headers

HTTP headers that reduce attack surface.

```javascript
// Using helmet middleware
import helmet from 'helmet';
app.use(helmet({
  contentSecurityPolicy: { ... },
  hsts: { maxAge: 31536000, includeSubDomains: true, preload: true }
}));
```

Key headers:
- `Strict-Transport-Security` (HSTS)
- `Content-Security-Policy` (CSP)
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY` (anti-clickjacking)
- `Referrer-Policy: strict-origin-when-cross-origin`

### Layer 10: AI-specific security (for LLM apps)

Unique attack surfaces:

**Prompt injection** — user input manipulates system prompt
- Validate user input before incorporating in prompts
- Use clear delimiters between system instructions and user data
- Never let user input determine tool usage decisions directly

**Indirect prompt injection** — content fetched from web/files contains hostile prompts
- Treat fetched content as untrusted data, not instructions
- Sandbox LLM tool execution

**Data exfiltration via prompts** — LLM tricked into revealing system info
- System prompts should not contain secrets
- Don't expose internal data unnecessarily

**Cost-based attacks** — repeated expensive requests
- Rate limit AI calls per user
- Budget alerts on unusual usage

## OWASP Top 10 quick reference

The standard threat model. Each release should be checked:

1. **Broken Access Control** — verify ownership/permissions checks
2. **Cryptographic Failures** — proper hashing, TLS, secret management
3. **Injection** — parameterized queries, input validation, output escaping
4. **Insecure Design** — threat-model before building
5. **Security Misconfiguration** — defaults secure, headers set, debug off
6. **Vulnerable Components** — dependency audit current
7. **Identification & Authentication Failures** — strong auth, session handling
8. **Software & Data Integrity** — signed updates, CI security
9. **Security Logging & Monitoring** — detect attacks in progress
10. **Server-Side Request Forgery (SSRF)** — validate URLs, deny internal IPs

## OWASP Top 10 for LLM Applications

Separate list, focused on AI:

1. **Prompt Injection** — direct or indirect
2. **Insecure Output Handling** — LLM output as code
3. **Training Data Poisoning** (for fine-tuned models)
4. **Model Denial of Service** — expensive operations
5. **Supply Chain** — model files, plugins
6. **Sensitive Information Disclosure** — model leaks training data
7. **Insecure Plugin Design** — tool/plugin security
8. **Excessive Agency** — agent does too much
9. **Overreliance** — trusting LLM output without verification
10. **Model Theft** — protecting custom models

## Security checklist before production deploy

- [ ] All secrets in env vars, not code
- [ ] HTTPS enforced
- [ ] All API endpoints have auth + authz checks
- [ ] Input validation on all inputs
- [ ] Output escaping on all user content
- [ ] Rate limiting on auth endpoints
- [ ] `npm audit` clean (no high+ vulnerabilities)
- [ ] Security headers configured (helmet)
- [ ] Logging doesn't include sensitive data
- [ ] CSP configured
- [ ] No debug routes in production
- [ ] Error messages don't leak stack traces to users
- [ ] If AI: prompt injection mitigations in place

## Anti-patterns

- **"Security comes later"** — retrofitting security is dramatically harder than building in.
- **Hardcoded secrets, even temporarily.** Git history is forever. Commit a secret once, rotate forever.
- **Custom crypto.** Use proven libraries. Don't invent your own hashing/encryption.
- **Permissive defaults.** "Open everything, lock down later" — usually never happens. Default deny.
- **"It's behind a firewall"** — assume firewall is breached. Defense in depth.
- **Logging everything.** Excessive logs leak data and become themselves attack surface.
- **No threat modeling.** Building features without thinking through abuse.
- **Trust user input.** Even "from your own app" — could be MITM, replay, manipulated.

## Integration with other skills

- `dependency-management` — supply chain security
- `qa/security-testing-protocol` — security testing process
- `devops-deployment` — secrets management in CI/CD
- `architecture-decision-records` — security decisions documented
- `observability-setup` — security event logging
- Used during every `iterative-implementation` workflow
