# Tutorial: Hello to Adapt

This tutorial walks you through METALOGOS step by step, from a minimal program to adaptive machine-learning patterns. Each step builds on the last.

## Step 1: Hello, World!

Create a file called `hello.mlog`:

```mlog
entity greeting: String = "Hello, Metalogos!"

pattern Shout(s: String) -> String { return upper(s) + "!" }

flow Main { input: String = greeting -> Shout -> output }
```

Run it:

```bash
mlog run hello.mlog
# Output: HELLO, METALOGOS!
```

What happened? Three declarations:
1. **`entity`** — declares a named value. `greeting` is a `String` with the value `"Hello, Metalogos!"`.
2. **`pattern`** — declares a pure function. `Shout` takes a `String` and returns it uppercased with `"!"`.
3. **`flow`** — declares a data pipeline. Data flows left-to-right: `greeting` feeds into `Shout`, whose output becomes `output`.

## Step 2: Entity Types and Records

Define structured data with entity types:

```mlog
entity Message {
  text: String
  urgency: Float
}

entity msg: Message = { text: "Help needed", urgency: 0.9 }
```

An entity type (`Message`) is a schema — it defines fields and their types. An entity record (`msg`) is an instance of that type.

## Step 3: Learnable Patterns (LLM-powered)

A learnable pattern delegates to a language model:

```mlog
learnable pattern Greet(name: String) -> String {
  prompt: "hello"
}
```

When invoked in a flow, the pattern sends the prompt and input to the LLM, returning its response. In tests, the `MockLlm` backend returns deterministic outputs.

```mlog
pattern RunGreet(input: String) -> String {
  return Greet(input)
}

flow Main { input: String = "world" -> RunGreet -> output }
```

## Step 4: Adaptation

METALOGOS patterns can learn from feedback at runtime:

```mlog
learnable pattern Sentiment(text: String) -> String {
  prompt: "Classify sentiment"
}

adapt Sentiment add_example("great service", "positive")
```

The `adapt` declaration adds a training example to the learnable pattern. You can also use `mutate` with rollback conditions:

```mlog
mutate Sentiment {
  add_example("terrible experience", "negative")
  rollback_if: accuracy < 0.9
}
```

## Step 5: Sandbox

Adaptive operations must run inside a sandbox for safety:

```mlog
sandbox test_sandbox {
  allowed: [compute]
  forbidden: [network, write_permanent]
  timeout: 10
}
```

## Step 6: Fluid Types

Fluid types represent uncertainty — a value can be multiple types simultaneously:

```mlog
fluid x = Float[42.0][0.9] or String["answer"][0.1]
```

This declares `x` as a superposition: 90% confident it's a `Float(42.0)`, 10% confident it's a `String("answer")`. When used in a typed context, the highest-confidence matching variant collapses automatically.

```mlog
pattern Double(n: Float) -> Float { return n + n }
flow Main { input: Float = x -> Double -> output }
```

Here `x` collapses to `Float(42.0)` because `Double` expects a `Float`.

## Step 7: Rules

Rules provide conditional logic with priority-based conflict resolution:

```mlog
rule If(msg.text contains "urgent") then msg.urgency = 0.9
rule If(msg.text contains "invoice") then msg.urgency = 0.7
```

Higher-priority rules take precedence when multiple rules fire on the same target.

## Step 8: Memory

METALOGOS has built-in memory constructs:

```mlog
memorize "user prefers dark mode" with priority=0.9
forget "temporary cache entry" after 30.days
```

## Step 9: Standard Library

Import standard library modules:

```mlog
import std/string

entity raw: String = "  hello world  "
entity cleaned: String = trim(raw)
entity result: String = replace(cleaned, "world", "METALOGOS")

flow Main { input: String = result -> output }
```

Available modules:
- `std/string` — trim, replace, split, join
- `std/math` — abs, min, max, clamp, round
- `std/collections` — first, last, push

## Step 10: Full Pipeline

Combine everything into a complete program:

```mlog
import std/string

learnable pattern Sentiment(text: String) -> String {
  prompt: "Classify sentiment"
}

adapt Sentiment add_example("great service", "positive")

sandbox test_sandbox {
  allowed: [compute]
  forbidden: [network, write_permanent]
  timeout: 10
}

mutate Sentiment {
  add_example("terrible experience", "negative")
  rollback_if: accuracy < 0.9
}

entity input_text: String = "great service"
entity cleaned: String = trim(input_text)

flow Main { input: String = cleaned -> Sentiment -> output }
```

## Next Steps

- Read the [Syntax Reference](./syntax.md) for the complete language specification
- Explore the [Standard Library](./stdlib.md) for available built-in functions
- Check the [ADR Index](./adr-index.md) for architectural decisions
- Use `mlog repl` for interactive exploration
- Install the VS Code extension for LSP-powered editor support
