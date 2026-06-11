# ADR-0054: Tool Abstraction — External Services as Language Constructs

**Date**: 2025-06-11
**Status**: Implemented
**Related**: ADR-0045 (Hooks), ADR-0052 (Event Stream), ADR-0048 (Model Routing)

## Context

Metalogos programs interact with external services (Telegram, GitHub, databases, REST APIs) via patterns. However, as the number of integrations grows, flat patterns lead to namespace pollution and poor organization. There is no linguistic mechanism to group related operations under a single namespace, making it difficult to understand which patterns belong to which external service.

Industry prior art provides strong patterns for this problem:
- **OpenHands ACI** (Agent-Computer Interface) defines tool schemas with typed parameters and return types
- **MCP (Model Context Protocol)** by Anthropic exposes tools as structured JSON schemas with namespace isolation
- **LangChain Tools** groups related operations under `@tool` decorators with type-safe parameter binding

## Decision

### New Construct: `tool`

Introduce a `tool` declaration that groups related operations (methods) under a named namespace:

```
tool telegram {
  send(chat_id: String, text: String) -> String {
    let token = env("TELEGRAM_BOT_TOKEN")
    let url = "https://api.telegram.org/bot" + token + "/sendMessage"
    let body = "{\"chat_id\":" + chat_id + ",\"text\":\"" + replace(text, "\"", "\\\"") + "\"}"
    return http_post(url, body, "application/json")
  }

  get_updates(offset: String) -> String {
    let token = env("TELEGRAM_BOT_TOKEN")
    return http_get("https://api.telegram.org/bot" + token + "/getUpdates?offset=" + offset)
  }
}
```

### Invocation Syntax

Tool methods are invoked via qualified calls, identical to module.pattern() syntax:

```
telegram.send("12345", "Hello from Metalogos!")
let updates = telegram.get_updates("0")
```

### Grammar

```
tool_decl     = { TOOL_KW ~ IDENT ~ "{" ~ tool_method* ~ "}" }
tool_method  = { IDENT ~ "(" ~ params? ~ ")" ~ ARROW ~ type_name ~ "{" ~ statement* ~ "}" }
```

### AST

```rust
pub struct ToolDecl {
    pub name: String,
    pub methods: Vec<ToolMethod>,
}

pub struct ToolMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Vec<Statement>,
}
```

### Implementation Strategy

Tool methods are compiled as regular patterns with qualified keys (`toolname.methodname`) in the interpreter's pattern registry. The tool name is registered as a module namespace (`module_namespaces["telegram"] = "tool:telegram"`), enabling resolution via the existing `QualifiedCall` expression handler.

The QualifiedCall handler distinguishes tool namespaces from import namespaces by checking the namespace prefix: tool namespaces start with `"tool:"` and use the qualified key for pattern lookup, while import namespaces use the plain function name (patterns are merged flat during import).

This design ensures:
1. **Namespace isolation**: Two tools can define methods with the same name without collision (e.g., `calc_a.compute()` vs `calc_b.compute()`)
2. **Code reuse**: Tool methods can call regular patterns and other tool methods via QualifiedCall
3. **Minimal runtime cost**: No new dispatch mechanism — reuses the existing QualifiedCall → pattern invocation path

### Namespace Collision Prevention

Tool methods are stored with qualified keys (`telegram.send`, `math_api.double`) in the `self.patterns` HashMap. When two tools define methods with the same name (e.g., `tool_a.compute` and `tool_b.compute`), they coexist without collision because their keys are fully qualified.

### Internal Method Calls

Within a tool method body, calling another method of the same tool requires the qualified form:

```
tool api {
  base_url() -> String { return "https://example.com" }
  full_url(path: String) -> String {
    let base = api.base_url()  // Must use qualified call
    return base + path
  }
}
```

This is because tool methods are not registered under their unqualified names — only under the qualified key. Plain `FnCall("base_url", [])` would not find the pattern.

### Keyword Handling

The keyword `tool` is handled context-sensitively in the grammar. The `declaration` rule tries `tool_decl` before other rules via PEG ordering. The IDENT rule does NOT exclude `"tool"`, allowing identifiers like `toolbox`, `tool_method`, `tools` to remain valid. The `step_ident` rule does exclude `"tool"` to prevent using tool as a flow step name.

## Contract

```
tool math_api {
  double(n: Float) -> Float { return n + n }
  square(n: Float) -> Float { return n * n }
}
entity x: Float = 5.0
pattern Test(n: Float) -> String {
  let d = math_api.double(n)
  let s = math_api.square(n)
  return to_string(d) + " " + to_string(s)
}
flow Main { input: Float = x -> Test -> output }
Expected: "10 25"
```

## Test Coverage

9 contract tests in `tests/tool_abstraction_contract.rs`:

1. `test_tool_basic_double_square` — double/square math contract from spec
2. `test_tool_string_methods` — string concatenation in tool methods
3. `test_tool_namespace_isolation` — same method name in different tools, no collision
4. `test_tool_empty` — empty tool parses and runs without error
5. `test_tool_method_calls_pattern` — tool method invoking a regular pattern
6. `test_tool_multi_param_flow` — multi-parameter tool method in flow pipeline
7. `test_tool_in_flow_pipeline` — tool method used as intermediate flow step
8. `test_tool_undefined_error` — undefined tool produces "undefined module" error
9. `test_tool_three_methods` — three methods with inter-method calls

## Future Directions

- **OpenAPI code generation**: Auto-generate `tool` declarations from OpenAPI/Swagger specs
- **MCP protocol bridge**: Expose Metalogos tools as MCP tools for LLM agents
- **Type checking**: Semantic analysis to enforce return type correctness in tool methods
- **Async support**: Long-running tool methods with timeout and retry policies
