# Syntax Reference

Complete reference for METALOGOS surface syntax as implemented in v0.3.0.

## Declarations

Every `.mlog` file is a sequence of top-level declarations.

### Entity (simple)

```mlog
entity name: Type = value
```

Declares a named value of a given type.

```mlog
entity greeting: String = "Hello"
entity score: Float = 42.5
entity count: Float = 10
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
pattern Name(param: Type, ...) -> ReturnType { return expr }
```

A pure function. Parameters are positional and typed. The body contains `return` statements.

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

A pattern backed by a language model. The `prompt` field guides the LLM's behavior.

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

Branch conditions compare a field to a threshold.

### Fluid

```mlog
fluid name = Type1[value1][confidence1] or Type2[value2][confidence2]
```

A superposition of typed variants with confidence scores. Collapses to the highest-confidence matching variant when used in a typed context.

```mlog
fluid x = Float[42.0][0.9] or String["answer"][0.1]
```

### Rule

```mlog
rule If(target.field op value) then target.field = new_value
rule If(condition) then target.field = value with priority=N
```

Conditional logic with optional priority for conflict resolution.

```mlog
rule If(msg.text contains "urgent") then msg.urgency = 0.9
rule If(score > 90) then grade = "A" with priority=10
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

Adds a training example to a learnable pattern.

### Mutation

```mlog
mutate PatternName {
  add_example("input", "output")
  rollback_if: accuracy op threshold
}
```

Adds examples with a safety rollback condition.

### Sandbox

```mlog
sandbox name {
  allowed: [capability, ...]
  forbidden: [capability, ...]
  timeout: N
}
```

Defines a safety sandbox for adaptive operations.

### Import

```mlog
import std/module
```

Loads a standard library module.

### Relate

```mlog
relate "from" to "to" as "relation"
```

Creates a knowledge graph edge.

## Expressions

| Expression | Example | Description |
|-----------|---------|-------------|
| String literal | `"hello"` | Double-quoted string |
| Float literal | `42.0`, `10` | Numeric value |
| Identifier | `greeting` | Reference to an entity |
| Field access | `msg.text` | Access entity field |
| Function call | `upper(s)` | Invoke a pattern or builtin |
| Binary op | `a + b`, `a - b`, `a * b`, `a / b` | Arithmetic |

## Built-in Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `upper(s)` | `String -> String` | Uppercase |
| `lower(s)` | `String -> String` | Lowercase |
| `len(s)` | `String -> Float` | String length |
| `str(x)` | `Any -> String` | Convert to string |
| `print(x)` | `Any -> ()` | Print to stdout |
| `contains(s, sub)` | `(String, String) -> Float` | Substring check (1.0/0.0) |
| `float(s)` | `String -> Float` | Parse to float |
| `confidence(x)` | `Any -> Float` | Get confidence value |

## Comparison Operators (in rules/branches)

| Operator | Meaning |
|----------|---------|
| `>` | Greater than |
| `<` | Less than |
| `>=` | Greater or equal |
| `<=` | Less or equal |
| `==` | Equal |
| `contains` | Substring match |

## Comments

METALOGOS does not have a comment syntax in the current version.

## File Conventions

- Source files: `.mlog`
- Project manifest: `mlog.toml`
- Lock file: `mlog.lock`
- Standard library: `std/*.mlog`
