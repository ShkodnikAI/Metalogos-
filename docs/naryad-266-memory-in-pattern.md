# Naryad #266 (P2, bug/parser) — memorize/relate/forget inside pattern/route bodies: token soup instead of a loud parser error

**Fact.** The grammar knows `memorize <expr> [with priority=F]`,
`relate ... to ... as ...`, `forget ...` only at the top level
(`memorize_decl`/`relate_decl`/`forget_decl` in `src/grammar.pest:182+`,
`declaration = ...`). Inside `pattern_body = { statement* }` these rules
are ABSENT — and the statement grammar silently splits the line into garbage
(verified by an AST dump on `naryad-264` @ 3c4ab45, file
`examples/p8_route_patterns.mlog`, line 20):

```
Stmt 0: ExprStmt { expr: Ident("memorize") }
Stmt 1: ExprStmt { expr: Ident("fact") }
Stmt 2: ExprStmt { expr: Ident("with") }
Stmt 3: Assign { name: "priority", value: FloatLit(0.8) }
```

Runtime behavior of TW when the pattern is invoked (probe on main @ 29f0592):

```
$ mlog run probe_mem.mlog        # flow calls Remember("hello")
error: undefined variable: memorize      exit=1
```

The example `examples/p8_route_patterns.mlog` (Contract 3 "pattern with memory
from route") never worked end-to-end — it only compiled;
the `/remember` route would have returned 500 on the first POST. Before #264 this was silent:
`mlog check` let the token soup through, the compiler compiled the Assign, TW
only fell at runtime on invocation. #264 (the static immutability
check) pulled the garbage into static analysis — `mlog check` now honestly
failed with "cannot assign to immutable variable: priority" on the example, which
exposed the true root. `relate`/`forget` inside bodies are the same class by
grammar (not reproduced separately, but it is one rule).

**Task.**
1. Grammar: allow `memorize_decl`/`relate_decl`/`forget_decl`
   as statements inside `pattern_body` (and route bodies — they are also
   `statement*`): extend `pattern_body = { statement* }`
   (`statement += memory_stmt` with memorizer branches), reuse the lexer tokens
   `MEMORIZE_KW`/`RELATE_KW`/`FORGET_KW`. Semantics —
   same as for top-level declarations (writing to session/DB memory per the
   contract of the memory builtins).
2. Alternative (if bringing statement-memorize into execution is
   disproportionate): FORBID loudly — the parser must fail with
   a clear error "memorize is only allowed at top level" instead of
   token soup. Record the chosen option in the PR LOUDLY with
   the rationale.
3. Either way: a test for both files — a minimal pattern with
   `memorize fact with priority=0.8` inside the body (parses AND
   executes — or loudly fails to parse with a clear message), and
   `examples/p8_route_patterns.mlog` returned to Contract 3
   (restore of the `memorize fact with priority=0.8` line in the Remember body —
   it was removed by the truth-up of #264 with a comment pointing to this naryad).

**§3.** `src/grammar.pest`, `src/parser/stmt.rs`/`decl.rs` (depending
on the option), `src/semantic.rs` (if the statement-memorize requires
semantics), `src/interpreter`/`execution.rs` (execution), examples/p8,
REFERENCE (memory section: where memorize is allowed).

**Done, when:** the probe-fact is inverted: a pattern with `memorize ... with
priority=` inside the body either works (TW and VM identically) or does not
parse with a loud clear message — the third outcome, "silent token
soup", is excluded by a test; p8 restored and green on check+run; the existing
memory tests (p7/m4/contracts) green; CI blocking green.

**Links.** Exposed by the execution of #264 (issue #280); priority P2, because
silent parse garbage is of the " dishonest silence" class, but the construct
was never promised to work (REFERENCE describes memorize at the
top level).
