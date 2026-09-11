# METALOGOS — Language Reference

> **Version:** 0.17.0
> **Single source of truth** for developers writing in Metalogos.
> Contains the full list of built-in functions with signatures, types, descriptions, and examples,
> as well as a reference for syntax, data types, and the CLI.

---

## Contents

1. [CLI](#1-cli-commands)
2. [Data Types](#2-data-types)
3. [Syntax](#3-syntax)
4. [Built-in Functions (Builtins)](#4-built-in-functions-builtins)
   - [Strings](#41-string-functions)
   - [Numbers and Math](#42-numbers-and-math)
   - [Collections (List)](#43-collections-list)
   - [Type Conversion](#44-type-conversion)
   - [LLM and AI](#45-llm-and-ai)
   - [HTTP](#46-http)
   - [JSON](#47-json)
   - [File I/O](#48-file-io)
   - [Memory (KV Store)](#49-memory-kv-store)
   - [Session Memory](#410-session-memory)
   - [Encryption and Security](#411-encryption-and-security)
   - [Authentication](#412-authentication)
   - [HTTP Server (mlogserver)](#413-http-server-mlogserver)
   - [Templates](#414-templates)
   - [Databases](#415-databases)
   - [Bots (Telegram/Discord)](#416-bots-telegramdiscord)
   - [Miscellaneous](#417-miscellaneous)
   - [PDF (pdf-inspector)](#418-pdf-naryad-48-pdf-inspector)
   - [SVG Graphics and Diagrams](#419-svg-graphics-and-diagrams-naryads-77-92-adr-0102)
   - [Email, Calendar, Contacts](#420-email-calendar-contacts-naryads-mlg-456)
   - [Reflex — Local Neural Models](#421-reflex--local-neural-models-naryads-177185-adr-011201140117)
5. [Top-Level Declarations](#5-top-level-declarations)
6. [Stdlib (Standard Library)](#6-stdlib-standard-library)
7. [Changelog](#7-changelog-brief)

---

## 1. CLI Commands

The `mlog` binary supports the following commands:

| Command | Description |
|---------|----------|
| `mlog run <file.mlog>` | Run the program (or `.mbc` bytecode) |
| `mlog repl` | Interactive session (REPL) with command history |
| `mlog check <file.mlog>` | Semantic analysis without execution |
| `mlog serve <file.mlog>` | Start an HTTP server from a `mlogserver`/`server` block |
| `mlog compile <file.mlog>` | Compile `.mlog` to `.mbc` bytecode |
| `mlog eval <file.mlog>` | Run `eval` blocks (testing learnable patterns) |
| `mlog resume <file.mlog> --flow=<name> --from=<checkpoint>` | Resume a flow from a checkpoint |
| `mlog audit <file.mlog>` | Static security audit without execution |
| `mlog test <file.mlog> [--filter=substring]` | Run `test` blocks (unit tests) |

### Environment variables

| Variable | Description |
|------------|----------|
| `METALOGOS_LLM_MOCK` | `true` (default) — mocked LLM responses; `false` — real calls |
| `METALOGOS_FORCE_PIPE` | `1` — force piped-mode REPL (for tests) |

---

## 2. Data Types

| Type | Description | Literal example |
|-----|----------|-----------------|
| `String` | A string (UTF-8, Unicode-aware) | `"Hello world"` |
| `Float` | A floating-point number (all numbers) | `42.0`, `3.14`, `-1.0` |
| `Bool` | A boolean value | `true`, `false` |
| `List` | A list of values | `[1.0, 2.0, "three"]` |
| `Struct` | A named structure with fields | `{ name: "Alice", age: 25.0 }` |
| `Html` | an opaque type for safe HTML (auto-escaping) | — |
| `Query` | an opaque type for SQL queries (injection is impossible) | — |
| `Secret` | an opaque type for secrets (not printed, not serialized) | — |
| `Encrypted` | an opaque type for encrypted data | — |
| `Hash` | an opaque type for password hashes | — |
| `Session` | an opaque type for sessions | — |
| `Unit` | an empty value (analogous to `null`/`void`) | — |
| `Fluid` | a probabilistic type (a superposition of variants with confidence) | — |

> **Note:** Metalogos has no separate `Int` type — all numbers are `Float`.
> Integers are written as `42.0`. To convert a string to an integer, use `to_int()`.

---

## 3. Syntax

### 3.1. Comments

```mlog
// single-line comment
```

### 3.2. Variables and bindings

```mlog
let x = 42.0
let name = "Metalogos"
let items = [1.0, 2.0, 3.0]
let result = if x > 10.0 then "big" else "small"   // let with an if-expression
```

**Mutable variables (`let mut`):** Since Naryad #14, variables are immutable by default. To reassign, use `let mut`. Since Naryad #264 the contract is enforced statically: `mlog check` reports assignment to a non-`mut` variable as an error, the compiler refuses to emit bytecode for it, and the VM fails loudly on assignment bytecode produced past the check (no backend assigns silently):

```mlog
let mut counter = 0.0
while counter < 10.0 {
  counter = counter + 1.0   // OK — counter is declared as mut
}
let x = 5.0
x = 10.0   // ERROR: "cannot assign to immutable variable: x"
```

**Scope of `let`:** `let` creates or overwrites a variable in the **current** environment. Inside blocks (`if`, `each`, `while`, `match`), `let` behaves as an overwrite — it modifies the variable from the outer environment rather than creating a local shadow copy. This means `let x = 999.0` inside an `if` will change `x` for all subsequent code, including code after the block.

```mlog
let x = 1.0
if x == 1.0 {
    let x = 999.0   // overwrites the outer x
}
// here x == 999.0
```

If a local redefinition that does not affect the outer `x` is needed, use a different name or a pattern with `let tmp_x = ...`. Contract: `examples/p30_scope_let.mlog` + `.expected`. This behavior may change in future versions (a move to lexical scoping with block isolation is planned).

**Mutable variables (`let mut`, Naryad #14):** Variables are immutable by default. For assignment, use `let mut` (contract: `examples/p30_assign_mut.mlog` + `.expected`, `examples/p30_assign_immutable.mlog` + `.error`; enforced by `mlog check` and the compiler since Naryad #264):
```mlog
let mut counter = 0.0
counter = counter + 1.0   // OK
let x = 5.0
x = 10.0                  // ERROR: cannot assign to immutable variable: x
```

### 3.3. Operators

| Category | Operators |
|-----------|-----------|
| Arithmetic | `+`, `-`, `*`, `/` |
| Comparison | `==`, `!=`, `>`, `<`, `>=`, `<=` |
| Unary minus | `-expr` |
| Field access | `obj.field` |
| Index access | `list[0]` |
| Function call | `func(arg1, arg2)` |
| Qualified call | `module.func(arg)` |

### 3.4. Control constructs

**If-else (block form):**
```mlog
if x > 10.0 {
  print("big")
} else if x > 5.0 {
  print("medium")
} else {
  print("small")
}
```

**If-then-else (expression):**
```mlog
let label = if score >= 90.0 then "A" else "B"
```

**Each (loop over a collection):**
```mlog
each item in items {
  print(item)
}
```

**While (conditional loop):**
```mlog
while count < 10.0 {
  let count = count + 1.0
}
```

**Match (pattern matching, Naryad #14):**
```mlog
match command {
  "start" then { print("starting") }
  starts_with "stop" then { print("stopping") }
  contains "help" then { print("helping") }
  > 100.0 then { print("too big") }
  else { print("unknown") }
}
```
Four kinds of arms are supported: an exact match (`"val" then {}`), a prefix (`starts_with "pre" then {}`), a substring (`contains "sub" then {}`), and a comparison (`> expr then {}` with any of `>`, `<`, `>=`, `<=`, `==`, `!=`). Match returns the value of the last expression in the selected arm.

> **TW-only:** `match` (as a statement) and `match` as an expression in a
> `let` binding (`let x = match y { ... }`, Naryad #173b) work only in the
> tree-walking interpreter. The bytecode VM does not support them — see
> [ADR-0105](docs/adr/0105-vm-experimental-scope.md).

**If-else block as an expression (Naryad #14):**
```mlog
let label = if score >= 90.0 { "A" } else { "B" }
let dept_color = if x == "osp" { "#FF0000" } else if x == "lz" { "#00FF00" } else { "#999999" }
```

**Try expression (error handling, Naryad #14):**
```mlog
let result = try http_post("https://api.example.com", body, "application/json")
// On error: result = Unit, in stderr: [try] caught error: ...
```

**Return:**
```mlog
return result
```

**Require (RBAC, Naryad #14):** Checks the current user's role (only in an HTTP context with the session middleware).
```mlog
let _ = require("admin")   // On refusal: execution aborts with the error "access denied"
```

### 3.5. String literals

```mlog
let s = "hello world"
let with_escape = "line1\nline2\ttabbed"
```

Supported escape sequences: `\"`, `\\`, `\n`, `\t`, `\r`.

### 3.6. Identifiers

Identifiers support ASCII, `_`, and Cyrillic (А-я):
```mlog
let имя = "Metalogos"
let счетчик = 0.0
pattern Приветствие(кто: String) -> String { ... }
```

---

## 4. Built-in Functions (Builtins)

> **Coverage note (v0.19):** This section documents **100%** of the 394 registered builtins (394 of 394): curated rows where present, handler `///`-doc rows otherwise; §6 is the generated full index over the registry.
> The §6 index at the bottom is generated from `src/builtins/registry.rs` (the authoritative list)
> and pinned by `tests/reference_consistency.rs` — adding an undocumented builtin fails CI.
>
> Registered builtins live in `src/builtins/` (26 Rust module files, ~23K lines).
> The registry (`spec!()` macro) is the single source of truth for names, arities,
> and categories — compiler, VM, and semantic analysis all derive from it.

All built-in functions are registered in a single registry, `BUILTIN_REGISTRY` (file `src/builtins/registry.rs`).

### 4.1. String functions

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `upper(s)` | `String -> String` | String | Converts a string to uppercase |
| `lower(s)` | `String -> String` | String | Converts a string to lowercase |
| `trim(s)` | `String -> String` | String | Trims whitespace from the edges |
| `replace(s, old, new)` | `String, String, String -> String` | String | Replaces all occurrences of `old` with `new`. Unicode-aware, works with Cyrillic and emoji |
| `split(s, sep)` | `String, String -> List` | List | Splits a string by a separator. An empty separator splits per character |
| `join(items, sep)` | `List, String -> String` | String | Joins list elements with a separator. Default `","` |
| `index_of(s, needle)` | `String, String -> Float` | Float | Returns the position (in characters, not bytes) of the first occurrence, or `-1.0` |
| `substring(s, start, end)` | `String, Float, Float -> String` | String | Extracts a substring by character indices. Soft-failure: an empty string on out-of-bounds |
| `char_at(s, index)` | `String, Float -> String` | String | Returns the character at an index. An empty string on out-of-bounds |
| `starts_with(s, prefix)` | `String, String -> Bool` | Bool | Checks whether the string starts with a prefix |
| `ends_with(s, suffix)` | `String, String -> Bool` | Bool | Checks whether the string ends with a suffix |
| `contains(s, needle)` | `String, String -> Float` | Float | Returns `1.0` if it contains it, `0.0` otherwise |
| `reverse(s)` | `String -> String` | String | Reverses the string (per character) |
| `length(s)` | `String -> Float` | Float | Length of the string in characters (Unicode-aware). Equivalent to `len()` |
| `len(s)` | `String\|List -> Float` | Float | Length of a string (characters) or a list (elements) |
| `escape_html(s)` | `String -> String` | String | Escapes HTML special characters: `& < > " '` |
| `escape_json(s)` | `String -> String` | String | Escapes JSON special characters: `" \ \n \t \r` |

**Examples:**
```mlog
let s = "Hello, world!"
upper(s)              // "HELLO, WORLD!"
lower(s)              // "hello, world!"
trim("  hello  ")     // "hello"
replace(s, "world", "friend")  // "Hello, friend!"
split("a,b,c", ",")   // ["a", "b", "c"]
join([1.0, 2.0], "-") // "1-2"
index_of(s, "world")  // 7.0
substring(s, 0.0, 5.0) // "Hello"
char_at(s, 7.0)       // "w"
starts_with(s, "Hell") // true
ends_with(s, "!")      // true
contains(s, "wor")     // 1.0
reverse("abc")         // "cba"
length("Hello!")       // 6.0
len([1.0, 2.0, 3.0])  // 3.0
escape_html("<script>alert('xss')</script>")
  // "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
escape_json("hello\"world\n")  // "hello\\\"world\\n"
```

### 4.2. Numbers and math

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `abs(n)` | `Float -> Float` | Float | Absolute value |
| `min(a, b)` | `Float, Float -> Float` | Float | Minimum of two numbers |
| `max(a, b)` | `Float, Float -> Float` | Float | Maximum of two numbers |
| `clamp(val, lo, hi)` | `Float, Float, Float -> Float` | Float | Clamps a value into the range `[lo, hi]` |
| `round(n)` | `Float -> Float` | Float | Rounds to the nearest integer |
| `exp(x)` | `Float -> Float` | Float | e^x. `exp(0)=1`, `exp(1)=e` |
| `ln(x)` | `Float -> Float` | Float | Natural logarithm. Soft-failure: `0.0` for `x <= 0` |
| `sqrt(x)` | `Float -> Float` | Float | Square root. Soft-failure: `0.0` for `x < 0` |
| `pow(base, exp)` | `Float, Float -> Float` | Float | base^exp |
| `tanh(x)` | `Float -> Float` | Float | Hyperbolic tangent. In (−1, 1). `tanh(1000)=1`, `tanh(-1000)=-1` |
| `sigmoid(x)` | `Float -> Float` | Float | The logistic function 1/(1+e^−x). Numerically stable: `sigmoid(1000)=1`, `sigmoid(-1000)=0` (not NaN) |
| `softmax(list)` | `List -> List` | List | Numerically stable softmax (subtracts max before exp). Output sums to 1.0 |
| `random_seed(n)` | `Float -> Unit` | Unit | Sets the seed for a deterministic PRNG (xorshift64). Subsequent `random()` calls are reproducible |
| `random()` | `-> Float` | Float | `[0.0, 1.0)`. If `random_seed()` was called — deterministic. Otherwise — non-deterministic (system time) |
| `to_float(s)` | `String\|Float\|Bool -> Float` | Float | Converts to Float. Soft-failure: `0.0` |
| `to_int(s)` | `String\|Float\|Bool -> Float` | Float | Converts to an integer (truncates the fractional part). Soft-failure: `0.0` |
| `float(s)` | `String\|Float -> Float` | Float | Equivalent to `to_float()`, but errors on an invalid string |

**Examples:**
```mlog
abs(-5.5)          // 5.5
min(3.0, 7.0)      // 3.0
max(3.0, 7.0)      // 7.0
clamp(15.0, 0.0, 10.0)  // 10.0
round(3.7)         // 4.0
to_float("3.14")   // 3.14
to_int("42abc")    // 0.0 (soft-failure)
to_int(3.9)        // 3.0
```

### 4.3. Collections (List)

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `get(list, index)` | `List, Float -> Value` | Value | Gets an element by index. Errors on out-of-bounds |
| `push(list, item)` | `List, Value -> List` | List | Appends an element (returns a new list) |
| `first(list)` | `List -> Value` | Value | The first element. An empty string if the list is empty |
| `last(list)` | `List -> Value` | Value | The last element. An empty string if the list is empty |
| `len(list)` | `List -> Float` | Float | Number of elements |
| `length(list)` | `List -> Float` | Float | Equivalent to `len()` |
| `reverse(list)` | `List -> List` | List | Reverses the list |
| `map(list, "pattern")` | `List, String -> List` | List | Applies a pattern to each element (requires `import std/collections`) |
| `zip(a, b)` | `List, List -> List` | List | Pairwise combination into `Pair{a, b}` |
| `sort_by(list, "field", desc)` | `List, String, Float -> List` | List | Sorts structs by a field (desc=1.0 → descending) |
| `filter(list, "field", value)` | `List, String, Value -> List` | List | Filters: field == value |
| `reduce(list, "field", init)` | `List, String, Float -> Float` | Float | Sum of a field's values across the list |
| `slice(list, start, end)` | `List, Float, Float -> List` | List | Slice of the list [start, end). Soft-failure: start >= len returns an empty list, end > len is clamped, start >= end returns an empty list (ADR-0069) |
| `dedup(list)` | `List -> List` | List | Removes duplicates, keeping the order of first occurrence |

**Examples:**
```mlog
let items = [10.0, 20.0, 30.0]
get(items, 1.0)        // 20.0
push(items, 40.0)      // [10.0, 20.0, 30.0, 40.0]
first(items)           // 10.0
last(items)            // 30.0
len(items)             // 3.0
reverse(items)         // [30.0, 20.0, 10.0]
slice(items, 1.0, 3.0) // [20.0, 30.0]
dedup([1.0, 2.0, 2.0])  // [1.0, 2.0]

import std/collections
let scored = map(actors, "ComputePotential")
let paired = zip(actors, scored)
let ranked = sort_by(paired, "b", 1.0)
```

### 4.4. Type conversion

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `str(value)` | `Any -> String` | String | Converts any value to a string |
| `to_string(value)` | `Any -> String` | String | Equivalent to `str()`. Float without `.0` for integers |
| `float(value)` | `String\|Float -> Float` | Float | Converts to a number (errors) |
| `to_float(value)` | `String\|Float\|Bool -> Float` | Float | Converts to a number (soft-failure: 0.0) |
| `to_int(value)` | `String\|Float\|Bool -> Float` | Float | Converts to an integer (soft-failure: 0.0) |

### 4.5. LLM and AI

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `call_llm(prompt, input)` | `String, String -> String` | String | Calls the LLM backend. By default returns a mock: `"[MOCK: prompt \| input]"`. A real call happens when `METALOGOS_LLM_MOCK=false` |
| `call_claude(api_key, model, system_prompt, user_message)` | `String, String, String, String -> String` | String | A direct call to the Anthropic Claude Messages API (v1/messages). Returns `content[0].text` |
| `llm_usage()` | `-> Struct` | Struct `{LlmUsage}` | LLM usage statistics: `total_calls`, `total_tokens`, `total_errors`, `providers` (a list of `{alias, calls, tokens, errors, avg_latency_ms, health_score}`) |
| `call_llm_schema(prompt, schema_json)` / `call_llm_schema(prompt, input, schema_json)` | `String, String[, String] -> Struct` | Struct `{Dict}` | Calls the LLM backend and requires the answer to be a single JSON value conforming to the schema. Supported schema subset (ADR-0133): `type`, `properties`, `required`, `items`, `enum`; annotation keywords (`title`, `description`, `$schema`, ...) are ignored; any other keyword is a loud `LLM_SCHEMA_UNSUPPORTED_FEATURE`. Answer fields beyond `properties` are rejected (strict-by-default). The result is a `Dict` Struct usable with `json_get`/`has_field`/`dict_*`. Parse/validation failures (including max_tokens truncation) are loud `LLM_SCHEMA_MISMATCH` and retry up to `METALOGOS_LLM_SCHEMA_RETRIES` (default 2, cap 10) with the validator report fed back into the prompt. Mock tier returns a deterministic minimal instance derived from the schema (default mock settings; `METALOGOS_LLM_MOCK=json` documents the intent explicitly) |
| `confidence(fluid_value)` | `Fluid -> Float` | Float | Returns the maximum confidence of the probabilistic type. Returns `1.0` for concrete values |

**Example:**
```mlog
let result = call_llm("Translate to English", "Hello world")
// By default: "[MOCK: Translate to English | Hello world]"

let claude_response = call_claude(env("ANTHROPIC_KEY"), "claude-sonnet-4-20250514", "You are helpful.", "Hello!")

// Structured output (Наряд №269, ADR-0133): the answer must be JSON per the schema
let person = call_llm_schema("Extract the user as JSON", input_text, "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"age\":{\"type\":\"integer\"}},\"required\":[\"name\"]}")
let name = json_get(person, "name")
```

### 4.6. HTTP

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `http_post(url, body)` | `String, String -> String` | String | A POST request. Content-Type defaults to `application/json`. 30s timeout. Errors on status >= 400 |
| `http_post(url, body, content_type)` | `String, String, String -> String` | String | POST with the given Content-Type |
| `http_post(url, body, content_type, headers)` | `String, String, String, String\|Struct -> String` | String | POST with headers. If the 4th argument is a String, it sets `Authorization: Bearer <token>`. If a Struct, headers are set from its fields |
| `http_get(url)` | `String -> String` | String | A GET request. 30s timeout. Errors on status >= 400 |
| `http_get(url, headers)` | `String, String\|Struct -> String` | String | GET with headers (a Bearer token or a Struct) |
| `http_post_multipart(url, fields, files)` | `String, Struct, Struct -> String` | String | A multipart POST. `fields` are text fields (a Struct), `files` are file fields (a Struct, whose values are file paths). File paths must be inside the read sandbox (relative paths); absolute paths, `..` and sandbox escapes are a loud `[SANDBOX_VIOLATION]` error. 120s timeout |
| `http_download(url, dest_path)` | `String, String -> Bool` | Bool | Downloads `url` into `dest_path` (the path must be inside the write sandbox). Returns `true` on success, `false` on any failure (network error, HTTP 4xx/5xx, sandbox violation, write error — the soft-failure contract). The URL is SSRF-gated like the other egress builtins: a blocked address is a LOUD `SSRF guard` error, not a silent `false` (naryad №261). 30s timeout |

All HTTP egress builtins (`http_get`, `http_post`, `http_post_multipart`, `http_download`) do **not** follow 3xx redirects — the 3xx response is returned as-is: its body for `http_get`/`http_post`/`http_post_multipart` (a 3xx is not an error) and as the written file content for `http_download`. Follow a redirect explicitly: issue a second call to the URL from the `Location` header — every call is SSRF-gated and resolve-pinned. Requests to private/loopback/blocked-range addresses (loopback, private, link-local, cloud metadata, IPv4-mapped IPv6, unspecified `0.0.0.0`/`::`, CGNAT `100.64.0.0/10`, benchmark `198.18.0.0/15`) are refused with a loud `SSRF guard` error unless `METALOGOS_HTTP_ALLOW_PRIVATE=1` is set (naryads №130/№150/№261).

**Examples:**
```mlog
// A simple POST
let resp = http_post("https://api.example.com/data", json_encode(payload))

// POST with a Bearer token
let resp = http_post("https://api.example.com/data", body, "application/json", env("API_TOKEN"))

// POST with custom headers
let headers = { "X-Custom": "value", "Authorization": "Bearer token123" }
let resp = http_post("https://api.example.com/data", body, "application/json", headers)

// Multipart POST (uploading a file created inside the sandbox —
// file paths must stay within the sandbox, see the table above)
let fields = {model: "whisper-1"}
let files = {file: "uploads/voice.ogg"}
let resp = http_post_multipart("https://api.openai.com/v1/audio/transcriptions", fields, files)

// GET
let data = http_get("https://api.example.com/users")
let data = http_get("https://api.example.com/users", env("API_TOKEN"))
```

### 4.6.1. Voice pipeline

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `whisper_transcribe(file_id, bot_token, api_key, provider)` | `String, String, String, String -> String` | String | Downloads a voice message from Telegram by `file_id`, sends it for transcription to the Whisper API. `provider`: `"openai"` (default) or `"groq"`. Returns the recognized text |
| `tts_send(text, voice, bot_token, chat_id)` | `String, String, String, String -> String` | String | Generates speech via the OpenAI TTS API (the `tts-1` model) and sends the audio to a Telegram chat. Requires `OPENAI_API_KEY`. Voices: `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"` |

### 4.7. JSON

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `parse_json(text)` | `String -> Struct\|List\|String\|Float\|Bool\|Unit` | Any | Parses a JSON string. Objects become a Struct with `type_name: "Json"`, arrays become a List, `null` becomes Unit |
| `json_encode(value)` | `Any -> String` | String | Serializes a value to a JSON string. Supports String, Float, Bool, Unit→null, List→array, Struct→object |
| `json_get(obj, field_path)` | `Struct, String -> Value` | Value | Accesses a field by a dot-path. Returns the **real value** (including String). Returns Unit if the field is absent or a SQL NULL. Supports dot-paths: `"voice.file_id"` |
| `json_get(obj, field_path, default)` | `Struct, String, Value -> Value` | Value | With a default value when the field is absent or a SQL NULL (v0.9.6) |
| `has_field(obj, field_path)` | `Struct, String -> Float` | Float | `1.0` if the field exists, `0.0` if not. Supports dot-paths |
| `dict_get(dict, key, default)` | `Struct, String, Any -> Any` | Any | Dict-style key access. Returns `default` if the key is absent. Does not support dot-paths (use `json_get` for that) |
| `dict_set(dict, key, value)` | `Struct, String, Any -> Struct` | Struct | Returns a **new** Struct with the key updated. The original dict is not mutated |
| `dict_has(dict, key)` | `Struct, String -> Bool` | Bool | `true` if the key exists in the dict. A direct check (no dot-path) |
| `dict_keys(dict)` | `Struct -> List` | List | Returns a list of all the dict's keys |
| `dict_values(dict)` | `Struct -> List` | List | Returns a list of all the dict's values (order matches `dict_keys`) |
| `escape_json(text)` | `String -> String` | String | Escapes special characters for embedding in JSON |

**Examples:**
```mlog
let data = parse_json("{\"name\": \"Alice\", \"age\": 30}")
let name = json_get(data, "name")           // "Alice"
let missing = json_get(data, "email")        // Unit (field absent)
let missing = json_get(data, "email", "none") // "none" (default)

let nested = parse_json("{\"a\": {\"b\": 42}}")
json_get(nested, "a.b")                      // 42.0
has_field(nested, "a.b")                     // 1.0

let encoded = json_encode({ key: "value", n: 42.0 })
// "{\"key\":\"value\",\"n\":42.0}"
```

### 4.8. File I/O

> **Important:** All file operations are sandboxed to the working directory.
> Absolute paths and `..` (path traversal) are rejected.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `read_file(path)` | `String -> String` | String | Reads a file. Soft-failure: an empty string when the file is missing or unreadable. Sandbox violations (absolute path, `..`, symlink escape, broken symlink) are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `write_file(path, content)` | `String, String -> String` | String | Writes a file (overwrite). Returns `"ok"` or `""` on an OS-level error; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `append_file(path, content)` | `String, String -> String` | String | Appends to the end of a file. Returns `"ok"` or `""`; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `delete_file(path)` | `String -> String` | String | Deletes a file. Returns `"ok"`, `""` when the file is missing; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `file_exists(path)` | `String -> Bool` | Bool | Checks whether a file exists |
| `list_dir(path)` | `String -> List` | List | A list of files in a directory. With no argument — the current directory |

**Examples:**
```mlog
write_file("data.txt", "hello world")  // "ok"
let content = read_file("data.txt")   // "hello world"
append_file("data.txt", "\nmore")     // "ok"
file_exists("data.txt")               // true
let files = list_dir(".")             // ["data.txt", ...]
delete_file("data.txt")               // "ok"
```

### 4.9. Memory (KV store)

A global in-memory KV store. When `memory { persist: "path.db" }` is set — also writes through to SQLite.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `kv_set(key, value)` | `String, String -> Unit` | Unit | Writes a key-value pair |
| `kv_get(key)` | `String -> String` | String | Reads a value (an empty string if the key is absent) |
| `kv_delete(key)` | `String -> Unit` | Unit | Deletes a key |
| `kv_exists(key)` | `String -> Bool` | Bool | Checks whether a key exists |
| `kv_list()` | `-> List` | List | Returns a list of all keys |
| `mem_set(key, value)` | `String, String -> String` | String | Equivalent to `kv_set`, but returns the value written |
| `mem_get(key)` | `String -> String` | String | Equivalent to `kv_get` |
| `mem_delete(key)` | `String -> String` | String | Equivalent to `kv_delete`, returns the deleted value |

### 4.9.1. Typed semantic memory with hybrid search (ADR-0093, ADR-0094)

Typed semantic memory with SQLite persistence, an FTS5 BM25
keyword index, cosine similarity, and merging via Reciprocal Rank
Fusion (k=60). Each entry carries a type tag for differentiated search.

**Memory types:** `persona`, `episodic`, `instruction`, `fact`

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `memorize(text, priority, type)` | `String, Float, String -> Unit` | Unit | Saves a fact with a priority (0.0-1.0) and a type. Example: `memorize("likes spicy food", 0.9, "persona")` |
| `recall_top_k(query, k, type)` | `String, Float, String -> String` | String (JSON) | Returns the top-K entries sorted by RRF score. A JSON array: `[{text, type, priority, score, created_at}]`. An empty type searches across all types |

**Scoring:** Reciprocal Rank Fusion (k=60). The BM25 and cosine-similarity
(with temporal decay and priority) results are ranked separately, then merged:
`score = 1/(60+bm25_rank) + 1/(60+cosine_rank)`. RRF is robust to differences
in the score distributions between signals.

**Examples:**
```mlog
// Saving with a type
memorize("user prefers email", 0.9, "persona")
memorize("project deadline is July 15", 0.8, "fact")
memorize("always greet politely", 1.0, "instruction")

// Searching the top 5 facts
let results = recall_top_k("user preferences", 5.0, "persona")

// Searching across all types
let all = recall_top_k("project", 10.0, "")
```

### 4.9.2. Memory Tree (mtree) — multi-level memory (naryad #63)

Hierarchical memory: L0 (raw) to L1 (cluster summary) to L2 (high-level).
A graph structure with path search and subgraph extraction.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `mtree_store(text, source?)` | `String, String? -> String` | String (id) | Saves text as an L0 node. `source` defaults to `"user"`. Returns the node ID. The admission score is computed from length/uniqueness |
| `mtree_retrieve(query, limit?)` | `String, Float? -> String` | String (JSON) | Searches the memory graph. `limit` defaults to 5. Returns a JSON array of nodes with metadata |
| `mtree_forget(id)` | `String -> Float` | Float | Deletes a node by ID. Returns `1.0` if it existed, `0.0` if not |
| `mtree_summarize()` | `-> Unit` | Unit | L0 to L1 to L2 clustering. Groups L0 nodes into L1 summaries, L1 into L2. Idempotent — a repeat call recomputes the summaries |
| `mtree_stats()` | `-> Dict` | Dict | Returns `MemoryStats { l0_count, l1_count, l2_count, total_nodes, edges }` |

### 4.9.3. Memory graph: paths and subgraphs

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `graph_query(query, limit?, level_filter?)` | `String, Float?, String? -> String` | String (JSON) | Searches the graph with an optional level filter (`"L0"`, `"L1"`, `"L2"`). `limit` defaults to 5 |
| `graph_path(from_id, to_id)` | `String, String -> String` | String (JSON) | The shortest path between two nodes. Returns a JSON array of nodes on the path. An empty array if there is no path |
| `graph_neighbors(id)` | `String -> String` | String (JSON) | A node's neighbors (direct links). A JSON array of nodes |
| `subgraph_extract(root_id, depth)` | `String, Float -> String` | String (JSON) | Extracts a subgraph from `root_id` to a given `depth`. JSON with nodes and edges |
| `subgraph_nodes(root_id, depth)` | `String, Float -> String` | String (JSON) | Only the subgraph's nodes (no edges). A JSON array of nodes |
| `subgraph_json(root_id, depth)` | `String, Float -> String` | String (JSON) | The full subgraph as JSON (nodes + edges). Ready for visualization |
| `trace_start(label)` | `String -> String` | String (id) | Starts a trace segment with a label. Returns a trace_id |
| `trace_end(trace_id)` | `String -> Dict` | Dict | Ends a trace segment. Returns `TraceResult { id, label, duration_ms }` |

### 4.10. Session memory

Temporary in-memory storage scoped to a session_id. Not persistent — it resets when `mlog serve` restarts.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `session_set(session_id, key, value)` | `String, String, String -> String` | String | Saves a value in the session |
| `session_get(session_id, key)` | `String, String -> String` | String | Reads a value from the session (an empty string if absent) |
| `session_clear(session_id)` | `String -> String` | String | Deletes all of the session's data. Returns `"ok"` |

### 4.11. Encryption and security

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `env(key)` | `String -> String` (→ `Secret` in an entity context) | String/Secret | Reads an environment variable. An empty string if not found (the soft-failure contract). **Serve gate (naryad №259)**: inside serve route bodies `env()` is denied by default with a loud `ENV_NOT_PERMITTED` error — route code often receives untrusted input and must not read the process's secrets. Escape hatches (alternatives, not AND): `METALOGOS_SERVE_ALLOW_ENV=1` allows all env reads in route bodies, or `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` allows exactly the listed names. Outside serve (`mlog run`, `mlog check`, repl, serve top level) the read is ungated, as before. The denial is identical for existing and non-existing names (the gate runs before the read). See also the exec gates (`EXEC_NOT_PERMITTED`, naryad №253) in the threat model |
| `generate_key()` | `-> Secret` | Secret | Generates a 256-bit random key (64 hex characters) |
| `encrypt(data, key)` | `String, Secret -> Encrypted` | Encrypted | Encrypts with AES-256-GCM using a random 96-bit nonce. The key is 64 hex characters |
| `decrypt(encrypted, key)` | `Encrypted, Secret -> String` | String | Decrypts AES-256-GCM. Errors on a wrong key |
| `hash_password(password)` | `String -> Hash` | Hash | Hashes a password (Argon2id with a random salt) |
| `verify_password(password, hash)` | `String, Hash -> Bool` | Bool | Verifies a password (constant-time comparison) |
| `sha256(text)` | `String -> String` | String | The SHA-256 hash, hex representation (64 characters). Unicode-aware: hashes the UTF-8 bytes |
| `hmac_sha256(key, message)` | `String, String -> String` | String | HMAC-SHA256 with a key, hex representation (64 characters) |
| `hex_encode(text)` | `String -> String` | String | Encodes a string to hex (UTF-8 bytes to hex characters) |
| `hex_decode(hex_str)` | `String -> String` | String | Decodes hex to a string. Invalid UTF-8 becomes a `<binary: N bytes>` placeholder |
| `require(condition)` | `Bool -> Unit` | Unit | A runtime assertion. Errors if `false` |
| `require(condition, message)` | `Bool, String -> Unit` | Unit | An assertion with an error message |

**Examples:**
```mlog
entity db_url: Secret = env("DATABASE_URL")
let key = generate_key()
let encrypted = encrypt("secret data", key)
let decrypted = decrypt(encrypted, key)  // "secret data"

let hash = hash_password("mypassword")     // Hash (opaque)
verify_password("mypassword", hash)        // true
verify_password("wrong", hash)             // false

require(user.role == "admin")              // panics if not admin
require(age >= 18.0, "Access denied")      // with a message
```

### 4.12. Authentication

> **Note:** In `mlog run` mode, these functions return mock values.
> Real behavior only occurs in a `mlog serve` context (the Axum server).

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `authenticate(email, password)` | `String, Secret\|String -> Unit` | Unit | Authenticates a user. In a server context, checks the credentials |
| `session_login(user_id)` | `String -> Session` | Session | Creates a session for the user |
| `session_logout(session)` | `Session -> Unit` | Unit | Destroys the session |

### 4.13. HTTP server (mlogserver)

Functions for use inside route handlers of `mlogserver`/`server` blocks.

**Request body limit (Naryad #255):** the server accepts request bodies up to **2 MiB** (2 097 152 bytes, `REQUEST_BODY_LIMIT_BYTES` in `src/server.rs`) — a deliberate constant, not the implicit axum default. A larger body is rejected with HTTP 413 Payload Too Large. The same limit applies to every route, TW and VM backends alike.

**Query parameter decoding (Naryad #257):** `query_param` percent-decodes keys and values with RFC 3986 byte semantics — `%XX` bytes are reassembled and interpreted as UTF-8, so `%D0%B6` yields `"ж"` (fixed in №257; before it produced mojibake). Documented choices: `+` decodes to space (the `application/x-www-form-urlencoded` convention — send a literal `+` as `%2B`); invalid escapes (`%ZZ`) and a truncated `%` pass through literally (query parsing never fails on user input); invalid UTF-8 decodes lossily (U+FFFD); decoding is single-pass (`%25D0%25B6` → `%D0%B6`).

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `respond(status_line)` | `String -> HttpResponse` | HttpResponse | Builds an HTTP response. Format: `"200 OK"`, `"404 Not Found"`, etc. |
| `respond_html(status, html)` | `String, String -> HttpResponse` | HttpResponse | An HTML response with the given status |
| `form_data()` | `-> Struct {FormData}` | Struct | Parses data from an `application/x-www-form-urlencoded` request body |
| `json_body()` | `-> Struct {JsonBody}` | Struct | Parses JSON from the request body |
| `query_param(name)` | `String -> String` | String | Gets a query parameter from the URL. `curl "localhost:8080/search?q=hello" -> query_param("q") == "hello"`. An empty string if the parameter is absent. Percent-decoding: RFC 3986 bytes reassembled as UTF-8 (`%D0%B6` → `"ж"`), `+` → space (form-urlencoded convention), invalid escapes pass through literally, invalid UTF-8 is lossy — see the §4.13 note (Naryad #257) |

**Example:**
```mlog
mlogserver {
  port: 8080
  route "/hello" method=GET {
    respond("200 OK")
  }
  route "/api/data" method=POST {
    let data = json_body()
    let name = json_get(data, "name", "unknown")
    respond_html("200", "<h1>Hello " + escape_html(name) + "</h1>")
  }
  route "/search" method=GET {
    let q = query_param("q")
    respond("200 " + q)
  }
}
```

### 4.14. Templates

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `render(template_name, key1, val1, key2, val2, ...)` | `String, String, Any, ... -> Html` | Html | Renders a template, substituting `{{ var }}` variables. The number of arguments after the template name must be even (key/value pairs) |

**Example:**
```mlog
template Page(title: String, body: String) -> Html {
  <html><head><title>{{ title }}</title></head><body>{{ body }}</body></html>
}

// In a route handler:
let page = render(Page, "title", "My Page", "body", "Hello!")
return page
```

### 4.15. Databases

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `query(sql, params)` | `String, List -> List` | List | An SQL query. SELECT returns List[Row{...}], everything else returns a string with the number of affected rows |
| `db_execute(sql)` | `String -> Unit` | Unit | Executes an SQL query without returning data |
| `db_insert(table, struct)` | `String, Struct -> Float` | Float | A parameterized INSERT. Returns last_insert_rowid (Problem C) |

**Schema-as-code** (ADR-0060) — declaring tables directly in .mlog:

```mlog
db { url: "sqlite::memory:" }

schema my_dept {
  table analysis {
    id: Int primary_key auto_increment
    topic: String
    status: String default("drafted")
  }
}
```

Types: Int→INTEGER, Float→REAL, String/Text→TEXT, Bool→INTEGER, DateTime→TEXT. Modifiers: primary_key, auto_increment, nullable, references(table.field). Defaults: default("value"), default(now()). Migration: additive-only (CREATE TABLE IF NOT EXISTS).
### 4.16. Bots (Telegram/Discord)

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `send_message(chat_id, text)` | `String\|Float, String -> Unit` | Unit | Sends a message to a chat (Telegram/Discord). In interpreter mode — logs to `[AUDIT]` |

### 4.17. Miscellaneous

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `print(s)` | `String -> String` | String | Prints a string to stdout, returns it |
| `inspect(pattern_name)` | `String -> Struct\|Unit` | Struct (or Unit if the pattern is not found) | Returns a pattern's statistics (ADR-0051): `examples_count`, `invocation_count`, `last_invocation_at`, `mode` (for learnable: TEACHING/DISTILLED). Soft-failure — a nonexistent pattern returns `Unit`, not an error |
| `base64_encode(s)` | `String -> String` | String | Encodes a string as Base64 (standard alphabet). Unicode-aware: encodes the UTF-8 bytes |
| `base64_decode(s)` | `String -> String` | String | Decodes Base64. Errors if not valid Base64 or not UTF-8 |
| `toon_encode(value)` | `Any -> String` | String | Encodes a value as TOON (Token-Optimized Object Notation). Prefixed with `TOON:`. Any Value becomes a string |
| `toon_decode(s)` | `String -> Any` | Value | Decodes a TOON string back into a Value. A recursive-descent parser |
| `assert_eq(actual, expected)` | `Any, Any -> Any` | Any | A runtime equality assertion. Returns actual on success, panics with `actual != expected` |
| `assert_contains(haystack, needle)` | `Any, Any -> Unit` | Unit | Panics if the string representation of `needle` is not found in `haystack` |

### 4.17.1. OpenPlanter-inspired system builtins

> Source: naryad #64 (OpenPlanter agent utilities). Used in
> agent flows for budget-aware execution, state snapshots, and
> safe exec.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `budget_check(step, total_steps)` | `Float, Float -> Dict` | Dict | Returns `BudgetStatus { step, total_steps, remaining, fraction, over_budget }`. Errors if `total_steps == 0` |
| `replay_snapshot(data)` | `List -> Dict` | Dict | Serializes a list of values into a JSON snapshot. Returns `ReplaySnapshot { seq, items, json, created_at }`. seq=0 is a full snapshot, seq=N is a delta |
| `policy_check(command)` | `String -> Dict` | Dict | Checks a command against policy: heredoc `<<`, pipe `|`, background `&`, redirect `>`. Returns `PolicyResult { command, allowed, reason }` |

### 4.17.2. Fluid-type budget control

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `budget_check(step, total_steps)` | `Float, Float -> Dict` | Dict | (See 4.17.1 — listed under the `system` category, also used in fluid pipelines to control a cascade of edits) |

### 4.17.3. Cron scheduler (naryad #35)

A persistent cron scheduler for deferred tasks. Jobs are saved to
JSON and survive a process restart. Expressions use the standard 5-field
cron format (`min hour dom month dow`).

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `cron_add(cron_expr, prompt)` | `String, String -> String` | String (id) | Adds a cron job. `cron_expr` is a 5-field cron expression (e.g. `"0 9 * * 1-5"` — every weekday at 9:00). `prompt` is what to run. Returns the job's ID |
| `cron_list()` | `-> List` | List | A list of all cron jobs. Each element is `CronJob { id, cron_expr, prompt, force_run, run_count, last_run, created_at }` |
| `cron_remove(id)` | `String -> Float` | Float | Deletes a job by ID. Returns `1.0` if deleted, `0.0` if not found |
| `cron_run(id)` | `String -> Float` | Float | Forces a job to run outside its schedule (sets `force_run=true`). Returns `1.0` if found, `0.0` if not |
| `cron_mark_fired(id)` | `String -> Float` | Float | Marks a job as run: clears `force_run`, increments `run_count`, updates `last_run`. Returns `1.0` if found, `0.0` if not |

### 4.17.4. Time and dates

All functions work with Unix timestamps (Float, seconds since the 1970-01-01 epoch).
The local timezone is used for `format_date` / `weekday_name` /
`date_parts` (on a server, the process's timezone).

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `time()` | `-> Float` | Float | The current Unix timestamp (seconds since the epoch). High precision (sub-second) |
| `sleep(seconds)` | `Float -> Unit` | Unit | Blocks the current thread for `seconds` seconds. Use carefully in `mlog serve` — it blocks request handling |
| `format_date(fmt?, timestamp?)` | `String?, Float? -> String` | String | Formats a timestamp using a strftime string. `fmt` defaults to `"%Y-%m-%d %H:%M:%S"`. `timestamp` defaults to the current moment |
| `date_parts(timestamp?)` | `Float? -> Dict` | Dict | Returns `DateParts { year, month, day, hour, minute, second, weekday }`. `timestamp` defaults to the current moment |
| `days_between(ts1, ts2)` | `Float, Float -> Float` | Float | The absolute difference between two timestamps, in days. `|ts1 - ts2| / 86400` |
| `days_in_month(year, month)` | `Float, Float -> Float` | Float | The number of days in a month. `month` is 1-12. Errors if the month is out of range. Accounts for leap years |
| `is_leap_year(year)` | `Float -> Bool` | Bool | `true` if the year is a leap year (Gregorian rules) |
| `add_days(timestamp, days)` | `Float, Float -> Float` | Float | Adds `days` to a timestamp. Negative `days` subtracts. `ts + days * 86400` |
| `add_hours(timestamp, hours)` | `Float, Float -> Float` | Float | Adds `hours` to a timestamp. Negative `hours` subtracts. `ts + hours * 3600` |
| `weekday_name(timestamp)` | `Float -> String` | String | The weekday's name (localized via chrono::Local). E.g. `"Monday"` |

### 4.18. PDF (Naryad #48, pdf-inspector)

Native PDF processing in Rust via the `pdf-inspector` crate. Classification,
text extraction to Markdown, region analysis, and an OCR fallback.
Zero IPC, <200ms on text-based PDFs.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `pdf_classify(path)` | `String -> Dict` | Dict | Classifies a PDF: TextBased / Scanned / ImageBased / Mixed. Keys: type, confidence, pages_needing_ocr, page_count |
| `pdf_to_markdown(path)` | `String -> Dict` | Dict | The full pipeline: classification + text extraction + Markdown. Keys: markdown, page_count, pdf_type, has_tables, confidence, processing_time_ms |
| `pdf_extract_regions(path, filter)` | `String, String -> List` | List | Extracts text regions with coordinates. A list of dicts: text, needs_ocr, ocr_reason, page, x, y |
| `pdf_ocr(path)` | `String -> Dict` | Dict | An OCR fallback for scans (requires `--features pdf-ocr` and a system Tesseract). Keys: markdown, ocr_confidence, pages_processed |

**Examples:**
```mlog
// Classifying a PDF
let info = pdf_classify("report.pdf")
// -> { type: "TextBased", confidence: 0.95, pages_needing_ocr: [], page_count: 12 }

// Extracting text to Markdown
let result = pdf_to_markdown("report.pdf")
let md = json_get(result, "markdown", "")

// For scans — OCR
let ocr = pdf_ocr("scan.pdf")
let text = json_get(ocr, "markdown", "")
```

> **Note:** `pdf_ocr` requires a build with the `--features pdf-ocr` flag and an installed
> system `tesseract-ocr` package with CJK training data.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `now()` | `-> Float` | Float | The current Unix timestamp, in seconds |
| `str(value)` | `Any -> String` | String | Converts any value to a string |
| `to_string(value)` | `Any -> String` | String | Equivalent to `str()` (Float without `.0` for integers) |

---

### 4.19. SVG graphics and diagrams (naryads #77-92, ADR-0102)

44 built-in functions, hand-written in pure Rust — with no
external SVG/chart/rendering library at all. All are dispatched through a
common path (not a special case), with TW/VM parity for paths that both backends can execute (see ADR-0105; the VM does not yet support `match`/block if-else)
and verified by `crosscheck`. The full decision history is in ADR-0102 and naryads
#77-92.

> **Security:** every function is classified in `svg_security_lint`
> (`src/semantic.rs`) — text arguments are either automatically
> escaped by the runtime (`SVG_AUTO_ESCAPE_BUILTINS`, a warning
> at `mlog check`), or structural arguments (`d`, `viewbox`,
> `transform` — mini-languages that cannot be safely escaped)
> produce a hard compile **error** on suspected injection
> (`SVG_NO_ESCAPE_BUILTINS`). Naryad #92 checked all 44 functions —
> no gaps were found.

#### Palette

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `color_palette(intent, mode)` | `String, String -> Struct` | `DiagramStyle` | An HSL-cascade palette. `intent` in {calm, tension, energy, authority, warmth}, `mode` in {light, dark}. Output — 5 tokens (`paper`, `ink`, `accent`, `muted`, `rule`) |
| `diagram_style(paper, ink, accent, muted, rule)` | `String×5 -> Struct` | `DiagramStyle` | A validator for manually specified tokens (without generating via `color_palette`) |

#### SVG primitives

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `svg_rect(x, y, w, h, fill, stroke)` | `Float×4, String×2 -> String` | String | A rectangle |
| `svg_circle(cx, cy, r, fill, stroke)` | `Float×3, String×2 -> String` | String | A circle |
| `svg_line(x1, y1, x2, y2, stroke, width)` | `Float×4, String, Float -> String` | String | A line |
| `svg_text(x, y, text, font_size, fill, anchor)` | `Float×2, String, Float, String×2 -> String` | String | Text (auto-escaped) |
| `svg_path(d, fill, stroke)` | `String×3 -> String` | String | An arbitrary path. `d` is a structural argument, **not escaped**, a compile error on injection |
| `svg_group(children, transform)` | `List, String -> String` | String | A group with an optional transform (`transform` is structural, like `d`) |
| `svg_canvas(w, h, viewbox, children)` | `Float×2, String, List -> String` | String | A root `<svg>` (`viewbox` is structural) |
| `svg_canvas_preset(preset_name, viewbox, children)` | `String, String, List -> String` | String | The same, with a named canvas size: `doc_inline` (960×600), `slide_16x9` (1280×720), `social_og` (1200×632), `print_a4_landscape`, `print_a4_portrait` |
| `svg_icon(name, x, y, size, fill)` | `String, Float×3, String -> String` | String | A ready-made icon. `name` in {server, laptop, phone, database, cloud, arrow-right, check, warning, user, document} |
| `svg_sketchy_filter(id, roughness)` | `String, Float -> String` | String | A "hand-drawn" style SVG filter (`id` is structural) |
| `svg_generate(kind, intent, w, h)` | `String, String, Float×2 -> String` | String | A procedural background. `kind` in {flow, grid, noise}. Deterministic (the same `intent` gives an identical result, no `rand`/system clock) |

#### Charts

| Function | Data shape | Notes |
|---------|-------------|-------------|
| `chart_bar(data, style)` | `List<Struct{label, value}>` | A basic bar chart |
| `chart_donut(data, style)` | `List<Struct{label, value}>` | Arcs via `svg_path` |
| `chart_line(data, style)` | `List<Struct{label, value}>` | Connected points |
| `chart_area(data, style)` | `List<Struct{label, value}>` | A fill under the line |
| `chart_scatter(data, style)` | `List<Struct{x, y, label?}>` | Independent scaling on both axes — a **different data shape** than the others |
| `chart_heatmap(data, style)` | `List<List<Float>>` | An HSL color interpolation by value; no text — deliberately outside the security lint |
| `chart_radar(data, style)` | `Struct{axes: List<String>, series: List<Struct{name, values}>}` | Multi-series, polar coordinates, a fixed palette for up to 5 series |
| `chart_boxplot(data, style)` | `List<Struct{label, values: List<Float>}>` | Real quartiles (linear interpolation / the R-7 method), 1.5x IQR whiskers, outliers |

#### Diagrams — hierarchies and flows

| Function | Data shape | Notes |
|---------|-------------|-------------|
| `diagram_tree(data, style)` | `Struct{label, children: List<Struct>}` (recursive) | Separate layout for each subtree |
| `diagram_org_chart(data, style)` | The same plus an optional `title` | A thin wrapper over `diagram_tree` |
| `diagram_flowchart(data, style)` | `Struct{nodes, edges}` | A topological sort by layer; **a cycle is an error** with an explicit message |
| `diagram_layers(data, style)` | `List<Struct{label, description?}>` | Horizontal bands |

#### Diagrams — temporal and process

| Function | Data shape | Notes |
|---------|-------------|-------------|
| `diagram_sequence(data, style)` | `Struct{actors: List<String>, messages: List<Struct{from, to, label?}>}` | Vertical lifelines, `actors` is a list of strings, not structs |
| `diagram_timeline(data, style)` | `List<Struct{date, label, description?}>` | The anti-overlap engine is applied automatically (naryad #87) |
| `diagram_gantt(data, style)` | `List<Struct{task, start, duration}>` | Arbitrary time units, not tied to a calendar |
| `diagram_process(data, style)` | `List<Struct{label, description?}>` | Strictly linear — **do not confuse** with `diagram_flowchart` |
| `diagram_loop(data, style)` | `List<Struct{label, description?}>` | A closed loop, polar coordinates, at least 3 steps |

#### Diagrams — sets and comparisons

| Function | Data shape | Notes |
|---------|-------------|-------------|
| `diagram_venn(data, style)` | `Struct{circles: List<Struct{label, value?}>, overlap_label?}` | Strictly 2 or 3 circles, a fixed symmetric geometry — a general N-circle Venn diagram is out of scope |
| `diagram_quadrant(data, style)` | `Struct{x_axis_label, y_axis_label, items: List<Struct{label,x,y}>}` | `x`,`y` in [-1.0, 1.0] |
| `diagram_pyramid(data, style)` | `List<Struct{label, value?}>` | The first element is the top, narrow layer (not the base) |
| `diagram_nested(data, style)` | `List<Struct{label, value?}>` | Concentric rings, the first element is the outermost |
| `diagram_medallion(data, style)` | `List<Struct{icon?, label, value?}>` | `icon` is a name from `svg_icon`, validation reused |

#### Diagrams — data and states

| Function | Data shape | Notes |
|---------|-------------|-------------|
| `diagram_er(data, style)` | `Struct{entities: List<Struct{name, fields: List<String>}>, relations}` | A simple 3-in-a-row grid, not a relationship analysis for positioning |
| `diagram_state(data, style)` | `Struct{states: List<String>, transitions, initial?}` | **Cycles and self-loops are valid** (unlike a flowchart) |
| `diagram_swimlane(data, style)` | `Struct{lanes: List<String>, steps}` | Position by `order`, not by index |
| `diagram_data_flow(data, style)` | `Struct{nodes, edges}` | Cycles are valid (data feedback) |
| `diagram_high_level(data, style)` | `Struct{nodes, edges}` | No icons, acyclic (topological sort) |
| `diagram_architecture(data, style)` | `Struct{nodes: List<Struct{id,label,icon?}>, edges}` | The same plus `svg_icon` for nodes |

#### Composition and quality

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `template_render(template, data)` | `String, Struct -> Html` | Html | A separate engine (not an extension of `render()`): `{{ var }}` (escaped), `{{{ var }}}` (unescaped), `{{#if}}/{{else}}`, `{{#each}}`. Template content is trusted author code, not scanned by the lint |
| `infographic_qa(svg)` | `String -> Struct{passed, warnings, checks_run}` | Struct | An advisory check: contrast (WCAG, a threshold of 4.5), saturation discipline (more than 2 colors with S>60% triggers a warning), element density. `passed: false` is advice, not a block |
| `html_render(html, width, height)` | `String, Float×2 -> String` | String | A screenshot via a headless browser (`METALOGOS_BROWSER_BIN`, no default path). The only function in this family that spawns an external process — via `exec_restricted` (argv, not a shell). Network isolation is not guaranteed at the OS level — the input must be self-contained HTML |

`std/infographic.mlog` — top-level patterns composing what's
listed above: `InfographicPoster`, `InfographicDashboard` (KPI cards
plus a 2x2 chart grid), `InfographicComparison` (side-by-side, requires
the same `chart_type` on the left and right), `InfographicTimeline`
(a wrapper over `diagram_timeline`).

---

### 4.20. Email, calendar, contacts (naryads MLG-4/5/6)

27 functions, all in pure Rust (`lettre`/`imap` for mail, a hand-rolled
CalDAV/CardDAV client for calendar and contacts). Two different models
for credential handling — don't confuse them when using them.

#### Email (SMTP/IMAP) — credentials via environment variables

**They do not accept host/user/pass as arguments.** Configuration is read
from the environment on every call: `SMTP_HOST`, `SMTP_PORT` (defaults to
587), `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` (defaults to `SMTP_USER`);
`IMAP_HOST`, `IMAP_PORT`, `IMAP_USER`, `IMAP_PASS`. A missing required
variable produces a clear error (`smtp_send: SMTP_HOST env
not set`), not a silent failure.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `smtp_send(to, subject, body, attachments_json?, from?, reply_to?)` | `String×3, String?×3 -> String` | String | Sends a plain-text email, TLS/STARTTLS. Arity 3..6 |
| `smtp_send_html(to, subject, html_body, attachments_json?)` | `String×3, String? -> String` | String | An HTML email. Arity 3..4 |
| `imap_list(folder, limit, offset?)` | `String, Float, Float? -> String` | String (JSON) | A list of emails — envelope + flags. Arity 2..3 |
| `imap_read(message_id)` | `String -> String` | String | The full email: headers, body, attachments |
| `imap_search(folder, query)` | `String, String -> String` | String (JSON) | A text search (the IMAP `TEXT` criteria) |
| `imap_mark_read(message_id)` | `String -> String` | String | Marks it as read |
| `imap_move(message_id, target_folder)` | `String, String -> String` | String | Moves it (RFC 6851 `MOVE`, falling back to `COPY`+`DELETE`) |

#### Calendar (CalDAV/iCal) — a session via `cal_connect`

**A session-based model**, not environment variables. `cal_connect`
returns a `session_id`, which is passed to all subsequent calls —
closer to how `whisper_transcribe`/other stateful
integrations work than to the email functions above.

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `cal_connect(url, user, pass)` | `String×3` (the password accepts `Secret`, naryad #70) `-> String` | String | A `session_id`. PROPFIND, `calendar-home-set` discovery |
| `cal_list(session_id)` | `String -> String` | String (JSON) | A list of calendars (PROPFIND Depth:1) |
| `cal_events(session_id, start_date, end_date)` | `String×3 -> String` | String (JSON) | Events within a range (CalDAV REPORT `calendar-query`, RFC 4791 section 7.8) |
| `cal_read(event_url)` | `String -> String` | String | A single event by URL |
| `cal_create(calendar_id, summary, start, end, description?, location?, attendees_json?)` | `String×4, String?×3 -> String` | String | Creates a `.ics`, returns a UID. Arity 4..7 |
| `cal_update(event_url, fields_json)` | `String×2 -> String` | String | GET+PUT with `ETag`/`If-Match` |
| `cal_delete(event_url)` | `String -> String` | String | DELETE with `If-Match` |
| `cal_freebusy(session_id, start_date, end_date)` | `String×3 -> String` | String (JSON) | CalDAV REPORT `free-busy-query`, RFC 4791 section 7.10 |
| `ical_parse(text)` | `String -> String` | String (JSON) | Parses iCalendar text (RFC 5545) |
| `ical_generate(json)` | `String -> String` | String | Builds a `VEVENT`+`VCALENDAR` from JSON |

#### Contacts (CardDAV/vCard) — the same session model

| Function | Signature | Return | Description |
|---------|-----------|---------|----------|
| `card_connect(url, user, pass)` | `String×3` (`Secret`-compatible) `-> String` | String | A `session_id`. PROPFIND, `addressbook-home-set` discovery |
| `card_list(session_id)` | `String -> String` | String (JSON) | A list of address books |
| `card_contacts(session_id, addressbook_id)` | `String×2 -> String` | String (JSON) | A book's contacts (CardDAV REPORT `addressbook-query`, RFC 6352 section 8.6) |
| `card_read(contact_url)` | `String -> String` | String | A single contact by URL |
| `card_create(addressbook_id, fn, email, tel?, org?, title?, note?)` | `String×3, String?×4 -> String` | String | Creates a `.vcf`, returns a UID. Arity 3..7 |
| `card_update(contact_url, fields_json)` | `String×2 -> String` | String | GET+PUT with `ETag`/`If-Match` |
| `card_delete(contact_url)` | `String -> String` | String | DELETE with `If-Match` |
| `card_search(session_id, query)` | `String×2 -> String` | String (JSON) | Searches all books (`FN` + `EMAIL`) |
| `vcard_parse(text)` | `String -> String` | String (JSON) | Parses vCard text (RFC 6350) |
| `vcard_generate(json)` | `String -> String` | String | Builds a vCard v4.0 from JSON |

> **Connection configuration** — unlike the email functions above,
> `cal_connect`/`card_connect` **do not have** a built-in fallback to
> environment variables — all three arguments (`url`, `user`, `pass`)
> are required on every call. The naming convention `CALDAV_URL`/
> `CALDAV_USER`/`CALDAV_PASS`, `CARDDAV_URL`/`CARDDAV_USER`/
> `CARDDAV_PASS` is a recommendation for `.mlog` code that itself
> reads them via `env()` and passes them to `connect()`, not
> behavior of the builtin itself.

### 4.21. Reflex — local neural models (naryads #177-185, ADR-0112/0114/0117)

The `Reflex` pillar trains, predicts, persistently stores, and distills local neural models. The LLM acts as the teacher, the local head as the student (ADR-0112). Models are full-fledged language declarations (`reflex Name { ... }`), opaque to `Value`: only the `ReflexId` handle enters the value, weights never leak.

**Boundary per ADR-0117 section 3** — `reflex` and `reflex_seq` classify into a **closed set of labels** (`labels: [...]`), they do not generate text. Free-form token-by-token generation is explicitly out of scope. This is a symmetric restriction for both plain `reflex` and `reflex_seq`.

| Function | Signature | Returns | Description |
|---|---|---|---|
| `reflex_train(model, data, epochs, metric, threshold)` | `(Reflex, List<List<Float>>, Float, String, Float) -> Struct` | `Struct{loss: Float, accuracy: Float, metric: String, threshold_met: Bool}` | Trains model `model` on `data`. Each row of `data` is `[features..., class_idx]` (the last element is the label index). An 80/20 holdout split (ADR-0115), a minimum of 10 examples. `epochs` is the number of epochs (>=0), `metric` is a metric name from `METRIC_REGISTRY` (usually `"accuracy"`), `threshold` is a 0.0..1.0 threshold for `threshold_met`. `learning_rate` is fixed at 0.1. |
| `reflex_predict(model, input)` | `(Reflex, List<Float>) -> Fluid` | A `Fluid` with variants per label | Predicts for a new input. Returns a `Fluid` with one variant per label: `type_name: "Label"`, `value: String(label_name)`, `confidence: Float` (the softmax probability). Variants are sorted by descending confidence — `to_string(fluid)` shows the label with the highest confidence. |
| `reflex_save(model)` | `(Reflex) -> Unit` | `Unit` | Saves the trained weights plus metadata to SQLite (configured via `memory { persist: "path.db" }`, ADR-0116). The key is the model's name from its declaration. Format checks: a `REFLEX_VERSION` or shape mismatch is an explicit error, not silent corruption. |
| `reflex_load(name)` | `(String) -> Reflex` | `Reflex` (a handle to the existing model) | Loads the weights for a previously saved model and applies them to the *current* `reflex` declaration with the same name. Does not register a new model — it mutates the weights of the existing one. Errors on a shape mismatch if the declaration has changed. |
| `reflex_metrics(model)` | `(Reflex) -> Struct` | `Struct{name, is_trained, last_metric, input_size, labels}` (Naryad #187) | Read-only introspection: returns the model's metadata (NOT the weights, ADR-0114). `is_trained: Bool` (true if `last_metric` is set), `last_metric: Float\|Unit` (the last accuracy/loss, `Unit` if untrained), `input_size: Float`, `labels: List<String>`. Works for both `reflex` and `reflex_seq`. |
| `reflex_list()` | `() -> List<String>` | `List<String>` (Naryad #187) | Returns the names of all declared `reflex`/`reflex_seq` models in declaration order. Read-only — for monitoring, dashboards, an externally-checked `rollback_if`. |

**The `reflex` declaration** (naryad #178):

```mlog
reflex SentimentClassifier {
  input: embedding(2)                          // input dimensionality
  layers: [dense(8, relu), dense(2, softmax)]  // layers from LAYER_REGISTRY
  labels: ["positive", "negative"]              // a closed set of labels (ADR-0117 section 3)
  seed: 42                                     // deterministic weight init (xorshift64)
}
```

**The `reflex_seq` declaration** (naryads #183-185, ADR-0119) — for sequences:

```mlog
reflex_seq TinyClassifier {
  input: embedding(64)
  seq_len: 16                                  // a fixed sequence length
  layers: [attention(4, 64)]                   // SequenceLayer types from SEQUENCE_LAYER_REGISTRY
  labels: ["signal", "noise"]                  // required for reflex_seq (ADR-0117 section 3)
  seed: 42
}
```

`reflex_seq` requires the optional `candle` feature (`cargo build --features candle`, ADR-0118). Without it, the declaration fails with a clean error. Available SequenceLayer types: `attention(heads, dim)`, `rms_norm(dim, [eps])`, `swiglu(dim, ff_dim)`, `transformer_block(heads, dim, ff_dim)`.

**Distillation** — a `learnable pattern` can `distill_to` a reflex model (naryad #181):

```mlog
learnable pattern Classify(text: String) -> String {
  distill_to: SentimentClassifier
  fallback_if: confidence < 0.85
  // pattern body (call_llm) — the LLM is called in TEACHING mode,
  // then replaced by the local head once confidence >= threshold
}
```

**The labels contract** — the absence of `labels` in `reflex` or `reflex_seq` is a parse-time error (ADR-0117 section 3, symmetric for both kinds). Not a panic, not silent.

---

### 4.22. Vision — provenance, persistence, LoRA adapters, and safe weight loading (naryads #210-#244, ADR-0122/0124/0125)

The `Vision` pillar generates images from `.mlog` (the `vision "name" { ... }` declaration plus `vision_generate`, naryads #238/#240). As of R5 (naryad #241, ADR-0125), every generated artifact is **signed by construction**: an LSB watermark in the PNG (the `MLGV` magic bytes plus the model hash) and a provenance manifest (model id, weights-tree SHA, seed, prompt hash, policy, timestamp, the final PNG's SHA). Security is a type, not a procedure.

This section documents the **real** built-in provenance/persistence/loading functions. The `vision_generate`/`vision_edit`/`vision_list`/`vision_export`/`vision_export_raw`/`vision_save`/`vision_load`/`vision_lora_load`/`vision_lora_generate`/`vision_fetch_weights` family is intercepted (or dispatched) by the interpreter/VM (registry state and/or the program's database connection) and is specified in ADR-0122/0124/0125; `vision_edit` has been a real in-context editing path since #243 (R6.2); `vision_save`/`vision_load` are real SQLite-persistence paths since #242 (R6.1); `vision_lora_load`/`vision_lora_generate` are real LoRA-adapter paths since #244 (R6.3 — the family reached the ADR-0124 §3 ceiling of 10 functions).

**Composite semantics of `model_sha256` with a LoRA adapter applied (#244, Block 2.4)**: on the `vision_lora_generate` path the manifest field carries the composite fingerprint `sha256("{base}\nlora:{name}:{lora_sha256}")`, where `base` is the plain weights-tree fingerprint (`weights_tree_sha256`, including the honest "unpinned" marker — the composite is honest over the marker too), `name` is the adapter's persistent key in the database, and `lora_sha256` is the SHA-256 of the adapter bytes from the DB (the same value the integrity pin holds). `model_id` and the watermark stay the BASE model — an adapter is a delta, not a model; the formula lives in `vision_lora_composite_model_sha256` and is mirrored in the `VisionManifest::model_sha256` doc comment (the 7 manifest fields are NOT extended).

| Function | Signature | Returns | Description |
|---|---|---|---|
| `vision_edit(handle, prompt)` | `(Vision, String) -> Vision` | `Vision` (a new handle) | **In-context editing of a signed artifact** (#243, R6.2). The source MUST be signed: an artifact with no provenance manifest is a loud refusal (there is nothing honest to inherit; producing an unsigned artifact through the real compute path is forbidden by ADR-0125) — raw export (`vision_export_raw`) is unaffected. Pipeline: source PNG to decode to a loud dims contract (R4.1: 256..=4096, x16; a multiple of the VAE factor — silent resizing is forbidden, the output keeps the source's resolution) to the VAE encoder (non-decoder prefixes of the same pinned VAE file; encode in posterior MODE) to a reference latent to Qwen3-4B on the edit prompt to the Z-Image DiT with in-context token concatenation (a noise branch plus the reference at every step, `euler_step` applies only to the noise branch) to `EDIT_STEPS = 8` (the distilled turbo NFE) to decode to PNG. The output is ALWAYS signed: a watermark plus a manifest, where `model_id`/`policy`/`seed` are inherited from the source, `prompt_sha256` is the hash of the edit prompt, and `timestamp`/`png_sha256` are fresh. Loud refusals: an unsigned source, an unknown handle, a wrong type, an empty prompt, dimensions outside R4.1 or not a multiple of the factor, `MLOG_VISION_WEIGHTS_DIR` not set (the variable and how to set it are named verbatim). UserInput taint on the prompt triggers an audit warning, `VISION_PROMPT_USER_INPUT` (the same check-id as `vision_generate`'s). |
| `vision_save(handle, name)` | `(Vision, String) -> String` | `String` (the name) | **SQLite persistence of an artifact** (#242, R6.1). Writes the artifact (a PNG as a BLOB plus a JSON provenance manifest) to the program's database (the `db { url: "sqlite:..." }` declaration) — the `vision_artifacts` table, whose persistent key is `name` (the registry id is a session-scoped handle and is not persisted). Loud refusals: no database (with a hint at the declaration), an empty name, a name collision (a silent overwrite would be a silent loss of the provenance chain; upsert/delete are out of scope for #242), an unknown handle. A verbatim round trip: the `timestamp` and the manifest's fields are not regenerated. |
| `vision_load(name)` | `(String) -> Vision` | `Vision` (a new handle) | **Loading an artifact from the program's database** (#242, R6.1): exactly the bytes and the manifest that were saved (provenance persistence neither regenerates nor supplements them). Inserted into the session's registry with a new, monotonic id. Loud refusals: no database, an unknown name (with a list of what is saved), a malformed manifest JSON in the database (silent degradation to unsigned is forbidden). An artifact with `manifest: None` loads as-is and is still refused by a signed `vision_export` (`VISION_UNSIGNED_EXPORT`) / works with `vision_export_raw` — #241's backstop survives persistence. |
| `vision_lora_load(name, path)` | `(String, String) -> String` | `String` (the name) | **Loading a LoRA adapter into the SQLite BLOB store** (#244, R6.3; ADR-0124 §6 — the adapter lives ONLY in the program's database, no session state). Reads the safetensors file ONCE — only inside `MLOG_VISION_WEIGHTS_DIR` (a relative path, no traversal, `.safetensors` extension, file must exist; the path contract lives in `vision_lora_check_adapter_path`), validates loudly (both canonical name forms — diffusers-PEFT and ComfyUI; rank = the mean pair dimension, scale = alpha/rank with the loud 1.0 default; non-F32 upcast is loud; targets must be attention projections `to_q/to_k/to_v/to_out.0` per `zimage_expected_keys`; half pairs, non-attention targets, unknown prefixes, orphaned keys, mismatched dimensions are loud errors with the FULL list) and stores the bytes plus fixed-shape metadata (`LoraMeta`: sha256/rank/alpha/scale/targets) into the `vision_lora_adapters` table. The prescribed check order: arity → no-db → env → path → feature gate → read/parse → insert; a name collision is a loud error (no upsert). Returns the persistent key `name`. |
| `vision_lora_generate(decl_name, prompt, lora_name)` | `(String, String, String) -> Vision` | `Vision` (a new handle) | **Generation with a LoRA adapter applied** (#244, R6.3). The same pipeline as `vision_generate`, but the DiT is built through `from_weights_with_lora`: `W' = W + scale·(up@down)` in F32 over the attention projections; the adapter is resolved from the database on every call with a loud **integrity pin** (`sha256(bytes) != meta.sha256` is an error BEFORE any compute). Provenance: the composite `model_sha256` (see above), `model_id`/watermark/policy/seed/steps from the base declaration; sign ALWAYS. Loud refusals: arity/types, empty prompt, unknown decl (with the list), unknown model, no database, unknown `lora_name` (with `lora_list`), corrupted meta JSON, integrity mismatch, missing env/component, non-1024. UserInput-tainted prompts raise the `VISION_PROMPT_USER_INPUT` audit Warning (the same check id; args 0 and 2 are never flagged). |
| `vision_export_raw(handle, path)` | `(Vision, String) -> String` | `String` (the path) | **An explicit opt-out from signing** (ADR-0125). Writes the artifact's PNG bytes as-is: no watermark, no sidecar manifest, no signature requirement. Every call triggers an audit warning, `VISION_UNSIGNED_EXPORT_RAW` (advisory, `mlog audit`). Signed export (PNG + `<path>.manifest.json`) goes through `vision_export`; an artifact with no manifest cannot be exported that way (the runtime backstop `VISION_UNSIGNED_EXPORT`). |
| `vision_fetch_weights(manifest_url, dest_dir)` | `(String, String) -> String` | `String` (dest_dir) | **SSRF-guarded, allowlist-gated, SHA-pinned weight downloading** (ADR-0125's `MODEL_WEIGHTS_UNSAFE`). Layers of protection: (1) the `MLOG_VISION_WEIGHTS_ALLOWLIST` allowlist — **default-deny**: an unset/empty env produces a loud refusal before any network access; (2) an SSRF guard (`check_url_ssrf`): private/loopback/link-local/metadata addresses are forbidden, DNS resolutions are pinned against rebinding; (3) only `manifest.json`-class URLs (a bare `.safetensors` has "no pin" and is refused; pickle-class `.pkl/.pt/.pth/.ckpt/.bin/...` is refused by extension); (4) SHA-256 pinning of every manifest entry (reusing `WeightsManifest`) — a mismatch is a loud refusal, and the file is NOT written; entry names must be bare `*.safetensors`. The static gate `MODEL_WEIGHTS_UNSAFE` (an audit Error, Category A) additionally catches literal URLs with an SSRF-blocked host, a bare `.safetensors`, and the pickle class. The downloaded tree (`manifest.json` plus shards) is consumed by `vision_generate` via `MLOG_VISION_WEIGHTS_DIR` — with re-verification of the SHA at load time (defense in depth). |

**Example** (downloading an allowed weight package and generating):

```mlog
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }

pattern Fetch(dest: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/manifest.json", dest)
}
// then MLOG_VISION_WEIGHTS_DIR=dest -> vision_generate("poster", "...")
```

**Example** (persisting an artifact to the program's database, #242):

```mlog
db { url: "sqlite:gallery.db" }
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }

pattern Keep(id: Vision, name: String) -> String {
    return vision_save(id, name)
}
// vision_load("poster") in a NEW session will return exactly the bytes and
// the manifest that were saved (the id will be different — the id is
// session-scoped, the persistent key is the name).
```

---

## 5. Top-level declarations

### 5.1. Pattern (a function)

```mlog
pattern Name(param1: Type, param2: Type) -> ReturnType {
  // body
  return result
}
```

### 5.2. Learnable Pattern (an AI function)

```mlog
learnable pattern Classify(text: String) -> Category {
  prompt: "Classify this message. Return JSON: {category, confidence}"
  context: auto
  model: "gpt-4"
  max_tokens: 100
  cache: true
  cache_ttl: 5.0 minutes
  max_context_tokens: 4000
}
```

`context` field values:
- `context: recall("query", limit = 5)` — a semantic search in memory
- `context: auto` — automatic strategy selection
- `context: none` — no context
- `context: "literal text"` — literal context

### 5.3. Entity (a data structure)

```mlog
// Defining a type
entity User {
  id: String,
  name: String,
  role: String = "viewer"    // a default value
}

// An instance (record)
entity alice: User = { id: "1", name: "Alice", role: "admin" }

// A simple entity (a single value)
entity db_url: Secret = env("DATABASE_URL")
```

### 5.4. Flow (a pipeline)

```mlog
flow ProcessMessage {
  input: String = "Hello world"
  -> Normalize -> Classify -> checkpoint("classified") -> Format -> output

  Classify {
    result.confidence > 0.8 -> HighConfidenceHandler
    result.confidence < 0.3 -> LowConfidenceHandler
  }
}
```

### 5.5. Rule

```mlog
rule If(status contains "error") then alert.level = "high" with priority = 10
```

### 5.6. Server / MlogServer (an HTTP server)

```mlog
server {
  port: 8080
  host: "127.0.0.1"
  middleware: [session, csrf, security_headers]

  route "/" method=GET {
    respond("200 OK")
  }

  route "/admin" method=GET requires=[admin] {
    respond("200 Secret")
  }
}
```

The `server` keyword is a synonym for `mlogserver`.

Keys: `port` (Int, default 8080), `host` (String, optional, default `"0.0.0.0"`), `middleware`, `route`.
Available middleware: `session`, `csrf`, `security_headers`.

The `csrf` middleware enforces the double-submit pattern STRICTLY (naryad #262): a mutating request (POST/PUT/DELETE) must carry the `_mlog_csrf` cookie AND the matching `X-CSRF-Token` header, and the token must have been ISSUED by this server process (GET responses set the cookie; the token store is process-local, so a restart invalidates outstanding tokens — the request gets 403 «CSRF token validation failed» and a page reload re-issues a fresh token). The token is bound to the session it was issued for: a session-bound token presented with a foreign or missing session is rejected with 403 «CSRF session binding mismatch» (an audit entry is written); a token issued without a session is valid only for sessionless requests. The token TTL is 15 minutes (expired → 403, re-issued on the next GET).

### 5.7. Template (an HTML template)

```mlog
template Page(title: String, body: String) -> Html {
  <!DOCTYPE html>
  <html>
  <head><title>{{ title }}</title></head>
  <body>{{ body }}</body>
  </html>
}
```

The return type `Html` is opaque, providing automatic XSS escaping.

### 5.8. Import (modules)

```mlog
import std/string as str
import std/math
import ./my_utils
import pkg/utils as u

// A qualified call
str.trim("  hello  ")
math.abs(-5.0)
```

### 5.9. Memory

```mlog
// In-memory (default)
memory { }

// With SQLite persistence
memory { persist: "./data/memory.db" }

// With KV configuration
memory { kv: { type: key_value, persist: true } }
```

### 5.10. DB (database)

```mlog
db {
  url: env("DATABASE_URL")
  pool_size: 10
  migrate: "./migrations"
}
```

### 5.11. LLM (provider configuration)

```mlog
llm {
  providers: [
    { alias: openai, provider: openai, key: env("OPENAI_KEY") },
    { alias: claude, provider: anthropic, key: env("ANTHROPIC_KEY"), url: "https://api.anthropic.com" }
  ],
  default_model: "gpt-4",
  failover: auto,
  circuit_breaker: 3,
  timeout: 30
}
```

### 5.12. Hook (lifecycle hooks, ADR-0045 + ADR-0064)

5 lifecycle points (inspired by obsidian-mind):

| Hook | Trigger point | Variables |
|------|-------------------|-------------|
| `hook on_session_start { ... }` | The start of `run()` | (none) |
| `hook on_write { ... }` | Before write builtins | `target`, `args` |
| `hook before_pattern { ... }` | Before a pattern call | `pattern_name`, `args` |
| `hook after_pattern { ... }` | After a pattern returns | `pattern_name`, `args`, `result`, `confidence` |
| `hook on_session_end { ... }` | The end of `run()` | (none) |

Write builtins (trigger `on_write`): `mem_set`, `mtree_store`, `db_execute`, `write_file`, `append_file`.

```mlog
hook on_session_start { mem_set("start_time", now_iso()) }
hook on_write { print("WRITE: " + target) }
hook before_pattern { print("calling: " + pattern_name) }
hook after_pattern { print("result: " + to_string(result)) }
hook on_session_end { print("Session done") }
```

Errors inside hooks are ignored (advisory, not blocking).

### 5.13. Tool (a tool abstraction)

```mlog
tool telegram {
  send(chat_id: String, text: String) -> String {
    http_post("https://api.telegram.org/bot" + token + "/sendMessage",
      json_encode({ chat_id: chat_id, text: text }))
  }
}
```

Call: `telegram.send("123", "hello")`.

### 5.14. Eval (testing patterns)

```mlog
eval Classify {
  dataset: [
    ("Hello", "greeting"),
    ("Fix bug #123", "task"),
    ("Please help", "question")
  ],
  metric: accuracy,
  threshold: 0.8
}
```

Run with: `mlog eval file.mlog`.

### 5.15. Sandbox, Mutate, Adapt, Memorize, Forget, Relate

```mlog
// Sandbox (execution restriction)
sandbox safe_executor {
  allowed: [upper, lower, trim, split],
  forbidden: [http_post, http_get, write_file],
  timeout: 5
}

// Mutate (adaptation with rollback)
mutate Classify {
  add_example("new input", "new output")
  rollback_if: accuracy < 0.7
}

// Adapt (adding an example)
adapt Classify add_example("input", "output")

// Memorize / Forget (semantic memory)
memorize "important fact" with priority = 0.9
forget "outdated fact" after 30.days

// Relate (a knowledge graph)
relate entity1 to entity2 as "relationship"
```

**Where `memorize` / `forget` / `relate` are allowed (naryad #266)**: in TWO
positions with identical semantics — at top level (as declarations) and as
STATEMENTS inside `pattern`, `route`, `hook`, `tool` and `test` bodies:

```mlog
pattern Remember(fact: String) -> String {
  memorize fact with priority=0.8   // statement form: sees pattern params/locals
  return "ok"
}
```

Inside a body the value/query expressions are evaluated in the body's
environment, so pattern parameters and locals are visible. Both backends
(tree-walking interpreter and VM) execute the statement form identically
(`mlog check` accepts it; before naryad #266 such lines silently degraded
into garbage statements — "token soup" — and failed only at runtime).
Top-level placement remains a declaration evaluated against global entities.

**Note on `accuracy` in `rollback_if` (mock value)**: in the current implementation the `accuracy` compared in `rollback_if` is a fixed mock value (0.95), not a real accuracy computation — the rollback logic exists and is tested, but does not yet respond to actual quality degradation. Revisit point (recorded 2026-09-10 after an external audit): revisit only on a real `mutate` use case where the mock value creates a concrete problem (ADR-0112 addendum).

**Sandbox `timeout` caveat**: `timeout > 0` cancels BOTH the calling thread's wait AND the underlying LLM request at the deadline, on every call path: SmartRouter routes via the HTTP client timeout (real TCP drop; Naryad №156); the legacy backend via `call_with_deadline` (Naryad №248 — RealLlm drops the TCP connection at min(deadline, 120s), the mock sleeps min(delay, deadline)). External on-demand abort is out of scope — revisit when a real use case appears (server client-disconnect or a language construct).

### 5.16. Conversation (configuration)

```mlog
conversation {
  ttl: 1800,
  max_messages: 50,
  compress_after: 20
}
```

### 5.17. Fluid Types (probabilistic types)

```mlog
fluid x = String["answer"][0.9] or String["question"][0.1]
```

### 5.18. Type Aliases

**Naryad #119.** Type aliases allow creating a short name for an existing type. An alias fully inherits the target type's semantics, including the protections of opaque types (e.g. Secret).

```mlog
type Token = Secret
type UserName = String

entity api_key: Token = "sk-..."    // gets Secret's protection
entity name: UserName = "Alice"      // equivalent to String
```

Chains of aliases are resolved automatically (max depth: 10):

```mlog
type A = B
type B = String
// A -> B -> String
```

Cyclic aliases are detected at startup and raise an error:

```mlog
type X = Y
type Y = X   // ERROR: cyclic type alias
```

The syntax does not conflict with `type` inside `memory { kv: { type: key_value } }` — the PEG grammar distinguishes them by context.

### 5.19. Test (unit tests)

```mlog
test "test name" {
  let result = PatternName(args)
  assert_eq(result, expected)
}
```

Run with: `mlog test file.mlog`. The `--filter=substring` flag filters tests by a substring in their name.

---

## 6. Stdlib (standard library)

The standard library lives in `std/` and is imported via `import`:

### std/string

```mlog
import std/string as str

str.trim(s: String) -> String       // trim whitespace
str.replace(s, old, new) -> String  // replace all occurrences
str.split(s, sep) -> List           // split by separator
str.join(items, sep) -> String      // join list into string
```

### std/math

```mlog
import std/math

math.abs(n: Float) -> Float         // absolute value
math.min(a, b) -> Float             // minimum
math.max(a, b) -> Float             // maximum
math.clamp(val, lo, hi) -> Float    // clamp to range
math.round(n) -> Float              // round to nearest
```

### std/collections

```mlog
import std/collections

collections.first(items: List) -> String   // first element
collections.last(items: List) -> String    // last element
collections.push(items, item) -> List      // append to list
```

> **Note:** All stdlib functions are also available as top-level builtins (without importing):
> `trim()`, `replace()`, `split()`, `join()`, `abs()`, `min()`, `max()`, `clamp()`, `round()`, `first()`, `last()`.

---

## 7. Changelog (brief)

| Version | Date | What's new |
|--------|------|------------|
| **0.12.0** | 2025-08 | PDF builtins (pdf-inspector), typed memory (FTS5 BM25 + cosine RRF), modular builtins structure |
| **0.11.0** | 2025-07 | obsidian-mind: 5 lifecycle hooks, config_load YAML (ADR-0064, ADR-0065) |
| **0.10.0** | 2025-06 | obsidian-mind: semantic_search, config_load, vault_validate |
| **0.9.5** | 2025-06 | OpenPlanter: fuzzy, hashline, compact, budget, replay, policy (ADR-0063) |
| **0.9.4** | 2025-06 | AgentSkillOS: recipe system, DAG orchestration (ADR-0062) |
| **0.9.3** | 2025-06 | sqz: string/list/token utilities (ADR-0058+) |
| **0.9.1** | 2025-06 | Collection ops sync, BUILTIN_REGISTRY SSOT |
| **0.8.9** | 2025-06 | Fix: else-branch, BlockIfElse mutation, 4 contract tests |
| **0.8.0** | 2025-05 | Time, weather, geo, reminders, HTTP server, encryption, auth, CSRF, OWASP |
| **0.7.x** | 2025-05 | Telegram bot, memory tree L0/L1/L2, cron, goals/todos |
| **0.6.x** | 2025-05 | let/if/each/while, modules, break/continue, match |
| **0.4.0** | 2025-06-03 | Phase 6: HTTP server, templates, DB, encryption, auth, CSRF, bot integration, 40+ builtins |
| **0.3.0** | — | Phase 5: let/if, each/while, List literals, string operations, modules, REPL |
| **0.2.0** | — | Phases 1-4: fluid types, knowledge graph, vector recall, CLI, codegen |
| **0.1.0** | — | M1-M5: entity, rule, learnable pattern, semantic memory, sandbox, adapt |

See the full CHANGELOG in [`CHANGELOG.md`](CHANGELOG.md).
See the architecture decisions in [`docs/adr/`](docs/adr/).

<!-- BEGIN GENERATED BUILTIN INDEX (scripts/gen_reference.py — do not edit inside) -->

## 6. Builtin Index — 394 registered builtins (100% of `spec!`)

> Generated from `BUILTIN_REGISTRY` (`src/builtins/registry.rs`) by `scripts/gen_reference.py` — the SSOT per `AGENT.md` §5. Arity follows ADR-0095 (`variadic` = any count). Descriptions are imported from the curated sections above when present, otherwise from the handler's doc comment; `TODO(doc)` marks a description nobody has written yet — `tests/reference_consistency.rs` keeps the NAMES at 100%, humans keep the prose honest.

### `bot` — 35 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `answer_callback_query(...)` | 1..3 | — | `answer_callback_query(callback_query_id, text?, show_alert?)` — respond to Telegram inline keyboard callback. `callback_query_id` from update.callback_query.id. `text` — notification text (max 200 chars). `show_alert` — 1.0 = alert popup, 0.0 = toast (default). |
| `ask_approval(...)` | 1 | — | `ask_approval(title, description)` — create an approval request. Returns Struct { id, title, description, approved, status }. The `approved` field is 0.0 (pending). Use kv_get("approval:<id>") to poll. In Telegram bot context, this would generate an inline keyboard. |
| `cancel_remind(...)` | 1 | — | `cancel_remind(id)` — cancel reminder. Returns "ok" or "not_found". |
| `check_reminders(...)` | variadic | — | `check_reminders()` — get due reminders. One-shot deactivated; recurring advanced. |
| `compress_html(...)` | 1 | — | `compress_html(html)` — convert HTML to clean readable text. Strips all tags, decodes HTML entities, adds newlines at block boundaries. CJK characters preserved grapheme-by-grapheme. Returns compressed String. |
| `edit_message_text(...)` | 3..4 | — | `edit_message_text(chat_id, message_id, text, reply_markup?)` — edit existing Telegram message. Used to update inline keyboard buttons after callback. |
| `estimate_tokens(...)` | 1 | — | `estimate_tokens(text)` — rough token count heuristic (len / 4 for CJK+Latin mix). ADR note: temporary heuristic, replace with proper tokenizer when available. |
| `extract_entities(...)` | 1 | — | `extract_entities(text)` — extract named entities from text using regex heuristics. Returns List of Struct { kind, name, start, end }. Kinds detected: person (capitalized word sequences), email, url, phone, date. |
| `extract_param(...)` | 2 | — | `extract_param(text, index)` — parse colon-separated callback_data, return N-th segment. Example: extract_param("dept:osp:watch:42", 2) → "watch" |
| `get_profile(...)` | variadic | — | `get_profile()` — get all active user preferences. Returns List of Struct { class, key, value, evidence, state }. |
| `goal_complete(...)` | variadic | — | `goal_complete()` — mark the current thread goal as complete. Returns Struct { status, objective }. |
| `goal_get(...)` | variadic | — | `goal_get()` — get the current thread goal. Returns Struct or empty struct if no goal is set. |
| `goal_set(...)` | 2 | — | `goal_set(objective, budget?)` — set the current thread goal. Returns Struct { objective, status, budget, spent }. |
| `goals_add(...)` | 1 | — | `goals_add(text)` — add a long-term goal (max 8). Returns Struct { id, text, status }. |
| `goals_list(...)` | variadic | — | `goals_list()` — list all long-term goals. Returns List of Struct { id, text, status }. |
| `goals_reflect(...)` | variadic | — | `goals_reflect()` — returns a summary of goals for reflection. This is a stub: real implementation would call LLM to evaluate goals. Returns Struct { goal_count, active, status }. |
| `human_create(...)` | 2 | — | `human_create(name, traits)` — create or update a persona. `traits` is a string describing personality: "friendly, professional, speaks Russian". Stores persona in KV under `human_persona:{name}`. Returns Struct {name, traits, created_at, memory_count}. |
| `human_delete(...)` | 1 | — | `human_delete(persona)` — delete a persona and all its memories. Returns Struct {deleted_memories: Float, status: String}. |
| `human_forget(...)` | 2 | — | `human_forget(persona, key?)` — delete a specific memory or all memories for a persona. With 2 args: deletes specific memory by key. Returns "ok" or "not_found". With 1 arg: deletes ALL memories for persona. Returns count of deleted memories. |
| `human_mood(...)` | 3 | — | `human_mood(persona, mood?, intensity?)` — get or set persona's emotional state. With 1 arg: returns current mood as Struct {mood, intensity, updated_at}. With 2+ args: sets mood. `intensity` is 0.0–1.0 (default 0.5). `mood` examples: "happy", "sad", "focused", "creative", "neutral", "excited". |
| `human_personas(...)` | variadic | — | `human_personas()` — list all created personas. Returns List of PersonaSummary structs: {name, traits, mood, memory_count, created_at}. |
| `human_recall(...)` | 3 | — | `human_recall(persona, query, limit?)` — search persona's memories by keyword match. Returns List of Memory structs sorted by importance (descending), then by recency. Each struct: {key, content, importance, created_at, access_count, relevance}. |
| `human_remember(...)` | 4 | — | `human_remember(persona, key, content, importance?)` — store a memory in persona's memory tree. `importance` is 0.0–1.0 (default 0.5). Higher importance = recalled first. Stores as KV entry `human_mem:{persona}:{key}` with metadata. |
| `human_respond(...)` | 2 | — | `human_respond(persona, message, context?)` — generate a human-like response. Uses the persona's traits, mood, and recalled memories to craft a response via LLM. `context` is optional additional context (e.g., conversation history). Returns the generated response as String. |
| `learn_preference(...)` | 2 | — | `learn_preference(class, key, value)` — record a preference observation. class: "style" \| "identity" \| "tooling" \| "veto" \| "goal" \| "channel" Stores in KV under "pref:<class>:<key>" with timestamp and evidence count. Returns Struct { class, key, value, status }. |
| `list_reminders(...)` | variadic | — | `list_reminders()` — list all active reminders. |
| `memory_score(...)` | 1 | — | `memory_score(text, metadata?)` — score a text chunk for memory admission. Returns Struct { score, admitted, signals: {token_count, unique_words, entity_density} }. Signals: token_count: 0-1, plateau over chunk size (10-8000 tokens) unique_words: 0-1, type-token ratio (lexical diversity) entity_density: 0-1, entities per token (capped) Admission threshold: score >= 0.3 |
| `read_file_tokens(...)` | 1 | — | `read_file_tokens(path)` — read file and return {content, tokens} struct. Convenience for skill_index: read skill file + estimate its token cost in one call. |
| `remind(...)` | 3 | — | `remind(message, timestamp, data?)` — one-time reminder. Returns ID. |
| `remind_recurring(...)` | 2 | — | `remind_recurring(message, interval_seconds, data?)` — recurring reminder. Returns ID. |
| `send_document(...)` | 2..3 | — | `send_document(chat_id, file_path, caption) → { ok }` |
| `send_message(...)` | 2..3 | `String\ | Unit |
| `todo_add(...)` | 2 | — | `todo_add(title, status?)` — add a todo card. Default status: "todo". Returns Struct { id, title, status, created_at }. |
| `todo_list(...)` | variadic | — | `todo_list()` — list all todos. Returns List of Struct { id, title, status, created_at }. |
| `todo_update(...)` | 2 | — | `todo_update(id, new_status)` — update a todo's status. Returns Struct { id, old_status, new_status, updated }. |

### `calendar` — 10 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `cal_connect(...)` | 3 | `String×3` (the password accepts `Secret`, naryad #70) `-> String` | A `session_id`. PROPFIND, `calendar-home-set` discovery |
| `cal_create(...)` | 4..7 | `String×4, String?×3 -> String` | Creates a `.ics`, returns a UID. Arity 4..7 |
| `cal_delete(...)` | 1 | `String -> String` | DELETE with `If-Match` |
| `cal_events(...)` | 3 | `String×3 -> String` | Events within a range (CalDAV REPORT `calendar-query`, RFC 4791 section 7.8) |
| `cal_freebusy(...)` | 3 | `String×3 -> String` | CalDAV REPORT `free-busy-query`, RFC 4791 section 7.10 |
| `cal_list(...)` | 1 | `String -> String` | A list of calendars (PROPFIND Depth:1) |
| `cal_read(...)` | 1 | `String -> String` | A single event by URL |
| `cal_update(...)` | 2 | `String×2 -> String` | GET+PUT with `ETag`/`If-Match` |
| `ical_generate(...)` | 1 | `String -> String` | Builds a `VEVENT`+`VCALENDAR` from JSON |
| `ical_parse(...)` | 1 | `String -> String` | Parses iCalendar text (RFC 5545) |

### `chart` — 8 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `chart_area(...)` | 2 | — | chart_area: area chart — same shape as chart_line, but the path closes down to the baseline (chart_y_bottom) and is filled with a translucent accent color. A solid stroke line is drawn on top for definition. |
| `chart_bar(...)` | 2 | — | `chart_bar(data, style?)` — vertical bar chart as SVG from a List of numbers or {label, value} Structs; optional style Struct (colors, size); deterministic output (golden-test invariant). |
| `chart_boxplot(...)` | 2 | — | chart_boxplot: per-label statistical box-and-whisker plot. |
| `chart_donut(...)` | 2 | — | `chart_donut(data, style?)` — donut chart as SVG with a right-side legend (same layout family as chart_radar); deterministic output. |
| `chart_heatmap(...)` | 2 | — | Parse "#rrggbb" hex color to (h, s, l) in HSL space. h: 0..=360 degrees, s: 0..=1, l: 0..=1. Returns None if the string is malformed (wrong prefix, wrong length, or non-hex digits). |
| `chart_line(...)` | 2 | — | chart_line: line chart with one `<path>` through all points. |
| `chart_radar(...)` | 2 | — | chart_radar: multi-series radar chart in polar coordinates. |
| `chart_scatter(...)` | 2 | — | chart_scatter: scatter plot with two independent numeric axes. |

### `contacts` — 10 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `card_connect(...)` | 3 | `String×3` (`Secret`-compatible) `-> String` | A `session_id`. PROPFIND, `addressbook-home-set` discovery |
| `card_contacts(...)` | 2 | `String×2 -> String` | A book's contacts (CardDAV REPORT `addressbook-query`, RFC 6352 section 8.6) |
| `card_create(...)` | 3..7 | `String×3, String?×4 -> String` | Creates a `.vcf`, returns a UID. Arity 3..7 |
| `card_delete(...)` | 1 | `String -> String` | DELETE with `If-Match` |
| `card_list(...)` | 1 | `String -> String` | A list of address books |
| `card_read(...)` | 1 | `String -> String` | A single contact by URL |
| `card_search(...)` | 2 | `String×2 -> String` | Searches all books (`FN` + `EMAIL`) |
| `card_update(...)` | 2 | `String×2 -> String` | GET+PUT with `ETag`/`If-Match` |
| `vcard_generate(...)` | 1 | `String -> String` | Builds a vCard v4.0 from JSON |
| `vcard_parse(...)` | 1 | `String -> String` | Parses vCard text (RFC 6350) |

### `convert` — 3 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `float(...)` | 1 | `String\ | Float |
| `to_float(...)` | 1 | `String\ | Bool -> Float` |
| `to_string(...)` | 1 | `Any -> String` | Equivalent to `str()`. Float without `.0` for integers |

### `cron` — 5 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `cron_add(...)` | 2 | `String, String -> String` | Adds a cron job. `cron_expr` is a 5-field cron expression (e.g. `"0 9 * * 1-5"` — every weekday at 9:00). `prompt` is what to run. Returns the job's ID |
| `cron_list(...)` | variadic | `-> List` | A list of all cron jobs. Each element is `CronJob { id, cron_expr, prompt, force_run, run_count, last_run, created_at }` |
| `cron_mark_fired(...)` | 1 | `String -> Float` | Marks a job as run: clears `force_run`, increments `run_count`, updates `last_run`. Returns `1.0` if found, `0.0` if not |
| `cron_remove(...)` | 1 | `String -> Float` | Deletes a job by ID. Returns `1.0` if deleted, `0.0` if not found |
| `cron_run(...)` | 1 | `String -> Float` | Forces a job to run outside its schedule (sets `force_run=true`). Returns `1.0` if found, `0.0` if not |

### `crypto` — 10 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `decrypt(...)` | 2 | `Encrypted, Secret -> String` | Decrypts AES-256-GCM. Errors on a wrong key |
| `encrypt(...)` | 2 | `String, Secret -> Encrypted` | Encrypts with AES-256-GCM using a random 96-bit nonce. The key is 64 hex characters |
| `generate_key(...)` | variadic | `-> Secret` | Generates a 256-bit random key (64 hex characters) |
| `hash_password(...)` | 1 | `String -> Hash` | Hashes a password (Argon2id with a random salt) |
| `hex_decode(...)` | 1 | `String -> String` | Decodes hex to a string. Invalid UTF-8 becomes a `<binary: N bytes>` placeholder |
| `hex_encode(...)` | 1 | `String -> String` | Encodes a string to hex (UTF-8 bytes to hex characters) |
| `hmac_sha256(...)` | 2 | `String, String -> String` | HMAC-SHA256 with a key, hex representation (64 characters) |
| `secret(...)` | 1 | — | `secret(key)` — reads an environment variable as an OPAQUE `Value::Secret`: hard-failure when the variable is missing, the value never prints, concatenates, or converts to String, and `respond(secret(...))` is rejected at compile time (`SECRET_LEAK`, Category A). See ADR-0116-era threat-model row and `env()` for the readable variant. |
| `sha256(...)` | 1 | `String -> String` | The SHA-256 hash, hex representation (64 characters). Unicode-aware: hashes the UTF-8 bytes |
| `verify_password(...)` | 2 | `String, Hash -> Bool` | Verifies a password (constant-time comparison) |

### `db` — 2 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `db_execute(...)` | 1..2 | `String -> Unit` | Executes an SQL query without returning data |
| `query(...)` | 1..2 | `String, List -> List` | An SQL query. SELECT returns List[Row{...}], everything else returns a string with the number of affected rows |

### `diagram` — 21 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `diagram_architecture(...)` | 2 | — | `diagram_architecture(data, style) -> String` |
| `diagram_data_flow(...)` | 2 | — | `diagram_data_flow(data, style) -> String` |
| `diagram_er(...)` | 2 | — | `diagram_er(data, style) -> String` |
| `diagram_flowchart(...)` | 2 | — | `diagram_flowchart(data, style) -> String` |
| `diagram_gantt(...)` | 2 | — | `diagram_gantt(data, style) -> String` |
| `diagram_high_level(...)` | 2 | — | `diagram_high_level(data, style) -> String` |
| `diagram_layers(...)` | 2 | — | `diagram_layers(data, style) -> String` |
| `diagram_loop(...)` | 2 | — | `diagram_loop(data, style) -> String` |
| `diagram_medallion(...)` | 2 | — | `diagram_medallion(data, style) -> String` |
| `diagram_nested(...)` | 2 | — | `diagram_nested(data, style) -> String` |
| `diagram_org_chart(...)` | 2 | — | `diagram_org_chart(data, style) -> String` |
| `diagram_process(...)` | 2 | — | `diagram_process(data, style) -> String` |
| `diagram_pyramid(...)` | 2 | — | `diagram_pyramid(data, style) -> String` |
| `diagram_quadrant(...)` | 2 | — | `diagram_quadrant(data, style) -> String` |
| `diagram_sequence(...)` | 2 | — | `diagram_sequence(data, style?)` — UML-style sequence diagram as SVG: participants as lifelines, ordered messages as arrows between them. |
| `diagram_state(...)` | 2 | — | `diagram_state(data, style) -> String` |
| `diagram_swimlane(...)` | 2 | — | `diagram_swimlane(data, style) -> String` |
| `diagram_timeline(...)` | 2 | — | `diagram_timeline(data, style?)` — horizontal event timeline as SVG: dated events placed on one axis; overlaps detected where possible. |
| `diagram_tree(...)` | 2 | — | `diagram_tree(data, style) -> String` |
| `diagram_venn(...)` | 2 | — | `diagram_venn(data, style) -> String` |
| `infographic_qa(...)` | 1 | `String -> Struct{passed, warnings, checks_run}` | An advisory check: contrast (WCAG, a threshold of 4.5), saturation discipline (more than 2 colors with S>60% triggers a warning), element density. `passed: false` is advice, not a block |

### `email` — 7 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `imap_list(...)` | 2..3 | `String, Float, Float? -> String` | A list of emails — envelope + flags. Arity 2..3 |
| `imap_mark_read(...)` | 1 | `String -> String` | Marks it as read |
| `imap_move(...)` | 2 | `String, String -> String` | Moves it (RFC 6851 `MOVE`, falling back to `COPY`+`DELETE`) |
| `imap_read(...)` | 1 | `String -> String` | The full email: headers, body, attachments |
| `imap_search(...)` | 2 | `String, String -> String` | A text search (the IMAP `TEXT` criteria) |
| `smtp_send(...)` | 3..6 | `String×3, String?×3 -> String` | Sends a plain-text email, TLS/STARTTLS. Arity 3..6 |
| `smtp_send_html(...)` | 3..4 | `String×3, String? -> String` | An HTML email. Arity 3..4 |

### `encoding` — 4 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `base64_decode(...)` | 1 | `String -> String` | Decodes Base64. Errors if not valid Base64 or not UTF-8 |
| `base64_encode(...)` | 1 | `String -> String` | Encodes a string as Base64 (standard alphabet). Unicode-aware: encodes the UTF-8 bytes |
| `toon_decode(...)` | 1 | `String -> Any` | Decodes a TOON string back into a Value. A recursive-descent parser |
| `toon_encode(...)` | 1 | `Any -> String` | Encodes a value as TOON (Token-Optimized Object Notation). Prefixed with `TOON:`. Any Value becomes a string |

### `fluid` — 2 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `budget_check(...)` | 2 | `Float, Float -> Dict` | Returns `BudgetStatus { step, total_steps, remaining, fraction, over_budget }`. Errors if `total_steps == 0` |
| `confidence(...)` | 1 | `Fluid -> Float` | Returns the maximum confidence of the probabilistic type. Returns `1.0` for concrete values |

### `graph` — 8 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `graph_neighbors(...)` | variadic | `String -> String` | A node's neighbors (direct links). A JSON array of nodes |
| `graph_path(...)` | 2 | `String, String -> String` | The shortest path between two nodes. Returns a JSON array of nodes on the path. An empty array if there is no path |
| `graph_query(...)` | 1..3 | `String, Float?, String? -> String` | Searches the graph with an optional level filter (`"L0"`, `"L1"`, `"L2"`). `limit` defaults to 5 |
| `subgraph_extract(...)` | variadic | `String, Float -> String` | Extracts a subgraph from `root_id` to a given `depth`. JSON with nodes and edges |
| `subgraph_json(...)` | variadic | `String, Float -> String` | The full subgraph as JSON (nodes + edges). Ready for visualization |
| `subgraph_nodes(...)` | variadic | `String, Float -> String` | Only the subgraph's nodes (no edges). A JSON array of nodes |
| `trace_end(...)` | variadic | `String -> Dict` | Ends a trace segment. Returns `TraceResult { id, label, duration_ms }` |
| `trace_start(...)` | variadic | `String -> String` | Starts a trace segment with a label. Returns a trace_id |

### `io` — 12 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `append_file(...)` | 2 | `String, String -> String` | Appends to the end of a file. Returns `"ok"` or `""`; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `delete_file(...)` | 1 | `String -> String` | Deletes a file. Returns `"ok"`, `""` when the file is missing; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `exec(...)` | 1 | — | `exec(cmd)` — execute a shell command and return stdout. |
| `exec_argv(...)` | 1..2 | — | `exec_argv(binary: String, args: List<String>) -> String` |
| `file_exists(...)` | 1 | `String -> Bool` | Checks whether a file exists |
| `git_push(...)` | 1 | — | `git_push(message?) -> String` — git add/commit/push via subprocess. Uses GITHUB_TOKEN and GITHUB_REPO env vars for authentication. Usage: git_push("commit message") -> "ok" \| "nothing to commit" \| error |
| `list_dir(...)` | 1 | `String -> List` | A list of files in a directory. With no argument — the current directory |
| `mcp_call(...)` | 4 | — | `mcp_call(command, args_list, tool_name, arguments_json) -> String`. |
| `mcp_list_tools(...)` | 2 | — | `mcp_list_tools(command, args_list) -> List[Struct{name, description, input_schema}]`. |
| `print(...)` | 1 | `String -> String` | Prints a string to stdout, returns it |
| `read_file(...)` | 1 | `String -> String` | Reads a file. Soft-failure: an empty string when the file is missing or unreadable. Sandbox violations (absolute path, `..`, symlink escape, broken symlink) are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |
| `write_file(...)` | 2 | `String, String -> String` | Writes a file (overwrite). Returns `"ok"` or `""` on an OS-level error; sandbox violations are a loud `[SANDBOX_VIOLATION]` error (Naryad #254) |

### `json` — 9 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `dict_get(...)` | 3 | `Struct, String, Any -> Any` | Dict-style key access. Returns `default` if the key is absent. Does not support dot-paths (use `json_get` for that) |
| `dict_has(...)` | 2 | `Struct, String -> Bool` | `true` if the key exists in the dict. A direct check (no dot-path) |
| `dict_keys(...)` | 1 | `Struct -> List` | Returns a list of all the dict's keys |
| `dict_set(...)` | 3 | `Struct, String, Any -> Struct` | Returns a **new** Struct with the key updated. The original dict is not mutated |
| `dict_values(...)` | 1 | `Struct -> List` | Returns a list of all the dict's values (order matches `dict_keys`) |
| `has_field(...)` | 2 | `Struct, String -> Float` | `1.0` if the field exists, `0.0` if not. Supports dot-paths |
| `json_encode(...)` | 1 | `Any -> String` | Serializes a value to a JSON string. Supports String, Float, Bool, Unit→null, List→array, Struct→object |
| `json_get(...)` | 2..3 | `Struct, String -> Value` | Accesses a field by a dot-path. Returns the **real value** (including String). Returns Unit if the field is absent or a SQL NULL. Supports dot-paths: `"voice.file_id"` |
| `parse_json(...)` | 1..2 | `String -> Struct\ | String\ |

### `list` — 17 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `chunk(...)` | 2 | — | `chunk(items, size)` — split a list into sublists of at most `size` elements. The last chunk may be shorter. Returns List<List>. Error if size <= 0. |
| `compact_list(...)` | 3 | — | `compact_list(items, keep_first, keep_last)` — context condensation for lists. Protects the first `keep_first` and last `keep_last` items, replaces middle items with a single placeholder struct { compacted: true, removed_count: N }. Inspired by OpenPlanter's compact_messages() for managing long conversation histories. |
| `condense(...)` | 1 | — | `condense(list)` — collapse consecutive identical string elements with count. Example: ["a","a","b"] -> ["a","×2","b"] |
| `dedup(...)` | 1 | `List -> List` | Removes duplicates, keeping the order of first occurrence |
| `filter(...)` | 3 | `List, String, Value -> List` | Filters: field == value |
| `first(...)` | 1 | `List -> Value` | The first element. An empty string if the list is empty |
| `get(...)` | 2 | `List, Float -> Value` | Gets an element by index. Errors on out-of-bounds |
| `last(...)` | 1 | `List -> Value` | The last element. An empty string if the list is empty |
| `make_list(...)` | variadic | — | `make_list(a, b, c, ...) -> List` — create a list from variadic arguments. Eliminates race conditions from write_file/read_file workarounds for returning multiple values from patterns. Usage: make_list("red", "green", "blue") -> List ["red", "green", "blue"] |
| `matches_any(...)` | 2 | — | `matches_any(text, triggers_list)` — case-insensitive substring match. Returns 1.0 if ANY trigger string is found in text, 0.0 otherwise. Used by skill_index tier matching (Problem A). |
| `push(...)` | 2 | `List, Value -> List` | Appends an element (returns a new list) |
| `reduce(...)` | 3 | `List, String, Float -> Float` | Sum of a field's values across the list |
| `slice(...)` | 3 | `List, Float, Float -> List` | Slice of the list [start, end). Soft-failure: start >= len returns an empty list, end > len is clamped, start >= end returns an empty list (ADR-0069) |
| `sort(...)` | 1 | — | `sort(list)` — sort list elements in ascending order. |
| `sort_by(...)` | 2..3 | `List, String, Float -> List` | Sorts structs by a field (desc=1.0 → descending) |
| `unique(...)` | 1 | — | `unique(list)` — remove duplicates preserving first-occurrence order. Uses the same equality semantics as `dedup` (JSON serialization for complex types = deep structural comparison for Struct, not reference identity). |
| `zip(...)` | 2 | `List, List -> List` | Pairwise combination into `Pair{a, b}` |

### `llm` — 4 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `call_claude(...)` | 4 | `String, String, String, String -> String` | A direct call to the Anthropic Claude Messages API (v1/messages). Returns `content[0].text` |
| `call_llm(...)` | 1..2 | `String, String -> String` | Calls the LLM backend. By default returns a mock: `"[MOCK: prompt \ |
| `call_llm_schema(...)` | 2..3 | `String, String[, String] -> Struct` | Calls the LLM backend and requires the answer to be a single JSON value conforming to the schema. Supported schema subset (ADR-0133): `type`, `properties`, `required`, `items`, `enum`; annotation keywords (`title`, `description`, `$schema`, ...) are ignored; any other keyword is a loud `LLM_SCHEMA_UNSUPPORTED_FEATURE`. Answer fields beyond `properties` are rejected (strict-by-default). The result is a `Dict` Struct usable with `json_get`/`has_field`/`dict_*`. Parse/validation failures (including max_tokens truncation) are loud `LLM_SCHEMA_MISMATCH` and retry up to `METALOGOS_LLM_SCHEMA_RETRIES` (default 2, cap 10) with the validator report fed back into the prompt. Mock tier returns a deterministic minimal instance derived from the schema (default mock settings; `METALOGOS_LLM_MOCK=json` documents the intent explicitly) |
| `llm_usage(...)` | variadic | `-> Struct` | LLM usage statistics: `total_calls`, `total_tokens`, `total_errors`, `providers` (a list of `{alias, calls, tokens, errors, avg_latency_ms, health_score}`) |

### `math` — 14 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `abs(...)` | 1 | `Float -> Float` | Absolute value |
| `clamp(...)` | 3 | `Float, Float, Float -> Float` | Clamps a value into the range `[lo, hi]` |
| `exp(...)` | 1 | `Float -> Float` | e^x. `exp(0)=1`, `exp(1)=e` |
| `ln(...)` | 1 | `Float -> Float` | Natural logarithm. Soft-failure: `0.0` for `x <= 0` |
| `max(...)` | 2 | `Float, Float -> Float` | Maximum of two numbers |
| `min(...)` | 2 | `Float, Float -> Float` | Minimum of two numbers |
| `pow(...)` | 2 | `Float, Float -> Float` | base^exp |
| `random(...)` | variadic | `-> Float` | `[0.0, 1.0)`. If `random_seed()` was called — deterministic. Otherwise — non-deterministic (system time) |
| `random_seed(...)` | 1 | `Float -> Unit` | Sets the seed for a deterministic PRNG (xorshift64). Subsequent `random()` calls are reproducible |
| `round(...)` | 1 | `Float -> Float` | Rounds to the nearest integer |
| `sigmoid(...)` | 1 | `Float -> Float` | The logistic function 1/(1+e^−x). Numerically stable: `sigmoid(1000)=1`, `sigmoid(-1000)=0` (not NaN) |
| `softmax(...)` | 1 | `List -> List` | Numerically stable softmax (subtracts max before exp). Output sums to 1.0 |
| `sqrt(...)` | 1 | `Float -> Float` | Square root. Soft-failure: `0.0` for `x < 0` |
| `tanh(...)` | 1 | `Float -> Float` | Hyperbolic tangent. In (−1, 1). `tanh(1000)=1`, `tanh(-1000)=-1` |

### `memory` — 18 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `deref(...)` | 1 | — | `deref(hash)` — retrieve content by SHA-256 hash from ref store. |
| `kv_delete(...)` | 1 | `String -> Unit` | Deletes a key |
| `kv_exists(...)` | 1 | `String -> Bool` | Checks whether a key exists |
| `kv_get(...)` | 1 | `String -> String` | Reads a value (an empty string if the key is absent) |
| `kv_list(...)` | variadic | `-> List` | Returns a list of all keys |
| `kv_set(...)` | 2 | `String, String -> Unit` | Writes a key-value pair |
| `mem_delete(...)` | 1 | `String -> String` | Equivalent to `kv_delete`, returns the deleted value |
| `mem_get(...)` | 1 | `String -> String` | Equivalent to `kv_get` |
| `mem_set(...)` | 2 | `String, String -> String` | Equivalent to `kv_set`, but returns the value written |
| `memorize(...)` | 2..3 | `String, Float, String -> Unit` | Saves a fact with a priority (0.0-1.0) and a type. Example: `memorize("likes spicy food", 0.9, "persona")` |
| `memory_boost(...)` | variadic | — | `memory_boost(id, amount?)` — boost a memory node's score by amount (default 0.1, capped at 1.0). Updates last_accessed timestamp. Returns Struct { id, new_score, access_count }. |
| `memory_decay(...)` | variadic | — | `memory_decay(lambda?)` — apply exponential decay to all memory node scores. `lambda` controls decay rate (default 0.01 = gentle). Formula: score *= e^(-lambda * hours_since_access). Returns Struct { decayed: <count>, nodes: <total>, edges: <total> }. |
| `memory_prune(...)` | variadic | — | `memory_prune(threshold?, min_age_hours?)` — remove dead memory nodes. `threshold`: minimum score to keep (default 0.05). `min_age_hours`: minimum age in hours before pruning (default 24, protects fresh entries). Returns Struct { pruned: <count>, remaining: <total> }. |
| `memory_revise(...)` | variadic | — | `memory_revise(id, new_text, new_score?)` — update a node and resolve Contradicts. If the node contradicts others, the system keeps the higher-scoring belief and demotes the loser (score *= 0.3, adds Supersedes edge). Returns Struct { action, winner_id, superseded_id? }. |
| `ref(...)` | 1 | — | `ref(content)` — compute SHA-256 hash, store in KV, return hash string. Idempotent. |
| `session_clear(...)` | variadic | `String -> String` | Deletes all of the session's data. Returns `"ok"` |
| `session_get(...)` | 1 | `String, String -> String` | Reads a value from the session (an empty string if absent) |
| `session_set(...)` | 2 | `String, String, String -> String` | Saves a value in the session |

### `mtree` — 5 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `mtree_forget(...)` | 1 | `String -> Float` | Deletes a node by ID. Returns `1.0` if it existed, `0.0` if not |
| `mtree_retrieve(...)` | 1..2 | `String, Float? -> String` | Searches the memory graph. `limit` defaults to 5. Returns a JSON array of nodes with metadata |
| `mtree_stats(...)` | variadic | `-> Dict` | Returns `MemoryStats { l0_count, l1_count, l2_count, total_nodes, edges }` |
| `mtree_store(...)` | 2 | `String, String? -> String` | Saves text as an L0 node. `source` defaults to `"user"`. Returns the node ID. The admission score is computed from length/uniqueness |
| `mtree_summarize(...)` | variadic | `-> Unit` | L0 to L1 to L2 clustering. Groups L0 nodes into L1 summaries, L1 into L2. Idempotent — a repeat call recomputes the summaries |

### `orchestration` — 2 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `dag_phases(...)` | 1 | — | `dag_phases(dag)` — extract parallel execution phases from a DAG. |
| `topo_sort(...)` | 1 | — | `topo_sort(dag)` — topological sort of a DAG. |

### `pdf` — 25 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `html_to_pdf(...)` | 2 | — | `html_to_pdf(html, path) → { path, size }` |
| `pdf_add_image(...)` | 4..6 | — | `pdf_add_image(id, x, y, image_path [,width, height]) → { ok }` |
| `pdf_add_page(...)` | 3 | — | `pdf_add_page(id, width, height) → { page }` |
| `pdf_classify(...)` | 1 | `String -> Dict` | Classifies a PDF: TextBased / Scanned / ImageBased / Mixed. Keys: type, confidence, pages_needing_ocr, page_count |
| `pdf_create(...)` | variadic | — | `pdf_create() → { id }` |
| `pdf_delete_pages(...)` | 3 | — | `pdf_delete_pages(path, pages_json, output_path) → { ok, pages_remaining }` |
| `pdf_draw_line(...)` | 5..6 | — | `pdf_draw_line(id, x1, y1, x2, y2, width) → { ok }` |
| `pdf_draw_rect(...)` | 5..7 | — | `pdf_draw_rect(id, x, y, w, h, stroke, fill) → { ok }` |
| `pdf_draw_table(...)` | 5..6 | — | `pdf_draw_table(id, x, y, col_widths_json, rows_json [,style_json]) → { ok }` |
| `pdf_extract_images(...)` | 1..2 | — | `pdf_extract_images(path [,output_dir]) → [paths]` |
| `pdf_extract_regions(...)` | 2 | `String, String -> List` | Extracts text regions with coordinates. A list of dicts: text, needs_ocr, ocr_reason, page, x, y |
| `pdf_fill_form(...)` | 3 | — | `pdf_fill_form(path, fields_json, output_path) → { path, fields_filled }` |
| `pdf_merge(...)` | 2 | — | `pdf_merge(paths_json, output) → { path, pages, size }` |
| `pdf_metadata(...)` | 1 | — | `pdf_metadata(path) → { title, author, subject, creator, producer, pages, created, modified }` |
| `pdf_ocr(...)` | 1 | `String -> Dict` | An OCR fallback for scans (requires `--features pdf-ocr` and a system Tesseract). Keys: markdown, ocr_confidence, pages_processed |
| `pdf_page_numbers(...)` | 1..4 | — | `pdf_page_numbers(id [,format, x, y]) → { ok }` |
| `pdf_rotate_page(...)` | 4 | — | `pdf_rotate_page(path, page_number, degrees, output_path) → { ok }` |
| `pdf_save(...)` | 2 | — | `pdf_save(id, path) → { path, size }` |
| `pdf_set_metadata(...)` | 3 | — | `pdf_set_metadata(path, key, value) → { ok }` |
| `pdf_set_page_footer(...)` | 2..4 | — | `pdf_set_page_footer(id, text [,font, size]) → { ok }` |
| `pdf_set_page_header(...)` | 2..4 | — | `pdf_set_page_header(id, text [,font, size]) → { ok }` |
| `pdf_split(...)` | 3 | — | `pdf_split(path, ranges_json, output_dir) → { files, pages }` |
| `pdf_to_markdown(...)` | 1 | `String -> Dict` | The full pipeline: classification + text extraction + Markdown. Keys: markdown, page_count, pdf_type, has_tables, confidence, processing_time_ms |
| `pdf_watermark(...)` | 2..5 | — | `pdf_watermark(id, text [,font, size, opacity]) → { ok }` |
| `pdf_write_text(...)` | 4..6 | — | `pdf_write_text(id, x, y, text, font, size) → { ok }` |

### `recipe` — 3 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `recipe_list(...)` | variadic | — | `recipe_list()` — return all known recipe names. args: [] (reads from recipe index key) |
| `recipe_save(...)` | variadic | — | `recipe_save(name, description, skills, plan)` — persist a recipe. args: [name: String, description: String, skills: List, plan: Struct/any] Stores in KV under `__recipe:<name>` as JSON. Updates recipe index. |
| `recipe_search(...)` | 1..2 | — | `recipe_search(query)` — search recipes by description similarity (substring match). args: [query: String] Iterates all recipes stored under `__recipe:*` in KV, returns matching ones. NOTE: This is a simplified implementation using substring matching. Full semantic search (cosine similarity) requires embedding infrastructure. |

### `reflex` — 14 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `reflex_bpe_decode(...)` | 2 | — | `reflex_bpe_decode(tokens: List<Float>, vocab: BpeVocab) -> String` |
| `reflex_bpe_encode(...)` | 2 | — | `reflex_bpe_encode(text: String, vocab: BpeVocab) -> List<Float>` |
| `reflex_bpe_load(...)` | 1 | — | `reflex_bpe_load(name: String) -> BpeVocab` (Наряд №195, ADR-0116) |
| `reflex_bpe_save(...)` | 1 | — | `reflex_bpe_save(vocab: BpeVocab) -> Unit` (Наряд №195, ADR-0116) |
| `reflex_bpe_train(...)` | 2 | — | `reflex_bpe_train(corpus: String, vocab_size: Float) -> BpeVocab` |
| `reflex_detokenize(...)` | 1 | — | `reflex_detokenize(tokens) -> String` |
| `reflex_generate(...)` | 4 | — | Stub — VM not yet supported (Наряд №193 — text generation). |
| `reflex_list(...)` | variadic | `() -> List<String>` | Returns the names of all declared `reflex`/`reflex_seq` models in declaration order. Read-only — for monitoring, dashboards, an externally-checked `rollback_if`. |
| `reflex_load(...)` | 1 | `(String) -> Reflex` | Loads the weights for a previously saved model and applies them to the *current* `reflex` declaration with the same name. Does not register a new model — it mutates the weights of the existing one. Errors on a shape mismatch if the declaration has changed. |
| `reflex_metrics(...)` | 1 | `(Reflex) -> Struct` | Read-only introspection: returns the model's metadata (NOT the weights, ADR-0114). `is_trained: Bool` (true if `last_metric` is set), `last_metric: Float\ |
| `reflex_predict(...)` | 2 | `(Reflex, List<Float>) -> Fluid` | Predicts for a new input. Returns a `Fluid` with one variant per label: `type_name: "Label"`, `value: String(label_name)`, `confidence: Float` (the softmax probability). Variants are sorted by descending confidence — `to_string(fluid)` shows the label with the highest confidence. |
| `reflex_save(...)` | 1 | `(Reflex) -> Unit` | Saves the trained weights plus metadata to SQLite (configured via `memory { persist: "path.db" }`, ADR-0116). The key is the model's name from its declaration. Format checks: a `REFLEX_VERSION` or shape mismatch is an explicit error, not silent corruption. |
| `reflex_tokenize(...)` | 1 | — | `reflex_tokenize(text) -> List<Float>` |
| `reflex_train(...)` | 5 | `(Reflex, List<List<Float>>, Float, String, Float) -> Struct` | Trains model `model` on `data`. Each row of `data` is `[features..., class_idx]` (the last element is the label index). An 80/20 holdout split (ADR-0115), a minimum of 10 examples. `epochs` is the number of epochs (>=0), `metric` is a metric name from `METRIC_REGISTRY` (usually `"accuracy"`), `threshold` is a 0.0..1.0 threshold for `threshold_met`. `learning_rate` is fixed at 0.1. |

### `std` — 11 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `__abs(...)` | 1 | — | `__abs(x)` — std-library primitive behind the std `abs` wrapper. |
| `__clamp(...)` | 3 | — | `__clamp(x, lo, hi)` — std-library primitive behind the std `clamp` wrapper. |
| `__first(...)` | 1 | — | `__first(list)` — std-library primitive behind the std `first` wrapper: the first element (soft-failure semantics of the std layer apply). |
| `__join(...)` | 2 | — | `__join(list, sep)` — std-library primitive behind the std/string `join` wrapper: joins a List of strings with the separator. |
| `__last(...)` | 1 | — | `__last(list)` — std-library primitive behind the std `last` wrapper: the last element (soft-failure semantics of the std layer apply). |
| `__max(...)` | 2 | — | `__max(a, b)` — std-library primitive behind the std `max` wrapper. |
| `__min(...)` | 2 | — | `__min(a, b)` — std-library primitive behind the std `min` wrapper. |
| `__replace(...)` | 3 | — | `__replace(s, from, to)` — std-library primitive behind the std/string `replace` wrapper: replaces every occurrence of `from` with `to`. |
| `__round(...)` | 1 | — | `__round(x)` — std-library primitive behind the std `round` wrapper. |
| `__split(...)` | 2 | — | `__split(s, sep)` — std-library primitive behind the std/string `split` wrapper: splits on the separator into a List of strings. |
| `__trim(...)` | 1 | — | `__trim(s)` — std-library primitive behind the std/string `trim` wrapper: strips leading and trailing whitespace. The `__` prefix marks a primitive used by `std/*.mlog` pattern wrappers (prefer the wrapper in user code). |

### `string` — 45 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `capitalize(...)` | 1 | — | `capitalize(s)` — uppercases the first character and lowercases the rest. |
| `char_at(...)` | 2 | `String, Float -> String` | Returns the character at an index. An empty string on out-of-bounds |
| `chomp(...)` | 1 | — | `chomp(s)` -- remove a single trailing newline (\n or \r\n). |
| `contains(...)` | 2 | `String, String -> Float` | Returns `1.0` if it contains it, `0.0` otherwise |
| `ends_with(...)` | 2 | `String, String -> Bool` | Checks whether the string ends with a suffix |
| `escape_html(...)` | 1 | `String -> String` | Escapes HTML special characters: `& < > " '` |
| `escape_js(...)` | 1 | — | `escape_js(s) -> String` -- escape a string for safe insertion into JavaScript. Escapes: backslash, single quote, double quote, newline, carriage return, tab, line separator, paragraph separator, and NUL. |
| `escape_json(...)` | 1 | `String -> String` | Escapes JSON special characters: `" \ \n \t \r` |
| `format(...)` | variadic | — | `format(template, arg1, arg2, ...)` -- positional string interpolation. Replaces `{}` placeholders in template with arguments. Usage: format("Hello {}, you are {} years old", name, age) |
| `fuzzy_find_best(...)` | 2 | — | `fuzzy_find_best(query, candidates)` — find the best match for `query` in a list of candidate strings. Returns a struct { index, candidate, score } or Unit if list is empty. |
| `fuzzy_match(...)` | 2 | — | `fuzzy_match(a, b)` -- Jaro-Winkler similarity between two strings (0.0..1.0). Ported from OpenPlanter's wiki/matching.rs NameRegistry pattern. |
| `hashline_edit(...)` | 2 | — | `hashline_edit(text, edits)` — apply edits to text using hashline-verified line references. `edits` is a list of structs, each with an `op` field ("set_line", "replace_lines", "insert_after") and corresponding fields. |
| `hashline_read(...)` | 1 | — | `hashline_read(text)` — annotate each line with a 2-char CRC32 hash prefix. Output format: "N:HH\|content" per line. Inspired by OpenPlanter's tools.py hashline system for safe LLM editing. |
| `index_of(...)` | 2 | `String, String -> Float` | Returns the position (in characters, not bytes) of the first occurrence, or `-1.0` |
| `join(...)` | 2 | `List, String -> String` | Joins list elements with a separator. Default `","` |
| `len(...)` | 1 | `String\ | Float |
| `length(...)` | 1 | `String -> Float` | Length of the string in characters (Unicode-aware). Equivalent to `len()` |
| `lines(...)` | 1 | — | `lines(s)` -- split string into list of lines (no trailing empty element). |
| `lower(...)` | 1 | `String -> String` | Converts a string to lowercase |
| `pad_left(...)` | 3 | — | `pad_left(s, n, fill)` -- left-pad string with fill character to length n. |
| `pad_right(...)` | 3 | — | `pad_right(s, n, fill)` -- right-pad string with fill character to length n. |
| `regex_captures(...)` | 2 | — | `regex_captures(pattern, text)` → List |
| `regex_match(...)` | 2 | — | `regex_match(pattern, text)` → Bool |
| `regex_replace(...)` | 3 | — | `regex_replace(pattern, text, replacement)` → String |
| `repeat(...)` | 2 | — | `repeat(s, n)` -- repeat string n times. |
| `replace(...)` | 3 | `String, String, String -> String` | Replaces all occurrences of `old` with `new`. Unicode-aware, works with Cyrillic and emoji |
| `reverse(...)` | 1 | `String -> String` | Reverses the string (per character) |
| `slugify(...)` | 1 | — | `slugify(s)` — URL-safe slug: lowercase, non-alphanumerics collapsed to single hyphens, leading/trailing hyphens trimmed. |
| `split(...)` | 2 | `String, String -> List` | Splits a string by a separator. An empty separator splits per character |
| `squeeze(...)` | 2 | — | `squeeze(s, chars)` -- collapse consecutive identical characters from `chars`. |
| `starts_with(...)` | 2 | `String, String -> Bool` | Checks whether the string starts with a prefix |
| `str(...)` | 1 | `Any -> String` | Converts any value to a string |
| `strip(...)` | 2 | — | `strip(s, chars)` -- remove characters from both ends of string. |
| `substring(...)` | 3 | `String, Float, Float -> String` | Extracts a substring by character indices. Soft-failure: an empty string on out-of-bounds |
| `title_case(...)` | 1 | — | `title_case(s)` — uppercases the first character of every word (previous character non-letter acts as the word boundary). |
| `to_int(...)` | 1 | `String\ | Bool -> Float` |
| `token_count(...)` | 1 | — | `token_count(text)` — estimate token count. Cyrillic: chars/2, Latin: chars/4. |
| `trim(...)` | 1 | `String -> String` | Trims whitespace from the edges |
| `trim_end(...)` | 1 | — | `trim_end(s)` — strips trailing whitespace (Unicode-aware). |
| `trim_start(...)` | 1 | — | `trim_start(s)` — strips leading whitespace (Unicode-aware). |
| `truncate(...)` | 2 | — | `truncate(s, max_len)` — cuts the string to at most `max_len` characters (char-wise) appending an ellipsis `…` when truncation happens; `max_len` 0 yields the empty string. |
| `type_of(...)` | 1 | — | `type_of(value) -> String` — returns the runtime type name as a String. Useful for safe checking after json_get: `if type_of(x) == "Unit" { ... }` |
| `upper(...)` | 1 | `String -> String` | Converts a string to uppercase |
| `word_wrap(...)` | 2 | — | `word_wrap(s, width)` — reflows text to `width` columns without breaking words; errors on width 0. |
| `words(...)` | 1 | — | `words(s)` -- split string into list of words by whitespace. |

### `stub` — 26 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `authenticate(...)` | 2 | `String, Secret\ | Unit |
| `conv_add(...)` | 3 | — | VM-native conversation context (handled inside `src/vm.rs`, no host handler): appends a message to the conversation. |
| `conv_context(...)` | 1 | — | VM-native conversation context (handled inside `src/vm.rs`, no host handler): returns the compacted conversation context window. |
| `conv_end(...)` | 1 | — | VM-native conversation context (handled inside `src/vm.rs`, no host handler): closes the conversation. |
| `conv_history(...)` | 1 | — | VM-native conversation context (handled inside `src/vm.rs`, no host handler): returns the conversation message history. |
| `conv_start(...)` | 1 | — | VM-native conversation context (handled inside `src/vm.rs`, no host handler): opens a conversation by id. |
| `db_insert(...)` | variadic | `String, Struct -> Float` | A parameterized INSERT. Returns last_insert_rowid (Problem C) |
| `event_count(...)` | variadic | — | VM-native event analytics (handled inside `src/vm.rs`, no host handler): counts events in the event log, optionally filtered by type. (The registry's old "planned, no handler" comment is stale — the VM implements it.) |
| `event_sum(...)` | 2 | — | VM-native event analytics (handled inside `src/vm.rs`, no host handler): sums a numeric field across events of a type. |
| `events_since(...)` | 1 | — | VM-native event analytics (handled inside `src/vm.rs`, no host handler): lists events since a sequence/timestamp marker. |
| `find(...)` | 4 | — | VM-native entity-store query (handled inside `src/vm.rs`, no host handler): scans globals for Struct values matching (type, field, operator, threshold). |
| `fit_to_budget(...)` | variadic | — | VM-native fluid-budget helper (handled inside `src/vm.rs`, no host handler): trims a List of items to fit a token budget. |
| `forget(...)` | variadic | — | VM-native memory forget (handled inside `src/vm.rs`, no host handler): removes matching memory entries by query. |
| `if_eq(...)` | 3 | — | Registry-only stub — no handler on TW or VM; calling the name errors on both backends (the language's `if` comparison is an expression, not a builtin). |
| `inspect(...)` | 1 | `String -> Struct\ | Struct (or Unit if the pattern is not found) |
| `is_string_token(...)` | 1 | — | Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness). |
| `map(...)` | variadic | `List, String -> List` | Applies a pattern to each element (requires `import std/collections`) |
| `newline(...)` | variadic | — | Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness). |
| `query_row(...)` | variadic | — | VM-native DB helper (handled inside `src/vm.rs`, no host handler): executes an SQL query and returns the first row as a Dict. |
| `query_scalar(...)` | variadic | — | VM-native DB helper (handled inside `src/vm.rs`, no host handler): executes an SQL query and returns the first column of the first row as a scalar. |
| `recall(...)` | variadic | — | VM-native memory recall (handled inside `src/vm.rs`, no host handler): returns the best memory match for the query, optional minimum-confidence threshold. Registry arity entry kept for VM bytecode validation. |
| `resolve_skill_index(...)` | 1 | — | VM-native skill resolver (handled inside `src/vm.rs`, no host handler): resolves a skill index entry for the VM execution path. |
| `session_login(...)` | 2 | `String -> Session` | Creates a session for the user |
| `session_logout(...)` | 1 | `Session -> Unit` | Destroys the session |
| `split_tokens(...)` | variadic | — | Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness). |
| `stdin(...)` | variadic | — | Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness). |

### `svg` — 13 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `color_palette(...)` | 2 | `String, String -> Struct` | An HSL-cascade palette. `intent` in {calm, tension, energy, authority, warmth}, `mode` in {light, dark}. Output — 5 tokens (`paper`, `ink`, `accent`, `muted`, `rule`) |
| `svg_callout(...)` | 5..6 | — | `svg_callout(x, y, w, h, text, id?)` — callout box with a dashed border and connector (visually distinct from solid diagram connections). |
| `svg_canvas(...)` | 4 | `Float×2, String, List -> String` | A root `<svg>` (`viewbox` is structural) |
| `svg_canvas_preset(...)` | 3 | `String, String, List -> String` | The same, with a named canvas size: `doc_inline` (960×600), `slide_16x9` (1280×720), `social_og` (1200×632), `print_a4_landscape`, `print_a4_portrait` |
| `svg_circle(...)` | 4 | `Float×3, String×2 -> String` | A circle |
| `svg_generate(...)` | 4 | `String, String, Float×2 -> String` | A procedural background. `kind` in {flow, grid, noise}. Deterministic (the same `intent` gives an identical result, no `rand`/system clock) |
| `svg_group(...)` | 1..2 | `List, String -> String` | A group with an optional transform (`transform` is structural, like `d`) |
| `svg_icon(...)` | 5 | `String, Float×3, String -> String` | A ready-made icon. `name` in {server, laptop, phone, database, cloud, arrow-right, check, warning, user, document} |
| `svg_line(...)` | 5..6 | `Float×4, String, Float -> String` | A line |
| `svg_path(...)` | 2..3 | `String×3 -> String` | An arbitrary path. `d` is a structural argument, **not escaped**, a compile error on injection |
| `svg_rect(...)` | 5..6 | `Float×4, String×2 -> String` | A rectangle |
| `svg_sketchy_filter(...)` | 1..5 | `String, Float -> String` | A "hand-drawn" style SVG filter (`id` is structural) |
| `svg_text(...)` | 5..6 | `Float×2, String, Float, String×2 -> String` | Text (auto-escaped) |

### `system` — 3 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `env(...)` | 1 | `String -> String` (→ `Secret` in an entity context) | Reads an environment variable. An empty string if not found (the soft-failure contract). **Serve gate (naryad №259)**: inside serve route bodies `env()` is denied by default with a loud `ENV_NOT_PERMITTED` error — route code often receives untrusted input and must not read the process's secrets. Escape hatches (alternatives, not AND): `METALOGOS_SERVE_ALLOW_ENV=1` allows all env reads in route bodies, or `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` allows exactly the listed names. Outside serve (`mlog run`, `mlog check`, repl, serve top level) the read is ungated, as before. The denial is identical for existing and non-existing names (the gate runs before the read). See also the exec gates (`EXEC_NOT_PERMITTED`, naryad №253) in the threat model |
| `policy_check(...)` | 1 | `String -> Dict` | Checks a command against policy: heredoc `<<`, pipe ` |
| `replay_snapshot(...)` | 1 | `List -> Dict` | Serializes a list of values into a JSON snapshot. Returns `ReplaySnapshot { seq, items, json, created_at }`. seq=0 is a full snapshot, seq=N is a delta |

### `template` — 1 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `template_render(...)` | 2 | `String, Struct -> Html` | A separate engine (not an extension of `render()`): `{{ var }}` (escaped), `{{{ var }}}` (unescaped), `{{#if}}/{{else}}`, `{{#each}}`. Template content is trusted author code, not scanned by the lint |

### `test` — 2 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `assert_contains(...)` | 2 | `Any, Any -> Unit` | Panics if the string representation of `needle` is not found in `haystack` |
| `assert_eq(...)` | 2 | `Any, Any -> Any` | A runtime equality assertion. Returns actual on success, panics with `actual != expected` |

### `time` — 11 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `add_days(...)` | 2 | `Float, Float -> Float` | Adds `days` to a timestamp. Negative `days` subtracts. `ts + days * 86400` |
| `add_hours(...)` | 2 | `Float, Float -> Float` | Adds `hours` to a timestamp. Negative `hours` subtracts. `ts + hours * 3600` |
| `date_parts(...)` | 1 | `Float? -> Dict` | Returns `DateParts { year, month, day, hour, minute, second, weekday }`. `timestamp` defaults to the current moment |
| `days_between(...)` | 2 | `Float, Float -> Float` | The absolute difference between two timestamps, in days. ` |
| `days_in_month(...)` | 2 | `Float, Float -> Float` | The number of days in a month. `month` is 1-12. Errors if the month is out of range. Accounts for leap years |
| `format_date(...)` | 2 | `String?, Float? -> String` | Formats a timestamp using a strftime string. `fmt` defaults to `"%Y-%m-%d %H:%M:%S"`. `timestamp` defaults to the current moment |
| `is_leap_year(...)` | 1 | `Float -> Bool` | `true` if the year is a leap year (Gregorian rules) |
| `now(...)` | variadic | `-> Float` | The current Unix timestamp, in seconds |
| `sleep(...)` | 1 | `Float -> Unit` | Blocks the current thread for `seconds` seconds. Use carefully in `mlog serve` — it blocks request handling |
| `time(...)` | variadic | `-> Float` | The current Unix timestamp (seconds since the epoch). High precision (sub-second) |
| `weekday_name(...)` | 1 | `Float -> String` | The weekday's name (localized via chrono::Local). E.g. `"Monday"` |

### `tokens` — 1 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `diagram_style(...)` | 1 | `String×5 -> Struct` | A validator for manually specified tokens (without generating via `color_palette`) |

### `vault` — 3 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `config_load(...)` | 1 | — | `config_load(path)` — load a JSON or YAML config file and return as struct. |
| `semantic_search(...)` | 3 | — | `semantic_search(query, documents, top_k)` — semantic similarity search. |
| `vault_validate(...)` | 2 | — | `vault_validate(config, required_fields)` — validate a loaded config against required fields. |

### `vision` — 10 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `vision_edit(...)` | 2 | `(Vision, String) -> Vision` | **In-context editing of a signed artifact** (#243, R6.2). The source MUST be signed: an artifact with no provenance manifest is a loud refusal (there is nothing honest to inherit; producing an unsigned artifact through the real compute path is forbidden by ADR-0125) — raw export (`vision_export_raw`) is unaffected. Pipeline: source PNG to decode to a loud dims contract (R4.1: 256..=4096, x16; a multiple of the VAE factor — silent resizing is forbidden, the output keeps the source's resolution) to the VAE encoder (non-decoder prefixes of the same pinned VAE file; encode in posterior MODE) to a reference latent to Qwen3-4B on the edit prompt to the Z-Image DiT with in-context token concatenation (a noise branch plus the reference at every step, `euler_step` applies only to the noise branch) to `EDIT_STEPS = 8` (the distilled turbo NFE) to decode to PNG. The output is ALWAYS signed: a watermark plus a manifest, where `model_id`/`policy`/`seed` are inherited from the source, `prompt_sha256` is the hash of the edit prompt, and `timestamp`/`png_sha256` are fresh. Loud refusals: an unsigned source, an unknown handle, a wrong type, an empty prompt, dimensions outside R4.1 or not a multiple of the factor, `MLOG_VISION_WEIGHTS_DIR` not set (the variable and how to set it are named verbatim). UserInput taint on the prompt triggers an audit warning, `VISION_PROMPT_USER_INPUT` (the same check-id as `vision_generate`'s). |
| `vision_export(...)` | 2 | — | Last-resort stub — real path is the intercepted `vision_export_dispatch`. |
| `vision_export_raw(...)` | 2 | `(Vision, String) -> String` | **An explicit opt-out from signing** (ADR-0125). Writes the artifact's PNG bytes as-is: no watermark, no sidecar manifest, no signature requirement. Every call triggers an audit warning, `VISION_UNSIGNED_EXPORT_RAW` (advisory, `mlog audit`). Signed export (PNG + `<path>.manifest.json`) goes through `vision_export`; an artifact with no manifest cannot be exported that way (the runtime backstop `VISION_UNSIGNED_EXPORT`). |
| `vision_fetch_weights(...)` | 2 | `(String, String) -> String` | **SSRF-guarded, allowlist-gated, SHA-pinned weight downloading** (ADR-0125's `MODEL_WEIGHTS_UNSAFE`). Layers of protection: (1) the `MLOG_VISION_WEIGHTS_ALLOWLIST` allowlist — **default-deny**: an unset/empty env produces a loud refusal before any network access; (2) an SSRF guard (`check_url_ssrf`): private/loopback/link-local/metadata addresses are forbidden, DNS resolutions are pinned against rebinding; (3) only `manifest.json`-class URLs (a bare `.safetensors` has "no pin" and is refused; pickle-class `.pkl/.pt/.pth/.ckpt/.bin/...` is refused by extension); (4) SHA-256 pinning of every manifest entry (reusing `WeightsManifest`) — a mismatch is a loud refusal, and the file is NOT written; entry names must be bare `*.safetensors`. The static gate `MODEL_WEIGHTS_UNSAFE` (an audit Error, Category A) additionally catches literal URLs with an SSRF-blocked host, a bare `.safetensors`, and the pickle class. The downloaded tree (`manifest.json` plus shards) is consumed by `vision_generate` via `MLOG_VISION_WEIGHTS_DIR` — with re-verification of the SHA at load time (defense in depth). |
| `vision_generate(...)` | 2 | — | Last-resort stub — real path is the intercepted `vision_generate_dispatch`. |
| `vision_list(...)` | variadic | — | Last-resort stub — real path is the intercepted `vision_list_dispatch`. |
| `vision_load(...)` | 1 | `(String) -> Vision` | **Loading an artifact from the program's database** (#242, R6.1): exactly the bytes and the manifest that were saved (provenance persistence neither regenerates nor supplements them). Inserted into the session's registry with a new, monotonic id. Loud refusals: no database, an unknown name (with a list of what is saved), a malformed manifest JSON in the database (silent degradation to unsigned is forbidden). An artifact with `manifest: None` loads as-is and is still refused by a signed `vision_export` (`VISION_UNSIGNED_EXPORT`) / works with `vision_export_raw` — #241's backstop survives persistence. |
| `vision_lora_generate(...)` | 3 | `(String, String, String) -> Vision` | **Generation with a LoRA adapter applied** (#244, R6.3). The same pipeline as `vision_generate`, but the DiT is built through `from_weights_with_lora`: `W' = W + scale·(up@down)` in F32 over the attention projections; the adapter is resolved from the database on every call with a loud **integrity pin** (`sha256(bytes) != meta.sha256` is an error BEFORE any compute). Provenance: the composite `model_sha256` (see above), `model_id`/watermark/policy/seed/steps from the base declaration; sign ALWAYS. Loud refusals: arity/types, empty prompt, unknown decl (with the list), unknown model, no database, unknown `lora_name` (with `lora_list`), corrupted meta JSON, integrity mismatch, missing env/component, non-1024. UserInput-tainted prompts raise the `VISION_PROMPT_USER_INPUT` audit Warning (the same check id; args 0 and 2 are never flagged). |
| `vision_lora_load(...)` | 2 | `(String, String) -> String` | **Loading a LoRA adapter into the SQLite BLOB store** (#244, R6.3; ADR-0124 §6 — the adapter lives ONLY in the program's database, no session state). Reads the safetensors file ONCE — only inside `MLOG_VISION_WEIGHTS_DIR` (a relative path, no traversal, `.safetensors` extension, file must exist; the path contract lives in `vision_lora_check_adapter_path`), validates loudly (both canonical name forms — diffusers-PEFT and ComfyUI; rank = the mean pair dimension, scale = alpha/rank with the loud 1.0 default; non-F32 upcast is loud; targets must be attention projections `to_q/to_k/to_v/to_out.0` per `zimage_expected_keys`; half pairs, non-attention targets, unknown prefixes, orphaned keys, mismatched dimensions are loud errors with the FULL list) and stores the bytes plus fixed-shape metadata (`LoraMeta`: sha256/rank/alpha/scale/targets) into the `vision_lora_adapters` table. The prescribed check order: arity → no-db → env → path → feature gate → read/parse → insert; a name collision is a loud error (no upsert). Returns the persistent key `name`. |
| `vision_save(...)` | 2 | `(Vision, String) -> String` | **SQLite persistence of an artifact** (#242, R6.1). Writes the artifact (a PNG as a BLOB plus a JSON provenance manifest) to the program's database (the `db { url: "sqlite:..." }` declaration) — the `vision_artifacts` table, whose persistent key is `name` (the registry id is a session-scoped handle and is not persisted). Loud refusals: no database (with a hint at the declaration), an empty name, a name collision (a silent overwrite would be a silent loss of the provenance chain; upsert/delete are out of scope for #242), an unknown handle. A verbatim round trip: the `timestamp` and the manifest's fields are not regenerated. |

### `voice` — 2 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `tts_send(...)` | 4..5 | `String, String, String, String -> String` | Generates speech via the OpenAI TTS API (the `tts-1` model) and sends the audio to a Telegram chat. Requires `OPENAI_API_KEY`. Voices: `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"` |
| `whisper_transcribe(...)` | 1 | `String, String, String, String -> String` | Downloads a voice message from Telegram by `file_id`, sends it for transcription to the Whisper API. `provider`: `"openai"` (default) or `"groq"`. Returns the recognized text |

### `web` — 18 builtin(s)

| Builtin | Arity | Signature (curated) | Description |
|---|---|---|---|
| `form_data(...)` | 1 | `-> Struct {FormData}` | Parses data from an `application/x-www-form-urlencoded` request body |
| `geo_distance(...)` | 2..5 | — | `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance. unit: "km"(default), "mi", "nm", "m". |
| `geo_ip(...)` | 0..1 | — | `geo_ip(ip?)` — geolocate by IP. Uses ip-api.com (free, no key). Returns Struct {ip, city, region, country, country_code, lat, lon, isp, timezone}. |
| `html_render(...)` | 3 | `String, Float×2 -> String` | A screenshot via a headless browser (`METALOGOS_BROWSER_BIN`, no default path). The only function in this family that spawns an external process — via `exec_restricted` (argv, not a shell). Network isolation is not guaranteed at the OS level — the input must be self-contained HTML |
| `http_download(...)` | 2..3 | `String, String -> Bool` | Downloads `url` into `dest_path` (the path must be inside the write sandbox). Returns `true` on success, `false` on any failure (network error, HTTP 4xx/5xx, sandbox violation, write error — the soft-failure contract). The URL is SSRF-gated like the other egress builtins: a blocked address is a LOUD `SSRF guard` error, not a silent `false` (naryad №261). 30s timeout |
| `http_get(...)` | 1..4 | `String -> String` | A GET request. 30s timeout. Errors on status >= 400 |
| `http_post(...)` | 2..6 | `String, String -> String` | A POST request. Content-Type defaults to `application/json`. 30s timeout. Errors on status >= 400 |
| `http_post_multipart(...)` | 2..4 | `String, Struct, Struct -> String` | A multipart POST. `fields` are text fields (a Struct), `files` are file fields (a Struct, whose values are file paths). File paths must be inside the read sandbox (relative paths); absolute paths, `..` and sandbox escapes are a loud `[SANDBOX_VIOLATION]` error. 120s timeout |
| `json_body(...)` | variadic | `-> Struct {JsonBody}` | Parses JSON from the request body |
| `query_param(...)` | 1 | `String -> String` | Gets a query parameter from the URL. `curl "localhost:8080/search?q=hello" -> query_param("q") == "hello"`. An empty string if the parameter is absent. Percent-decoding: RFC 3986 bytes reassembled as UTF-8 (`%D0%B6` → `"ж"`), `+` → space (form-urlencoded convention), invalid escapes pass through literally, invalid UTF-8 is lossy — see the §4.13 note (Naryad #257) |
| `render(...)` | 2..3 | `String, String, Any, ... -> Html` | Renders a template, substituting `{{ var }}` variables. The number of arguments after the template name must be even (key/value pairs) |
| `request_body(...)` | variadic | — | `json_body()` / `request_body()` — raw body of the current route request, parsed as JSON into a `Dict` Struct; `request_body` is the explicit-name alias. Result carries `UserInput` taint (untrusted request data). |
| `require(...)` | 1..2 | `Bool -> Unit` | A runtime assertion. Errors if `false` |
| `respond(...)` | 1..2 | `String -> HttpResponse` | Builds an HTTP response. Format: `"200 OK"`, `"404 Not Found"`, etc. |
| `respond_html(...)` | 1 | `String, String -> HttpResponse` | An HTML response with the given status |
| `weather(...)` | 2 | — | `weather(city_or_lat, lon?)` — current weather via Open-Meteo (FREE, no API key). `weather("Minsk")` or `weather(53.9, 27.57)`. Returns Struct {temp, feels_like, temp_min, temp_max, humidity, description, wind_speed, wind_direction, pressure, cloud_cover, is_day, city, country}. |
| `weather_forecast(...)` | 1..3 | — | `weather_forecast(city_or_lat, lon?, days?)` — multi-day forecast via Open-Meteo (FREE, no API key). `weather_forecast("Minsk", 7)` or `weather_forecast(53.9, 27.57, 3)`. Default: 7 days. Max: 16 days. Returns List of DayForecast structs. |
| `web_search(...)` | 1..2 | — | `web_search(query, num_results?) -> String` — search via SerpAPI. Uses SERPAPI_KEY env var. Returns raw JSON string. Usage: web_search("query") -> JSON string Usage: web_search("query", 5) -> JSON string with 5 results |

<!-- END GENERATED BUILTIN INDEX -->
