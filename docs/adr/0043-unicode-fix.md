# ADR-0043: Unicode Fix — Cyrillic String Handling

**Status:** Implemented
**Date:** 2025-06-07
**Priority:** Blocker — 2 departments of Fosved Office could not work without this fix.

## Context

Metalogos `.mlog` files containing Cyrillic text caused runtime panics or incorrect results. Root cause: multiple locations used byte-level indexing on UTF-8 strings where character-level indexing was required.

In UTF-8, Cyrillic characters are 2 bytes each. `str.len()` returns byte count (e.g., "Привет" = 12 bytes), while users expect character count (6). Byte-level slicing at non-char-boundary positions causes panics.

## Problem Areas

### 1. `len()` builtin — byte count instead of character count
**File:** `src/builtins.rs:151`
```rust
// BEFORE (broken):
Ok(Value::Float(s.len() as f64))           // "Привет" → 12.0
// AFTER (fixed):
Ok(Value::Float(s.chars().count() as f64)) // "Привет" → 6.0
```

### 2. `index_of()` builtin — byte offset instead of character position
**File:** `src/builtins.rs:224-231`
```rust
// BEFORE (broken):
match haystack.find(&needle) {  // returns byte offset
    Some(pos) => Ok(Value::Float(pos as f64)),  // "Привет, мир" find "мир" → 12
// AFTER (fixed):
let char_pos = haystack.char_indices()
    .find(|(byte_idx, _)| haystack[byte_idx..].starts_with(&needle))
    .map(|(byte_idx, _)| haystack[..byte_idx].chars().count());
// "Привет, мир" index_of "мир" → 8 (character position)
```

### 3. Negative string index — byte length in interpreter
**File:** `src/interpreter.rs:1633`
```rust
// BEFORE (broken):
let abs_idx = s.len().wrapping_sub((-idx) as usize);  // byte count
// AFTER (fixed):
let char_len = s.chars().count();  // character count
let abs_idx = char_len.wrapping_sub((-idx) as usize);
```
Impact: `s[-1]` on "Привет" (6 chars, 12 bytes) computed `12-1=11`, then `chars().nth(11)` returned '\0' instead of 'т'.

### 4. Template body extraction — char index used as byte offset
**File:** `src/parser.rs:179-188`
```rust
// BEFORE (broken for Unicode in templates):
let chars: Vec<char> = result[abs_brace..].chars().collect();
for (i, &ch) in chars.iter().enumerate() {
    // 'i' is char index, but used as byte offset!
    end_pos = Some(abs_brace + i);
// AFTER (fixed):
for (byte_offset, ch) in result[abs_brace..].char_indices() {
    end_pos = Some(abs_brace + byte_offset);  // actual byte offset
```
Impact: Templates containing Cyrillic text would cause incorrect body boundaries or panic on non-char-boundary slice.

## Decision

- All string length operations use `.chars().count()`
- All string position/index operations use `.char_indices()` for byte-to-char mapping
- String indexing (`s[i]`) uses `.chars().nth(i)` with character indices
- Template preprocessing uses `char_indices()` for Unicode-safe byte positioning
- Pest grammar `ANY` token is Unicode-aware (confirmed correct)
- No hand-rolled lexer exists; Pest is the sole parser

## Safe Areas (No Change Needed)

- `substring()`, `char_at()`, `length()`, `reverse()` — already char-based
- `trim()`, `replace()`, `contains()`, `starts_with()`, `ends_with()` — stdlib methods, char-boundary safe
- `split()`, `join()` — operate on `&str` slices, char-boundary safe
- `escape_html_chars()`, `escape_json()` — iterate over `.chars()`
- Grammar `STRING_LITERAL`, `inner_string`, `ESCAPE_SEQ` — Pest `ANY` is Unicode-aware
- `unescape_string()` quote stripping — `"` is ASCII, always at byte boundaries

## Verification

- `len("Привет, Металогос! Это тест кириллицы.")` → 39 (not 69)
- `index_of("Привет, мир", "мир")` → 8 (not 12)
- `char_at("Привет", 0)` → "П"
- `s[-1]` on "Привет" → "т"
- Templates with Cyrillic body content parse without panic
- All existing tests remain green (len on ASCII strings unchanged behavior)

## Files Changed

- `src/builtins.rs` — `len()`, `index_of()` Unicode fix
- `src/interpreter.rs` — negative string index Unicode fix
- `src/parser.rs` — `preprocess_templates()` char_indices fix
- `examples/p7_cyrillic.mlog` — contract test
- `examples/bug_cyrillic.mlog` — bug reproduction
