---
name: api-design
description: REST API design principles for Fosved projects. URL structure, HTTP method semantics, status codes, request/response shapes, versioning strategy, error handling, authentication patterns, pagination. APIs are contracts — once shipped, breaking them is expensive. Design with future in mind.
---

# API Design — Contracts That Last

An API is a promise to consumers. Once apps depend on it, changes break them. Design well now to avoid breaking changes later.

## Prerequisites

- Resource model defined (entities, relationships)
- Consumer profile known (mobile app, web app, third-party integrations)
- Authentication strategy decided

## Core principle

> Design APIs from the consumer's perspective, not the database's. The API hides the schema and exposes the business model. If a consumer needs to make 5 calls to get one logical thing, the design is broken. If a consumer needs to know your tables, the design is broken.

## URL structure

**Resources are nouns. Methods are verbs.**

```
GET    /users              # list users
POST   /users              # create user
GET    /users/123          # get one user
PATCH  /users/123          # partial update
PUT    /users/123          # full replace
DELETE /users/123          # delete

GET    /users/123/posts    # nested resource
POST   /users/123/posts    # create post for user
```

**Plural nouns** for collections — `/users` not `/user`. Consistency.

**Lowercase, kebab-case** — `/blog-posts` not `/blogPosts` or `/blog_posts`.

**Limit nesting to 2 levels.** Deep nesting becomes hard to navigate:
- ✓ `/users/123/posts`
- ✗ `/users/123/posts/456/comments/789` — too deep, prefer `/comments/789`

For deeply nested: provide top-level resource with filter — `/comments?postId=456`.

## HTTP methods semantics

| Method | Purpose | Idempotent | Safe |
|--------|---------|------------|------|
| GET | Read | Yes | Yes |
| POST | Create | No | No |
| PUT | Replace | Yes | No |
| PATCH | Partial update | Should be | No |
| DELETE | Remove | Yes | No |

**Idempotent** = calling N times has same effect as 1 time. Important for retry logic.

**Safe** = doesn't modify server state. GET should never have side effects.

Common violations:
- ✗ GET /users/delete/123 — uses GET for deletion (NOT safe)
- ✗ POST /users/123/activate — should be PATCH or PUT
- ✓ DELETE /users/123 — explicit and correct

## Status codes

**Success (2xx):**
- `200 OK` — successful read or update
- `201 Created` — successful creation (return new resource)
- `204 No Content` — successful operation, nothing to return (delete)

**Redirect (3xx):**
- `301 Moved Permanently` — API endpoint relocated
- `304 Not Modified` — caching

**Client error (4xx):**
- `400 Bad Request` — invalid syntax/data
- `401 Unauthorized` — auth missing or invalid
- `403 Forbidden` — auth ok but no permission
- `404 Not Found` — resource doesn't exist
- `409 Conflict` — request conflicts with state (duplicate, etc.)
- `422 Unprocessable Entity` — valid syntax but invalid semantics
- `429 Too Many Requests` — rate limited

**Server error (5xx):**
- `500 Internal Server Error` — generic server failure
- `502 Bad Gateway` — upstream service failed
- `503 Service Unavailable` — server overloaded or maintenance

**Don't invent codes.** Use standard semantics.

**Difference between 400 and 422:**
- 400: "I can't parse this" (malformed JSON, missing required field)
- 422: "I understand but it's wrong" (validation failures, business rule violations)

## Request and response shapes

**Use JSON.** XML is dead for new APIs.

**Consistent envelope (optional but recommended):**

```json
// Success
{
  "data": { "id": 123, "name": "..." }
}

// List
{
  "data": [
    { "id": 1, ... },
    { "id": 2, ... }
  ],
  "pagination": {
    "page": 1,
    "perPage": 20,
    "total": 145,
    "hasMore": true
  }
}

// Error
{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Email already in use",
    "details": [
      { "field": "email", "issue": "duplicate" }
    ]
  }
}
```

Why envelope: easier to extend (add metadata) without breaking changes.

**Camel case in JSON:** `firstName` not `first_name` (matches JS conventions).

## Pagination

For any list that could grow:

**Page-based (simple, default):**
```
GET /posts?page=1&perPage=20

Response includes:
{ "data": [...], "pagination": { "page": 1, "perPage": 20, "total": 145, "hasMore": true } }
```

**Cursor-based (for real-time data):**
```
GET /posts?cursor=abc123&limit=20

Response:
{ "data": [...], "nextCursor": "def456" }
```

Use cursor for:
- Real-time feeds (Twitter-like)
- Very large datasets
- When skipping ahead doesn't make sense

Use page-based for:
- UI with page numbers
- Small to medium datasets
- When stable ordering exists

**Default limit:** 20-50. Maximum 100 (cap it).

## Filtering, sorting, searching

**Filtering** via query params:
```
GET /posts?status=published&author=123
```

**Sorting:**
```
GET /posts?sort=-createdAt,title   # - prefix for descending
```

**Searching:**
```
GET /posts?q=javascript            # simple search
GET /posts?title:contains=React    # field-specific
```

For complex queries (multiple AND/OR): consider GraphQL or POST /search endpoint.

## Versioning

**Two approaches:**

**URL versioning:**
```
GET /v1/users
GET /v2/users
```

**Header versioning:**
```
GET /users
Accept-Version: v2
```

URL is simpler, header is "purer" REST. For most projects, URL versioning is fine.

**Version when needed, not preemptively.** v1 for first release. Bump to v2 when breaking change required.

**Maintain old versions for transition period** (3-12 months typically). Announce deprecation in advance.

**Non-breaking changes don't need version bump:**
- Adding new endpoints
- Adding optional query params
- Adding optional response fields
- Adding new optional request body fields

**Breaking changes need version bump:**
- Removing endpoint, field, or param
- Renaming
- Changing type of existing field
- Changing required vs optional
- Changing default behavior

## Authentication patterns

**For Fosved internal/owner APIs:**
- Telegram WebApp initData verification (existing pattern in fosved-miniapp)
- API key in header for service-to-service

**For public APIs:**
- OAuth 2.0 with PKCE for user-authenticated
- API keys for B2B with rate limiting per key
- JWT for stateless distributed systems

**Header conventions:**
```
Authorization: Bearer <token>
X-API-Key: <key>
```

Never put auth in URL params (logged everywhere). Always headers.

## Idempotency for critical operations

Payment, order creation, etc.:

```
POST /orders
Idempotency-Key: unique-client-generated-string-123

Response on retry with same key: same result, no duplicate order.
```

Server stores `(idempotency_key, response)` pairs for X hours. Replay returns cached response.

## Rate limiting

For any public-facing API:

Response headers:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 73
X-RateLimit-Reset: 1620000000
```

When exceeded: 429 with retry-after.

Strategy:
- Per IP for unauthenticated
- Per API key for authenticated
- Different limits per endpoint (cheap reads vs expensive writes)

## Error response standard

Every error response:
```json
{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Human-readable explanation",
    "details": [{ "field": "email", "issue": "invalid_format" }],
    "requestId": "abc-123-def"
  }
}
```

**Codes are machine-readable strings** — `VALIDATION_FAILED`, `RESOURCE_NOT_FOUND`, `RATE_LIMITED`. Stable across versions.

**Messages are human-readable** — for logging and debugging.

**Details** for field-level validation errors.

**requestId** for support: user reports issue with requestId, support can find logs.

## API documentation

**OpenAPI (Swagger) spec** for every API:
- Auto-generates docs
- Enables client generation
- Provides contract for testing
- Discoverable at `/docs` endpoint

For Next.js APIs: use `next-swagger-doc` or generate manually.

Document:
- Every endpoint
- All parameters with types
- Request/response examples
- Error codes
- Authentication requirements
- Rate limits

## Anti-patterns

- **CRUD-only APIs.** APIs that just mirror tables. Should expose business operations: `POST /orders/123/cancel` not `PATCH /orders/123 {status: 'cancelled'}`.
- **GET with body.** Not all clients support. Use query params.
- **Verbs in URLs.** `/getUsers`, `/createOrder` — use HTTP methods.
- **Inconsistent naming.** `/Users` vs `/blog_posts` vs `/comments`. Pick one.
- **No status codes.** Returning 200 with `{ success: false }`. Use 4xx/5xx properly.
- **Vague errors.** `{"error": "something went wrong"}`. Be specific.
- **No pagination.** Returning all 50,000 records. Crashes on large data.
- **Plaintext IDs as auth.** `/admin/users?key=abc123` — easy to leak.
- **Sync-style for async ops.** Returning 200 for "started" — use 202 Accepted + polling endpoint.
- **Mixing API and webhook conventions.** API is request-response. Webhooks are server-initiated.
- **Breaking without versioning.** Removing endpoint, every existing client breaks.

## API design checklist

Before shipping:
- [ ] URL structure consistent (plurals, lowercase, kebab-case)
- [ ] HTTP methods used correctly
- [ ] Status codes per standard
- [ ] Pagination implemented for lists
- [ ] Error responses follow standard
- [ ] Authentication explicit
- [ ] Rate limiting in place if public
- [ ] OpenAPI spec generated
- [ ] Versioning strategy documented
- [ ] Idempotency for critical writes
- [ ] CORS configured if browser clients
- [ ] requestId in errors for traceability

## Integration

- `nextjs-architecture` defines where API routes live (`src/app/api/`)
- `database-design` provides entities API exposes
- `dev-handoff-specs` includes OpenAPI for design-to-dev handoff
- `qa/integration-testing-patterns` tests API contracts
- `observability-setup` logs requestId for debugging
