# ADR-0059: Struct via Entity Reuse — No New Keyword

**Date:** 2026-07-12
**Status:** Accepted
**Context:** Наряд METALOGOS_4_PRIMITIVES v2, Problem B (STOP Trigger #1)

## Decision

The narad proposed a new `struct` keyword for Problem B's actor potential formula. STOP Trigger #1 required checking whether existing `EntityType` already covers this need.

**Verdict: `entity TypeName { field: Type, ... }` IS the struct system. No new keyword.**

### What EntityType Already Provides

| Capability | Implementation |
|---|---|
| Named struct type | `entity Actor { name: String, A: Float }` → AST: `EntityTypeDecl` |
| Typed fields with defaults | `FieldDecl { name, type_name, default }` |
| Field access | `a.A`, `a.name` → `Expr::FieldAccess` → `Value::get_field()` |
| Struct literal | `{key: val}` → `Expr::StructLit` |
| String-keyed index | `s["field"]` → `Expr::IndexAccess` |
| Runtime representation | `Value::Struct { type_name, fields: HashMap<String, Value> }` |

### What the Narad Wanted vs What Exists

```mlog
// Narad proposed:
struct Actor { name: String, A: Float, ... }

// Actual equivalent (already working):
entity Actor { name: String, A: Float, ... }
```

The only difference is the keyword (`struct` vs `entity`). The syntax, semantics, and runtime behavior are identical.

### Rationale Against Adding `struct` as Alias

1. **Keyword proliferation**: Every new keyword increases parser complexity and potential for ambiguity. The PEG grammar already has 40+ keywords.
2. **Zero functional gap**: EntityType provides everything Problem B needs.
3. **Backward compatibility**: All existing code uses `entity`. Adding `struct` creates two ways to do the same thing.
4. **Cognitive load**: One concept, one keyword is clearer than one concept, two keywords.

### New Builtins Added

The actual value of Problem B is in the builtins, not the type keyword:

- **`map(list, "pattern_name")`**: Now works in all three backends (tree-walking, bytecode/VM, JIT). Previously tree-walking only.
- **`zip(list_a, list_b)`**: Already worked everywhere via BuiltinRegistry. Returns `List[Pair{a, b}]`.
- **`sort_by(list, "field", descending)`**: Already worked everywhere via BuiltinRegistry.
- **`filter(list, "field", value)`**: Already worked everywhere via BuiltinRegistry.
- **`reduce(list, "field", initial)`**: Already worked everywhere via BuiltinRegistry.

### Tuple Access Convention

`zip` returns `Pair` structs with fields `.a` and `.b`. After `sort_by`, access the paired values via `.a` (original) and `.b` (computed), not via numeric index `[0]`/`[1]`.

## Consequences

- All three backends now support `map()` consistently
- `IndexAccess` instruction added to `execute_code` (was missing — pattern bodies in VM couldn't use `[N]` syntax)
- No grammar changes required
- No new AST nodes required