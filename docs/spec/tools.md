# Topic 6 — Tools and the abstraction

**Status: normative (the №645 protocol).** Every norm below was probed
on BOTH backends before it was written; the conformance pairs prove the
record on every CI run. This topic records the live tool-abstraction
contract landed by №684 (the VM tool-method resolution — the compiler
mirrors the interpreter's QualifiedCall dispatch order EXACTLY, the
parity postulate): the contract's 22 both-backend tests
(`tests/tool_abstraction_contract.rs`) are the probe source, and the
pairs below re-prove the language-facing facts through the production
paths on every CI run.

### S-TOOL-001 — A tool declaration exposes methods through the qualified call

`tool <name> { <method>... }` declares a namespace of named methods; a
qualified call `name.method(args)` invokes the method and yields its
return value, identically on BOTH backends.
**Anchors:** the compiler's tool registration + body compile
`Declaration::Tool` (src/compiler.rs:549 and :986 — methods compile
into the qualified `tool.method` pattern table, №684); the
interpreter's QualifiedCall tool lookup (src/interpreter/execution.rs,
the qualified-key `patterns.get("tool.method")` branch).
**Conformance:** `tests/conformance/stool_001_tool_basic_call.mlog`.

### S-TOOL-002 — The qualified-call resolution order: intercepts, then builtins (the shadowing), then the tool method

A qualified call on a tool namespace resolves STRICTLY in this order,
mirrored on BOTH backends (№684): (1) the runtime-op intercepts by
function name (`query`, `db_execute`, `db_execute_with_grant`,
`deny_event`, `deny_reason`, `inspect`); (2) the plain builtin table
by function name — a plain builtin SHADOWS a tool method of the same
name (the tool body is unreachable); (3) the tool's own method lookup;
(4) otherwise a LOUD refusal — never a silent fallthrough to flat
patterns or the global-builtin fallback.
**Anchors:** the compiler's mirror chain (src/compiler.rs:1306+ —
`builtin_indices` intercept/shadow check → `pattern_indices` qualified
lookup → the loud compile error); the interpreter's dispatch order
(src/interpreter/execution.rs:1806+ — the intercepts, the builtin
table, then the qualified tool lookup).
**Conformance:** `tests/conformance/stool_002_builtin_shadows_tool_method.mlog`
(the shadowing tier: the builtin runs, the tool body does not).

### S-TOOL-003 — Tool namespaces are isolated

Two tools declaring the SAME method name do not cross-talk: the
qualified call resolves inside the named tool's namespace only, on
BOTH backends.
**Anchors:** the qualified-key pattern table (`tool.method`) — the
compiler's `pattern_indices` (src/compiler.rs:986+ insert) and the
interpreter's `patterns.get(&qualified_key)`
(src/interpreter/execution.rs:1910+).
**Conformance:** `tests/conformance/stool_003_namespace_isolation.mlog`.

### S-TOOL-004 — A tool method may call a sibling method through the qualified form

Inside a tool body, a sibling method is callable by the qualified form
`tool.method(...)` (ADR-0054 «Internal Method Calls»), identically on
BOTH backends.
**Anchors:** the compiler compiles each method body into the same
qualified `tool.method` table (src/compiler.rs:986+, the
№684 shape), so the sibling call re-enters the S-TOOL-002 order; the
interpreter resolves it through the same namespace.
**Conformance:** `tests/conformance/stool_004_inter_method_calls.mlog`.

### S-TOOL-005 — A tool method may call a regular pattern

A tool method body can invoke a plain pattern by its name; the call
resolves and returns like any pattern call, on BOTH backends.
**Anchors:** the method-body compile compiles pattern calls like a
pattern body (src/compiler.rs:986+ — `compile_pattern_body_with_locals`,
the №684 contract); the interpreter's flat pattern lookup inside the
method invocation.
**Conformance:** `tests/conformance/stool_005_method_calls_pattern.mlog`.

### S-TOOL-006 — Method arguments bind positionally

A qualified call binds its arguments to the method's parameters by
position; the arity must match the declaration, and both backends
answer identically for a well-formed call.
**Anchors:** the compiler's arg emit for the QualifiedCall tool branch
(src/compiler.rs:1306+ — the `CallPattern`/`CallBuiltin` arg count,
the №684 arg-emit repair); the interpreter's positional zip of
`eval_args` onto `pattern.params` (src/interpreter/execution.rs:1925+).
**Conformance:** `tests/conformance/stool_006_multi_param_methods.mlog`.

### S-TOOL-007 — A tool call works as an intermediate flow step

A pattern that invokes a tool method composes into a flow pipeline as
an intermediate step; the value flows through the pipeline identically
on BOTH backends.
**Anchors:** the tool method call inside a pattern body is ordinary
expression compilation (src/compiler.rs:1306+; the flow pipeline
emission is value-agnostic); the interpreter evaluates it in the
pattern body the same way.
**Conformance:** `tests/conformance/stool_007_tool_in_flow_pipeline.mlog`.

## Honest limits (the probe findings this topic records WITHOUT norms)

- **The negative-path refusals carry NO stable code yet.** The unknown
  module (`undefined module: '...' (did you import it?)`,
  src/interpreter/execution.rs:1807; the compiler's namespace check)
  and the unknown method (`undefined method '...' in tool '...'`,
  src/interpreter/execution.rs:1912 / src/compiler.rs:1370) refuse
  LOUDLY with MIRRORED wording on both backends, but the messages
  carry no `[CODE]` stamp — and the conformance contract REQUIRES a
  stable code for an error pair. The refusal norms (unknown tool,
  unknown method, wrong arity) are therefore NOT recorded here:
  landing the stamps is the №385 additive vocabulary lane (a separate
  naryad — the spec references codes, it does not invent them).
  Recorded as a probe finding, flagged in the topic's landing PR.
- **The wrong-arity refusal of a tool method** is checked at runtime
  (the interpreter's positional zip; the VM mirrors at the call
  boundary) — the same codeless-refusal posture as above; the same
  lane would unlock its norm.
- **The empty tool body** compiles and runs on both backends
  (the contract's `test_tool_vm_empty`) — recorded here as a probe
  fact, not a norm: it carries no distinct language-facing guarantee
  beyond S-TOOL-001's declaration rule.
