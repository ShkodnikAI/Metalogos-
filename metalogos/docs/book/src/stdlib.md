# Standard Library Reference

METALOGOS includes a standard library organized into modules. Import modules with `import std/module_name`.

## std/string

String manipulation patterns.

### `trim(s: String) -> String`

Removes leading and trailing whitespace.

```mlog
import std/string
entity raw: String = "  hello  "
entity cleaned: String = trim(raw)
```

### `replace(s: String, old: String, new: String) -> String`

Replaces all occurrences of `old` with `new` in string `s`.

```mlog
import std/string
entity result: String = replace("hello world", "world", "METALOGOS")
```

### `split(s: String, sep: String) -> List`

Splits a string by a separator, returning a list.

```mlog
import std/string
entity parts: List = split("a,b,c", ",")
```

### `join(items: List, sep: String) -> String`

Joins a list of strings with a separator.

```mlog
import std/string
entity result: String = join(parts, "-")
```

## std/math

Mathematical operations.

### `abs(n: Float) -> Float`

Absolute value.

```mlog
import std/math
entity result: Float = abs(-5.0)
```

### `min(a: Float, b: Float) -> Float`

Returns the smaller of two values.

```mlog
import std/math
entity result: Float = min(3.0, 7.0)
```

### `max(a: Float, b: Float) -> Float`

Returns the larger of two values.

```mlog
import std/math
entity result: Float = max(3.0, 7.0)
```

### `clamp(val: Float, lo: Float, hi: Float) -> Float`

Constrains a value to a range.

```mlog
import std/math
entity result: Float = clamp(val, 0.0, 1.0)
```

### `round(n: Float) -> Float`

Rounds to the nearest integer.

```mlog
import std/math
entity result: Float = round(3.7)
```

## std/collections

Basic collection operations.

### `first(items: List) -> String`

Returns the first element of a list.

```mlog
import std/collections
entity head: String = first(items)
```

### `last(items: List) -> String`

Returns the last element of a list.

```mlog
import std/collections
entity tail: String = last(items)
```

### `push(items: List, item: String) -> List`

Adds an item to the end of a list, returning a new list.

```mlog
import std/collections
entity updated: List = push(items, "new")
```
