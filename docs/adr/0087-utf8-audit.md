# ADR-0087: Full UTF-8 Audit — Наряд №11

**Status:** Implemented
**Date:** 2026-06-08

## Context

Наряд №11 raised the concern that Cyrillic text was broken in Metalogos runtime. The primary evidence was error strings in the binary containing "byte index", suggesting `substring()`, `char_at()`, and other builtins used byte offsets instead of character offsets.

A comprehensive audit of all 19 `.rs` files in `src/` was performed to locate every byte-indexed string operation.

## Findings

### Already Fixed (ADR-0043 + Cyrillic support)

ADR-0043 (commit `df8ef66`) and subsequent Cyrillic support work (commit `30be466`) already fixed all critical string operations:

| Builtin | Fix | How |
|---------|-----|-----|
| `len()` | `s.len()` → `s.chars().count()` | Returns character count, not byte count |
| `substring(s, start, end)` | Byte slice → `Vec<char>` indexing | Collects chars, slices by char index |
| `char_at(s, idx)` | Byte slice → `Vec<char>` with `.get()` | Returns char at position, empty string on OOB |
| `index_of(haystack, needle)` | `.find()` byte offset → `char_indices()` + `.chars().count()` | Returns character position, not byte position |
| Negative index (interpreter) | `s.len()` → `s.chars().count()` + `chars().nth()` | Correctly indexes from end |
| `preprocess_templates` (parser) | `chars().enumerate()` → `char_indices()` | Correct byte offsets for Unicode source |

### One Remaining Bug

**File:** `src/embeddings.rs`, line 185

```rust
// BEFORE (byte-length — wrong for Cyrillic):
.filter(|w| w.len() > 1)

// AFTER (char-count — correct for all Unicode):
.filter(|w| w.chars().count() > 1)
```

**Impact:** The TF-IDF tokenizer filters out single-character tokens. With byte-length, a single Cyrillic character like `"а"` (2 bytes) passes the filter, inflating the vocabulary with unwanted single-letter entries. With `chars().count()`, `"а"` correctly has count 1 and is filtered out.

### Verified Safe (No Changes Needed)

All other string operations are inherently Unicode-safe:

- `upper()`, `lower()` — Rust's `to_uppercase()` / `to_lowercase()` handle Unicode
- `contains()`, `starts_with()`, `ends_with()` — Rust's `str` methods are Unicode-aware
- `trim()`, `replace()`, `split()` — Same
- `reverse()` — Uses `chars().rev().collect()`
- `escape_html()`, `escape_json()` — Char-by-char iteration
- Parser quote stripping (`s[1..s.len()-1]`) — ASCII `"`/`'` are always at char boundaries
- Server cookie/header parsing — All delimiters are ASCII
- `llm.rs` truncate helpers — Already use `char_indices()`
- `embeddings.rs:364-376` truncate — Already uses `char_indices()`

## Decision

Fix the single remaining bug in `embeddings.rs`. All 16 user-facing string builtins are already Unicode-safe thanks to ADR-0043.

## Consequences

- **Positive:** Cyrillic TF-IDF embeddings now have correct tokenization. Single Cyrillic letters are correctly filtered from the vocabulary.
- **Negative:** None. This is a non-breaking improvement.
- **Testing:** Contract test `examples/p11_utf8_full.mlog` covers all 14 string operation categories with Cyrillic input.
