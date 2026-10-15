# Topic 4 — Routes and the HTTP egress

**Status: normative (the №645 protocol).** Every norm below was probed
on BOTH backends before it was written; the conformance pairs prove the
record on every CI run. Where the probes found a cross-backend
divergence, there is NO norm — the finding lives in «Honest limits» and
needs its own repair naryad. This topic's probe found NO divergence:
the route surface is shared-code parity by construction (the semantic
checks and the respond contracts run before any backend splits).

The route DECLARATION grammar, the middleware semantics (the CSRF
double-submit, the session store, the rate limiter) and the server keys
are documented in [REFERENCE §server](../REFERENCE.md) — this topic
records the NORMATIVE behavior contracts only, never re-states the
prose.

### S-RT-001 — The route declaration vocabulary

A server declaration (`server`/`mlogserver` — synonyms) carries
`port`, `host`, `middleware` (a list of the five names: `session`,
`csrf`, `security_headers`, `rate_limit`, `cors`), optional
`rate_limit` and `redact_mode`, and one or more `route "path"
method=M [requires=[role…]] { body }` declarations. The method set is
the seven HTTP methods (`GET`, `POST`, `PUT`, `DELETE`, `PATCH`,
`HEAD`, `OPTIONS`); anything else is a check error (a plain check
message — no stable code; the vocabulary is pinned by the accepted set
of the conformance pair). The `requires` role names collect for the
role cross-reference.
**Anchors:** `VALID_METHODS`/`VALID_MIDDLEWARE` and the route/middleware
validation (src/semantic.rs:1336–1337, the server-decl walk at
src/semantic.rs:6888+); the declaration shapes
`MlogServerDecl`/`RouteDecl` (src/ast.rs:552–577).
**Conformance:** `tests/conformance/srt_001_route_vocabulary_ok.mlog`.

### S-RT-002 — The route body's DIRECT respond* answers early

A route body's DIRECT `respond*` statement is the early answer (the
TW serve lane is the etalon; the VM lowers the same shape); so are the
DIRECT statements of a top-level block-form if/else branch in the
route body. The `return respond(...)` form remains the recommended
explicit spelling (the one whose behavior does not depend on the
statement form).
**Anchors:** `route_body_with_terminal_responds` (src/compiler.rs, the
№584 + №600 SSOT mode); `RespondPosition`/`check_route_respond_terminality`
(src/semantic.rs).
**Conformance:** `tests/conformance/srt_002_route_direct_respond_ok.mlog`.

### S-RT-003 — A respond* at depth 2 inside a block-form guard refuses

A `respond*` carried by a statement NESTED under another block-form if
inside the route body (the depth-2 guard-bypass shape — the audit's
session-guard scenario: the 403 guard INSIDE an outer guard) is the
BLOCKING `RESPOND_SWALLOWED` refusal at startup on BOTH backends
(№600/№601; the one-word migration `return respond(...)` of the same
route compiles and answers). The refusal is fail-closed BY DESIGN: the
serve lane would otherwise discard the nested value and continue past
the guard.
**Anchors:** the same SSOT as S-RT-002 (the depth rule inside
`route_body_with_terminal_responds`); the stable code
`CODE_RESPOND_SWALLOWED` (src/semantic.rs:67,159).
**Conformance:** `tests/conformance/srt_003_respond_swallowed_refused.mlog`.

### S-RT-004 — The raw template position is Html-only

A `render()` template parameter used in the raw `{{{ var }}}` position
must be declared `Html` AND built by an explicit Html producer; a
String-shaped argument (or an undeclared raw position) is the
`RAW_INSERT_NOT_HTML` refusal BEFORE execution on BOTH backends; the
escaped `{{ var }}` positions are unchanged. The raw positions CONDUCT
the taint/label instead of lifting it (the №685 sanitizer→conductor
split).
**Anchors:** the №685 declaration gate + the call-site coercion
(src/semantic.rs, src/builtins/http.rs — the render raw-scan; the
N9-actualization trail).
**Conformance:** the №685 pairs —
`tests/conformance/raw_insert_not_html_string_param.mlog` (the
refusal) and `tests/conformance/raw_insert_not_html_html_param_ok.mlog`
(the legal path) — referenced, not duplicated.

### S-RT-005 — The protective headers belong to the middleware

The three PROTECTIVE headers (`Content-Security-Policy`,
`X-Content-Type-Options`, `Strict-Transport-Security`) are
unconditional `security_headers`-middleware inserts on EVERY response;
a route cannot set them: a literal protective name through
`set_header` is the `HTTP_HEADER_NOT_ALLOWED` refusal (the TW stamps it
at the runtime builtin; the VM at the static lane — the CODE is the
pinned fact, the wording is not). The application header set is the
five-name allowlist (`Cache-Control`, `ETag`, `Content-Type`,
`Location`, `Content-Disposition`).
**Anchors:** `SET_HEADER_ALLOWLIST` (src/builtins/http.rs:352); the
static lane `check_set_header` (src/audit.rs:2664+); the N8
every-response inserts (src/metalogos-server — the middleware).
**Conformance:** `tests/conformance/srt_005_set_header_protective_refused.mlog`.

### S-RT-006 — The respond_html dual contract

`respond_html(body)` (the 1-arg office form) and `respond_html(title,
body)` (the 2-arg full-document form) are BOTH live on both backends;
the wire carries `text/html`. The explicit status form is
`respond_html_status(status, body)` — the HttpResponse struct shape the
`set_header` builtin consumes.
**Anchors:** the №673 contract restoration (src/builtins/http.rs — the
respond_html impls); the №565 status form (the registry row).
**Conformance:** `tests/conformance/srt_006_respond_html_forms_ok.mlog`.

## Honest limits (the probe findings this topic records WITHOUT norms)

The probe found NO cross-backend divergence — the four refusal/egress
contracts above are shared-code parity by construction (the semantic
checks and the builtin refusals run identically for both backends).
The by-design boundaries this topic operates UNDER are recorded where
they live, and are mirrored already:

- **№584/№600 — the respond-form boundary** (which statement forms
  answer early vs refuse): the full record lives in
  [limitations.md](../limitations.md) (the `respond*` form-specific
  row, the release-block carrier gh#1041) — the norms S-RT-002/003
  fixate the machine-checkable core of it.
- **№685 — the raw-insert contract**: the limitations row (the
  `{{{ var }}}` raw template row) and the threat-model entry carry the
  full two-contract record; S-RT-004 fixates the language-facing half.
- The word-based static lane marks route `respond` literals that carry
  secret-class words (a probe route answering a literal containing
  «secret» refuses with `PII_EGRESS_OUTPUT`) — the leak-suite
  vocabulary working as designed; the corpus-facing record lives in
  the №325 lane's tests, not in this topic (no divergence, no
  new edge).
