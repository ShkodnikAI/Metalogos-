# Syntax Reference

Complete reference for METALOGOS surface syntax as implemented in v0.7.7.

## Comments

```mlog
// single-line comment
```

## Declarations

Every `.mlog` file is a sequence of top-level declarations (24 types).

### Entity (simple)

```mlog
entity name: Type = value
```

Declares a named value of a given type.

```mlog
entity greeting: String = "Hello"
entity score: Float = 42.5
entity api_key: Secret = env("API_KEY")
```

### Entity Type

```mlog
entity TypeName { field: Type, field: Type = default }
```

Defines a structured type schema with named fields.

```mlog
entity Message {
  text: String
  urgency: Float
}
```

### Entity Record

```mlog
entity name: TypeName = { field: value, ... }
```

Creates an instance of an entity type.

```mlog
entity msg: Message = { text: "Help!", urgency: 0.9 }
```

### Pattern

```mlog
pattern Name(param: Type, ...) -> ReturnType {
  // body with statements
  return expr
}
```

A pure function. Parameters are positional and typed.

```mlog
pattern Shout(s: String) -> String { return upper(s) + "!" }
pattern Add(a: Float, b: Float) -> Float { return a + b }
```

### Learnable Pattern

```mlog
learnable pattern Name(param: Type, ...) -> ReturnType {
  prompt: "instruction for LLM"
}
```

A pattern backed by a language model.

```mlog
learnable pattern Classify(text: String) -> String {
  prompt: "Classify: question | complaint | greeting"
}
```

### Flow

```mlog
flow Name { input: Type = source -> step1 -> step2 -> output }
```

A data pipeline. Data flows left-to-right through pattern invocations.

```mlog
flow Main { input: String = greeting -> Shout -> output }
```

### Flow with Branching

```mlog
flow Name {
  input: Type = source -> Triage -> output
  Triage {
    label1 (condition) -> TargetPattern
    label2 (condition) -> TargetPattern
  }
}
```

### Fluid

```mlog
fluid name = Type1[value1][confidence1] or Type2[value2][confidence2]
```

A superposition of typed variants with confidence scores.

```mlog
fluid x = Float[42.0][0.9] or String["answer"][0.1]
```

### Rule

```mlog
rule If(target.field op value) then target.field = new_value
rule If(condition) then target.field = value with priority=N
```

### Memory

```mlog
memorize "fact" with priority=0.9
forget "query" after N.days
```

### Adaptation

```mlog
adapt PatternName add_example("input", "output")
```

### Mutation

```mlog
mutate PatternName {
  add_example("input", "output")
  rollback_if: accuracy op threshold
}
```

### Sandbox

```mlog
sandbox name {
  allowed: [capability, ...]
  forbidden: [capability, ...]
  timeout: N
}
```

### Import

```mlog
import std/module
import std/string as str
```

### Relate

```mlog
relate "from" to "to" as "relation"
```

### Server (HTTP)

```mlog
server {
  port: 8080
  route "/api/items" method=GET { ... }
  route "/api/items" method=POST requires=[admin] { ... }
}
```

### Template

```mlog
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>
}
```

### Tool

```mlog
tool telegram {
  send(chat_id: String, text: String) -> String { ... }
}
```

### Hook

```mlog
hook before_pattern { print("calling: " + pattern_name) }
hook after_pattern { print("result: " + to_string(result)) }
```

### Eval

```mlog
eval Classify {
  dataset: [
    { input: "hello", expected: "greeting" }
  ]
  threshold: 0.8
}
```

### Conversation

```mlog
conversation {
  ttl: 1800
  max_messages: 50
  compress_after: 20
}
```

### LLM Config

```mlog
llm {
  providers: [
    { name: "openai", model: "gpt-4", api_key: env("OPENAI_KEY") }
  ]
  default: "openai"
  timeout: 30
}
```

## Statements

### Let Binding

```mlog
let x = 42.0
let name = if x > 10.0 then "big" else "small"
```

### Assignment

```mlog
x = 10.0
```

### Each Loop

```mlog
each item in items {
  print(item)
}
```

### Each With Index

```mlog
each i, item in items {
  print(to_string(i) + ": " + item)
}
```

### While Loop

```mlog
while count < 10.0 {
  let count = count + 1.0
}
```

### Break / Continue

```mlog
each item in items {
  if item == "stop" then { break }
  if item == "skip" then { continue }
  print(item)
}
```

### If-Else Block

```mlog
if x > 10.0 {
  print("big")
} else if x > 5.0 {
  print("medium")
} else {
  print("small")
}
```

### If-Then-Else Expression

```mlog
let label = if score >= 90.0 then "A" else "B"
```

### Match

```mlog
match command {
  "start" then { print("starting") }
  starts_with "stop" then { print("stopping") }
  contains "help" then { print("helping") }
  > 100.0 then { print("too big") }
  else { print("unknown") }
}
```

### Return

```mlog
return result
```

### Try

```mlog
let result = try risky_operation()
```

## Expressions

| Expression | Example | Description |
|-----------|---------|-------------|
| String literal | `"hello"` | Double-quoted string, supports `\" \\ \n \t \r` |
| Float literal | `42.0`, `3.14`, `-1.0` | All numbers are Float |
| Bool literal | `true`, `false` | Boolean values |
| Identifier | `greeting` | Reference to a variable |
| Field access | `msg.text` | Access struct field |
| Function call | `upper(s)` | Invoke a pattern or builtin |
| Qualified call | `str.trim(s)` | Module-qualified call |
| Binary op | `a + b`, `a == b` | Arithmetic, comparison |
| Unary minus | `-x` | Negation |
| Index access | `list[0]` | List index |
| If-else | `if c then a else b` | Ternary expression |
| List literal | `[1.0, 2.0, "three"]` | Heterogeneous list |
| Struct literal | `{ name: "Alice", age: 25.0 }` | Named fields |
| Block if/else | `if c { ... } else { ... }` | Block if as expression |
| Try | `try expr` | Error-catching expression |

## Data Types

| Type | Description |
|------|-------------|
| `String` | UTF-8 string |
| `Float` | All numbers (no separate Int) |
| `Bool` | `true` / `false` |
| `List` | Heterogeneous list |
| `Struct` | Named fields |
| `Html` | Opaque: auto-escaped, XSS-safe |
| `Query` | Opaque: parameterized SQL |
| `Secret` | Opaque: cannot be printed |
| `Encrypted` | Opaque: encrypted data |
| `Hash` | Opaque: password hashes |
| `Session` | Opaque: session token |
| `Unit` | Null/void |
| `Fluid` | Probabilistic superposition |

## File Conventions

- Source files: `.mlog`
- Bytecode files: `.mbc`
- Project manifest: `mlog.toml`
- Lock file: `mlog.lock`
- Standard library: `std/*.mlog`