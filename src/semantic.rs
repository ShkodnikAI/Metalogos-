// ── Semantic analysis for METALOGOS ──────────────────────────────
// Validates declarations without execution. Reports errors and warnings.
// Phase 6+: Enforces opaque type constraints (Html, Query, Secret, etc.)
//
// Наряд №165: every diagnostic carries the AST `Span` of the offending
// node. The previous `Vec<String>` API was a flat list — diagnostics
// lost the position that наряд №121 already added to every AST node.
// `SpannedError` restores the link. Callers that previously iterated
// `result.errors.iter()` as `&String` now see `&SpannedError` and must
// read `.message` (or `.span` for LSP / programmatic consumers).

#![deny(clippy::wildcard_enum_match_arm)] // №532: the N-1 class lives in wildcard Value arms — new wildcards need an explicit allow + reason
use crate::ast::*;
use crate::audit::{audit_category_a, Severity};
use crate::builtins_classification::{classify, Reversibility, Role};
use crate::labels::{legacy_taint_label, Label};
use std::collections::{BTreeMap, HashMap, HashSet};

/// A diagnostic with the AST `Span` of the offending node.
///
/// `span` is the source position of the AST node that triggered the
/// diagnostic — never computed, always taken from the node via
/// `decl.span()` / `expr.span()`. If a node genuinely has no span
/// (synthetic nodes, programmatic constructions), `Span::unknown()`
/// (all-zero) is used and consumers should treat `start_line == 0` as
/// "no position available".
#[derive(Debug, Clone)]
pub struct SpannedError {
    pub message: String,
    pub span: Span,
    /// Наряд №523: the structured KIND of the finding — set at the
    /// construction site, never derived from the message text. The
    /// run/serve blocking gates classify by this enum (ONE place,
    /// `is_exempt_from_blocking`), never by substring. (The compile path
    /// blocks in the bytecode compiler itself — №479 coded errors — and
    /// never consults this enum.)
    pub kind: SemanticErrorKind,
}

/// Наряд №523: the structured kinds of semantic findings. The blocking
/// gates in the run/serve paths exempt ONLY the kinds listed in
/// `is_exempt_from_blocking` — the ONE explicit place. The previous
/// substring filters (`contains("distill_to")`, `contains("[DENY_")`)
/// are gone: a message-substring filter is a liar-class mechanism (the
/// audit 30.09 N-1 lesson — "a semantic error cannot be a value").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticErrorKind {
    /// UNDEFINED_FUNCTION — the called name is not a builtin, pattern,
    /// or learnable known to the semantic pass (`check_expr_calls`).
    /// With the №523 static import merge the pass sees every imported
    /// symbol, so this kind now blocks like everything else; the kind
    /// stays named so the gate's exemption list can be explicit and
    /// future kinds can be classified without new plumbing.
    UndefinedFunction,
    /// №799 (T2, gh#1488): the called name IS defined — it is a parked
    /// builtin (the C3 (a) tier, №796) whose row does not compile into
    /// THIS build. The refusal carries `[BUILTIN_PARKED]`, the feature
    /// and the parking basis (`builtins::parked::refusal_message`);
    /// blocks the run/serve gates like every semantic finding (the
    /// exemption list stays empty).
    BuiltinParked,
    /// №799 (T2, gh#1488): the called builtin's tier is `experimental`
    /// — a WARNING, never an error. The finding is pushed into the
    /// errors vec with this kind and check_program partitions it into
    /// `warnings` at the tail (the ONE place) — the blocking gates never
    /// see it, `mlog check` stays exit-0, and the warning prints with
    /// the `[BUILTIN_EXPERIMENTAL]` class prefix.
    BuiltinExperimental,
    /// №801 (gh#1548, the gateway G2): MCP_POLICY_INVALID — an mcp_policy
    /// entry that cannot be a class: an unknown Label-annotation word
    /// (the label must parse per ADR-0154 §3) or a bare (non-`upstream.`)
    /// tool name (the local-tool namespace policy compiles from the
    /// body, №394). Blocks like every semantic finding.
    McpPolicyInvalid,
    /// №581 (the audit d63cc1d X-1, release-block): RESPOND_NOT_TERMINAL —
    /// a bare `respond*`/`respond_html*` call (an ExprStmt, no enclosing
    /// `return`) sits in a NON-terminal position of a route body (not the
    /// route tail, not the tail chain of a final if/match branch). On the
    /// tree-walking serve lane such a call answers IMMEDIATELY from any
    /// block (the HttpResponse-as-Return propagation), while the VM
    /// compiles it with `Instruction::Pop` — the guard pattern
    /// `if is_admin(...) == false { respond("403", ...) }` silently falls
    /// through to the protected code on the VM backend. Blocks run/serve
    /// on BOTH backends (fail-closed); the migration is one word:
    /// `return respond(...)`.
    RespondNotTerminal,
    /// №600 (the audit 25b375e Y-1, High, release-block): RESPOND_SWALLOWED —
    /// a bare `respond*` call carried by a statement NESTED under a
    /// top-level block-form `if/else` branch (depth ≥ 2). The serve lane's
    /// per-statement walk of a top-level `IfElseBlock` branch discards the
    /// value of every NESTED statement, so the response never reaches the
    /// route on EITHER backend and the branch continues — the guard
    /// `if is_admin(...) == false { respond("403") }` inside another
    /// block-if silently falls through to the protected code. №584 lowered
    /// the whole early-answer surface at once and this form lost the
    /// fail-closed refusal it had under №581 — the regression is closed
    /// fail-closed again: the finding is a BLOCKING error (run/serve), the
    /// migration is one word — `return respond(...)`, the same shape the
    /// current advisory already recommends. The precise surface enum lives
    /// in `RespondPosition` (the SSOT predicate shared with the №584
    /// lowering); the depth-≥2 semantics question itself (an early answer
    /// everywhere vs the error forever) is a separate owner gate (№603).
    RespondSwallowed,
    /// №802 (gh#1549, волна 65, the C6 R-b 2 static check): UNCHECKED_RESULT —
    /// the `.value` of a try-result (the ADR-0198 `Result<T>` — the
    /// ADR-0142 `Struct{ok, value, error}`) is read outside a branch
    /// where `.ok` was PROVEN (`if r.ok { … }`, the early-exit
    /// `r.ok == false { return … }`, the short-circuit `r.ok and …`).
    /// The failed try's value is Unit — the read is the runtime
    /// refusal shifted left to check/startup time on BOTH backends.
    /// The sanctioned escape: `// allow(unchecked_result): <reason>`
    /// (a bare allow without the reason is NOT a suppression).
    UncheckedResult,
    /// №802: UNCHECKED_OPTION — a `T|Unit` value (the ADR-0198
    /// `Option<T>`; the union spelling carries a Unit arm —
    /// `query_scalar`'s `Float|String|Unit`, the declared `-> T|Unit`
    /// rows) is used where `T` is required: the arithmetic/ordering
    /// operands, the `each` iterable, the string-key index, the
    /// declared precise parameters. `to_string` is the sanctioned
    /// total projection. The escape: `// allow(unchecked_option): <reason>`.
    UncheckedOption,
    /// №617 (ledger gh#967 §3): UNDEFINED_VARIABLE — a bare `Ident` read
    /// (or an assignment target) that the variable-scope walk cannot
    /// resolve against the TW scope model (pattern params + entity
    /// globals + `let` bindings + `each` loop variables — the flat,
    /// never-popped env of `eval_statements_cf`). The message is the TW
    /// runtime лекало verbatim (execution.rs: "undefined variable: {n}")
    /// and the stable code is the runtime origin's code — the static
    /// refusal fires earlier (run/check/serve startup) on BOTH backends,
    /// the runtime stays the backstop for the conditionally-bound cases
    /// the flat walk cannot prove.
    UndefinedVariable,
    /// №617 (ledger gh#967 §3): OPAQUE_CONCAT — a `+` (concatenation)
    /// operand carries a DECLARED opaque type (Html/Query/Secret/
    /// Encrypted/Hash/Session — the entity/param/let-chain fact). The
    /// static twin of the runtime guard (execution.rs eval_binop:
    /// "cannot concatenate opaque type …") — the same refusal shifted
    /// left to check/startup time. No stable code: the runtime origin
    /// refuses with a plain message, no code exists there — none is
    /// invented here (the honest-gap rule).
    OpaqueConcat,
    /// Every other semantic finding — blocks run/serve unconditionally.
    Other,
    /// №662 (ADR-0188 option (б), the owner's verdict gh#1123): the
    /// IMPLICIT String→opaque coercion at an assignment/argument position —
    /// a String-shaped value (a literal, a `+` chain) flows into a target
    /// declared with an opaque type (Html/…) where the runtime coercion
    /// used to be silent (№114). The late failure was the SECURITY
    /// exposure (an injection-shaped value reaches the page); the explicit
    /// path (an Html producer — render()) stays legal. No stable code: the
    /// runtime origin has none (the honest-gap rule, the OpaqueConcat
    /// posture).
    ImplicitOpaqueCoercion,
    /// №662 (ADR-0188 option (б), the owner's verdict gh#1123): a
    /// `render("Name", …)` whose statically-resolvable name is absent
    /// from the program's template declarations (the №250 registration
    /// set — the same names the compiler registers). The late failure was
    /// the request-time 500; the static refusal shifts it left. Dynamic
    /// (non-literal, non-ident) names stay the runtime's backstop. No
    /// stable code (the runtime origin has none).
    UnknownTemplate,
    /// №673 (gh#1200, В41): BREAK_OUTSIDE_LOOP — a break/continue (а)
    /// outside any loop (№658-b) or (б) crossing a value-channel boundary
    /// (an arm body of a match/if-as-value, №658-a). Both were the
    /// «молча неверные» class-4 entries (№653): the TW refused at RUNTIME,
    /// the VM silently absorbed the signal (or compiled the statement to
    /// nothing). №673 refuses BEFORE execution on BOTH backends through
    /// the shared semantic contour (check_program + compile_program); the
    /// runtime mirrors stay untouched as backstops. The stable code is
    /// CODE_BREAK_OUTSIDE_LOOP (the №523 first-stamp shape).
    BreakOutsideLoop,
    /// №685 (gh#1252, В44 — the S-1 audit): RAW_INSERT_NOT_HTML — a
    /// template parameter used in the RAW insert `{{{ param }}}` is
    /// declared with a non-Html type. The N9 raw form (gh#1219) landed
    /// the value VERBATIM in the page while every layer answered
    /// «sanitized» — the XSS scenario passed check/run/serve silently.
    /// The gate refuses at START on BOTH backends (check_program +
    /// compile_program, the №673 contour shape); the stable code is
    /// CODE_RAW_INSERT_NOT_HTML (the №523 stamp, the №385/ADR-0169
    /// classification — added, nothing renamed).
    RawInsertNotHtml,
    /// №695 (gh#1274, В46 — ADR-0188 phase 2): SAFE_PIPE_NOT_HTML — a
    /// template parameter used in the safe stage `{{ param | safe }}` is
    /// declared with a non-Html type. The safe stage inserts the value
    /// RAW (the escape-exempt), so the same S-1 logic as №685 applies:
    /// the raw slot must carry an explicitly produced Html value, and
    /// the declaration must be honest about the type. The gate refuses
    /// at START on BOTH backends (check_program + compile_program, the
    /// №673 contour shape); the stable code is CODE_SAFE_PIPE_NOT_HTML
    /// (the №385/ADR-0169 classification — added, nothing renamed).
    SafePipeNotHtml,
    /// №695: UNSAFE_PIPE_FORM — a pipe tag the v1 grammar does not
    /// recognize (an unknown stage, multiple pipes, a pipe in the raw
    /// triple-brace form, an expr that is not a declared param). The N9
    /// refusal (gh#1219) narrows to exactly these forms and shifts left
    /// to the semantic gate on BOTH backends (the registration refusal
    /// stays as the runtime backstop). No stable code — the v1 grammar
    /// names no origin for the class (the honest gap stays unstamped).
    UnsafePipeForm,
}

impl SemanticErrorKind {
    /// Наряд №523: the stable №479 diagnostic code for this kind, when one
    /// exists. The blocking gate prefixes its refusal with the FIRST
    /// blocking finding's code so machine consumers (the №465 diff-fuzzer's
    /// class signature, log scrapers) read the CODE, not the prose.
    /// Kinds without a code return None — no invented codes (the honest
    /// gap stays unstamped until an origin site owns one).
    pub fn stable_code(self) -> Option<&'static str> {
        match self {
            SemanticErrorKind::UndefinedFunction => {
                Some(crate::interpreter::values::CODE_UNDEFINED_FUNCTION)
            }
            SemanticErrorKind::RespondNotTerminal => Some(CODE_RESPOND_NOT_TERMINAL),
            SemanticErrorKind::RespondSwallowed => Some(CODE_RESPOND_SWALLOWED),
            SemanticErrorKind::UncheckedResult => Some(CODE_UNCHECKED_RESULT),
            SemanticErrorKind::UncheckedOption => Some(CODE_UNCHECKED_OPTION),
            SemanticErrorKind::UndefinedVariable => {
                Some(crate::interpreter::values::CODE_UNDEFINED_VARIABLE)
            }
            SemanticErrorKind::OpaqueConcat => None,
            SemanticErrorKind::ImplicitOpaqueCoercion => None,
            SemanticErrorKind::UnknownTemplate => None,
            SemanticErrorKind::BreakOutsideLoop => Some(CODE_BREAK_OUTSIDE_LOOP),
            SemanticErrorKind::RawInsertNotHtml => Some(CODE_RAW_INSERT_NOT_HTML),
            SemanticErrorKind::SafePipeNotHtml => Some(CODE_SAFE_PIPE_NOT_HTML),
            SemanticErrorKind::UnsafePipeForm => None,
            SemanticErrorKind::BuiltinParked => Some(CODE_BUILTIN_PARKED),
            SemanticErrorKind::McpPolicyInvalid => Some(CODE_MCP_POLICY_INVALID),
            SemanticErrorKind::BuiltinExperimental => Some(CODE_BUILTIN_EXPERIMENTAL),
            SemanticErrorKind::Other => None,
        }
    }
}

// №799 (T2, gh#1488): the parked refusal's stable code — the SAME
// constant the coded errors use (values.rs), so the check gate, the run
// gate's position-0 stamp, the VM compile origin and the try-branching
// mechanics all name one class.
use crate::interpreter::values::CODE_BUILTIN_PARKED;
/// №799: the experimental-tier WARNING class — a check-lane warning
/// (never an error; the exit-0 semantics), printed by `mlog check` and
/// partitioned out of the blocking path (see check_program's tail).
pub const CODE_BUILTIN_EXPERIMENTAL: &str = "BUILTIN_EXPERIMENTAL";
/// №801 (gh#1548, the gateway G2): the mcp_policy entry is not a legal
/// class — an unknown Label-annotation word or a bare (local-namespace)
/// tool name.
pub const CODE_MCP_POLICY_INVALID: &str = "MCP_POLICY";

/// Наряд №523: the ONE explicit exemption list for the run/serve
/// blocking gates. EVERY `check_program` error blocks by default; a kind
/// is exempt only if listed here — by enum, never by message substring.
///
/// Current exemptions: NONE. The №523 static import merge (the gate
/// resolves imports before checking) removed the only historical reason
/// to exempt a class — the false-positive "undefined function" on
/// symbols that the interpreter resolves from imported modules at
/// runtime. The function stays as the single named place: a future
/// exemption must land HERE, visibly, with its rationale — not as a
/// new substring filter at a call site.
pub fn is_exempt_from_blocking(kind: SemanticErrorKind) -> bool {
    let _ = kind; // the explicit list is empty today (see the doc comment)
    false
}

/// Наряд №523: one formatting rule for the blocking gates' per-finding
/// lines — the semantic message, plus the 1-indexed line when the span
/// is known (line 0 is the historical `at_line(0)` unknown-span
/// fallback; "(line 0)" would be a lie of its own).
pub fn format_blocking_line(err: &SpannedError) -> String {
    if err.span.start_line > 0 {
        format!("{} (line {})", err.message, err.span.start_line)
    } else {
        err.message.clone()
    }
}

// ── №673 (gh#1200, В41): BREAK_OUTSIDE_LOOP — the semantic gate ─────────
//
// The R-2 audit of the revision `7c3456b` (v0.30.1) recorded TWO «молча
// неверные» break/continue edges (the class-4 dictionary №653):
//
//   • №658-b — a bare break/continue OUTSIDE any loop: the TW refused at
//     RUNTIME («break/continue used outside of a loop»), the VM compiled
//     the statement to NOTHING and continued (the audit's import_batch
//     shape: `break` on the error path instead of `return`, and the code
//     after it ran);
//   • №658-a — a break/continue crossing the VALUE channel (an arm body
//     of a match/if-as-value): the TW refused at RUNTIME (the
//     eval_statements boundary cannot carry a control signal), the VM
//     silently ABSORBED the signal and ran the loop to its condition.
//
// №673 repairs both at the SEMANTIC level (the audit's recommendation —
// not a runtime mechanism): the refusal fires BEFORE execution on BOTH
// backends. The walker below is the ONE shared implementation, wired into
// BOTH production gates — `check_program` (the TW run/serve path) and
// `compile_program` (the VM compile→run path). The loop-context model:
//
//   • loop bodies (each/each-with-index/while) OPEN the loop context;
//   • statement-position branches (if/else-if/else, if-then, match arms)
//     PRESERVE it — a break nested through if/match stays legal (the
//     №658 contract the norms S-BLK-011/012 pin, unchanged);
//   • VALUE-CHANNEL bodies (BlockIfElse / MatchExpr arm statements — any
//     expression-position body) FORBID break/continue outright: the
//     control signal cannot cross the value channel (the TW's own
//     runtime rule, now static);
//   • a loop inside a value-channel arm re-opens the legal loop context
//     for its own body (the boundary the signal must reach is that loop).
//
// `Statement::Break/Continue` carry no span (the AST unit variants), so
// the finding names the PLACE the honest way — the enclosing declaration
// (pattern/tool/route/hook/on_deny/test) — and the line stays the
// unknown-span fallback (format_blocking_line omits it; the CODE is what
// machine consumers read, №479).

/// The №673 context walk over one statement list. `in_loop` — a loop body
/// encloses this position; `in_value` — a value-channel arm body encloses
/// it. `place` — the human-readable enclosing-declaration name for the
/// message.
fn break_walk_stmts(
    stmts: &[Statement],
    in_loop: bool,
    in_value: bool,
    place: &str,
    errors: &mut Vec<SpannedError>,
) {
    for stmt in stmts {
        match stmt {
            Statement::Break | Statement::Continue => {
                let word = if matches!(stmt, Statement::Break) {
                    "break"
                } else {
                    "continue"
                };
                if in_value {
                    // №658-a: the signal crosses the value channel.
                    errors.push(
                        SpannedError::at_line(
                            format!(
                                "{} crosses the value channel (an arm body of a match/if-as-value) in {}: \
                                 control signals cannot leave the value channel — keep the break/continue \
                                 in the loop body (№658-a, REPAIRED by №673)",
                                word, place
                            ),
                            0,
                        )
                        .with_kind(SemanticErrorKind::BreakOutsideLoop),
                    );
                } else if !in_loop {
                    // №658-b: no enclosing loop to exit or skip.
                    errors.push(
                        SpannedError::at_line(
                            format!(
                                "{} outside any loop in {}: there is no enclosing each/while to exit \
                                 or skip — use return (№658-b, REPAIRED by №673)",
                                word, place
                            ),
                            0,
                        )
                        .with_kind(SemanticErrorKind::BreakOutsideLoop),
                    );
                }
            }
            Statement::Each { iterable, body, .. } => {
                break_walk_expr(iterable, in_loop, place, errors);
                break_walk_stmts(body, true, false, place, errors);
            }
            Statement::EachWithIndex { iterable, body, .. } => {
                break_walk_expr(iterable, in_loop, place, errors);
                break_walk_stmts(body, true, false, place, errors);
            }
            Statement::While {
                condition, body, ..
            } => {
                break_walk_expr(condition, in_loop, place, errors);
                break_walk_stmts(body, true, false, place, errors);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                break_walk_expr(condition, in_loop, place, errors);
                break_walk_stmts(then_body, in_loop, in_value, place, errors);
                for (cond, body) in else_ifs {
                    break_walk_expr(cond, in_loop, place, errors);
                    break_walk_stmts(body, in_loop, in_value, place, errors);
                }
                if let Some(eb) = else_body {
                    break_walk_stmts(eb, in_loop, in_value, place, errors);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                break_walk_expr(condition, in_loop, place, errors);
                break_walk_stmts(body, in_loop, in_value, place, errors);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                break_walk_expr(scrutinee, in_loop, place, errors);
                for arm in arms {
                    break_walk_stmts(arm.body(), in_loop, in_value, place, errors);
                }
                if let Some(eb) = else_body {
                    break_walk_stmts(eb, in_loop, in_value, place, errors);
                }
            }
            Statement::LetBinding { value, .. } => {
                break_walk_expr(value, in_loop, place, errors);
            }
            Statement::Assign { value, .. } => {
                break_walk_expr(value, in_loop, place, errors);
            }
            Statement::Return { value, .. } => {
                break_walk_expr(value, in_loop, place, errors);
            }
            Statement::ExprStmt { expr, .. } => {
                break_walk_expr(expr, in_loop, place, errors);
            }
            Statement::Memorize(m) => {
                break_walk_expr(&m.value, in_loop, place, errors);
            }
            Statement::Forget(f) => {
                break_walk_expr(&f.query, in_loop, place, errors);
            }
            Statement::Relate(r) => {
                break_walk_expr(&r.from, in_loop, place, errors);
                break_walk_expr(&r.to, in_loop, place, errors);
            }
        }
    }
}

/// The №673 expression walk: value-channel arm bodies (BlockIfElse /
/// MatchExpr) are entered with `in_value = true`; every other variant
/// recurses into its sub-expressions with the context unchanged. The
/// break/continue themselves are STATEMENTS — an expression can carry one
/// only through a value-channel arm body.
fn break_walk_expr(expr: &Expr, in_loop: bool, place: &str, errors: &mut Vec<SpannedError>) {
    match expr {
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            break_walk_expr(condition, in_loop, place, errors);
            break_walk_stmts(then_body, in_loop, true, place, errors);
            for (cond, body) in else_ifs {
                break_walk_expr(cond, in_loop, place, errors);
                break_walk_stmts(body, in_loop, true, place, errors);
            }
            if let Some(eb) = else_body {
                break_walk_stmts(eb, in_loop, true, place, errors);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            break_walk_expr(scrutinee, in_loop, place, errors);
            for arm in arms {
                break_walk_stmts(arm.body(), in_loop, true, place, errors);
            }
            if let Some(eb) = else_body {
                break_walk_stmts(eb, in_loop, true, place, errors);
            }
        }
        Expr::FieldAccess { object, .. } => {
            break_walk_expr(object, in_loop, place, errors);
        }
        Expr::FnCall { args, .. } => {
            for a in args {
                break_walk_expr(a, in_loop, place, errors);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                break_walk_expr(a, in_loop, place, errors);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            break_walk_expr(left, in_loop, place, errors);
            break_walk_expr(right, in_loop, place, errors);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            break_walk_expr(condition, in_loop, place, errors);
            break_walk_expr(then_branch, in_loop, place, errors);
            break_walk_expr(else_branch, in_loop, place, errors);
        }
        Expr::List { items, .. } => {
            for it in items {
                break_walk_expr(it, in_loop, place, errors);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            break_walk_expr(object, in_loop, place, errors);
            break_walk_expr(index, in_loop, place, errors);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                break_walk_expr(v, in_loop, place, errors);
            }
        }
        Expr::Try { expr, .. } => {
            break_walk_expr(expr, in_loop, place, errors);
        }
        Expr::ProvBind { inner, .. } => {
            break_walk_expr(inner, in_loop, place, errors);
        }
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::HandleSource { .. } => {}
    }
}

/// №673: the BREAK_OUTSIDE_LOOP violations over the whole program — every
/// executable statement body of every declaration kind that carries one
/// (pattern, tool method, server route, hook, on_deny handler, test).
/// The ONE shared implementation: called by `check_program` (the TW
/// run/serve gate) and by `compile_program` (the VM compile gate) — the
/// refusal is backend-independent by construction.
pub fn break_outside_loop_violations(declarations: &[Declaration]) -> Vec<SpannedError> {
    let mut errors: Vec<SpannedError> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let place = format!("pattern '{}'", p.name);
                break_walk_stmts(&p.body, false, false, &place, &mut errors);
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let place = format!("tool '{}' method '{}'", t.name, m.name);
                    break_walk_stmts(&m.body, false, false, &place, &mut errors);
                }
            }
            Declaration::MlogServer(s) => {
                for r in &s.routes {
                    let place = format!("route '{}'", r.path);
                    break_walk_stmts(&r.body, false, false, &place, &mut errors);
                }
            }
            Declaration::Hook(h) => {
                let place = format!("hook '{:?}'", h.phase);
                break_walk_stmts(&h.body, false, false, &place, &mut errors);
            }
            Declaration::OnDeny(d) => {
                let place = format!("on_deny('{}')", d.class);
                break_walk_stmts(&d.body, false, false, &place, &mut errors);
            }
            Declaration::Test(t) => {
                let place = format!("test '{}'", t.name);
                break_walk_stmts(&t.body, false, false, &place, &mut errors);
            }
            // №532: no wildcard arms — the declaration kinds WITHOUT an
            // executable statement body (templates, db/schema, entities,
            // flows, config declarations, ...) carry nothing to walk. A
            // NEW declaration kind must be classified here explicitly —
            // if it gains a statement body, the walker must see it.
            Declaration::Template(_)
            | Declaration::Db(_)
            | Declaration::Schema(_)
            | Declaration::SkillIndex(_)
            | Declaration::Memory(_)
            | Declaration::Import(_)
            | Declaration::EntityType(_)
            | Declaration::EntityRecord(_)
            | Declaration::EntitySimple(_)
            | Declaration::Rule(_)
            | Declaration::Memorize(_)
            | Declaration::Forget(_)
            | Declaration::Fluid(_)
            | Declaration::Adapt(_)
            | Declaration::Relate(_)
            | Declaration::Sandbox(_)
            | Declaration::Mutate(_)
            | Declaration::Eval(_)
            | Declaration::LearnablePattern(_)
            | Declaration::Flow(_)
            | Declaration::Conversation(_)
            | Declaration::LlmConfig(_)
            | Declaration::ContextBudget(_)
            | Declaration::TypeAlias(_)
            | Declaration::Reflex(_)
            | Declaration::ReflexSeq(_)
            | Declaration::ReflexGen(_)
            | Declaration::Vision(_)
            | Declaration::Profile(_)
            | Declaration::McpPolicy(_)
            | Declaration::Origin(_) => {}
        }
    }
    errors
}

/// №685 (gh#1252, В44 — the S-1 audit): the RAW_INSERT_NOT_HTML
/// violations over the whole program — every template declaration whose
/// body uses a parameter in the RAW insert `{{{ param }}}` while the
/// parameter is NOT declared `Html`.
///
/// THE WHY: the N9 raw form (gh#1219) lands the value VERBATIM in the
/// page. The «safe-by-construction» claim assumed Html-typed raw
/// parameters, but nothing ENFORCED it — the audit's XSS scenario
/// (`template Comment(author: String, text: String) { … {{{ text }}} … }`
/// fed from `json_body()`) passed check/run/serve silently, because
/// every static layer answered «sanitized» at the render boundary. The
/// gate makes the assumption a FACT: after it, a raw value can only
/// come from an explicit Html producer — already escaped (ADR-0188 (б)).
///
/// The ONE shared implementation: called by `check_program` (the TW
/// run/serve gate) and by `compile_program` (the VM compile gate — the
/// №673 contour shape). The scan reuses THE tokenizer that renders
/// (`template_scan::raw_param_positions`, the UNGATED home — render()
/// is an ungated builtin) — a second parser would drift from the
/// render-time behavior, and the drift is exactly how S-1 happened.
/// Alias-resolved parameter types (the №119 map) — a
/// `type Body = Html` alias is honest Html.
pub fn raw_insert_not_html_violations(declarations: &[Declaration]) -> Vec<SpannedError> {
    let mut errors: Vec<SpannedError> = Vec::new();
    let (type_alias_map, _) = build_type_alias_map(declarations);
    for decl in declarations {
        let Declaration::Template(t) = decl else {
            continue;
        };
        // The per-(template, param) dedup: the same raw parameter may
        // appear many times in the body — ONE refusal names it once.
        let mut seen: HashSet<usize> = HashSet::new();
        let param_names: Vec<String> = t.params.iter().map(|p| p.name.clone()).collect();
        for idx in crate::template_scan::raw_param_positions(&t.body, &param_names) {
            if !seen.insert(idx) {
                continue;
            }
            let prm = &t.params[idx];
            let resolved = type_alias_map
                .get(&prm.type_name)
                .map(|s| s.as_str())
                .unwrap_or(&prm.type_name);
            if resolved != "Html" {
                // The literal brace forms are built by repetition — the
                // format! escaping of nested braces is exactly the kind
                // of noise a security message does not need.
                let raw_form = format!("{} {} {}", "{".repeat(3), prm.name, "}".repeat(3));
                let esc_form = format!("{} {} {}", "{".repeat(2), prm.name, "}".repeat(2));
                errors.push(
                    SpannedError::at(
                        format!(
                            "template '{}': parameter '{}' is used in the raw insert {} but is declared '{}' — \
                             raw-insert parameters must be Html (declare the parameter Html and build the value \
                             with an explicit Html producer, or use the escaped '{}' form) \
                             (S-1, REPAIRED by №685)",
                            t.name, prm.name, raw_form, prm.type_name, esc_form
                        ),
                        t.span.clone(),
                    )
                    .with_kind(SemanticErrorKind::RawInsertNotHtml),
                );
            }
        }
    }
    errors
}

/// №695 (gh#1274, В46 — ADR-0188 phase 2): the safe-stage violations
/// over the whole program, TWO classes in ONE scan (the shared
/// `template_scan::pipe_scan` — the ONE tokenizer, no second parser):
///
///   • SAFE_PIPE_NOT_HTML — a template parameter used in the recognized
///     safe stage `{{ param | safe }}` is declared with a non-Html
///     type. The safe stage inserts the value RAW (the escape-exempt),
///     so the same S-1 logic as №685 applies — the raw slot must carry
///     an explicitly produced Html value, and the declaration must be
///     honest about the type.
///   • UNSAFE_PIPE_FORM — a pipe tag the v1 grammar does not recognize
///     (an unknown stage, multiple pipes, a pipe in the raw triple-brace
///     form, an expr that is not a declared param). The N9 refusal
///     (gh#1219) narrows to exactly these forms and shifts left here;
///     the registration refusal stays as the runtime backstop.
///
/// THE WHY: the safe stage is the escape-exempt — the one form where a
/// String value would land VERBATIM in the page by design. The check-time
/// Html typing is what makes that design honest (the ADR-0188 phase-2
/// verdict): after the gate, a raw landing in a safe slot can only come
/// from a declared-Html parameter fed by an explicit Html producer.
///
/// The ONE shared implementation: called by `check_program` (the TW
/// run/serve gate) and by `compile_program` (the VM compile gate — the
/// №673 contour shape). Alias-resolved parameter types (the №119 map) —
/// a `type Body = Html` alias is honest Html.
pub fn safe_pipe_violations(declarations: &[Declaration]) -> Vec<SpannedError> {
    let mut errors: Vec<SpannedError> = Vec::new();
    let (type_alias_map, _) = build_type_alias_map(declarations);
    for decl in declarations {
        let Declaration::Template(t) = decl else {
            continue;
        };
        // The per-(template, param) dedup for the safe class — the same
        // parameter may appear in several safe slots; ONE refusal names
        // it once. The unsupported forms name each distinct tag once.
        let mut seen_safe: HashSet<usize> = HashSet::new();
        let mut seen_unsupported: HashSet<String> = HashSet::new();
        let param_names: Vec<String> = t.params.iter().map(|p| p.name.clone()).collect();
        let (safe_idx, unsupported) = crate::template_scan::pipe_scan(&t.body, &param_names);
        for idx in safe_idx {
            if !seen_safe.insert(idx) {
                continue;
            }
            let prm = &t.params[idx];
            let resolved = type_alias_map
                .get(&prm.type_name)
                .map(|s| s.as_str())
                .unwrap_or(&prm.type_name);
            if resolved != "Html" {
                let safe_form = format!("{} {} | safe {}", "{".repeat(2), prm.name, "}".repeat(2));
                errors.push(
                    SpannedError::at(
                        format!(
                            "template '{}': parameter '{}' is used in the safe insert {} but is declared '{}' — \
                             safe-insert parameters must be Html (declare the parameter Html and build the value \
                             with an explicit Html producer, or drop the stage — the escaped default) \
                             (№695, ADR-0188 phase 2)",
                            t.name, prm.name, safe_form, prm.type_name
                        ),
                        t.span.clone(),
                    )
                    .with_kind(SemanticErrorKind::SafePipeNotHtml),
                );
            }
        }
        for tag in unsupported {
            if !seen_unsupported.insert(tag.clone()) {
                continue;
            }
            errors.push(
                SpannedError::at(
                    format!(
                        "template '{}': the pipe tag '{{ {} }}' is not part of the v1 pipe grammar — \
                         the ONLY supported stage is '{{ param | safe }}' with the parameter declared Html \
                         (№695, ADR-0188 phase 2; the N9 refusal shifted left)",
                        t.name, tag
                    ),
                    t.span.clone(),
                )
                .with_kind(SemanticErrorKind::UnsafePipeForm),
            );
        }
    }
    errors
}

/// Наряд №523: statically resolve a program's import tree so the
/// semantic pass sees every symbol the interpreter will have at runtime.
///
/// THE WHY: the historical reason the run path did NOT block on every
/// semantic finding was a false-positive class — "undefined function"
/// on symbols merged from imported modules at runtime (`handle_import`
/// loads `{base_dir}/{path}.mlog` and registers its patterns flat).
/// With the gate resolving the same import tree BEFORE checking, the
/// false-positive class is gone and nothing needs to be exempt.
///
/// MECHANICS (mirror the runtime loader and the bytecode compiler's
/// `resolve_import` — same file-lookup rule `{base_dir}/{path}.mlog`,
/// same recursive sub-import walk, same visited-set on module path):
///
///   * the importing file's declarations WIN over module declarations
///     (runtime parity: imports register first, main registration is
///     last-wins);
///   * name collisions BETWEEN modules are deduplicated first-wins —
///     the runtime treats a flat-merge collision as a warning, so the
///     gate must not invent a "duplicate pattern" error the runtime
///     never raises;
///   * a cycle is skipped via the visited set (no new error class — the
///     runtime loader still rejects circular imports with its own loud
///     error, so net run behavior is unchanged);
///   * `Profile` is skipped (the runtime import loader skips it too).
///
/// The result is the module-side declaration list to be PREPENDED to the
/// program's own declarations before `check_program`. This is a gate
/// helper: it never executes anything, it only parses.
pub fn resolve_imports_statically(
    declarations: &[Declaration],
    base_dir: &std::path::Path,
) -> Result<Vec<Declaration>, String> {
    // Reserve the importing file's names first — main wins.
    let mut reserved: HashSet<(String, String)> =
        declarations.iter().filter_map(decl_import_ident).collect();
    let mut merged: Vec<Declaration> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    collect_import_decls(
        declarations,
        base_dir,
        &mut visited,
        &mut reserved,
        &mut merged,
    )?;
    Ok(merged)
}

/// DFS over `Declaration::Import` nodes: parse each module file once
/// (visited-set on the trimmed import path), recurse into sub-imports,
/// and append the module's own declarations (deduplicated) to `out`.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn collect_import_decls(
    declarations: &[Declaration],
    base_dir: &std::path::Path,
    visited: &mut HashSet<String>,
    reserved: &mut HashSet<(String, String)>,
    out: &mut Vec<Declaration>,
) -> Result<(), String> {
    for decl in declarations {
        if let Declaration::Import(import) = decl {
            // Parser may include trailing whitespace in `import_path` when
            // the optional `as alias` group is present — the runtime loader
            // trims (modules.rs handle_import); mirror it exactly.
            let module_path = import.path.trim().to_string();
            if !visited.insert(module_path.clone()) {
                continue;
            }
            // №558: the ONE module-search rule (src/module_path.rs) — the
            // same SSOT the runtime loader and the compiler call.
            let file_path = crate::module_path::resolve_module_file(base_dir, &module_path);
            // №475: the import source loader — compile-time, AUTHOR-controlled
            // source text (the same trust domain as the file being checked);
            // the sandbox targets PROGRAM-RUNTIME I/O. The runtime loader
            // (interpreter/modules.rs) and the bytecode compiler's
            // `resolve_import` carry the same justification.
            #[allow(clippy::disallowed_methods)]
            // №475: the import source loader — compile-time, AUTHOR-controlled
            // source text (the same trust domain as the file being checked);
            // the sandbox targets PROGRAM-RUNTIME I/O. The runtime loader
            // (interpreter/modules.rs) and the bytecode compiler's
            // `resolve_import` carry the same justification.
            #[allow(clippy::disallowed_methods)]
            // №475: the import source loader — compile-time, AUTHOR-controlled
            // source text (the same trust domain as the file being checked);
            // the sandbox targets PROGRAM-RUNTIME I/O. The runtime loader
            // (interpreter/modules.rs) and the bytecode compiler's
            // `resolve_import` carry the same justification.
            #[allow(clippy::disallowed_methods)]
            let source = std::fs::read_to_string(&file_path).map_err(|e| {
                format!(
                    "import '{}': cannot read {:?}: {} (the №523 semantic gate resolves imports statically with the same file-lookup rule as the runtime loader)",
                    module_path, file_path, e
                )
            })?;
            let module_decls = crate::parser::parse(&source)
                .map_err(|e| format!("parse error in module '{}': {}", module_path, e))?;
            collect_import_decls(&module_decls, base_dir, visited, reserved, out)?;
            for module_decl in module_decls {
                match &module_decl {
                    // Sub-imports were handled by the recursion above.
                    Declaration::Import(_) => {}
                    // The runtime import loader skips profiles (№325).
                    Declaration::Profile(_) => {}
                    _ => match decl_import_ident(&module_decl) {
                        // Collision: the runtime flat-merge WARNS, it does not
                        // error — the gate must not invent a duplicate error.
                        Some(key) if reserved.contains(&key) => {}
                        Some(key) => {
                            reserved.insert(key);
                            out.push(module_decl);
                        }
                        None => out.push(module_decl),
                    },
                }
            }
        }
    }
    Ok(())
}

/// The (kind-tag, name) pair a declaration is deduplicated by during the
/// static import merge. The kind tag matches `check_program`'s own
/// per-kind duplicate namespaces (pattern / learnable / entity_type /
/// entity / flow / reflex / template / vision / ...): a Pattern and an
/// Entity may legally share a name, two Patterns may not. Unnamed
/// declarations (rules, memorize, hooks, ...) return None and are
/// always included — they carry no duplicate-name error class.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn decl_import_ident(decl: &Declaration) -> Option<(String, String)> {
    let tag: &str;
    let name: String;
    match decl {
        Declaration::Pattern(p) => {
            tag = "pattern";
            name = p.name.clone();
        }
        Declaration::LearnablePattern(lp) => {
            tag = "learnable";
            name = lp.name.clone();
        }
        Declaration::EntityType(e) => {
            tag = "entity_type";
            name = e.name.clone();
        }
        Declaration::EntityRecord(e) => {
            tag = "entity";
            name = e.name.clone();
        }
        Declaration::EntitySimple(e) => {
            tag = "entity";
            name = e.name.clone();
        }
        Declaration::Flow(f) => {
            tag = "flow";
            name = f.name.clone();
        }
        Declaration::Reflex(r) => {
            tag = "reflex";
            name = r.name.clone();
        }
        Declaration::Template(t) => {
            tag = "template";
            name = t.name.clone();
        }
        Declaration::Vision(v) => {
            tag = "vision";
            name = v.name.clone();
        }
        Declaration::Sandbox(s) => {
            tag = "sandbox";
            name = s.name.clone();
        }
        Declaration::Tool(t) => {
            tag = "tool";
            name = t.name.clone();
        }
        _ => return None,
    }
    Some((tag.to_string(), name))
}

/// №581 (the audit d63cc1d X-1): the stable №479 diagnostic code of the
/// respond-terminality advisory. №584 (X-1 step 2) DOWNGRADED the finding
/// from a blocking error to a style WARNING — the VM now compiles bare
/// respond* as terminal in ANY route position (exact TW parity), so the
/// class is closed at the language level; the code survives as the stable
/// machine tag of the advisory (log scrapers, the corpus tooling read the
/// CODE, not the prose).
pub const CODE_RESPOND_NOT_TERMINAL: &str = "RESPOND_NOT_TERMINAL";

/// №600 (the audit 25b375e Y-1, High): the stable №479 diagnostic code of
/// the RESPOND_SWALLOWED refusal — a bare respond* carried by a statement
/// nested under a top-level block-form if/else branch is discarded by the
/// serve lane on BOTH backends (the guard-bypass regression №584 opened);
/// the finding blocks run/serve again and the refusal stamps this code
/// first (the №523 shape), so machine consumers read the CODE.
pub const CODE_RESPOND_SWALLOWED: &str = "RESPOND_SWALLOWED";

/// №673 (gh#1200, В41): the stable №479 diagnostic code of the
/// BREAK_OUTSIDE_LOOP refusal — a break/continue (а) outside any loop
/// (№658-b: the TW refused at runtime, the VM compiled the statement to
/// NOTHING and continued — the audit's import_batch shape: `break` on the
/// error path instead of `return`, and the code after it RAN), or (б)
/// crossing a value-channel boundary — an arm body of a match/if-as-value
/// (№658-a: the TW refused at runtime, the VM silently ABSORBED the signal
/// and ran the loop to its condition). Both are the «молча неверные»
/// class-4 dictionary entries (№653); №673 shifts the refusal to the
/// semantic gate — a loud refusal BEFORE execution on BOTH backends (the
/// check lives in the shared semantic contour, wired into check_program
/// AND compile_program; the runtime mirrors stay untouched as backstops).
/// The gate stamps this code first (the №523 shape).
pub const CODE_BREAK_OUTSIDE_LOOP: &str = "BREAK_OUTSIDE_LOOP";

/// №685 (gh#1252, В44 — the S-1 audit): the stable code of the
/// RAW_INSERT_NOT_HTML gate — a template parameter used in the RAW
/// insert `{{{ param }}}` must be declared `Html`; anything else refuses
/// BEFORE execution on BOTH backends (check_program AND compile_program,
/// the №673 contour shape; the runtime mirrors stay untouched as
/// backstops). The №385/ADR-0169 classification: the code is ADDED,
/// nothing renamed. The gate stamps this code first (the №523 shape).
pub const CODE_RAW_INSERT_NOT_HTML: &str = "RAW_INSERT_NOT_HTML";

/// №695 (gh#1274, В46 — ADR-0188 phase 2): the fail-closed companion of
/// RAW_INSERT_NOT_HTML for the safe stage — the escape-exempt insert
/// `{{ param | safe }}` must be declared `Html`; anything else refuses
/// BEFORE execution on BOTH backends (check_program AND
/// compile_program, the №673 contour shape; the registration refusal
/// stays as the runtime backstop). The №385/ADR-0169 classification:
/// the code is ADDED, nothing renamed. The gate stamps this code first
/// (the №523 shape).
pub const CODE_SAFE_PIPE_NOT_HTML: &str = "SAFE_PIPE_NOT_HTML";

/// №600 (the audit 25b375e Y-1): the SSOT POSITION PREDICATE of the №584
/// early-answer surface — ONE enumeration consumed by BOTH the semantic
/// walk (`check_route_respond_terminality`) and the route-body compiler
/// (`route_body_with_terminal_responds`, which reuses these variants as
/// its recursion mode). The compiler is the REFERENCE enumerator (the TW
/// serve lane is the etalon); the semantic check never invents a second
/// enumeration — every position class below names the exact serve-lane
/// behavior it mirrors:
///
///   * the route body's DIRECT statements answer immediately (the
///     per-statement route walk turns a bare respond* value into the
///     response);
///   * `IfThen` / `Match` / cycle bodies propagate the HttpResponse from
///     ANY depth (eval_block!'s ControlFlow::Return) — cycles answer on
///     the FIRST iteration;
///   * a TOP-LEVEL block-form if/else (`Statement::IfElseBlock`) branch
///     is walked by a PER-STATEMENT loop in server.rs: a DIRECT bare
///     respond* answers, but the value of every NESTED statement is
///     DISCARDED and the branch continues — depth ≥ 2 does NOT stop the
///     route (the compiler's reference comment, compiler.rs).
///
/// The transition table (both consumers walk the SAME transitions):
///   route body → `OnEarlyAnswerSurface`;
///   `OnEarlyAnswerSurface` + IfThen/Match/cycle body →
///     `OnEarlyAnswerSurface` (the lowering descends in full-terminal
///     mode; the bodies propagate from any depth);
///   `OnEarlyAnswerSurface` + IfElseBlock branch → `DirectOnlyBranch`
///     (only the branch's DIRECT bare respond* answers);
///   `DirectOnlyBranch` + any NESTED statement body → `SwallowedNested`
///     (the serve lane discards nested-statement responses; the lowering
///     never descends here — it keeps the pre-№584 shape);
///   `SwallowedNested` + anything → `SwallowedNested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespondPosition {
    /// A bare respond* statement HERE answers the route early on BOTH
    /// backends (the №584 lowering rewrites it to `return respond*(...)`):
    /// the route body's direct statements and the IfThen/Match/cycle
    /// bodies at any depth. The finding stays the style advisory
    /// (RESPOND_NOT_TERMINAL, warnings) — the form works, `return
    /// respond(...)` remains the recommended explicit shape.
    OnEarlyAnswerSurface,
    /// The DIRECT-statement surface of a top-level block-form if/else
    /// branch: a DIRECT bare respond* still answers on both backends
    /// (advisory as above), but every statement body NESTED here descends
    /// to `SwallowedNested` — the per-statement branch walk discards
    /// nested-statement values.
    DirectOnlyBranch,
    /// A statement body nested under a top-level block-form if/else branch
    /// (depth ≥ 2): the serve lane discards EVERY response produced here
    /// and the branch continues on BOTH backends — a bare respond* here is
    /// the blocking `RESPOND_SWALLOWED` refusal (№600, fail-closed per the
    /// №581 образец; the migration is one word: `return respond(...)`).
    SwallowedNested,
}

/// №581: the four route-response builtins whose BARE (statement) form owns
/// the early-answer semantics. Exactly the registry
/// set (src/builtins/registry.rs, the "web" family) — a fifth name,
/// `html_response`, lives only in the sink-classification tables and is NOT
/// a registered builtin (a call to it already fails UNDEFINED_FUNCTION).
/// №584: pub(crate) — the ROUTE-BODY compiler reuses the SAME name set for
/// the terminal lowering (one SSOT for "what is a bare respond* call").
pub(crate) const RESPOND_STATEMENT_NAMES: [&str; 4] = [
    "respond",
    "respond_html",
    "respond_html_status",
    "respond_html_doc",
];

/// №581: is this expression a BARE call of one of the RESPOND_STATEMENT_NAMES?
/// Only the direct `name(args)` form counts: a respond* call wrapped in a
/// larger expression (passed as an argument, bound by `let`, compared) does
/// NOT terminate a route on EITHER backend (the value is consumed inline on
/// both) — it is not the X-1 divergence class, and inventing a broader net
/// would manufacture false positives the fail-closed rule forbids.
/// №584: pub(crate) — the route-body compiler reuses this exact predicate
/// for the terminal lowering (a wrapped call stays a plain value there too).
pub(crate) fn bare_respond_call(expr: &Expr) -> bool {
    match expr {
        Expr::FnCall { name, .. } => RESPOND_STATEMENT_NAMES.contains(&name.as_str()),
        // №532 ledger: the parse-only/report tail arm — no security
        // decision reads the non-call variants here (a respond* consumed
        // inside any larger expression terminates nothing on either
        // backend; see the doc comment above).
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::FieldAccess { .. }
        | Expr::QualifiedCall { .. }
        | Expr::BinaryOp { .. }
        | Expr::IfElse { .. }
        | Expr::List { .. }
        | Expr::IndexAccess { .. }
        | Expr::StructLit { .. }
        | Expr::BlockIfElse { .. }
        | Expr::MatchExpr { .. }
        | Expr::Try { .. }
        | Expr::HandleSource { .. }
        | Expr::ProvBind { .. } => false,
    }
}

/// №581 (the audit d63cc1d X-1) → №584 (X-1 step 2) → №600 (the audit
/// 25b375e Y-1): the respond-terminality walk over route bodies.
///
/// THE HISTORY (why this walk exists): before №584 the VM honored a bare
/// respond* only in the TAIL CHAIN (the route's final statement, or the
/// final statement of an if/match branch that itself ended the route — the
/// №574 branch-tail keep, recursively) and compiled every other position
/// with `Instruction::Pop` — the audit's depth-1 guard shape silently fell
/// through to the protected code while the TW answered immediately. №581
/// closed that window fail-closed (a blocking semantic error); №584 closed
/// the CLASS itself: the route-body compiler (compiler.rs `compile_routes`)
/// now lowers every bare respond* statement to `Call + Return` on the
/// TW early-answer surface — the route body's direct statements, the
/// IfThen/Match/cycle bodies (any depth), and the DIRECT statements of
/// top-level if/else branches. The VM stops at a bare respond* wherever
/// the TW does.
///
/// WHAT REMAINS ADVISORY is a style finding: `return respond(...)` stays
/// the RECOMMENDED form (the explicit early answer — self-documenting
/// control flow, and the shape the entire pre-№584 corpus was migrated
/// to). The advisory carries the stable `RESPOND_NOT_TERMINAL` code as its
/// kind so machine consumers keep reading one tag; it is pushed to
/// `warnings` and NEVER blocks run or serve.
///
/// №600 RESTORED the fail-closed refusal for the ONE form the №584
/// lowering does NOT reach: a bare respond* carried by a statement NESTED
/// under a top-level block-form if/else branch (the serve lane's
/// per-statement branch walk discards nested-statement responses on BOTH
/// backends — the audit's guard `if is_admin(...) == false { respond("403")
/// }` inside another block-if executed the protected DELETE for a
/// non-admin under a style-only warning). №584 retired the №581 error for
/// ALL forms at once, including this non-working one — the protection
/// regression is closed here: the position predicate `RespondPosition`
/// (the SSOT shared with the lowering — the compiler is the reference
/// enumerator) classifies every respond* site, and a `SwallowedNested`
/// site pushes the BLOCKING `RESPOND_SWALLOWED` error into `errors`
/// (with Span and the one-word migration hint). False positives are
/// impossible by construction: the predicate enumerates EXACTLY the
/// positions the lowering rewrites; everything else on the early-answer
/// surface keeps the advisory, and the positions the serve lane discards
/// now refuse loudly.
///
/// The recommended-form walk keeps the terminal-context recursion: a bare
/// respond in the route's tail chain (the №574 shapes) is idiomatic as-is
/// and stays unflagged; everything else (mid-route statements, non-tail
/// branches, cycle bodies) gets the return-form hint once per call site.
fn check_route_respond_terminality(
    stmts: &[Statement],
    terminal_ctx: bool,
    position: RespondPosition,
    route_label: &str,
    warnings: &mut Vec<SpannedError>,
    errors: &mut Vec<SpannedError>,
) {
    let last = stmts.len().checked_sub(1);
    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = Some(i) == last;
        match stmt {
            Statement::ExprStmt { expr, span } => {
                if !bare_respond_call(expr) {
                    continue;
                }
                if position == RespondPosition::SwallowedNested {
                    // №600: the serve lane discards this response on BOTH
                    // backends and the branch continues — the guard-bypass
                    // regression. Fail-closed again (the №581 posture):
                    // a blocking error with Span and the one-word fix.
                    errors.push(
                        SpannedError::at(
                            format!(
                                "respond() nested under a block-form if/else branch does NOT \
                                 stop the route on either backend — write `return respond(...)`; \
                                 ({route_label})"
                            ),
                            span.clone(),
                        )
                        .with_kind(SemanticErrorKind::RespondSwallowed),
                    );
                    continue;
                }
                // On the early-answer surface (direct route-body statements,
                // IfThen/Match/cycle bodies, DIRECT statements of a top-level
                // if/else branch): the form WORKS on both backends — style
                // advisory only, per the tail-chain rules.
                if !(terminal_ctx && is_last) {
                    warnings.push(
                        SpannedError::at(
                            format!(
                                "style: a bare respond* call outside the route tail chain — \
                                 prefer the explicit early-answer form `return respond(...)`; \
                                 ({route_label})"
                            ),
                            span.clone(),
                        )
                        .with_kind(SemanticErrorKind::RespondNotTerminal),
                    );
                }
            }
            Statement::Return { .. } => {
                // The recommended early-answer form on both backends — the
                // respond* call under a `return` is never flagged.
            }
            // IfThen/Match/cycle bodies: from the early-answer surface the
            // bodies stay on it (the lowering descends in full-terminal
            // mode — the bodies propagate the response from any depth);
            // from DirectOnlyBranch or SwallowedNested everything below is
            // swallowed (the serve lane discards nested-statement values).
            Statement::IfThen { body, .. } => {
                check_route_respond_terminality(
                    body,
                    terminal_ctx && is_last,
                    nested_respond_position(position),
                    route_label,
                    warnings,
                    errors,
                );
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                // The block-form if/else: a TOP-LEVEL one (entered from the
                // early-answer surface) walks its branches per-statement —
                // only the branch's DIRECT responds answer (DirectOnlyBranch);
                // a NESTED one (already DirectOnlyBranch/SwallowedNested) is
                // itself a discarded nested statement — swallowed below.
                let branch_ctx = terminal_ctx && is_last;
                let branch_position = branch_respond_position(position);
                check_route_respond_terminality(
                    then_body,
                    branch_ctx,
                    branch_position,
                    route_label,
                    warnings,
                    errors,
                );
                for (_, body) in else_ifs {
                    check_route_respond_terminality(
                        body,
                        branch_ctx,
                        branch_position,
                        route_label,
                        warnings,
                        errors,
                    );
                }
                if let Some(eb) = else_body {
                    check_route_respond_terminality(
                        eb,
                        branch_ctx,
                        branch_position,
                        route_label,
                        warnings,
                        errors,
                    );
                }
            }
            Statement::Match {
                arms, else_body, ..
            } => {
                let branch_ctx = terminal_ctx && is_last;
                for arm in arms {
                    check_route_respond_terminality(
                        arm.body(),
                        branch_ctx,
                        nested_respond_position(position),
                        route_label,
                        warnings,
                        errors,
                    );
                }
                if let Some(eb) = else_body {
                    check_route_respond_terminality(
                        eb,
                        branch_ctx,
                        nested_respond_position(position),
                        route_label,
                        warnings,
                        errors,
                    );
                }
            }
            // Cycle bodies: the TW answers on the FIRST iteration (the
            // HttpResponse-as-Return propagation) and the №584 VM lowering
            // (Return inside the compiled loop body) answers on the first
            // iteration too — full parity, advisory severity like anywhere
            // else ON THE SURFACE; swallowed under a block-if branch like
            // every other nested body. A bare respond inside a cycle is
            // unusual style, hence the same return-form hint.
            Statement::While { body, .. }
            | Statement::Each { body, .. }
            | Statement::EachWithIndex { body, .. } => {
                check_route_respond_terminality(
                    body,
                    false,
                    nested_respond_position(position),
                    route_label,
                    warnings,
                    errors,
                );
            }
            // LetBinding/Assign/Memorize/Forget/Relate/Break/Continue
            // carry no statement bodies; a respond* consumed inside
            // their VALUE expressions does not terminate a route on
            // either backend (see bare_respond_call).
            Statement::LetBinding { .. }
            | Statement::Assign { .. }
            | Statement::Break
            | Statement::Continue
            | Statement::Memorize(_)
            | Statement::Forget(_)
            | Statement::Relate(_) => {}
        }
    }
}

/// №600: the statement-body transition of the SSOT position predicate
/// (IfThen/Match/cycle bodies — the surfaces that propagate the response
/// from any depth). From the early-answer surface the bodies stay on it;
/// from a top-level if/else branch (DirectOnlyBranch) or deeper under one
/// (SwallowedNested) everything below is swallowed — the serve lane
/// discards the whole nested statement's value. THE SAME transition the
/// lowering applies when it decides whether to descend (compiler.rs: the
/// non-surface modes keep the pre-№584 shape — no rewrite could make a
/// discarded response answer).
fn nested_respond_position(position: RespondPosition) -> RespondPosition {
    match position {
        RespondPosition::OnEarlyAnswerSurface => RespondPosition::OnEarlyAnswerSurface,
        RespondPosition::DirectOnlyBranch | RespondPosition::SwallowedNested => {
            RespondPosition::SwallowedNested
        }
    }
}

/// №600: the branch-body transition of the SSOT position predicate
/// (Statement::IfElseBlock — the block-form if/else the serve lane walks
/// per-statement). A TOP-LEVEL one (entered from the early-answer surface)
/// puts its branches in DirectOnlyBranch (only the DIRECT bare respond*
/// answers); a nested one is itself a discarded statement — its branches
/// are swallowed. The lowering applies the SAME transition
/// (compiler.rs: OnEarlyAnswerSurface descends the branches in
/// DirectOnlyBranch mode; the other modes keep the pre-№584 shape).
fn branch_respond_position(position: RespondPosition) -> RespondPosition {
    match position {
        RespondPosition::OnEarlyAnswerSurface => RespondPosition::DirectOnlyBranch,
        RespondPosition::DirectOnlyBranch | RespondPosition::SwallowedNested => {
            RespondPosition::SwallowedNested
        }
    }
}

/// №581: the declaration-level entry — every route of every server walks
/// the respond-terminality classification. №584: the surface findings land
/// in `warnings` (advisory); nothing on the early-answer surface blocks
/// run or serve. №600: the swallowed sites (statements nested under a
/// top-level block-form if/else branch) push the BLOCKING
/// RESPOND_SWALLOWED error into `errors` — the fail-closed posture is
/// restored exactly where the serve lane discards responses.
fn check_respond_terminality(
    declarations: &[Declaration],
    warnings: &mut Vec<SpannedError>,
    errors: &mut Vec<SpannedError>,
) {
    for decl in declarations {
        if let Declaration::MlogServer(srv) = decl {
            for r in &srv.routes {
                let label = format!("route {} {}", r.method, r.path);
                check_route_respond_terminality(
                    &r.body,
                    true,
                    RespondPosition::OnEarlyAnswerSurface,
                    &label,
                    warnings,
                    errors,
                );
            }
        }
    }
}

impl SpannedError {
    /// Build a `SpannedError` from a message and an explicit span.
    /// The kind defaults to `Other` (blocking); use `with_kind` at the
    /// construction sites that produce a named kind.
    pub fn at(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: SemanticErrorKind::Other,
        }
    }

    /// Наряд №523: attach an explicit structured kind (builder style).
    pub fn with_kind(mut self, kind: SemanticErrorKind) -> Self {
        self.kind = kind;
        self
    }

    /// Build a `SpannedError` from a message and a declaration's span.
    /// Mirrors the legacy `with_line_prefix` shape — but instead of
    /// prefixing the message with the line number, it carries the span
    /// structurally so LSP / programmatic consumers can read it
    /// directly. The message itself stays clean.
    pub fn at_decl(decl: &Declaration, message: impl Into<String>) -> Self {
        Self::at(message, decl.span().clone())
    }

    /// Build a `SpannedError` from a message and an expression's span.
    pub fn at_expr(expr: &Expr, message: impl Into<String>) -> Self {
        Self::at(message, expr.span().clone())
    }

    /// Build a `SpannedError` with no source position. Used for
    /// diagnostics whose originating node is not in the AST (e.g.
    /// audit-category findings that come back with only a line number).
    /// If `line > 0`, the span is constructed to point at that line so
    /// the LSP can still highlight something; otherwise the span is
    /// `Span::unknown()`.
    pub fn at_line(message: impl Into<String>, line: usize) -> Self {
        if line > 0 {
            Self::at(message, Span::new(line as u32, 0, line as u32, 0))
        } else {
            Self::at(message, Span::unknown())
        }
    }
}

/// Result of semantic analysis: errors prevent execution, warnings are advisory.
///
/// Наряд №165: `errors` and `warnings` now carry `SpannedError` (with
/// `Span`) instead of bare `String`. This is an internal breaking
/// change — every caller inside the crate has been updated to access
/// `.message` for display and `.span` for LSP / programmatic use.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub errors: Vec<SpannedError>,
    pub warnings: Vec<SpannedError>,
}

impl AnalysisResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Format analysis result for display.
    ///
    /// Preserves the human-readable form previously produced by
    /// `with_line_prefix` (so `mlog check` output stays stable): each
    /// diagnostic is shown as `строка N: <message>` when a span is
    /// present, or just `<message>` when the span is unknown.
    pub fn format(&self) -> String {
        fn render(err: &SpannedError) -> String {
            if err.span.start_line > 0 {
                format!("строка {}: {}", err.span.start_line, err.message)
            } else {
                err.message.clone()
            }
        }
        let mut lines = Vec::new();
        if !self.errors.is_empty() {
            let n = self.errors.len();
            if n == 1 {
                lines.push("1 error:".to_string());
            } else {
                lines.push(format!("{} errors:", n));
            }
            for (i, e) in self.errors.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, render(e)));
            }
        }
        if !self.warnings.is_empty() {
            let n = self.warnings.len();
            if n == 1 {
                lines.push("1 warning:".to_string());
            } else {
                lines.push(format!("{} warnings:", n));
            }
            for (i, w) in self.warnings.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, render(w)));
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            lines.push("OK: no issues found.".to_string());
        }
        lines.join("\n")
    }
}

/// Legacy helper preserved for backward compatibility with call sites
/// that already pass `(decl, format!(...))`. Now returns a `SpannedError`
/// instead of a `String` — same ergonomic shape, but carries the span.
fn with_line_prefix(decl: &Declaration, msg: String) -> SpannedError {
    SpannedError::at_decl(decl, msg)
}

/// Valid middleware names for mlogserver blocks.
const VALID_MIDDLEWARE: &[&str] = &["session", "csrf", "security_headers", "rate_limit", "cors"];

/// Valid HTTP methods for route declarations.
const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

// ── Statement-level label inference (Наряд №323, ADR-0154 Appendix A) ──
//
// Static propagation of labels `(conf, integrity, consent-scope)` through
// pattern bodies: every statement kind has a contract (input reading,
// output label, side effects, merge rule) recorded in ADR-0154 Appendix A
// and REFERENCE §2.2. Join at merge points is componentwise:
// if/else-if/else branches, match arms, loop exits.
//
// Sources and sanitizers reuse the audit.rs vocabulary (binding_taint),
// projected onto the lattice via `labels::legacy_taint_label` — the bridge
// table of ADR-0154 §5. This is the machinery №325's sink-gate reads.

/// №689 (gh#1256, R-1): the registry private-field bridge — the FIRST
/// flow-path input for the private field marks. Before it, the flow
/// engine was NAME-based only (`label_source`) and the registry
/// field-meta never reached it: the audit's words — «the machine does
/// not check the private → sink flow because there are almost no
/// marks» (R-1: one private row on 526, the mail ПДн all `untrusted`).
///
/// The contract, deliberately NARROW: a registry row whose field-meta
/// section carries `private` seeds the flow layer with the type-layer
/// secret projection — conf=private, integrity=trusted (the ADR-0154 §2
/// form the type-layer `Labeled(String, Private)` carries; the same
/// projection the `env`/`secret` arm uses). Rows without a private
/// mark stay on the old argument-propagation path — the INTEGRITY
/// bridging (untrusted rows) is label_source's kind vocabulary, not
/// this tranche's. The whole-value posture is the fail-closed
/// over-approximation: a struct that contains private data flows as
/// private (`email.uid` inherits the conf the same way
/// `expr_label(FieldAccess)` inherits the object's label); the per-
/// field precision needs the type-aware environment — the stage-2
/// contour. The cure is the existing one: `redact` (the ONLY downward
/// move, ADR-0136/№326). RED-first proof:
/// tests/naryad_689_private_field_flow.rs.
fn registry_private_flow_label(fn_name: &str) -> Option<Label> {
    let spec = crate::builtins::registry::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == fn_name)?;
    if spec.field_meta.is_empty() {
        return None;
    }
    let meta = crate::builtins::sig_types::parse_field_meta(spec.field_meta)?;
    let carries_private = meta
        .iter()
        .any(|(_, l)| *l == crate::builtins::sig_types::Label::Private);
    if !carries_private {
        return None;
    }
    Some(Label {
        conf: crate::labels::Conf::Private,
        integrity: crate::labels::Integrity::Trusted,
        consent: Default::default(),
    })
}

/// Bounded-fixpoint pass cap for `while` bodies. The conf lattice has
/// height 4 (`public < consented < private < poisoned`); monotone joins
/// stabilize any var→var loop-carried chain within that many passes —
/// 8 = height × 2 safety factor. Deterministic, always terminates.
const LABEL_FIXPOINT_MAX_PASSES: usize = 8;

/// Result of the statement-level label inference for one pattern.
#[derive(Debug, Clone)]
pub struct LabelInference {
    /// Final label of every variable bound in the pattern body (params
    /// included), after all merge joins.
    pub var_labels: BTreeMap<String, Label>,
    /// Componentwise join of every `Return` value label and `ExprStmt`
    /// result label — the pattern's output label.
    pub output_label: Label,
}

/// Source/sanitizer vocabulary — mirrors audit.rs `binding_taint`, keyed
/// the same way, projected via the ADR-0154 §5 table. `None` = not a
/// source (the caller falls back to argument propagation).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn label_source(fn_name: &str, args: &[Expr], env: &BTreeMap<String, Label>) -> Option<Label> {
    // Static kind names only — the ADR-0154 §5 table covers them (pinned
    // by the exhaustiveness test in audit.rs); the fallback never fires.
    let kind_label = |kind: &str| legacy_taint_label(kind).unwrap_or_else(Label::bottom);
    match fn_name {
        // Secret sources. №544 step 1: the label is the ADR-0154 lattice
        // directly (conf=private, integrity=trusted) — the same projection
        // the type-layer source carries (semantic_types::secret_source_type
        // == Labeled(String, Private)); the "Secret" taint-kind
        // indirection is retired with the stage-2 migration.
        "env" | "secret" => Some(Label {
            conf: crate::labels::Conf::Private,
            integrity: crate::labels::Integrity::Trusted,
            consent: Default::default(),
        }),
        // LLM-output sources (model output is untrusted — ADR-0117).
        "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
            Some(kind_label("LlmOutput"))
        }
        // Untrusted-input sources (№268: MCP tool output reuses UserInput).
        // N2 (gh#1211): request_header — the same untrusted request lane.
        // N5 (gh#1215, ADR-0192): the html extraction family — scraped
        // content is untrusted input, same lane.
        "form_data" | "json_body" | "query_param" | "request_header" | "mcp_call"
        | "html_select" | "html_text" | "html_links" | "feed_parse" | "sitemap_parse"
        | "xml_parse" => Some(kind_label("UserInput")),
        // №325: network ingress and file ingress are untrusted sources —
        // the sink-clearance gate needs their labels to attribute
        // UNTRUSTED_EGRESS_* classes (ADR-0161 §3).
        // №455 layer 3: a file INGEST whose path is a LITERAL matching
        // the sensitive-path deny-list (`.env*`, `*.db`, `*.sqlite*`,
        // `.git/**`, `*.mlog`, `metalogos.toml`, `.mlog/**`) yields the
        // SECRET label (private conf), not the UserInput kind — the
        // stok-checks must treat allowlisted sensitive-file content as a
        // secret (the audit's finding: `Label::Internal` let a read
        // `.env` flow into outputs unchecked). Conservative superset of
        // the config-conditional wording: statically the label engine
        // cannot see `METALOGOS_SENSITIVE_PATH_ALLOWLIST`, so a
        // sensitive-named literal is marked Private unconditionally — the
        // runtime gate is the only way to actually read the file, and the
        // label matches the operator's explicit vouch. Non-literal paths
        // keep the source kinds below.
        "read_file" | "read_file_tokens"
            if args
                .first()
                .map(|a| match a {
                    Expr::StringLit { value, .. } => {
                        crate::builtins::io::sensitive_path_match(value)
                    }
                    _ => false,
                })
                .unwrap_or(false) =>
        {
            // №544 step 1: the lattice label directly (conf=private,
            // integrity=trusted) — the "Secret" taint row is retired.
            Some(Label {
                conf: crate::labels::Conf::Private,
                integrity: crate::labels::Integrity::Trusted,
                consent: Default::default(),
            })
        }
        "http_get" | "read_file" => Some(kind_label("UserInput")),
        // Sanitizers restore trust. №765 (gh#1418, Wave 59 — the audit
        // S-1b): render is REMOVED — a template with raw slots (`{{{ }}}`)
        // conducts the untrusted state verbatim (the let-form XSS); the
        // escape_html status is untouched (the audit's boundary).
        // A2 (naryad 781, the audit-23.10 S-1): escape_html restores the
        // INTEGRITY only — the CONFIDENTIALITY is the join of the
        // arguments (the N2/gh#1211 ct_eq form): an escaped secret is
        // still a secret. The HTML_INJECTION taint lane keeps its
        // Sanitized status (the №685/№695/№765 XSS contract is not
        // weakened — untrusted data stays cleaned for HTML output).
        "escape_html" => {
            let joined = args
                .iter()
                .fold(Label::bottom(), |acc, a| acc.join(&expr_label(a, env)));
            Some(Label {
                conf: joined.conf,
                integrity: crate::labels::Integrity::Trusted,
                consent: joined.consent,
            })
        }
        // N2 (gh#1211): ct_eq — the constant-time comparison verdict.
        // The INTEGRITY is restored (the verdict is the "validate before
        // decide" data the №327 decision gate asks for — comparing an
        // untrusted token against the expected secret IS the
        // validation). The CONFIDENTIALITY is the join of the arguments
        // (the 1-bit answer is still derived from the compared values —
        // a verdict over private material stays private for egress,
        // mirroring verify_password's Secret classification).
        "ct_eq" => {
            let joined = args
                .iter()
                .fold(Label::bottom(), |acc, a| acc.join(&expr_label(a, env)));
            Some(Label {
                conf: joined.conf,
                integrity: crate::labels::Integrity::Trusted,
                consent: joined.consent,
            })
        }
        // №274 (ADR-0136): redact masks secrets — the ONLY downward move
        // for `private` until №326 formalizes redact/declassify. Semantics:
        // mode "secrets"/"all" maps private → public/trusted; every other
        // label passes through UNCHANGED — quarantine (`poisoned`) is NOT
        // curable by redact (a channel is not a secret; ADR-0136 D2).
        "redact" => {
            let input = args
                .first()
                .map(|a| expr_label(a, env))
                .unwrap_or_else(Label::bottom);
            // №326: the policy is a VALUE — its target conf comes from
            // the registry (the single source of truth shared with the
            // runtime). Unknown/dynamic policies pass the input through
            // (conservative — no silent downward moves).
            let policy = args.get(1).and_then(|p| match p {
                Expr::StringLit { value, .. } => crate::builtins::string::redact_policy(value),
                _ => None,
            });
            let target_conf = policy.map(|pol| pol.target_conf);
            match (policy, target_conf) {
                // One-way: the data is destroyed — the result is a
                // compiler-derived value (bottom: public AND trusted).
                // Integrity is restored too (№327: the fingerprint
                // decisions are legal). QUARANTINE EXCEPTION: poisoned is
                // not curable by any policy (ADR-0154 §2.1 / §10) — the
                // channel is not the data.
                (_, Some("public")) => {
                    if input.conf == crate::labels::Conf::Poisoned {
                        Some(input)
                    } else {
                        Some(Label::bottom())
                    }
                }
                // №806 (B1): `hash_only` keeps the input CONFIDENTIALITY —
                // the unkeyed fingerprint of a low-entropy secret is
                // brute-forceable offline, so it must NOT declassify — but
                // still restores INTEGRITY: the full 256-bit digest is a
                // compiler-derived value for comparability (the №327
                // decision lift survives on non-secret data — the
                // error-code branching form). Public comparability of a
                // fingerprint is the keyed `hash_keyed` policy's job.
                (Some(pol), _) if pol.target_conf == "private" && pol.name == "hash_only" => {
                    if input.conf == crate::labels::Conf::Poisoned {
                        Some(input)
                    } else {
                        Some(Label {
                            integrity: crate::labels::Integrity::Trusted,
                            ..input
                        })
                    }
                }
                _ => Some(input),
            }
        }
        // №335 (spec §7.2 v2): consent_grant extends the value's
        // consent-scope set by the granted scope. Non-literal scope
        // cannot be named statically — conservative no-extension (the
        // redact dynamic-policy posture); the ledger still records the
        // runtime grant.
        "consent_grant" => {
            let input = args
                .first()
                .map(|a| expr_label(a, env))
                .unwrap_or_else(Label::bottom);
            match args.get(1) {
                Some(Expr::StringLit { value, .. }) => {
                    let consent = input
                        .consent
                        .clone()
                        .meet(crate::labels::ConsentScope::from_scopes([value.as_str()]));
                    Some(Label {
                        conf: input.conf,
                        integrity: input.integrity,
                        consent,
                    })
                }
                _ => Some(input),
            }
        }
        // №335: consent_revoke — the FLAT cascade entry point. The
        // revoked value (and every value derived from it) carries the
        // QUARANTINE label: poison is ABSORBING in the lattice
        // (ADR-0154 §2.1) — join with anything stays poisoned, so the
        // cascade is the lattice's own semantics, not a separate
        // analysis. Quarantine clears the consent scope too.
        "consent_revoke" => Some(Label {
            conf: crate::labels::Conf::Poisoned,
            integrity: crate::labels::Integrity::Untrusted,
            consent: Default::default(),
        }),
        // №599 (ADR-0182 §3.3 step 1): the Image media-input bridge —
        // the result label is the IMAGE argument's label: the image is
        // the DATA carrier, so its conf, integrity AND consent scope
        // ride into the description (the ADR's «private image in →
        // private description out» and the consent-scope-on-the-carrier
        // promise). The text parameters (prompt / lang / model)
        // contribute their conf and integrity conservatively — the
        // answer can mirror their content — but NOT their consent
        // scopes: a scope covers the data it was granted FOR, and the
        // description is the image's content, not the prompt's (the
        // prompt's own protection rides its conf axis). Without this
        // rule the generic argument join would start from `bottom` and
        // its intersection consent join would silently EMPTY the
        // handle's scope on every call — the consent ride the ADR
        // commits to would be dead on arrival (pinned by the
        // naryad-599 consent test; the string form's labels are
        // unchanged — its literals carry bottom anyway).
        "vision_understand" | "ocr_extract" => {
            let mut out = args
                .first()
                .map(|a| expr_label(a, env))
                .unwrap_or_else(Label::bottom);
            for a in args.iter().skip(1) {
                let l = expr_label(a, env);
                out.conf = out.conf.join(l.conf);
                out.integrity = out.integrity.join(l.integrity);
            }
            Some(out)
        }
        _ => None,
    }
}

/// Label of an expression under environment `env`. Join is componentwise;
/// literals and unknown identifiers are `bottom` (ADR-0154 Appendix A:
/// unannotated params and unresolved names start open — the sink-gate
/// №325 reads these labels, it does not trust them).
fn expr_label(expr: &Expr, env: &BTreeMap<String, Label>) -> Label {
    match expr {
        Expr::StringLit { .. } | Expr::FloatLit { .. } | Expr::BoolLit { .. } => Label::bottom(),
        // №332 (ADR-0164): HandleSource/ProvBind labels are resolved at
        // the LetBinding/Assign insertion points (origin declarations are
        // not in scope here). Legal constructions only appear there —
        // this arm is the conservative fallback (bottom) for the general
        // walker; the origin pass refuses illegal positions loudly.
        Expr::HandleSource { .. } => Label::bottom(),
        Expr::ProvBind { inner, .. } => expr_label(inner, env),
        Expr::Ident { name, .. } => env.get(name).cloned().unwrap_or_else(Label::bottom),
        Expr::FieldAccess { object, .. } => expr_label(object, env),
        Expr::FnCall { name, args, .. } => {
            // №765 (gh#1418, Wave 59 — the audit S-1b): render is NOT a
            // blanket sanitizer — the seeded raw map decides per position
            // (the №685 walk mirror): the all-escaped template keeps the
            // sanitizer semantics (the №685-D/n98 pins — the escaped slot
            // escapes at runtime), any raw slot conducts the join of its
            // raw args (the let-form XSS the audit pinned), and a
            // dynamic/unknown name is the conservative join of ALL value
            // args (the №685-E fail-closed pin). The BINARY predicate
            // lists took the blanket removal; the ENGINES carry the
            // position-aware form — the only shape that satisfies both
            // the audit's fix and the №685 interlocks.
            if name == "render" {
                let snapshot = crate::html_label::render_raw_snapshot();
                let raw_positions =
                    crate::html_label::resolve_raw_positions(&snapshot, args.first());
                let is_raw = |idx: usize| raw_positions.map(|p| p.contains(&idx)).unwrap_or(true);
                // A2 (naryad 781, the audit-23.10 S-1): an ESCAPED slot
                // restores the INTEGRITY only — its CONFIDENTIALITY is the
                // argument's (the ct_eq form); a raw slot conducts
                // EVERYTHING (№765 unchanged); an unknown/dynamic template
                // name stays the conservative conductor (fail-closed).
                let mut acc = Label::bottom();
                for (idx, a) in args.iter().skip(1).enumerate() {
                    let l = expr_label(a, env);
                    if is_raw(idx) {
                        acc = acc.join(&l);
                    } else {
                        acc = acc.join(&Label {
                            conf: l.conf,
                            integrity: crate::labels::Integrity::Trusted,
                            consent: l.consent,
                        });
                    }
                }
                return acc;
            }
            if let Some(l) = label_source(name, args, env) {
                return l;
            }
            // №689: the registry private-field bridge — the field marks
            // of a Struct-typed return reach the flow layer (the mail
            // ПДн egress speaks: the PII_EGRESS_* classes on
            // smtp_send/http_post/respond, the RED-first proof in
            // tests/naryad_689_private_field_flow.rs). After label_source
            // (the sanitizers keep precedence — redact cures), before the
            // argument join (the bottom fallback was the R-1 silence).
            if let Some(l) = registry_private_flow_label(name) {
                return l;
            }
            // Data flows through ordinary functions: join of the arguments.
            let mut acc = Label::bottom();
            for a in args {
                acc = acc.join(&expr_label(a, env));
            }
            acc
        }
        // Qualified calls (module functions): no source knowledge at this
        // slice — argument propagation only (recorded boundary).
        Expr::QualifiedCall { args, .. } => {
            let mut acc = Label::bottom();
            for a in args {
                acc = acc.join(&expr_label(a, env));
            }
            acc
        }
        Expr::BinaryOp { left, right, .. } => expr_label(left, env).join(&expr_label(right, env)),
        // Conditions do not taint values; branches do.
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => expr_label(then_branch, env).join(&expr_label(else_branch, env)),
        Expr::List { items, .. } => {
            let mut acc = Label::bottom();
            for item in items {
                acc = acc.join(&expr_label(item, env));
            }
            acc
        }
        // `list[index]` yields an ELEMENT of the object; the list label is
        // already the join of its elements, so the object label is the
        // sound answer (the index selects, it does not contribute).
        Expr::IndexAccess { object, .. } => expr_label(object, env),
        Expr::StructLit { fields, .. } => {
            let mut acc = Label::bottom();
            for v in fields.values() {
                acc = acc.join(&expr_label(v, env));
            }
            acc
        }
        // Block-expression branches: join of every expression label that
        // appears in the branch bodies (approximation of the block's value
        // — recorded in Appendix A).
        Expr::BlockIfElse {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let mut acc = block_expr_label(then_body, env);
            for (_, body) in else_ifs {
                acc = acc.join(&block_expr_label(body, env));
            }
            if let Some(body) = else_body {
                acc = acc.join(&block_expr_label(body, env));
            }
            acc
        }
        // №369: match-as-expression — the value carries the join of the
        // scrutinee's label (control dependence, REFERENCE §labels: the
        // scrutinee's label joins every variable assigned in any arm) with
        // every arm body's expression labels; else body joins too.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            let mut acc = expr_label(scrutinee, env);
            for arm in arms {
                acc = acc.join(&block_expr_label(arm.body(), env));
            }
            if let Some(eb) = else_body {
                acc = acc.join(&block_expr_label(eb, env));
            }
            acc
        }
        // `try expr` returns the value or Unit on error — the value label
        // is an upper bound, keep the inner label.
        Expr::Try { expr, .. } => expr_label(expr, env),
    }
}

/// Join of every expression label appearing (top-level-ish) in a block —
/// used for block-expression value approximation only.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn block_expr_label(stmts: &[Statement], env: &BTreeMap<String, Label>) -> Label {
    let mut acc = Label::bottom();
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                acc = acc.join(&expr_label(value, env));
            }
            Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                acc = acc.join(&expr_label(value, env));
            }
            Statement::IfThen { body, .. } => {
                acc = acc.join(&block_expr_label(body, env));
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                acc = acc.join(&block_expr_label(then_body, env));
                for (_, body) in else_ifs {
                    acc = acc.join(&block_expr_label(body, env));
                }
                if let Some(body) = else_body {
                    acc = acc.join(&block_expr_label(body, env));
                }
            }
            _ => {}
        }
    }
    acc
}

/// Merge rule shared by all merge points: per-variable componentwise join
/// of the entry environment and every branch environment (a branch that
/// did not assign the variable contributes the entry label — this is what
/// makes one-sided assignment conservative).
fn merge_envs(
    entry: &BTreeMap<String, Label>,
    branches: &[&BTreeMap<String, Label>],
) -> BTreeMap<String, Label> {
    let mut names: Vec<&String> = entry.keys().collect();
    for b in branches {
        names.extend(b.keys());
    }
    let mut out = BTreeMap::new();
    for name in names {
        let mut acc = entry.get(name).cloned().unwrap_or_else(Label::bottom);
        for b in branches {
            acc = acc.join(&b.get(name).cloned().unwrap_or_else(Label::bottom));
        }
        out.insert(name.clone(), acc);
    }
    out
}

/// Structurally collect every variable ASSIGNED or LET-BOUND in a
/// statement sequence (recursively through nested blocks) — used by the
/// Match scrutinee rule, which must trigger on assignment SHAPE, not on
/// label changes (a branch can assign the same label it inherited).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn collect_assigned_vars(stmts: &[Statement], out: &mut std::collections::HashSet<String>) {
    for st in stmts {
        match st {
            Statement::LetBinding { name, .. } | Statement::Assign { name, .. } => {
                out.insert(name.clone());
            }
            Statement::Each { variable, body, .. } => {
                out.insert(variable.clone());
                collect_assigned_vars(body, out);
            }
            Statement::EachWithIndex {
                index_var,
                item_var,
                body,
                ..
            } => {
                out.insert(index_var.clone());
                out.insert(item_var.clone());
                collect_assigned_vars(body, out);
            }
            Statement::While { body, .. } => collect_assigned_vars(body, out),
            Statement::IfThen { body, .. } => collect_assigned_vars(body, out),
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_assigned_vars(then_body, out);
                for (_, body) in else_ifs {
                    collect_assigned_vars(body, out);
                }
                if let Some(body) = else_body {
                    collect_assigned_vars(body, out);
                }
            }
            Statement::Match {
                arms, else_body, ..
            } => {
                for arm in arms {
                    let body = match arm {
                        MatchArm::Exact(_, b)
                        | MatchArm::StartsWith(_, b)
                        | MatchArm::Contains(_, b)
                        | MatchArm::Compare(_, _, b) => b,
                    };
                    collect_assigned_vars(body, out);
                }
                if let Some(body) = else_body {
                    collect_assigned_vars(body, out);
                }
            }
            _ => {}
        }
    }
}

/// Infer one statement sequence, mutating `env`; value-producing
/// statements (Return, ExprStmt) join the pattern `output`.
fn infer_block(stmts: &[Statement], env: &mut BTreeMap<String, Label>, output: &mut Label) {
    for st in stmts {
        infer_stmt(st, env, output);
    }
}

fn infer_stmt(st: &Statement, env: &mut BTreeMap<String, Label>, output: &mut Label) {
    match st {
        // LetBinding/Assign — the label comes from the RHS (Assign replaces:
        // reassignment to a safe value lowers the label in straight-line
        // code, mirroring the TaintTracker untaint semantics; merge points
        // re-add the conservatism).
        Statement::LetBinding { name, value, .. } => {
            let l = expr_label(value, env);
            env.insert(name.clone(), l);
        }
        Statement::Assign { name, value, .. } => {
            let l = expr_label(value, env);
            env.insert(name.clone(), l);
        }
        // Each: the iterator takes the ITERABLE's label; the body cannot
        // raise it (restored after the body). Loop exit: join of the entry
        // and post-body environments for every other variable.
        Statement::Each {
            variable,
            iterable,
            body,
            ..
        } => {
            let it_label = expr_label(iterable, env);
            env.insert(variable.clone(), it_label.clone());
            let entry = env.clone();
            infer_block(body, env, output);
            env.insert(variable.clone(), it_label);
            *env = merge_envs(&entry, &[&*env]);
        }
        Statement::EachWithIndex {
            index_var,
            item_var,
            iterable,
            body,
            ..
        } => {
            let it_label = expr_label(iterable, env);
            env.insert(item_var.clone(), it_label.clone());
            // The index is a position, not data: bottom (Appendix A).
            env.insert(index_var.clone(), Label::bottom());
            let entry = env.clone();
            infer_block(body, env, output);
            env.insert(item_var.clone(), it_label);
            env.insert(index_var.clone(), Label::bottom());
            *env = merge_envs(&entry, &[&*env]);
        }
        // While — bounded fixpoint: the body runs until the environment
        // stabilizes (≤ 8 passes). Monotone joins on a finite lattice make
        // this the exact fixpoint; the cap is a termination guard.
        // Condition labels are ignored (conditions do not taint values).
        Statement::While { body, .. } => {
            for _ in 0..LABEL_FIXPOINT_MAX_PASSES {
                let before = env.clone();
                infer_block(body, env, output);
                // join so loop-carried growth accumulates across passes
                for (k, v) in env.iter_mut() {
                    let b = before.get(k).cloned().unwrap_or_else(Label::bottom);
                    *v = b.join(v);
                }
                if *env == before {
                    break;
                }
            }
        }
        // If/else-if/else: every branch is inferred from the entry env;
        // merge is the componentwise join over all branches.
        Statement::IfElseBlock {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let entry = env.clone();
            infer_block(then_body, env, output);
            let then_env = env.clone();
            let mut branch_envs: Vec<BTreeMap<String, Label>> = vec![then_env];
            for (_, body) in else_ifs {
                let mut e = entry.clone();
                infer_block(body, &mut e, output);
                branch_envs.push(e);
            }
            if let Some(body) = else_body {
                let mut e = entry.clone();
                infer_block(body, &mut e, output);
                branch_envs.push(e);
            }
            let refs: Vec<&BTreeMap<String, Label>> = branch_envs.iter().collect();
            *env = merge_envs(&entry, &refs);
        }
        // Single-branch if: merge with an implicit empty else.
        Statement::IfThen { body, .. } => {
            let entry = env.clone();
            infer_block(body, env, output);
            *env = merge_envs(&entry, &[&*env]);
        }
        // Return/ExprStmt — the result label joins the pattern output.
        Statement::Return { value, .. } => {
            let l = expr_label(value, env);
            *output = output.join(&l);
        }
        Statement::ExprStmt { expr, .. } => {
            let l = expr_label(expr, env);
            *output = output.join(&l);
        }
        // Match: join over arms; the scrutinee's label additionally joins
        // every variable ASSIGNED in any arm — control dependence on the
        // scrutinee (decisions derived from private data taint outcomes).
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            let scrut = expr_label(scrutinee, env);
            let entry = env.clone();
            let mut branch_envs: Vec<BTreeMap<String, Label>> = Vec::new();
            let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut run_branch =
                |body: &Vec<Statement>,
                 branch_envs: &mut Vec<_>,
                 assigned: &mut std::collections::HashSet<String>,
                 entry: &BTreeMap<String, Label>| {
                    collect_assigned_vars(body, assigned);
                    let mut e = entry.clone();
                    let mut out_tmp = Label::bottom();
                    infer_block(body, &mut e, &mut out_tmp);
                    // NOTE: arm outputs join the caller's output too (a Return
                    // inside an arm is still a pattern exit).
                    *output = output.join(&out_tmp);
                    branch_envs.push(e);
                };
            for arm in arms {
                let body = match arm {
                    MatchArm::Exact(_, b)
                    | MatchArm::StartsWith(_, b)
                    | MatchArm::Contains(_, b)
                    | MatchArm::Compare(_, _, b) => b,
                };
                run_branch(body, &mut branch_envs, &mut assigned, &entry);
            }
            if let Some(body) = else_body {
                run_branch(body, &mut branch_envs, &mut assigned, &entry);
            }
            let refs: Vec<&BTreeMap<String, Label>> = branch_envs.iter().collect();
            let mut merged = merge_envs(&entry, &refs);
            for k in &assigned {
                // The variable is structurally assigned in some branch, so
                // every branch env (and the merge union) carries it.
                if let Some(m) = merged.get_mut(k) {
                    *m = m.join(&scrut);
                }
            }
            *env = merged;
        }
        // Loop control: no label effect, no merge contribution (Appendix A).
        Statement::Break | Statement::Continue => {}
        // Memory side-effect statements: the payload label does not enter
        // the value flow here; persistence gating is №325
        // (TAINT_PERSISTENCE class in the leak-suite vocabulary).
        Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
    }
}

/// Run the statement-level label inference over one pattern (Наряд №323).
/// Parameters: annotated → parsed label; unannotated → `bottom`. The
/// result is what №325's sink-gate will read; see ADR-0154 Appendix A
/// for the per-statement contracts and REFERENCE §2.2 for the table.
pub fn infer_pattern_labels(pattern: &PatternDecl) -> LabelInference {
    let mut env: BTreeMap<String, Label> = BTreeMap::new();
    for prm in &pattern.params {
        let l = match &prm.label {
            Some(ann) => Label::parse(&ann.raw).unwrap_or_else(|_| Label::bottom()),
            None => Label::bottom(),
        };
        env.insert(prm.name.clone(), l);
    }
    let mut output = Label::bottom();
    infer_block(&pattern.body, &mut env, &mut output);
    LabelInference {
        var_labels: env,
        output_label: output,
    }
}

// ── Label annotation validation (Наряд №322, ADR-0154) ───────────

/// Validate one label annotation, reporting a parse failure with the
/// annotation's span. Message follows the existing semantic convention
/// (context first, reason last) — e.g.
/// `label annotation '<private, bogus>' on parameter 's' of pattern 'p': unknown label word 'bogus'`.
fn validate_label_ann(ann: &LabelAnn, context: &str, errors: &mut Vec<SpannedError>) {
    if let Err(e) = crate::labels::Label::parse(&ann.raw) {
        errors.push(SpannedError::at(
            format!("label annotation '<{}>' on {}: {}", ann.raw, context, e),
            ann.span.clone(),
        ));
    }
}

/// Walk a declaration and validate every label annotation it carries.
///
/// Grammar-restricted positions (Наряд №322): pattern / learnable /
/// template / tool-method parameters, entity-type fields, and the type
/// position of entity record/simple declarations. Everywhere else a
/// `<...>` after a type name remains a parse error — annotations cannot
/// appear where the label system does not see them.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn validate_decl_labels(decl: &Declaration, errors: &mut Vec<SpannedError>) {
    match decl {
        Declaration::Pattern(p) => {
            for prm in &p.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!("parameter '{}' of pattern '{}'", prm.name, p.name),
                        errors,
                    );
                }
            }
        }
        Declaration::LearnablePattern(lp) => {
            for prm in &lp.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!(
                            "parameter '{}' of learnable pattern '{}'",
                            prm.name, lp.name
                        ),
                        errors,
                    );
                }
            }
        }
        Declaration::Template(t) => {
            for prm in &t.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!("parameter '{}' of template '{}'", prm.name, t.name),
                        errors,
                    );
                }
            }
        }
        Declaration::Tool(t) => {
            for m in &t.methods {
                for prm in &m.params {
                    if let Some(ann) = &prm.label {
                        validate_label_ann(
                            ann,
                            &format!(
                                "parameter '{}' of tool method '{}.{}'",
                                prm.name, t.name, m.name
                            ),
                            errors,
                        );
                    }
                }
            }
        }
        Declaration::EntityType(e) => {
            for f in &e.fields {
                if let Some(ann) = &f.label {
                    validate_label_ann(
                        ann,
                        &format!("field '{}' of entity type '{}'", f.name, e.name),
                        errors,
                    );
                }
            }
        }
        Declaration::EntityRecord(e) => {
            if let Some(ann) = &e.label {
                validate_label_ann(ann, &format!("entity '{}'", e.name), errors);
            }
        }
        Declaration::EntitySimple(e) => {
            if let Some(ann) = &e.label {
                validate_label_ann(ann, &format!("entity '{}'", e.name), errors);
            }
        }
        _ => {}
    }
}

// ── Effect trail (Наряд №324, ADR-0154 §9) ───────────────────────────

/// Parse the declared effect-trail words (`"io, audit"`) into a set.
/// Errors (unknown word / duplicate) are loud; `⟨⟩` (empty raw) is the
/// zero-effect declaration.
fn parse_effect_set(ann: &EffectAnn) -> Result<EffectSet, String> {
    let raw = ann.raw.trim();
    if raw.is_empty() {
        return Ok(EffectSet::new());
    }
    let mut set = EffectSet::new();
    for word in raw.split(',') {
        let w = word.trim();
        match Effect::parse_word(w) {
            Some(e) => {
                if !set.insert(e) {
                    return Err(format!("duplicate effect word '{w}'"));
                }
            }
            None => return Err(format!("unknown effect word '{w}'")),
        }
    }
    Ok(set)
}

/// Validate one effect trail, reporting a bad word with the trail's
/// span (same convention as `validate_label_ann`).
fn validate_effect_ann(ann: &EffectAnn, context: &str, errors: &mut Vec<SpannedError>) {
    if let Err(e) = parse_effect_set(ann) {
        errors.push(SpannedError::at(
            format!("effect trail '⟨{}⟩' on {}: {}", ann.raw, context, e),
            ann.span.clone(),
        ));
    }
}

// ── Media handle opacity (Наряд №331, ADR-0114 / ADR-0162 §2.5) ──────
//
// Media handles (`Value::Media`: Image/Audio/VideoFrame/VideoSegment)
// are opaque: bytes NEVER live in `Value`, so any field access on a
// media-typed expression is a COMPILE error (loud, with span) — bytes
// are reachable only through the sanctioned materialization sink
// (`media_save`, №325-gated). The language has no other byte-extraction
// syntax on handles; this pass closes the syntactic surface there is.

/// Builtins that PRODUCE a media handle (the four per-type stores +
/// `media_retain`, which returns the same handle).
fn is_media_producing_builtin(name: &str) -> bool {
    matches!(
        name,
        "media_store_image"
            | "media_store_audio"
            | "media_store_video_frame"
            | "media_store_video_segment"
            | "media_retain"
    )
}

/// `true` when the expression's value is a media handle by direct
/// construction (a producing builtin call).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn is_media_binding_expr(value: &Expr) -> bool {
    match value {
        Expr::FnCall { name, .. } => is_media_producing_builtin(name),
        // №332 (ADR-0164 §7.4): the perception constructions produce
        // media handles too — a bound Lift (`from <origin>
        // media_store_*(...)`) and a source capture (`source <origin>`)
        // are opaque handles like any other.
        Expr::ProvBind { inner, .. } => {
            matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
        }
        Expr::HandleSource { .. } => true,
        _ => false,
    }
}

/// Is this expression's static bottom a media-typed binding?
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn media_typed_object(object: &Expr, media_vars: &std::collections::HashSet<String>) -> bool {
    match object {
        Expr::Ident { name, .. } => media_vars.contains(name),
        Expr::FnCall { name, .. } => is_media_producing_builtin(name),
        // №332: perception constructions are media-typed bottoms too.
        Expr::HandleSource { .. } => true,
        Expr::ProvBind { inner, .. } => {
            matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
        }
        _ => false,
    }
}

/// Walk one expression for field accesses on media-typed objects and
/// for further media bindings (recursively).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_media_expr(
    expr: &Expr,
    media_vars: &mut std::collections::HashSet<String>,
    container: &str,
    violations: &mut Vec<MediaOpacityViolation>,
) {
    match expr {
        Expr::FieldAccess {
            object,
            field,
            span,
        } => {
            if media_typed_object(object, media_vars) {
                violations.push(MediaOpacityViolation {
                    container: container.to_string(),
                    field: field.clone(),
                    span: span.clone(),
                });
            } else {
                check_media_expr(object, media_vars, container, violations);
            }
        }
        Expr::FnCall { name, args, .. } => {
            // The call NAME may be a media binding (checked by the caller
            // when binding); the arguments still need the field-access walk.
            let _ = name;
            for a in args {
                check_media_expr(a, media_vars, container, violations);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_media_expr(a, media_vars, container, violations);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_media_expr(left, media_vars, container, violations);
            check_media_expr(right, media_vars, container, violations);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_media_expr(condition, media_vars, container, violations);
            check_media_expr(then_branch, media_vars, container, violations);
            check_media_expr(else_branch, media_vars, container, violations);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_media_expr(i, media_vars, container, violations);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            check_media_expr(object, media_vars, container, violations);
            check_media_expr(index, media_vars, container, violations);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_media_expr(v, media_vars, container, violations);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_media_expr(condition, media_vars, container, violations);
            check_media_stmts(then_body, media_vars, container, violations);
            for (cond, body) in else_ifs {
                check_media_expr(cond, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            if let Some(eb) = else_body {
                check_media_stmts(eb, media_vars, container, violations);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_media_expr(scrutinee, media_vars, container, violations);
            for arm in arms {
                check_media_stmts(arm.body(), media_vars, container, violations);
            }
            if let Some(eb) = else_body {
                check_media_stmts(eb, media_vars, container, violations);
            }
        }
        Expr::Try { expr, .. } => {
            check_media_expr(expr, media_vars, container, violations);
        }
        _ => {}
    }
}

/// Walk statements, tracking media-typed bindings (direct production
/// calls and one-step aliases) and checking every expression.
fn check_media_stmts(
    stmts: &[Statement],
    media_vars: &mut std::collections::HashSet<String>,
    container: &str,
    violations: &mut Vec<MediaOpacityViolation>,
) {
    for s in stmts {
        match s {
            Statement::LetBinding { name, value, .. } => {
                check_media_expr(value, media_vars, container, violations);
                if is_media_binding_expr(value) {
                    media_vars.insert(name.clone());
                } else if let Expr::Ident { name: src, .. } = value {
                    if media_vars.contains(src) {
                        media_vars.insert(name.clone());
                    }
                }
            }
            Statement::Assign { name, value, .. } => {
                check_media_expr(value, media_vars, container, violations);
                if is_media_binding_expr(value) {
                    media_vars.insert(name.clone());
                }
            }
            Statement::Each {
                variable,
                iterable,
                body,
                ..
            } => {
                let _ = variable;
                check_media_expr(iterable, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::EachWithIndex {
                item_var,
                iterable,
                body,
                ..
            } => {
                let _ = item_var;
                check_media_expr(iterable, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(then_body, media_vars, container, violations);
                for (cond, body) in else_ifs {
                    check_media_expr(cond, media_vars, container, violations);
                    check_media_stmts(body, media_vars, container, violations);
                }
                if let Some(eb) = else_body {
                    check_media_stmts(eb, media_vars, container, violations);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::Return { value, .. } => {
                check_media_expr(value, media_vars, container, violations);
            }
            Statement::ExprStmt { expr, .. } => {
                check_media_expr(expr, media_vars, container, violations);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_media_expr(scrutinee, media_vars, container, violations);
                for arm in arms {
                    check_media_stmts(arm.body(), media_vars, container, violations);
                }
                if let Some(eb) = else_body {
                    check_media_stmts(eb, media_vars, container, violations);
                }
            }
            Statement::Memorize(m) => {
                check_media_expr(&m.value, media_vars, container, violations);
            }
            Statement::Forget(f) => {
                check_media_expr(&f.query, media_vars, container, violations);
            }
            Statement::Relate(r) => {
                check_media_expr(&r.from, media_vars, container, violations);
                check_media_expr(&r.to, media_vars, container, violations);
            }
            Statement::Break | Statement::Continue => {}
        }
    }
}

/// One opacity violation (№331): a field access whose object bottoms out
/// at a media-typed binding/call. Collected by `media_opacity_violations`;
/// consumed by `check_program` (semantic errors) and the audit Category-A
/// check `MEDIA_HANDLE_OPAQUE` (the compile_program path).
pub struct MediaOpacityViolation {
    pub container: String,
    pub field: String,
    pub span: Span,
}

impl MediaOpacityViolation {
    /// The stable compile-error text (pinned by examples/w1_handle_opaque).
    pub fn message(&self) -> String {
        format!(
            "media handle is opaque (ADR-0114): field access '.{}' on a media \
             handle in {} — bytes never live in Value; use the sanctioned \
             materialization sink (media_save), gated by №325",
            self.field, self.container
        )
    }
}

/// Public entry point: every field access on a media-typed expression in
/// every statement container (the same containers the №324 effect fixpoint
/// walks). See ADR-0162 §2.5.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn media_opacity_violations(declarations: &[Declaration]) -> Vec<MediaOpacityViolation> {
    let mut violations = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut media_vars = std::collections::HashSet::new();
                // Params are typed (name: Type) — no media params exist
                // until №332 lands the perception AST; none to pre-track.
                check_media_stmts(
                    &p.body,
                    &mut media_vars,
                    &format!("pattern '{}'", p.name),
                    &mut violations,
                );
            }
            Declaration::LearnablePattern(_) => {
                // A learnable pattern is prompt-declared (no statement body
                // — the №324 effect walker treats it as an LLM call by
                // construction). Nothing to walk here.
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let mut media_vars = std::collections::HashSet::new();
                    check_media_stmts(
                        &m.body,
                        &mut media_vars,
                        &format!("tool method '{}.{}'", t.name, m.name),
                        &mut violations,
                    );
                }
            }
            _ => {}
        }
    }
    violations
}

// ── Origin chain (Наряд №332, ADR-0164 §7.4) ─────────────────────────
//
// The rule: a media handle without origin is NOT constructed. Every
// handle-producing construction must be traceable to a declared source:
//   - `source <origin>`  (HandleSource) — the origin IS the source;
//   - `from <origin> media_store_*(...)` (ProvBind over the Lift) — the
//     bind attaches the construction to the declared origin;
//   - `vision_generate` handles carry the №241 manifest (generation
//     provenance already exists — out of this rule's scope).
// Illegal positions (a construction nested inside another expression, a
// direct `media_source_capture`/`media_bind_origin` builtin call, an
// unknown origin name) are loud violations. The check runs in BOTH
// compile paths: check_program (semantic errors) and audit_category_a
// (ORIGIN_REQUIRED, Category-A) — mirroring the №331 opacity split.

/// What the origin pass tracks per construction site.
pub struct OriginViolation {
    pub message: String,
    pub span: Span,
}

/// The declared origin kinds (generation handles are Lift+ProvBind /
/// vision_generate manifests — `source` capture is camera|file only,
/// enforced in the runtime dispatch).
const ORIGIN_KINDS: &[&str] = &["camera", "file", "generation", "likeness"];
const ORIGIN_FIELDS: &[&str] = &["kind", "media", "label", "path"];

fn validate_origin_decls(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for decl in declarations {
        if let Declaration::Origin(o) = decl {
            let compiled = match crate::bytecode::CompiledOriginDecl::from_ast(o) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(SpannedError::at(e, o.span.clone()));
                    continue;
                }
            };
            // kind vocabulary
            if !ORIGIN_KINDS.contains(&compiled.kind.as_str()) {
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': unknown kind '{}' (expected {})",
                        o.name,
                        compiled.kind,
                        ORIGIN_KINDS.join(" | ")
                    ),
                    o.span.clone(),
                ));
            }
            // media vocabulary
            if crate::media::MediaKind::from_slug(&compiled.media).is_err() {
                // from_slug's message names the expected words; re-anchor it.
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': {}",
                        o.name,
                        crate::media::MediaKind::from_slug(&compiled.media)
                            .err()
                            .unwrap_or_default()
                    ),
                    o.span.clone(),
                ));
            }
            // label vocabulary — poisoned is NOT constructible via a
            // declaration (quarantine comes only from the taint machinery).
            if crate::media::parse_sensitivity(&compiled.conf).is_err() {
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': {}",
                        o.name,
                        crate::media::parse_sensitivity(&compiled.conf)
                            .err()
                            .unwrap_or_default()
                    ),
                    o.span.clone(),
                ));
            }
            // unknown fields are loud (a typo must never silently no-op)
            for (k, _) in &o.fields {
                if !ORIGIN_FIELDS.contains(&k.as_str()) {
                    errors.push(SpannedError::at(
                        format!(
                            "origin '{}': unknown field '{}' (expected {})",
                            o.name,
                            k,
                            ORIGIN_FIELDS.join(" | ")
                        ),
                        o.span.clone(),
                    ));
                }
            }
        }
    }
}

/// Public entry point: the origin-chain rule over every statement
/// container (the same containers the №331 opacity pass walks).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn media_origin_violations(declarations: &[Declaration]) -> Vec<OriginViolation> {
    // №337 (ADR-0166 §2.3): name → declared kind — the generation
    // contract message needs the KIND of the bound origin. Declared
    // origins that fail compilation (validate_origin_decls) simply do
    // not join the map — their own loud error already fired.
    let origins: std::collections::HashMap<String, String> = declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Origin(o) => crate::bytecode::CompiledOriginDecl::from_ast(o)
                .ok()
                .map(|c| (c.name, c.kind)),
            _ => None,
        })
        .collect();
    let mut violations = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut media_vars = std::collections::HashSet::new();
                check_origin_stmts(
                    &p.body,
                    &mut media_vars,
                    &origins,
                    &format!("pattern '{}'", p.name),
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let mut media_vars = std::collections::HashSet::new();
                    check_origin_stmts(
                        &m.body,
                        &mut media_vars,
                        &origins,
                        &format!("tool method '{}.{}'", t.name, m.name),
                        &mut violations,
                    );
                }
            }
            _ => {}
        }
    }
    violations
}

fn push_origin(violations: &mut Vec<OriginViolation>, span: &Span, message: String) {
    violations.push(OriginViolation {
        message,
        span: span.clone(),
    });
}

/// The construction forms the origin rule accepts at a binding site.
enum BindingForm {
    /// `source <origin>` — the origin is the source.
    Source(String),
    /// `from <origin> media_store_*(...)` — a bound Lift.
    BoundLift(String),
    /// An existing handle variable (alias) — inherits its binding state.
    Alias(String),
    /// Anything else: not a handle construction (checked for nested
    /// violations by the walker).
    Other,
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn classify_binding(value: &Expr) -> BindingForm {
    match value {
        Expr::HandleSource { origin, .. } => BindingForm::Source(origin.clone()),
        Expr::ProvBind { origin, inner, .. } => {
            if matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
            {
                BindingForm::BoundLift(origin.clone())
            } else {
                BindingForm::Other
            }
        }
        Expr::Ident { name, .. } => BindingForm::Alias(name.clone()),
        _ => BindingForm::Other,
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_origin_stmts(
    stmts: &[Statement],
    media_vars: &mut std::collections::HashSet<String>,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    for s in stmts {
        match s {
            Statement::LetBinding {
                name, value, span, ..
            }
            | Statement::Assign { name, value, span } => {
                match classify_binding(value) {
                    BindingForm::Source(origin) => {
                        if !origins.contains_key(&origin) {
                            push_origin(
                                violations,
                                span,
                                format!(
                                    "origin chain violation (ORIGIN_REQUIRED): source '{}' in {} names no declared origin — declare 'origin {} {{ kind: ..., media: ..., label: ... }}' (ADR-0164)",
                                    origin, container, origin
                                ),
                            );
                        }
                        media_vars.insert(name.clone());
                    }
                    BindingForm::BoundLift(origin) => {
                        if !origins.contains_key(&origin) {
                            push_origin(
                                violations,
                                span,
                                format!(
                                    "origin chain violation (ORIGIN_REQUIRED): from '{}' in {} names no declared origin (ADR-0164)",
                                    origin, container
                                ),
                            );
                        }
                        media_vars.insert(name.clone());
                    }
                    BindingForm::Alias(src) => {
                        if media_vars.contains(&src) {
                            media_vars.insert(name.clone());
                        }
                    }
                    BindingForm::Other => {
                        // Nested constructions (a bare Lift, a stray
                        // source/from, a direct builtin call) are loud.
                        check_origin_expr(value, origins, container, violations);
                    }
                }
            }
            Statement::ExprStmt { expr, .. } => {
                // A construction as a bare statement is a discarded handle —
                // the origin rule still refuses unbound Lifts (bytes enter
                // the store without provenance, even if the value is dropped).
                if let Expr::ProvBind { .. } = expr {
                    // `from o media_store_image(...)` as a statement: legal
                    // shape (bind + discard) — inner construction is bound.
                } else {
                    check_origin_expr(expr, origins, container, violations);
                }
            }
            Statement::Return { value, .. } => {
                if !matches!(value, Expr::HandleSource { .. } | Expr::ProvBind { .. }) {
                    check_origin_expr(value, origins, container, violations);
                }
            }
            Statement::Each { iterable, body, .. } => {
                check_origin_expr(iterable, origins, container, violations);
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::EachWithIndex { iterable, body, .. } => {
                check_origin_expr(iterable, origins, container, violations);
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::While { body, .. } => {
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_origin_stmts(then_body, media_vars, origins, container, violations);
                for (_, b) in else_ifs {
                    check_origin_stmts(b, media_vars, origins, container, violations);
                }
                if let Some(eb) = else_body {
                    check_origin_stmts(eb, media_vars, origins, container, violations);
                }
            }
            Statement::IfThen { body, .. } => {
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::Match {
                arms, else_body, ..
            } => {
                for arm in arms {
                    check_origin_stmts(arm.body(), media_vars, origins, container, violations);
                }
                if let Some(eb) = else_body {
                    check_origin_stmts(eb, media_vars, origins, container, violations);
                }
            }
            _ => {}
        }
    }
}

/// Nested-expression check: handle constructions are illegal outside
/// binding initializers.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_origin_expr(
    expr: &Expr,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    match expr {
        Expr::HandleSource { origin, span } => {
            push_origin(
                violations,
                span,
                format!(
                    "origin chain violation (ORIGIN_REQUIRED): 'source {}' in {} is only legal as a binding initializer (`let h = source {}`) — a source handle must be bound to a variable to be tracked (ADR-0164)",
                    origin, container, origin
                ),
            );
        }
        Expr::ProvBind {
            origin,
            inner,
            span,
        } => {
            if !matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
            {
                // №337 (ADR-0166 §2.3): a GENERATION bind over a
                // non-construction is the "generation lift without
                // synthetic: true" hole — the refusal names the marking
                // contract (an alias bind would either skip the marking
                // or falsify it — captured bytes marked synthetic).
                let msg = if origins.get(origin).map(|k| k.as_str()) == Some("generation") {
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): 'from {}' in {} must wrap a fresh handle construction (media_store_*) — a GENERATION lift sets synthetic: true on the new store entry (ADR-0166 §2.3); binding an existing handle would skip or falsify the Art. 50 marking",
                        origin, container
                    )
                } else {
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): 'from {}' in {} must wrap a handle construction (media_store_*) — a bind attaches provenance to a NEW handle (ADR-0164)",
                        origin, container
                    )
                };
                push_origin(violations, span, msg);
            }
            check_origin_expr(inner, origins, container, violations);
        }
        Expr::FnCall { name, args, span } => {
            if matches!(
                name.as_str(),
                "media_store_image"
                    | "media_store_audio"
                    | "media_store_video_frame"
                    | "media_store_video_segment"
            ) {
                push_origin(
                    violations,
                    span,
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): bare {} in {} — a handle without origin is not constructed (§7.4); wrap it: 'from <origin> {}(...)' with a declared origin (ADR-0164)",
                        name, container, name
                    ),
                );
            }
            if matches!(name.as_str(), "media_source_capture" | "media_bind_origin") {
                push_origin(
                    violations,
                    span,
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): direct {} call in {} — use the source/from syntax (ADR-0164)",
                        name, container
                    ),
                );
            }
            for a in args {
                check_origin_expr(a, origins, container, violations);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_origin_expr(a, origins, container, violations);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_origin_expr(left, origins, container, violations);
            check_origin_expr(right, origins, container, violations);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_origin_expr(condition, origins, container, violations);
            check_origin_expr(then_branch, origins, container, violations);
            check_origin_expr(else_branch, origins, container, violations);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_origin_expr(i, origins, container, violations);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            check_origin_expr(object, origins, container, violations);
            check_origin_expr(index, origins, container, violations);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_origin_expr(v, origins, container, violations);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_origin_expr(condition, origins, container, violations);
            check_origin_stmts(
                then_body,
                &mut std::collections::HashSet::new(),
                origins,
                container,
                violations,
            );
            for (_, b) in else_ifs {
                check_origin_stmts(
                    b,
                    &mut std::collections::HashSet::new(),
                    origins,
                    container,
                    violations,
                );
            }
            if let Some(eb) = else_body {
                check_origin_stmts(
                    eb,
                    &mut std::collections::HashSet::new(),
                    origins,
                    container,
                    violations,
                );
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_origin_expr(scrutinee, origins, container, violations);
            for arm in arms {
                for st in arm.body() {
                    check_origin_expr_stmt(st, origins, container, violations);
                }
            }
            if let Some(eb) = else_body {
                for st in eb {
                    check_origin_expr_stmt(st, origins, container, violations);
                }
            }
        }
        Expr::Try { expr, .. } => {
            check_origin_expr(expr, origins, container, violations);
        }
        _ => {}
    }
}

/// Statement-level helper for nested block/match bodies inside the
/// expression walker (lightweight: only the construction sites matter).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_origin_expr_stmt(
    st: &Statement,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    match st {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            check_origin_expr(value, origins, container, violations);
        }
        Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
            check_origin_expr(expr, origins, container, violations);
        }
        _ => {}
    }
}

/// Semantic-layer wiring: origin violations are loud compile errors.
fn check_media_origin_chain(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    validate_origin_decls(declarations, errors);
    for v in media_origin_violations(declarations) {
        errors.push(SpannedError::at(v.message, v.span));
    }
}

/// Public entry for the audit path (№332): declared-origin shape and
/// vocabulary errors — the SAME rules `check_program` applies, so a
/// mis-declared origin is loud on EVERY compile path, not only `mlog
/// check`. An unknown kind/media/label word must never silently no-op:
/// a mis-declared provenance source is a provenance lie (ADR-0164 §3).
pub fn origin_decl_errors(declarations: &[Declaration]) -> Vec<SpannedError> {
    let mut errors = Vec::new();
    validate_origin_decls(declarations, &mut errors);
    errors
}

/// Semantic-layer wiring: the violations are loud compile errors.
fn check_media_handle_opacity(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for v in media_opacity_violations(declarations) {
        errors.push(SpannedError::at(v.message(), v.span));
    }
}

// ── BackendSelect ladder companion check (Наряд №336, ADR-0165 §2.4) ──
//
// A statically-visible `backend_select("class", ["rung", …])` call site
// is verified against the №333 registry SSOT at BUILD time:
//   - the class word must be a §7.6 class;
//   - every rung must resolve in the registry (`find_by_name`);
//   - every rung's class must match the requested class;
//   - rungs must be unique (a rung tried twice is a contract bug);
//   - under `profile device { mode: production }` every rung must be
//     SHA-pinnable — a `ShaPin::PendingNo334` rung is UNVERIFIABLE for
//     the production profile and fails compilation (the §11.2 rule,
//     companion-check precedent).
// Non-literal ladders are not statically verifiable and stay with the
// runtime checks (documented in ADR-0165 §2.4).

fn check_backend_select_ladders(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for v in backend_select_ladder_violations(declarations) {
        errors.push(SpannedError::at(v.message, v.span));
    }
}

// ── License-class × profile permit rule (№839, ADR-0202 §2.3) ────────
//
// The Phase-2 composition rule (the plan's 2.3): "non-osi only in a
// profile that explicitly permits it". The №333 audit gate matches
// WEIGHTS-ID literals; the ladder rungs are backend NAMES — the audit
// lane never sees them (the w1_degrade golden rode that gap). This rule
// closes it at the SELECTION sites: every statically-visible
// `backend_select` ladder rung (by name) and every `vision` model field
// (by weights id) resolves in the №333 registry; a `NonOsi` /
// `Restrictive` (including the unverified default-deny) record whose
// license class is NOT permitted by the program's profile is a LOUD
// refusal BEFORE run, with the dedicated code `LICENSE_CLASS_NOT_ALLOWED`
// (the stable check id on every path — the compile gate, `mlog check`,
// and the audit Category-A companion).
//
// The permit predicate is `ResolvedProfiles::license_allows` — the
// SINGLE decision point shared with the audit gate: `Osi` always; the
// named `allow_license_class` permits (№839); the advisory bridge
// implies the blanket unlock it always carried (backward compat — the
// ADR-0163 surface is unchanged). The default posture (no licensing
// profile) allows `Osi` only; the default-deny (`MODEL_WEIGHTS_UNSAFE`,
// unverified → restrictive) is not weakened — the named permit sits on
// top of it, never instead of it.
//
// Раскладка (the PR report): MODEL_WEIGHTS_UNSAFE = the runtime fetch
// posture (the allowlist + SSRF-guard + SHA-pin, №334 — WHERE weights
// may be loaded from); BACKEND_LICENSE_DISTRIBUTION = the audit lane
// over ALL weights-id literal mentions (the distribution posture);
// THIS rule = the program-level selection posture (which license
// classes the program's backend selections may use, named per class).

/// The stable error code of the №839 rule (the `[CODE]` prefix shape of
/// the compile gates — the №673 family).
pub const CODE_LICENSE_CLASS_NOT_ALLOWED: &str = "LICENSE_CLASS_NOT_ALLOWED";

/// One statically-visible selection whose license class the profile
/// does not permit.
#[derive(Debug, Clone)]
pub struct LicenseClassViolation {
    pub message: String,
    pub span: Span,
}

fn push_license_class_violation(
    profiles: &crate::profile::ResolvedProfiles,
    site: &str,
    backend_name: &str,
    entry: &crate::backends::BackendEntry,
    span: &Span,
    out: &mut Vec<LicenseClassViolation>,
) {
    if profiles.license_allows(entry.license) {
        return;
    }
    let message = format!(
        "{site}: backend '{backend_name}' resolves to a {} record — \
         license class {} is not permitted by the program's profile \
         (license: {}). Add an explicit permit: 'profile licensing {{ \
         allow_license_class: \"{}\" }}' (the named class unlock, \
         ADR-0202; the default posture allows Osi only). The permit \
         failure is fail-closed before run — the runtime weight gates \
         (MODEL_WEIGHTS_UNSAFE, SHA-pin №334) stay independent.",
        entry.class.as_str(),
        entry.license.as_str(),
        entry.license_note,
        entry.license.as_str(),
    );
    out.push(LicenseClassViolation {
        message: format!("[{}] {}", CODE_LICENSE_CLASS_NOT_ALLOWED, message),
        span: span.clone(),
    });
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_license_class_stmts(
    stmts: &[Statement],
    profiles: &crate::profile::ResolvedProfiles,
    out: &mut Vec<LicenseClassViolation>,
) {
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                check_license_class_expr(value, profiles, out)
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                check_license_class_expr(expr, profiles, out)
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_license_class_expr(iterable, profiles, out);
                check_license_class_stmts(body, profiles, out);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_license_class_expr(condition, profiles, out);
                check_license_class_stmts(body, profiles, out);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_license_class_expr(condition, profiles, out);
                check_license_class_stmts(then_body, profiles, out);
                for (c, b) in else_ifs {
                    check_license_class_expr(c, profiles, out);
                    check_license_class_stmts(b, profiles, out);
                }
                if let Some(eb) = else_body {
                    check_license_class_stmts(eb, profiles, out);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_license_class_expr(condition, profiles, out);
                check_license_class_stmts(body, profiles, out);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_license_class_expr(scrutinee, profiles, out);
                for arm in arms {
                    check_license_class_stmts(arm.body(), profiles, out);
                }
                if let Some(eb) = else_body {
                    check_license_class_stmts(eb, profiles, out);
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_license_class_expr(
    expr: &Expr,
    profiles: &crate::profile::ResolvedProfiles,
    out: &mut Vec<LicenseClassViolation>,
) {
    match expr {
        Expr::FnCall { name, args, span } => {
            if name == "backend_select" {
                // The same literal-ladder shape the №336 verifier reads:
                // args[0] = the class word, args[1] = the literal list of
                // rung names. Non-literal ladders stay with the runtime.
                let rungs: Option<Vec<String>> = match args.get(1) {
                    Some(Expr::List { items, .. }) => {
                        let mut names = Vec::with_capacity(items.len());
                        let mut literal = true;
                        for i in items {
                            match i {
                                Expr::StringLit { value, .. } => names.push(value.clone()),
                                _ => {
                                    literal = false;
                                    break;
                                }
                            }
                        }
                        literal.then_some(names)
                    }
                    _ => None,
                };
                if let Some(names) = rungs {
                    for name in names {
                        if let Some(entry) = crate::backends::find_by_name(&name) {
                            push_license_class_violation(
                                profiles,
                                "backend_select ladder",
                                &name,
                                entry,
                                span,
                                out,
                            );
                        }
                    }
                }
            }
            for a in args {
                check_license_class_expr(a, profiles, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_license_class_expr(a, profiles, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_license_class_expr(left, profiles, out);
            check_license_class_expr(right, profiles, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_license_class_expr(condition, profiles, out);
            check_license_class_expr(then_branch, profiles, out);
            check_license_class_expr(else_branch, profiles, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_license_class_expr(i, profiles, out);
            }
        }
        Expr::FieldAccess { object, .. } => check_license_class_expr(object, profiles, out),
        Expr::IndexAccess { object, index, .. } => {
            check_license_class_expr(object, profiles, out);
            check_license_class_expr(index, profiles, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_license_class_expr(v, profiles, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_license_class_expr(condition, profiles, out);
            check_license_class_stmts(then_body, profiles, out);
            for (c, body) in else_ifs {
                check_license_class_expr(c, profiles, out);
                check_license_class_stmts(body, profiles, out);
            }
            if let Some(eb) = else_body {
                check_license_class_stmts(eb, profiles, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_license_class_expr(scrutinee, profiles, out);
            for arm in arms {
                check_license_class_stmts(arm.body(), profiles, out);
            }
            if let Some(eb) = else_body {
                check_license_class_stmts(eb, profiles, out);
            }
        }
        Expr::Try { expr, .. } => check_license_class_expr(expr, profiles, out),
        Expr::ProvBind { inner, .. } => check_license_class_expr(inner, profiles, out),
        Expr::HandleSource { .. }
        | Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
}

/// The №839 violations over one program (the shared walker — the
/// compile gate, `mlog check`, and the audit companion read the SAME
/// rule).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn license_class_violations(declarations: &[Declaration]) -> Vec<LicenseClassViolation> {
    let profiles = crate::profile::resolve(declarations);
    let mut out: Vec<LicenseClassViolation> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => check_license_class_stmts(&p.body, &profiles, &mut out),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_license_class_stmts(&m.body, &profiles, &mut out);
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    check_license_class_stmts(&r.body, &profiles, &mut out);
                }
            }
            Declaration::Hook(h) => check_license_class_stmts(&h.body, &profiles, &mut out),
            Declaration::Flow(f) => check_license_class_expr(&f.source, &profiles, &mut out),
            Declaration::Vision(v) => {
                // The declaration model field is the canonical reference
                // position (the vision model is a WEIGHTS id — the same
                // key the №333 gate matches).
                if let Some(entry) = crate::backends::find_by_weights_id_ci(&v.model) {
                    push_license_class_violation(
                        &profiles,
                        "vision declaration",
                        &v.model,
                        entry,
                        &v.span,
                        &mut out,
                    );
                }
            }
            _ => {}
        }
    }
    out
}

/// One statically-verifiable ladder defect. `kind` separates the two
/// Category-A check ids: a malformed ladder (BACKEND_SELECT_INVALID)
/// vs a ladder unverifiable for the production device profile
/// (BACKEND_LADDER_UNVERIFIABLE) — ADR-0165 §2.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderViolationKind {
    Invalid,
    UnverifiableForProduction,
}

#[derive(Debug, Clone)]
pub struct LadderViolation {
    pub kind: LadderViolationKind,
    pub message: String,
    pub span: Span,
}

/// Public entry for the audit path (№336): the SAME rules `check_program`
/// applies, so a statically-broken ladder is loud on EVERY compile path
/// (`compile_program`/`run_program_with_dir` call `audit_category_a`, not
/// `check_program` — the №332 origin-chain posture).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn backend_select_ladder_violations(declarations: &[Declaration]) -> Vec<LadderViolation> {
    let device_production = crate::profile::resolve(declarations).device_mode_production;
    let mut violations: Vec<LadderViolation> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                check_backend_stmts(&p.body, device_production, &mut violations)
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_backend_stmts(&m.body, device_production, &mut violations);
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    check_backend_stmts(&r.body, device_production, &mut violations);
                }
            }
            Declaration::Flow(f) => {
                check_backend_expr(&f.source, device_production, &mut violations)
            }
            _ => {}
        }
    }
    violations
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn verify_backend_ladder(
    args: &[Expr],
    span: &Span,
    device_production: bool,
    out: &mut Vec<LadderViolation>,
) {
    let push = |out: &mut Vec<LadderViolation>, kind, message: String| {
        out.push(LadderViolation {
            kind,
            message,
            span: span.clone(),
        });
    };
    // The class argument: verified only when literal.
    let class_word = match args.first() {
        Some(Expr::StringLit { value, .. }) => Some(value.clone()),
        _ => None,
    };
    if let Some(word) = &class_word {
        if crate::backends::BackendClass::parse(word).is_none() {
            push(
                out,
                LadderViolationKind::Invalid,
                format!(
                    "backend_select: unknown backend class '{}' (available: stt, tts, omni, vision-understanding, llm, ocr, video-understanding, embodied-sim, timeseries)",
                    word
                ),
            );
        }
    }
    // The ladder argument: verified only when a literal list of strings.
    let rung_names: Option<Vec<String>> = match args.get(1) {
        Some(Expr::List { items, .. }) => {
            let mut names = Vec::with_capacity(items.len());
            let mut literal = true;
            for it in items {
                match it {
                    Expr::StringLit { value, .. } => names.push(value.clone()),
                    _ => {
                        literal = false;
                        break;
                    }
                }
            }
            if literal {
                Some(names)
            } else {
                None
            }
        }
        _ => None,
    };
    let names = match rung_names {
        Some(n) => n,
        None => return, // non-literal ladder: runtime checks own it
    };
    if names.is_empty() {
        push(
            out,
            LadderViolationKind::Invalid,
            "backend_select: ladder is EMPTY — a ladder without rungs cannot \
             fall back (silent fallback forbidden, ADR-0165 §2.1)"
                .to_string(),
        );
        return;
    }
    let mut seen: Vec<&str> = Vec::new();
    for name in &names {
        if seen.contains(&name.as_str()) {
            push(
                out,
                LadderViolationKind::Invalid,
                format!(
                    "backend_select: duplicate ladder rung '{}' — a rung tried \
                     twice is a contract bug, not a fallback",
                    name
                ),
            );
        }
        seen.push(name.as_str());
        let entry = match crate::backends::find_by_name(name) {
            Some(e) => e,
            None => {
                push(
                    out,
                    LadderViolationKind::Invalid,
                    format!(
                        "backend_select: ladder rung '{}' has no registry record \
                         (the №333 registry is the SSOT)",
                        name
                    ),
                );
                continue;
            }
        };
        if let Some(word) = &class_word {
            if let Some(class) = crate::backends::BackendClass::parse(word) {
                if entry.class != class {
                    push(
                        out,
                        LadderViolationKind::Invalid,
                        format!(
                            "backend_select: ladder rung '{}' is class '{}', ladder \
                             serves '{}' (ADR-0165 §2.4)",
                            name,
                            entry.class.as_str(),
                            class.as_str()
                        ),
                    );
                }
            }
        }
        if device_production {
            if let crate::backends::ShaPin::PendingNo334 = entry.pin {
                push(
                    out,
                    LadderViolationKind::UnverifiableForProduction,
                    format!(
                        "backend_select: ladder rung '{}' (weights '{}') is \
                         UNVERIFIABLE for device profile production — the weights \
                         manifest is pending (№334 sha-pin path); a production \
                         ladder may only contain SHA-pinned backends (ADR-0165 §2.4)",
                        name, entry.weights_id
                    ),
                );
            }
        }
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_backend_stmts(stmts: &[Statement], production: bool, out: &mut Vec<LadderViolation>) {
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                check_backend_expr(value, production, out)
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                check_backend_expr(expr, production, out)
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_backend_expr(iterable, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(then_body, production, out);
                for (c, b) in else_ifs {
                    check_backend_expr(c, production, out);
                    check_backend_stmts(b, production, out);
                }
                if let Some(eb) = else_body {
                    check_backend_stmts(eb, production, out);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_backend_expr(scrutinee, production, out);
                for arm in arms {
                    check_backend_stmts(arm.body(), production, out);
                }
                if let Some(eb) = else_body {
                    check_backend_stmts(eb, production, out);
                }
            }
            _ => {}
        }
    }
}

fn check_backend_expr(expr: &Expr, production: bool, out: &mut Vec<LadderViolation>) {
    match expr {
        Expr::FnCall { name, args, span } => {
            if name == "backend_select" {
                verify_backend_ladder(args, span, production, out);
            }
            for a in args {
                check_backend_expr(a, production, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_backend_expr(a, production, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_backend_expr(left, production, out);
            check_backend_expr(right, production, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_backend_expr(condition, production, out);
            check_backend_expr(then_branch, production, out);
            check_backend_expr(else_branch, production, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_backend_expr(i, production, out);
            }
        }
        Expr::FieldAccess { object, .. } => check_backend_expr(object, production, out),
        Expr::IndexAccess { object, index, .. } => {
            check_backend_expr(object, production, out);
            check_backend_expr(index, production, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_backend_expr(v, production, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_backend_expr(condition, production, out);
            check_backend_stmts(then_body, production, out);
            for (c, body) in else_ifs {
                check_backend_expr(c, production, out);
                check_backend_stmts(body, production, out);
            }
            if let Some(eb) = else_body {
                check_backend_stmts(eb, production, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_backend_expr(scrutinee, production, out);
            for arm in arms {
                check_backend_stmts(arm.body(), production, out);
            }
            if let Some(eb) = else_body {
                check_backend_stmts(eb, production, out);
            }
        }
        Expr::Try { expr, .. } => check_backend_expr(expr, production, out),
        Expr::ProvBind { inner, .. } => check_backend_expr(inner, production, out),
        Expr::HandleSource { .. }
        | Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
}

// ── Naryad №440: the forecast surface companion (the backend_select
//    companion convention, ADR-0165 §2.4 template) ─────────────────────
//
// A statically-visible forecast constructor call site is verified at
// BUILD time:
//   - `series_make` with a literal struct form: the `label` field must
//     parse through the №322 lattice (ADR-0154) — an unknown word is a
//     loud compile error (FORECAST_LABEL_INVALID), never a silent
//     bottom label (under-tainting must be impossible by construction);
//   - `forecast_next` with a literal horizon: it must be a whole
//     number in 1..=1024 (FORECAST_HORIZON_INVALID — the allocation
//     guard, mirrored from the runtime refusal).
// Non-literal call sites are not statically verifiable and stay with
// the runtime checks (documented here, loud there).

/// One statically-verifiable forecast defect (the LadderViolation
/// template; the kind separates the two Category-A check ids).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastViolationKind {
    LabelInvalid,
    HorizonInvalid,
}

#[derive(Debug, Clone)]
pub struct ForecastViolation {
    pub kind: ForecastViolationKind,
    pub message: String,
    pub span: Span,
}

// ── Naryad #442: the recall surface companion ────────────────────────
//
// The same "broken call site must never surface as a runtime surprise"
// posture the forecast surface established (№440): a literal `recall`
// first argument that is NOT textual (the memory address space is
// textual — RECALL_QUERY_INVALID) and a literal min_confidence outside
// 0.0..=1.0 (the threshold contract every backend enforces at runtime
// — RECALL_CONFIDENCE_INVALID) are Category-A compile errors.

#[derive(Debug, Clone)]
pub enum RecallViolationKind {
    QueryInvalid,
    ConfidenceInvalid,
    // №445: the forgetting-surface companions — a literal dry_run flag
    // that is not a Bool (FORGET_DRYRUN_INVALID) and a literal ttl_secs
    // that is not a positive number (RETAIN_TTL_INVALID) are broken
    // call sites, loud on every compile path.
    ForgetDryRunInvalid,
    RetainTtlInvalid,
}

#[derive(Debug, Clone)]
pub struct RecallViolation {
    pub kind: RecallViolationKind,
    pub message: String,
    pub span: Span,
}

/// Public entry for the audit path (№442): the SAME rules the
/// companion applies, so a statically-broken recall call site is loud
/// on EVERY compile path.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn recall_surface_violations(declarations: &[Declaration]) -> Vec<RecallViolation> {
    let mut violations: Vec<RecallViolation> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => check_recall_stmts(&p.body, &mut violations),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_recall_stmts(&m.body, &mut violations);
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    check_recall_stmts(&r.body, &mut violations);
                }
            }
            Declaration::Flow(f) => check_recall_expr(&f.source, &mut violations),
            _ => {}
        }
    }
    violations
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn verify_recall_call(name: &str, args: &[Expr], span: &Span, out: &mut Vec<RecallViolation>) {
    let push = |out: &mut Vec<RecallViolation>, kind, message: String| {
        out.push(RecallViolation {
            kind,
            message,
            span: span.clone(),
        });
    };
    if name != "recall" && name != "forget" && name != "memory_retain_ttl" {
        return;
    }
    // №445: the forget front door — a literal dry_run flag must be a
    // Bool (the runtime refuses non-Bool; catching the literal here
    // makes the broken call site a compile error on EVERY path).
    if name == "forget" {
        match args.get(3) {
            None => {}
            Some(Expr::BoolLit { .. }) => {}
            Some(other) => {
                let got = match other {
                    Expr::StringLit { value, .. } => format!("the string literal \"{}\"", value),
                    Expr::FloatLit { value, .. } => format!("the numeric literal {}", value),
                    _ => "a non-Bool literal".to_string(),
                };
                push(
                    out,
                    RecallViolationKind::ForgetDryRunInvalid,
                    format!(
                        "forget: dry_run must be a Bool literal (true = the №280 preview, no state change) — got {}",
                        got
                    ),
                );
            }
        }
        return;
    }
    // №445: the canon retain(memory, ttl) — a literal ttl_secs must be
    // a positive number (the lifetime contract the runtime enforces).
    if name == "memory_retain_ttl" {
        match args.get(2) {
            Some(Expr::FloatLit { value, .. }) => {
                if !value.is_finite() || *value <= 0.0 {
                    push(
                        out,
                        RecallViolationKind::RetainTtlInvalid,
                        format!(
                            "memory_retain_ttl: ttl_secs {} is outside the lifetime contract (a finite value > 0.0)",
                            value
                        ),
                    );
                }
            }
            Some(Expr::StringLit { value, .. }) => {
                push(
                    out,
                    RecallViolationKind::RetainTtlInvalid,
                    format!(
                        "memory_retain_ttl: ttl_secs must be a number (a finite value > 0.0) — got the string literal \"{}\"",
                        value
                    ),
                );
            }
            Some(Expr::BoolLit { value, .. }) => {
                push(
                    out,
                    RecallViolationKind::RetainTtlInvalid,
                    format!(
                        "memory_retain_ttl: ttl_secs must be a number (a finite value > 0.0) — got the bool literal {}",
                        value
                    ),
                );
            }
            _ => {}
        }
        return;
    }
    // A literal query must be textual (the memory address space is
    // textual; a numeric/bool address is a broken call site).
    match args.first() {
        Some(Expr::StringLit { .. }) | None => {}
        Some(Expr::FloatLit { value, .. }) => {
            push(
                out,
                RecallViolationKind::QueryInvalid,
                format!(
                    "recall: the query must be a String (the memory address space is textual) — got the numeric literal {}",
                    value
                ),
            );
        }
        Some(Expr::BoolLit { value, .. }) => {
            push(
                out,
                RecallViolationKind::QueryInvalid,
                format!(
                    "recall: the query must be a String (the memory address space is textual) — got the bool literal {}",
                    value
                ),
            );
        }
        _ => {}
    }
    // A literal min_confidence must respect the threshold contract
    // (0.0..=1.0 — the runtime guard mirrored at check time).
    if let Some(Expr::FloatLit { value, .. }) = args.get(1) {
        if !value.is_finite() || *value < 0.0 || *value > 1.0 {
            push(
                out,
                RecallViolationKind::ConfidenceInvalid,
                format!(
                    "recall: min_confidence {} is outside the threshold contract (0.0..=1.0)",
                    value
                ),
            );
        }
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_recall_stmts(stmts: &[Statement], out: &mut Vec<RecallViolation>) {
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                check_recall_expr(value, out)
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                check_recall_expr(expr, out)
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_recall_expr(iterable, out);
                check_recall_stmts(body, out);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_recall_expr(condition, out);
                check_recall_stmts(body, out);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_recall_expr(condition, out);
                check_recall_stmts(then_body, out);
                for (c, b) in else_ifs {
                    check_recall_expr(c, out);
                    check_recall_stmts(b, out);
                }
                if let Some(eb) = else_body {
                    check_recall_stmts(eb, out);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_recall_expr(condition, out);
                check_recall_stmts(body, out);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_recall_expr(scrutinee, out);
                for arm in arms {
                    check_recall_stmts(arm.body(), out);
                }
                if let Some(eb) = else_body {
                    check_recall_stmts(eb, out);
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_recall_expr(expr: &Expr, out: &mut Vec<RecallViolation>) {
    match expr {
        Expr::FnCall { name, args, span } => {
            verify_recall_call(name, args, span, out);
            for a in args {
                check_recall_expr(a, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_recall_expr(a, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_recall_expr(left, out);
            check_recall_expr(right, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_recall_expr(condition, out);
            check_recall_expr(then_branch, out);
            check_recall_expr(else_branch, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_recall_expr(i, out);
            }
        }
        Expr::FieldAccess { object, .. } => check_recall_expr(object, out),
        Expr::IndexAccess { object, index, .. } => {
            check_recall_expr(object, out);
            check_recall_expr(index, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_recall_expr(v, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_recall_expr(condition, out);
            check_recall_stmts(then_body, out);
            for (c, body) in else_ifs {
                check_recall_expr(c, out);
                check_recall_stmts(body, out);
            }
            if let Some(eb) = else_body {
                check_recall_stmts(eb, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_recall_expr(scrutinee, out);
            for arm in arms {
                check_recall_stmts(arm.body(), out);
            }
            if let Some(eb) = else_body {
                check_recall_stmts(eb, out);
            }
        }
        _ => {}
    }
}

/// Public entry for the audit path (№440): the SAME rules the
/// companion applies, so a statically-broken forecast call site is
/// loud on EVERY compile path (`compile_program`/`run_program_with_dir`
/// call `audit_category_a`, not `check_program` — the №332 posture).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn forecast_surface_violations(declarations: &[Declaration]) -> Vec<ForecastViolation> {
    let mut violations: Vec<ForecastViolation> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => check_forecast_stmts(&p.body, &mut violations),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_forecast_stmts(&m.body, &mut violations);
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    check_forecast_stmts(&r.body, &mut violations);
                }
            }
            Declaration::Flow(f) => check_forecast_expr(&f.source, &mut violations),
            _ => {}
        }
    }
    violations
}

fn verify_forecast_call(name: &str, args: &[Expr], span: &Span, out: &mut Vec<ForecastViolation>) {
    let push = |out: &mut Vec<ForecastViolation>, kind, message: String| {
        out.push(ForecastViolation {
            kind,
            message,
            span: span.clone(),
        });
    };
    if name == "series_make" {
        // The struct form's `label` field: verified only when BOTH the
        // struct literal and the field value are literal.
        if let Some(Expr::StructLit { fields, .. }) = args.first() {
            if let Some(Expr::StringLit { value, .. }) = fields.get("label") {
                if let Err(e) = crate::labels::Label::parse(value) {
                    push(
                        out,
                        ForecastViolationKind::LabelInvalid,
                        format!(
                            "series_make: invalid label '{}' — the label must be a №322 lattice annotation ({})",
                            value, e
                        ),
                    );
                }
            }
        }
    }
    if name == "forecast_next" {
        // The literal horizon: whole number in 1..=1024 (the runtime
        // guard mirrored at check time).
        if let Some(Expr::FloatLit { value, .. }) = args.get(1) {
            if !value.is_finite()
                || value.fract() != 0.0
                || *value < 1.0
                || *value > crate::forecast::MAX_HORIZON
            {
                push(
                    out,
                    ForecastViolationKind::HorizonInvalid,
                    format!(
                        "forecast_next: horizon {} is outside the loud guard (a whole number in 1..={})",
                        value, crate::forecast::MAX_HORIZON as i64
                    ),
                );
            }
        }
    }
}

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_forecast_stmts(stmts: &[Statement], out: &mut Vec<ForecastViolation>) {
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                check_forecast_expr(value, out)
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                check_forecast_expr(expr, out)
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_forecast_expr(iterable, out);
                check_forecast_stmts(body, out);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_forecast_expr(condition, out);
                check_forecast_stmts(body, out);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_forecast_expr(condition, out);
                check_forecast_stmts(then_body, out);
                for (c, b) in else_ifs {
                    check_forecast_expr(c, out);
                    check_forecast_stmts(b, out);
                }
                if let Some(eb) = else_body {
                    check_forecast_stmts(eb, out);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_forecast_expr(condition, out);
                check_forecast_stmts(body, out);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_forecast_expr(scrutinee, out);
                for arm in arms {
                    check_forecast_stmts(arm.body(), out);
                }
                if let Some(eb) = else_body {
                    check_forecast_stmts(eb, out);
                }
            }
            _ => {}
        }
    }
}

fn check_forecast_expr(expr: &Expr, out: &mut Vec<ForecastViolation>) {
    match expr {
        Expr::FnCall { name, args, span } => {
            verify_forecast_call(name, args, span, out);
            for a in args {
                check_forecast_expr(a, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_forecast_expr(a, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_forecast_expr(left, out);
            check_forecast_expr(right, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_forecast_expr(condition, out);
            check_forecast_expr(then_branch, out);
            check_forecast_expr(else_branch, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_forecast_expr(i, out);
            }
        }
        Expr::FieldAccess { object, .. } => check_forecast_expr(object, out),
        Expr::IndexAccess { object, index, .. } => {
            check_forecast_expr(object, out);
            check_forecast_expr(index, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_forecast_expr(v, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_forecast_expr(condition, out);
            check_forecast_stmts(then_body, out);
            for (c, body) in else_ifs {
                check_forecast_expr(c, out);
                check_forecast_stmts(body, out);
            }
            if let Some(eb) = else_body {
                check_forecast_stmts(eb, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_forecast_expr(scrutinee, out);
            for arm in arms {
                check_forecast_stmts(arm.body(), out);
            }
            if let Some(eb) = else_body {
                check_forecast_stmts(eb, out);
            }
        }
        Expr::Try { expr, .. } => check_forecast_expr(expr, out),
        Expr::ProvBind { inner, .. } => check_forecast_expr(inner, out),
        Expr::HandleSource { .. }
        | Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
}

/// Effects of a builtin call site, read from the №316 SSOT map:
/// `Source` crosses the boundary inwards → `io`; `Sink` crosses
/// outwards → `io`, plus `audit` when the external effect is not
/// undoable-pure (a persistent, auditable write: state/db/file/memory
/// writes, delivery). `Pure`/`Lift` call sites carry no effect.
fn builtin_effects(name: &str) -> EffectSet {
    let mut set = EffectSet::new();
    // ── №352 (ADR-0174 §3.1): the directed audio effects ride the same
    //    trail gate — the direction table is the SSOT (beside the №332
    //    media-producing table). A call in an undeclared direction is a
    //    compile error (factual ⊑ declared).
    match audio_direction(name) {
        AudioDirection::Listen => {
            set.insert(Effect::Listen);
        }
        AudioDirection::Speak => {
            set.insert(Effect::Speak);
        }
        AudioDirection::Both => {
            set.insert(Effect::Listen);
            set.insert(Effect::Speak);
        }
        AudioDirection::None => {}
    }
    if let Some(class) = classify(name) {
        match class.role {
            Role::Source => {
                set.insert(Effect::Io);
            }
            Role::Sink => {
                set.insert(Effect::Io);
                if class.reversibility != Reversibility::Pure {
                    set.insert(Effect::Audit);
                }
            }
            Role::Pure | Role::Lift => {}
        }
    }
    set
}

// ── №352 (ADR-0174 §3.1): the directed-audio SSOT table ─────────────

/// The audio direction of a builtin (the naryad's two directed effects
/// over Audio). Channel-state builtins (`duplex_open`/`duplex_state`)
/// are direction-NEUTRAL: opening a channel and reading its state are
/// not flows of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioDirection {
    None,
    Listen,
    Speak,
    Both,
}

fn audio_direction(name: &str) -> AudioDirection {
    match name {
        // listen (audio in): STT-class surfaces + the listen stream.
        "stt_transcribe" | "whisper_transcribe" | "voice_enroll" | "listen_start"
        | "listen_stop" => AudioDirection::Listen,
        // speak (audio out): TTS-class surfaces + the speak stream.
        "tts_generate" | "tts_send" | "tts_speak" | "speak_start" | "speak_stop" => {
            AudioDirection::Speak
        }
        // The omni surface is both directions in one call (№334).
        "omni_ask" => AudioDirection::Both,
        _ => AudioDirection::None,
    }
}

/// Walk one expression collecting effect-bearing call sites.
/// `contract_of` resolves a call name (pattern, `tool.method`, or a
/// builtin) to its effect contract.
fn walk_effects_expr(expr: &Expr, contract_of: &dyn Fn(&str) -> EffectSet, acc: &mut EffectSet) {
    match expr {
        // №332 (ADR-0164): HandleSource carries the source-capture effect
        // (store insert — io by the №316 SSOT); ProvBind delegates to the
        // inner construction.
        Expr::HandleSource { .. } => acc.extend(builtin_effects("media_source_capture")),
        Expr::ProvBind { inner, .. } => walk_effects_expr(inner, contract_of, acc),
        Expr::FnCall { name, args, .. } => {
            acc.extend(contract_of(name));
            for a in args {
                walk_effects_expr(a, contract_of, acc);
            }
        }
        Expr::QualifiedCall {
            module,
            function,
            args,
            ..
        } => {
            let qualified = format!("{module}.{function}");
            acc.extend(contract_of(&qualified));
            for a in args {
                walk_effects_expr(a, contract_of, acc);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_effects_expr(left, contract_of, acc);
            walk_effects_expr(right, contract_of, acc);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_effects_expr(condition, contract_of, acc);
            walk_effects_expr(then_branch, contract_of, acc);
            walk_effects_expr(else_branch, contract_of, acc);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_effects_expr(condition, contract_of, acc);
            walk_effects_stmts(then_body, contract_of, acc);
            for (_, body) in else_ifs {
                walk_effects_stmts(body, contract_of, acc);
            }
            if let Some(eb) = else_body {
                walk_effects_stmts(eb, contract_of, acc);
            }
        }
        // №369: match-as-expression — scrutinee + arm bodies + else.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_effects_expr(scrutinee, contract_of, acc);
            for arm in arms {
                walk_effects_stmts(arm.body(), contract_of, acc);
            }
            if let Some(eb) = else_body {
                walk_effects_stmts(eb, contract_of, acc);
            }
        }
        Expr::Try { expr, .. } => walk_effects_expr(expr, contract_of, acc),
        Expr::List { items, .. } => {
            for i in items {
                walk_effects_expr(i, contract_of, acc);
            }
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_effects_expr(v, contract_of, acc);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            walk_effects_expr(object, contract_of, acc);
            walk_effects_expr(index, contract_of, acc);
        }
        Expr::FieldAccess { object, .. } => walk_effects_expr(object, contract_of, acc),
        Expr::StringLit { .. } | Expr::FloatLit { .. } | Expr::BoolLit { .. } => {}
        Expr::Ident { .. } => {}
    }
}

/// Walk one statement list collecting effects (recurses into nested
/// blocks; memory side-effect statements are `audit` effects).
fn walk_effects_stmts(
    stmts: &[Statement],
    contract_of: &dyn Fn(&str) -> EffectSet,
    acc: &mut EffectSet,
) {
    for stmt in stmts {
        match stmt {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                walk_effects_expr(value, contract_of, acc);
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                walk_effects_expr(iterable, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::While {
                condition, body, ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(then_body, contract_of, acc);
                for (_, body) in else_ifs {
                    walk_effects_stmts(body, contract_of, acc);
                }
                if let Some(eb) = else_body {
                    walk_effects_stmts(eb, contract_of, acc);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                walk_effects_expr(value, contract_of, acc);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                walk_effects_expr(scrutinee, contract_of, acc);
                for arm in arms {
                    let body = match arm {
                        MatchArm::Exact(_, b)
                        | MatchArm::StartsWith(_, b)
                        | MatchArm::Contains(_, b)
                        | MatchArm::Compare(_, _, b) => b,
                    };
                    walk_effects_stmts(body, contract_of, acc);
                }
                if let Some(eb) = else_body {
                    walk_effects_stmts(eb, contract_of, acc);
                }
            }
            Statement::Memorize(m) => {
                // Persistent memory write — an audit effect (ADR-0154 §9).
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&m.value, contract_of, acc);
            }
            Statement::Forget(f) => {
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&f.query, contract_of, acc);
            }
            Statement::Relate(r) => {
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&r.from, contract_of, acc);
                walk_effects_expr(&r.to, contract_of, acc);
            }
            Statement::Break | Statement::Continue => {}
        }
    }
}

/// One effect-trail container: a name key resolvable at call sites
/// (`"P"`, `"tool.method"`, `"LP"`), its optional declared trail, and
/// the body kind (learnables have no statement body — their factual
/// effect is fixed `{io}`: a model call).
struct EffectContainer<'a> {
    key: String,
    declared: Option<&'a EffectAnn>,
    name_for_errors: String,
    kind: ContainerKind<'a>,
}

enum ContainerKind<'a> {
    Statements(&'a [Statement]),
    Learnable,
}

/// Compute the whole-program effect trail: fixpoint over the pattern
/// call graph, then gate every DECLARED trail against the inferred
/// effects (factual ⊑ declared; excess = compile error with the list).
///
/// Recursion (ADR-0154 §9 decision): the effect domain is the
/// 4-element powerset of `{io, audit}` — union-joins over this finite
/// flat lattice converge in ≤ 4 passes, so the inference CONVERGES on
/// recursive patterns without annotations (the dispatcher's No-Go
/// signal does not fire; an explicit trail on recursive patterns is
/// welcome but not required). An unannotated recursive pattern
/// therefore behaves predictably: its effects are inferred, and the
/// gate applies only where a trail is declared.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_effect_trails(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    // ── Validate every declared trail (bad words are loud) and collect
    //    containers; a trail that failed validation is NOT registered
    //    in `declared`, so the gate skips it — the word error is the
    //    one the user sees.
    let mut containers: Vec<EffectContainer> = Vec::new();
    let mut declared: HashMap<String, EffectSet> = HashMap::new();

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                if let Some(ann) = &p.effects {
                    validate_effect_ann(ann, &format!("pattern '{}'", p.name), errors);
                }
            }
            Declaration::LearnablePattern(lp) => {
                if let Some(ann) = &lp.effects {
                    validate_effect_ann(ann, &format!("learnable pattern '{}'", lp.name), errors);
                }
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    if let Some(ann) = &m.effects {
                        validate_effect_ann(
                            ann,
                            &format!("tool method '{}.{}'", t.name, m.name),
                            errors,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
    fn register<'a>(
        containers: &mut Vec<EffectContainer<'a>>,
        declared: &mut HashMap<String, EffectSet>,
        key: String,
        name_for_errors: String,
        ann: Option<&'a EffectAnn>,
        kind: ContainerKind<'a>,
    ) {
        if let Some(ann) = ann {
            if let Ok(set) = parse_effect_set(ann) {
                declared.insert(key.clone(), set);
            }
        }
        containers.push(EffectContainer {
            key,
            declared: ann,
            name_for_errors,
            kind,
        });
    }

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => register(
                &mut containers,
                &mut declared,
                p.name.clone(),
                format!("pattern '{}'", p.name),
                p.effects.as_ref(),
                ContainerKind::Statements(&p.body),
            ),
            Declaration::LearnablePattern(lp) => register(
                &mut containers,
                &mut declared,
                lp.name.clone(),
                format!("learnable pattern '{}'", lp.name),
                lp.effects.as_ref(),
                ContainerKind::Learnable,
            ),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    register(
                        &mut containers,
                        &mut declared,
                        format!("{}.{}", t.name, m.name),
                        format!("tool method '{}.{}'", t.name, m.name),
                        m.effects.as_ref(),
                        ContainerKind::Statements(&m.body),
                    );
                }
            }
            _ => {}
        }
    }

    // A DECLARED trail is the call's contract (interface semantics);
    // otherwise the current fixpoint estimate; builtins via the №316
    // SSOT; unknown names carry no effects.
    let contract_of = |name: &str,
                       inferred: &HashMap<String, EffectSet>,
                       declared: &HashMap<String, EffectSet>|
     -> EffectSet {
        if let Some(d) = declared.get(name) {
            return d.clone();
        }
        if let Some(e) = inferred.get(name) {
            return e.clone();
        }
        builtin_effects(name)
    };

    let factual_effects = |kind: &ContainerKind,
                           inferred: &HashMap<String, EffectSet>,
                           declared: &HashMap<String, EffectSet>|
     -> EffectSet {
        match kind {
            ContainerKind::Learnable => {
                // A learnable pattern IS an LLM call — io by
                // construction (№316: call_llm is a Source).
                let mut s = EffectSet::new();
                s.insert(Effect::Io);
                s
            }
            ContainerKind::Statements(body) => {
                let mut acc = EffectSet::new();
                let resolve = |name: &str| contract_of(name, inferred, declared);
                walk_effects_stmts(body, &resolve, &mut acc);
                acc
            }
        }
    };

    // ── Fixpoint over the (possibly recursive) call graph ──
    let mut inferred: HashMap<String, EffectSet> = containers
        .iter()
        .map(|c| (c.key.clone(), EffectSet::new()))
        .collect();
    let max_passes = (containers.len() + 2).clamp(4, 16);
    for _ in 0..max_passes {
        let mut changed = false;
        for c in &containers {
            let new_set = factual_effects(&c.kind, &inferred, &declared);
            if inferred.get(&c.key) != Some(&new_set) {
                inferred.insert(c.key.clone(), new_set);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ── Gate every DECLARED trail: factual ⊑ declared ──
    for c in &containers {
        let Some(ann) = c.declared else { continue };
        let Some(declared_set) = declared.get(&c.key) else {
            continue;
        };
        let actual = factual_effects(&c.kind, &inferred, &declared);
        let excess: Vec<Effect> = actual.difference(declared_set).copied().collect();
        if excess.is_empty() {
            continue;
        }
        let fmt = |es: &[Effect]| {
            let words: Vec<&str> = es.iter().map(|e| e.word()).collect();
            if words.is_empty() {
                "⟨⟩".to_string()
            } else {
                format!("⟨{}⟩", words.join(", "))
            }
        };
        let declared_sorted: Vec<Effect> = declared_set.iter().copied().collect();
        let actual_sorted: Vec<Effect> = actual.iter().copied().collect();
        errors.push(SpannedError::at(
            format!(
                "effect trail violation on {}: declared {} but body requires {} (excess: {})",
                c.name_for_errors,
                fmt(&declared_sorted),
                fmt(&actual_sorted),
                excess
                    .iter()
                    .map(|e| e.word())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ann.span.clone(),
        ));
    }
}

// ── Sink clearance (Наряд №325, ADR-0161) ────────────────────────────

/// One sink call-site whose argument label does not clear the sink.
#[derive(Debug, Clone)]
pub struct SinkViolation {
    /// Container tag: `pattern P`, `tool t.m`, `route GET /x`, ...
    pub container: String,
    /// Sink builtin name (from the №316 classification — never a
    /// hand-written list).
    pub fn_name: String,
    /// 0-based index of the offending argument.
    pub arg_index: usize,
    pub span: Span,
    /// The argument's inferred label (№322/№323 machinery).
    pub label: Label,
    /// Which bridge threshold failed (№391): the clearance reason —
    /// e.g. "untrusted-exec", "secret-exec", "untrusted-egress",
    /// "private-egress", "private-db", "irreversible-content". Consumed
    /// by the deny message (explainable refusal) and by №392 DenyEvent.
    pub reason: &'static str,
}

/// Conservative confidentiality markers for string LITERALS (ADR-0161
/// §3): a literal carrying a personal-data marker (passport / SNILS /
/// diagnosis / confidential wording — the vocabulary is deliberately
/// small and bilingual-safe) or a private-infrastructure URL marker is
/// treated as `private, trusted`. Sound: markers are conservative —
/// a literal without markers stays bottom, an unannotated program
/// keeps passing unless it actually carries the marked data to a sink.
fn literal_confidentiality(text: &str) -> bool {
    let lower = text.to_lowercase();
    const WORD_MARKERS: &[&str] = &[
        "паспорт",
        "снилс",
        "диагноз",
        "конфиденциальн",
        "персональн",
        "секретн",
        "confidential",
        "passport",
        "social security",
    ];
    if WORD_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Private-infrastructure URL markers (ADR-0161 §3): a literal
    // address/host pointing at internal infrastructure names private
    // data — the destination IS the leak vector.
    const URL_MARKERS: &[&str] = &["internal", "intranet", "corp.", "private", "secret"];
    if URL_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Structural markers: passport RU (dddd dddddd / dddddddd), SNILS
    // (ddd-ddd-ddd d), card-like 16-digit groups are covered by the
    // word markers above in the corpus; the digit shapes below catch
    // the bare-number forms.
    let digits: Vec<char> = lower.chars().collect();
    let digit_at = |i: usize| digits.get(i).is_some_and(|c| c.is_ascii_digit());
    let pasp = (0..digits.len()).filter(|i| digit_at(*i)).any(|i| {
        // 4 digits, optional space/dash, 6 digits
        (0..4).all(|k| digit_at(i + k))
            && matches!(digits.get(i + 4), None | Some(' ') | Some('-'))
            && (0..6).all(|k| digit_at(i + 5 + k))
            && digits.get(i + 11).is_none_or(|c| !c.is_ascii_digit())
    });
    if pasp {
        return true;
    }
    let snils = (0..digits.len()).filter(|i| digit_at(*i)).any(|i| {
        (0..3).all(|k| digit_at(i + k))
            && digits.get(i + 3) == Some(&'-')
            && (0..3).all(|k| digit_at(i + 4 + k))
            && digits.get(i + 7) == Some(&'-')
            && (0..3).all(|k| digit_at(i + 8 + k))
            && digits.get(i + 11).is_none_or(|c| !c.is_ascii_digit())
    });
    snils
}

/// The seed label of a global entity initializer: a literal carrying
/// confidentiality markers is `private, trusted`; everything else is
/// bottom (the №322 permissive default).
fn entity_seed_label(value: Option<&Expr>) -> Label {
    match value {
        Some(Expr::StringLit { value, .. }) if literal_confidentiality(value) => Label {
            conf: crate::labels::Conf::Private,
            integrity: crate::labels::Integrity::Trusted,
            consent: Default::default(),
        },
        _ => Label::bottom(),
    }
}

/// Label of an expression, extending the №323 expression rules with the
/// literal confidentiality markers (ADR-0161 §3) and the №332 perception
/// forms: `source <origin>` carries the origin's declared label; `from
/// <origin> <construction>` carries the JOIN of the origin label and the
/// construction's data-flow label (conservative — the strongest axis wins).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn sink_arg_label(
    expr: &Expr,
    env: &BTreeMap<String, Label>,
    origin_labels: &BTreeMap<String, Label>,
) -> Label {
    match expr {
        Expr::StringLit { value, .. } if literal_confidentiality(value) => Label {
            conf: crate::labels::Conf::Private,
            integrity: crate::labels::Integrity::Trusted,
            consent: Default::default(),
        },
        Expr::HandleSource { origin, .. } => origin_labels
            .get(origin)
            .cloned()
            .unwrap_or_else(Label::bottom),
        Expr::ProvBind { origin, inner, .. } => {
            let o = origin_labels
                .get(origin)
                .cloned()
                .unwrap_or_else(Label::bottom);
            o.join(&expr_label(inner, env))
        }
        _ => expr_label(expr, env),
    }
}

type SinkCheck<'a> =
    &'a dyn Fn(&Expr, &str, &BTreeMap<String, Label>, &FlowCtx, &mut Vec<SinkViolation>);

/// Flow context for the №325 walk (Naryad #387): the var → origin-kind
/// alias map (so `let g = f; let h = g;` keeps the media provenance —
/// the alias-invariance contract), the origin name → kind vocabulary
/// (a clone — nested fns cannot capture the outer local), and the
/// likeness-token presence flag (a bound `likeness_verify` result
/// lexically before the gated call). Conservative by construction:
/// branch/loop-bound tokens and aliases do NOT escape their fork
/// (fail-closed — the entry state survives).
#[derive(Default, Clone)]
struct FlowCtx {
    /// var name -> origin kind ("camera" | "likeness" | ...).
    origin_kinds: BTreeMap<String, String>,
    /// origin name -> kind (the declared vocabulary, read-only).
    origin_kind_map: BTreeMap<String, String>,
    /// a `likeness_verify` result is bound in the current body scope.
    token_bound: bool,
    /// №349: `profile duty { materialization: denied }` resolved — the
    /// text-materialization of private-labeled values is a compile error.
    duty_materialization_denied: bool,
    /// №349: `profile duty { surfaces: local_only }` resolved — network
    /// sinks (the №316 Network label class) are compile errors.
    duty_surfaces_local_only: bool,
    /// №729: var name → the read-back domain the value carries ("cron" |
    /// "mtree") — populated from the paired Sources (direct call,
    /// let-chain, field read, `to_string`), dropped on any foreign
    /// binding (fail-closed, the №387 alias posture).
    readback_domains: BTreeMap<String, String>,
}

/// Resolve the origin kind a media argument carries: a direct
/// `source <origin>` handle, or a variable bound through the alias
/// chain (the №387 FlowCtx). Unknown provenance -> None.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn media_arg_origin_kind(expr: &Expr, ctx: &FlowCtx) -> Option<String> {
    match expr {
        Expr::HandleSource { origin, .. } => ctx.origin_kind_map.get(origin).cloned(),
        // №387: the ProvBind construction (`from <origin> media_store_*`)
        // carries the same provenance as a direct capture.
        Expr::ProvBind { origin, .. } => ctx.origin_kind_map.get(origin).cloned(),
        Expr::Ident { name, .. } => ctx.origin_kinds.get(name).cloned(),
        _ => None,
    }
}

/// №729: resolve the read-back domain an expression carries — a var
/// bound (directly or through a let-chain / field read / `to_string`)
/// from a paired Source. ANY other shape is foreign admixture -> None
/// (fail-closed — the conservative boundary of the pair table).
/// №532: the non-carrying tail is EXPLICIT — a new Expr variant is a
/// compile error here until it is consciously classified (the
/// read-back domain never rides a new shape by default).
fn readback_domain(expr: &Expr, ctx: &FlowCtx) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } => ctx.readback_domains.get(name).cloned(),
        Expr::FieldAccess { object, .. } => readback_domain(object, ctx),
        Expr::FnCall { name, args, .. } if name == "to_string" && args.len() == 1 => {
            readback_domain(&args[0], ctx)
        }
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::FnCall { .. }
        | Expr::QualifiedCall { .. }
        | Expr::BinaryOp { .. }
        | Expr::IfElse { .. }
        | Expr::List { .. }
        | Expr::IndexAccess { .. }
        | Expr::StructLit { .. }
        | Expr::BlockIfElse { .. }
        | Expr::MatchExpr { .. }
        | Expr::Try { .. }
        | Expr::HandleSource { .. }
        | Expr::ProvBind { .. } => None,
    }
}

/// №729: does the paired Source feed this sink's lookup-key position?
/// The exemption exists only for the pair table's (sink, key_arg) rows
/// AND only when the argument's provenance is the pair's own domain.
fn readback_pair_clears(fn_name: &str, arg_index: usize, arg: &Expr, ctx: &FlowCtx) -> bool {
    let domain = match readback_domain(arg, ctx) {
        Some(d) => d,
        None => return false,
    };
    READBACK_PAIRS
        .iter()
        .any(|p| p.sink == fn_name && p.key_arg == arg_index && p.domain == domain)
}

/// Collect sink call-sites whose argument labels do not clear the sink
/// (№325, ADR-0161 §2). The sink list comes from the №316 SSOT map
/// (`Role::Sink`) — never a hand-written list. Confidentiality
/// clearance: a sink requires `public` (and `poisoned` clears nothing).
/// The EXEC class additionally refuses untrusted data (an integrity
/// decision gate for command execution; the general integrity gate is
/// №327). The VOICE class additionally refuses anything without a
/// consent scope (consent SOURCES are Phase 2, №335 — until then every
/// voice egress is unconsented by default, loud by design).
/// ── Naryad #391: the data ↔ action bridge — per-sink thresholds ──────
///
/// The bridge rule for ACTION sinks: the decision argument (command /
/// URL / SQL / addressee) must satisfy BOTH axes of the label lattice
/// (ADR-0154):
///   - confidentiality: label.conf ⊑ Public (the action must not leak
///     secrets into its own trace);
///   - integrity: label.integrity ≥ Trusted (untrusted data must not
///     drive an irreversible action).
///
/// This is the SYSTEMATIC rule the point classes (UNTRUSTED_EXEC_DECISION,
/// SECRET_TO_EXEC, SECRET_EGRESS_VCS, SECRET_EGRESS_NETWORK, PII_EGRESS_*)
/// were hand-expressing; the classes stay (leak-suite vocabulary, DoD в —
/// nothing is weakened), the table documents the thresholds per sink.
///
/// Orthogonality with grants (№390): the grant check authorizes the
/// ACTION (scope/TTL/quota, runtime); the bridge gates the DATA that
/// feeds it (labels, compile time). `db_execute_with_grant` is not a
/// №325 sink — its grant gates are runtime-only; the bridge does not
/// duplicate them.
pub struct ActionBridgeThreshold {
    pub sink: &'static str,
    /// Human-readable decision-argument description.
    pub decision_arg: &'static str,
    /// The confidentiality threshold (always Public today — the action
    /// trace must not carry secrets).
    pub max_conf: &'static str,
    /// The integrity threshold (always Trusted today — untrusted data
    /// must not drive the action).
    pub min_integrity: &'static str,
    /// How the integrity half is enforced for this sink.
    pub integrity_enforcement: &'static str,
}

pub const ACTION_BRIDGE: &[ActionBridgeThreshold] = &[
    ActionBridgeThreshold {
        sink: "exec",
        decision_arg: "arg 0 — command",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EXEC_DECISION)",
    },
    ActionBridgeThreshold {
        sink: "exec_argv",
        decision_arg: "arg 0 — binary",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EXEC_DECISION)",
    },
    ActionBridgeThreshold {
        sink: "git_push",
        decision_arg: "arg 0 — remote URL/ref",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; №391 adds the integrity half)",
    },
    ActionBridgeThreshold {
        sink: "http_post",
        decision_arg: "arg 0 — URL",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; body args = egress classes)",
    },
    ActionBridgeThreshold {
        sink: "send_message",
        decision_arg: "arg 0 — chat/addressee",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; body args = egress classes)",
    },
    ActionBridgeThreshold {
        sink: "db_execute",
        decision_arg: "arg 0 — SQL",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance for confidentiality (private-db); integrity via SQL_DYNAMIC (non-literal SQL is refused before the bridge can see it)",
    },
];

/// ── Naryad #729 (gh#1350 Ask 2, the owner verbatim (a)) ──────────────
/// The sanctioned read-back pairs: a value originating from the paired
/// Source of the SAME domain, used as the destructive lookup key of the
/// paired Sink, clears the sink gate for that key position. The
/// mechanism fact (the #1350 package §2): the paired sinks are
/// MATCH-ONLY — the key never persists new data into the state
/// (`cron_remove` filters stored entries by the key, `builtins/cron.rs`;
/// `mtree_forget` removes the node the key addresses,
/// `builtins/memory.rs`) — the data returns to the domain it came from;
/// no egress channel by construction. The mtree twin is `mtree_forget`,
/// NOT `mtree_store`: store's arg 0 is the PAYLOAD text (auto-assigned
/// id, admission-scored — `memory.rs:478`) with NO key position, so a
/// store call stays gated in full (the #1350 issue's «mtree reads →
/// mtree_store» conjecture is corrected by the source).
///
/// The conservative boundary (fail-closed, №523/№531): the provenance
/// chain must be paired-source-only — field reads and `to_string`
/// preserve the domain, ANY foreign admixture (concat/interpolation
/// with data not originating from the paired Source) loses the
/// exemption; the key POSITION only (multi-arg sinks keep every other
/// position gated — the TAINT_PERSISTENCE vocabulary); a poisoned label
/// clears nothing anywhere (ADR-0154 §2.1 — the exemption never
/// overrides quarantine).
pub struct ReadbackPair {
    /// The Source builtin the key may originate from.
    pub source: &'static str,
    /// The Sink builtin whose lookup-key position is exempted.
    pub sink: &'static str,
    /// The lookup-key argument position the exemption covers.
    pub key_arg: usize,
    /// The domain both ends share ("cron" | "mtree").
    pub domain: &'static str,
}

pub const READBACK_PAIRS: &[ReadbackPair] = &[
    ReadbackPair {
        source: "cron_list",
        sink: "cron_remove",
        key_arg: 0,
        domain: "cron",
    },
    ReadbackPair {
        source: "mtree_retrieve",
        sink: "mtree_forget",
        key_arg: 0,
        domain: "mtree",
    },
];

#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn sink_clearance_violations(declarations: &[Declaration]) -> Vec<SinkViolation> {
    let mut violations = Vec::new();

    // ── Naryad #349: the duty-profile compile rules ──
    // The carrier is the `profile duty` declaration (№348/ADR-0172 §3.4):
    // `materialization: denied` makes the text-materialization of
    // private-labeled values compile errors; `surfaces: local_only`
    // makes network sinks (the №316 Network label class) compile errors.
    // Absent the declaration BOTH flags are false and this walk behaves
    // exactly as before — the existing corpus (214+ examples, legacy
    // profile included) is untouched by construction.
    let duty = crate::profile::resolve(declarations);
    let duty_materialization_denied = duty.duty_materialization_denied;
    let duty_surfaces_local_only = duty.duty_surfaces_local_only;

    // №332 (ADR-0164): declared origin labels — `source <origin>` /
    // `from <origin> ...` constructions carry their origin's declared
    // conf into the flow (joined with the data-flow label).
    // №387: the parallel name → kind vocabulary feeds the FlowCtx alias
    // tracking (media provenance through let-chains) and the
    // video-likeness presence gate.
    let mut origin_kind_map: BTreeMap<String, String> = BTreeMap::new();
    let origin_labels: BTreeMap<String, Label> = declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Origin(o) => {
                let conf = crate::media::parse_sensitivity(
                    &o.fields
                        .iter()
                        .find(|(k, _)| k == "label")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default(),
                )
                .ok()?;
                if let Some((_, kind)) = o.fields.iter().find(|(k, _)| k == "kind") {
                    origin_kind_map.insert(o.name.clone(), kind.clone());
                }
                Some((
                    o.name.clone(),
                    Label {
                        conf,
                        integrity: crate::labels::Integrity::Trusted,
                        consent: Default::default(),
                    },
                ))
            }
            _ => None,
        })
        .collect();

    // Seed environment: global entities (initializers with confidentiality
    // markers are private — ADR-0161 §3), all other names bottom.
    let mut seed: BTreeMap<String, Label> = BTreeMap::new();
    for decl in declarations {
        match decl {
            Declaration::EntitySimple(e) => {
                seed.insert(e.name.clone(), entity_seed_label(Some(&e.value)));
            }
            Declaration::EntityRecord(e) => {
                // Record entities are maps; the entity name itself seeds
                // with the strongest field marker.
                let strongest = e
                    .fields
                    .iter()
                    .map(|f| entity_seed_label(Some(&f.value)))
                    .fold(Label::bottom(), |a, b| a.join(&b));
                seed.insert(e.name.clone(), strongest);
            }
            _ => {}
        }
    }

    fn is_sink(name: &str) -> bool {
        matches!(
            classify(name).map(|c| c.role),
            Some(crate::builtins_classification::Role::Sink)
        )
    }

    fn sink_kind(name: &str) -> &'static str {
        // №331 boundary (loud): media GENERATION sinks (vision_*/video_*)
        // produce synthetic content — their egress is gated by the
        // marking machinery (MEDIA_SYNTHETIC_UNMARKED, №320) and their
        // label-side gating is Phase 2; they are outside №325.
        if name.starts_with("vision_") || name.starts_with("video_") || name == "tts_synthesize" {
            return "media";
        }
        match name {
            "exec" | "exec_argv" => "exec",
            "git_push" => "vcs",
            "tts_send" => "voice",
            "db_execute" => "db",
            "print"
            | "respond"
            | "respond_html"
            | "respond_html_status"
            | "respond_html_doc"
            | "html_response" => "output",
            "write_file" | "append_file" | "delete_file" => "file",
            // Persistent memory writes: untrusted data must not persist
            // (the TAINT_PERSISTENCE vocabulary; №266 statement forms are
            // covered separately).
            "memorize" | "mem_set" | "mtree_store" | "kv_set" => "memory",
            // №331 (ADR-0162): the sanctioned media materialization sink
            // is FILE egress — private-labelled handles fail the
            // default clearance (`private-egress`) at compile time; the
            // runtime backstop (MEDIA_SEALED_EGRESS) refuses non-public
            // entries even if the static layer was bypassed.
            "media_save" => "file",
            _ => "network",
        }
    }

    /// Clearance check for one sink argument. Returns the reason the
    /// argument fails, if any.
    fn clearance_failure(fn_name: &str, label: &Label) -> Option<&'static str> {
        // №335: the quarantine sink is the ONLY legal egress for a
        // poisoned value (unconditional audit event; №326 posture) —
        // exempt from every clearance class here. Every OTHER sink
        // still refuses poisoned below (quarantine absorbs them).
        if fn_name == "quarantine_write" {
            return None;
        }
        // Quarantine clears nothing, anywhere (ADR-0154 §2.1).
        if label.conf == crate::labels::Conf::Poisoned {
            return Some("poisoned");
        }
        match sink_kind(fn_name) {
            // Command execution: untrusted data must not drive it, and
            // secrets must never enter it.
            "exec" => {
                if label.integrity == crate::labels::Integrity::Untrusted {
                    Some("untrusted-exec")
                } else if label.conf != crate::labels::Conf::Public {
                    Some("secret-exec")
                } else {
                    None
                }
            }
            // Media generation: №331 Phase-2 boundary — the marking
            // machinery (MEDIA_SYNTHETIC_UNMARKED, №320) owns it here.
            "media" => None,
            // Voice egress requires a consent scope (Phase-2 sources, №335;
            // until then the scope is empty by default — loud by design).
            "voice" => {
                if label.consent.is_empty() {
                    Some("voice-unconsented")
                } else {
                    None
                }
            }
            // Irreversible DB writes with destructive literals are gated
            // regardless of label (grant algebra is Phase 3, №339).
            // NOTE: only schema-destroying forms (DROP/TRUNCATE) —
            // DELETE/ALTER are parameterized CRUD, gated by SQL_DYNAMIC.
            "db" => {
                if label.conf != crate::labels::Conf::Public {
                    Some("private-db")
                } else {
                    None
                }
            }
            // Public-output and memory sinks also refuse UNTRUSTED data
            // (the HTML-injection and taint-persistence vocabularies —
            // the leak-suite classes; the general integrity gate is
            // №327, these two special cases start here). Everything else:
            // confidentiality clearance (public only); network sinks
            // additionally refuse untrusted data (UNTRUSTED_EGRESS_*).
            _ => {
                let kind = sink_kind(fn_name);
                if label.conf != crate::labels::Conf::Public {
                    Some("private-egress")
                } else if label.integrity == crate::labels::Integrity::Untrusted
                    // №391 bridge: "vcs" joins the integrity-gated kinds —
                    // an untrusted URL must not drive git_push (the
                    // systematization of the point rules; the class stays
                    // UNTRUSTED_EGRESS_NETWORK, the leak-suite vocabulary).
                    && matches!(kind, "network" | "vcs" | "output" | "memory")
                {
                    Some("untrusted-egress")
                } else {
                    None
                }
            }
        }
    }

    let check_calls = |expr: &Expr,
                       container: &str,
                       env: &BTreeMap<String, Label>,
                       ctx: &FlowCtx,
                       violations: &mut Vec<SinkViolation>| {
        if let Expr::FnCall { name, args, .. } = expr {
            // ── Naryad #387 (ADR-0149 D1): VIDEO_LIKENESS_NO_CONSENT ──
            // video_render with I2V reference frames (positions 2/3 —
            // the UNTRUSTED_FRAME positions) originating from a
            // `kind: "likeness"` origin requires the likeness ritual: a
            // bound `likeness_verify` result in the body scope
            // (presence-based, the D2 honest boundary). Always a
            // compile error — no profile downgrades a deepfake gate.
            if name == "video_render" && args.len() >= 3 {
                for pos in [2usize, 3usize] {
                    if let Some(arg) = args.get(pos) {
                        let kind = media_arg_origin_kind(arg, ctx);
                        if kind.as_deref() == Some("likeness") && !ctx.token_bound {
                            violations.push(SinkViolation {
                                container: container.to_string(),
                                fn_name: name.clone(),
                                arg_index: pos,
                                span: expr.span().clone(),
                                label: sink_arg_label(arg, env, &origin_labels),
                                reason: "video-likeness-no-consent",
                            });
                        }
                    }
                }
            }
            // ── Naryad #349: the duty-profile compile rules ──
            // (а) `surfaces: local_only` — a network-class builtin (the
            // №316 Network label, the SSOT class list) is a compile
            // error in duty mode: the background contour is local-only.
            // Fires once per call site, regardless of argument labels.
            if ctx.duty_surfaces_local_only {
                let network_classed = classify(name)
                    .map(|c| c.default_label == crate::builtins_classification::Label::Network)
                    .unwrap_or(false);
                if network_classed {
                    violations.push(SinkViolation {
                        container: container.to_string(),
                        fn_name: name.clone(),
                        arg_index: 0,
                        span: expr.span().clone(),
                        label: Label::bottom(),
                        reason: "duty-network-sink",
                    });
                }
            }
            // (б) `materialization: denied` — lifting a private-labeled
            // value into TEXT (the canonical materializers: `str`,
            // `json_encode`) is a compile error in duty mode: the
            // background contour never renders private data, even
            // locally. The №325 sink gate does NOT catch these (the
            // materializers are Pure/Lift — the result may never
            // egress); duty mode refuses the lift itself.
            if ctx.duty_materialization_denied && matches!(name.as_str(), "str" | "json_encode") {
                for (i, a) in args.iter().enumerate() {
                    let l = sink_arg_label(a, env, &origin_labels);
                    if l.conf == crate::labels::Conf::Private {
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: i,
                            span: expr.span().clone(),
                            label: l,
                            reason: "duty-materialization",
                        });
                    }
                }
            }
            // ── Н455 layer 4: the file-path decision position ──
            // A file-INGEST path originating from an untrusted source
            // (query_param / json_body / form_data / http_get / …) is the
            // same decision-position violation the №325 exec class pins:
            // the path CHOOSES which bytes enter the program (the audit's
            // `read_file(query_param(...))` exfiltration vector).
            // Category A `UNTRUSTED_FILE_PATH`, by analogy with
            // UNTRUSTED_EXEC_DECISION (advisory profile downgrades it the
            // same way).
            if matches!(name.as_str(), "read_file" | "read_file_tokens") {
                if let Some(a) = args.first() {
                    let l = sink_arg_label(a, env, &origin_labels);
                    if l.integrity == crate::labels::Integrity::Untrusted {
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: 0,
                            span: expr.span().clone(),
                            label: l,
                            reason: "untrusted-file-path",
                        });
                    }
                }
            }
            if is_sink(name) {
                for (i, a) in args.iter().enumerate() {
                    // Destructive DB literals are gated by CONTENT (a
                    // DROP needs no tainted data to destroy state):
                    // the vocabulary is the runtime twin's population
                    // (grants.rs::extract_destructive_ops — the ops the
                    // grant algebra meters), plus the DROP-<object>
                    // superset the runtime scanner doesn't parse. The
                    // naryad-397 follow-up caught DELETE/ALTER slipping
                    // the static gate while the runtime gates metered
                    // them (the audit doc comment and REFERENCE.md
                    // always claimed DROP/DELETE/TRUNCATE/ALTER — and
                    // the bare runtime path trusts the static gate).
                    if name == "db_execute"
                        && matches!(
                            a,
                            Expr::StringLit { value, .. }
                                if ["drop table", "drop database", "drop index", "truncate", "delete from", "alter table"].iter().any(|w| value.to_lowercase().contains(w))
                        )
                    {
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: i,
                            span: expr.span().clone(),
                            label: Label::bottom(),
                            reason: "irreversible-content",
                        });
                        continue;
                    }
                    let label = sink_arg_label(a, env, &origin_labels);
                    // ── Naryad #387: the generalized media-sink egress ──
                    // media_save accepts a private/camera-origin handle
                    // when the author holds a legal credential: a
                    // non-empty consent scope (№335 consent_grant), or
                    // the likeness ritual (a bound likeness_verify
                    // result) for camera/likeness-kind origins. The
                    // public label and the poisoned refusal are
                    // UNCHANGED — the gate stays fail-closed (no
                    // credential, no egress).
                    if name == "media_save" {
                        if label.conf == crate::labels::Conf::Poisoned {
                            violations.push(SinkViolation {
                                container: container.to_string(),
                                fn_name: name.clone(),
                                arg_index: i,
                                span: expr.span().clone(),
                                label,
                                reason: "poisoned",
                            });
                            continue;
                        }
                        let kind = media_arg_origin_kind(a, ctx);
                        let consented = !label.consent.is_empty();
                        let likenessed =
                            matches!(kind.as_deref(), Some("likeness") | Some("camera"))
                                && ctx.token_bound;
                        if label.conf != crate::labels::Conf::Public && !consented && !likenessed {
                            violations.push(SinkViolation {
                                container: container.to_string(),
                                fn_name: name.clone(),
                                arg_index: i,
                                span: expr.span().clone(),
                                label,
                                reason: "private-egress",
                            });
                        }
                        continue;
                    }
                    if let Some(reason) = clearance_failure(name, &label) {
                        // ── №729 (gh#1350 Ask 2, the owner verbatim (a)):
                        // the sanctioned read-back pair — the destructive
                        // lookup key originating from the SAME domain's
                        // Source clears. The key never persists (the pair
                        // table's match-only mechanism fact). Poisoned
                        // clears nothing anywhere (ADR-0154 §2.1) — the
                        // exemption never overrides quarantine.
                        if label.conf != crate::labels::Conf::Poisoned
                            && readback_pair_clears(name, i, a, ctx)
                        {
                            continue;
                        }
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: i,
                            span: expr.span().clone(),
                            label,
                            reason,
                        });
                    }
                }
            }
        }
    };

    #[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
    fn walk_expr(
        expr: &Expr,
        container: &str,
        env: &BTreeMap<String, Label>,
        origin_labels: &BTreeMap<String, Label>,
        ctx: &FlowCtx,
        check: SinkCheck,
        violations: &mut Vec<SinkViolation>,
    ) {
        check(expr, container, env, ctx, violations);
        match expr {
            Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => {
                for a in args {
                    walk_expr(a, container, env, origin_labels, ctx, check, violations);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                walk_expr(left, container, env, origin_labels, ctx, check, violations);
                walk_expr(right, container, env, origin_labels, ctx, check, violations);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                walk_expr(
                    condition,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                );
                walk_expr(
                    then_branch,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                );
                walk_expr(
                    else_branch,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                );
            }
            Expr::BlockIfElse {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                // Expression position: env side effects of the branches do
                // not escape (№323 D5) — analyze the branches on forks.
                walk_expr(
                    condition,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                );
                let mut env_t = env.clone();
                let mut ctx_t = ctx.clone();
                walk_stmts(
                    then_body,
                    container,
                    &mut env_t,
                    origin_labels,
                    &mut ctx_t,
                    check,
                    violations,
                );
                for (_, b) in else_ifs {
                    let mut env_b = env.clone();
                    let mut ctx_b = ctx.clone();
                    walk_stmts(
                        b,
                        container,
                        &mut env_b,
                        origin_labels,
                        &mut ctx_b,
                        check,
                        violations,
                    );
                }
                if let Some(eb) = else_body {
                    let mut env_e = env.clone();
                    // №387: conservative fork (branch-local ctx discarded).
                    let mut ctx_e = ctx.clone();
                    walk_stmts(
                        eb,
                        container,
                        &mut env_e,
                        origin_labels,
                        &mut ctx_e,
                        check,
                        violations,
                    );
                }
            }
            Expr::Try { expr, .. } => {
                walk_expr(expr, container, env, origin_labels, ctx, check, violations)
            }
            Expr::List { items, .. } => {
                for i in items {
                    walk_expr(i, container, env, origin_labels, ctx, check, violations);
                }
            }
            Expr::StructLit { fields, .. } => {
                for v in fields.values() {
                    walk_expr(v, container, env, origin_labels, ctx, check, violations);
                }
            }
            Expr::IndexAccess { object, index, .. } => {
                walk_expr(
                    object,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                );
                walk_expr(index, container, env, origin_labels, ctx, check, violations);
            }
            Expr::FieldAccess { object, .. } => walk_expr(
                object,
                container,
                env,
                origin_labels,
                ctx,
                check,
                violations,
            ),
            _ => {}
        }
    }

    #[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        env: &mut BTreeMap<String, Label>,
        origin_labels: &BTreeMap<String, Label>,
        ctx: &mut FlowCtx,
        check: SinkCheck,
        violations: &mut Vec<SinkViolation>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { name, value, .. }
                | Statement::Assign { name, value, .. } => {
                    walk_expr(value, container, env, origin_labels, ctx, check, violations);
                    // Track the binding (flow-sensitive overwrite, №323
                    // LetBinding/Assign contract) so later sink calls see
                    // the carried label.
                    let label = sink_arg_label(value, env, origin_labels);
                    env.insert(name.clone(), label);
                    // ── Naryad #387: the FlowCtx update ──
                    // (a) alias tracking: a let-bound copy of a media
                    // handle keeps its origin kind (the alias-invariance
                    // contract — taint/consent/provenance never detach
                    // through let-chains);
                    // (b) token presence: a bound `likeness_verify`
                    // result marks the scope as likeness-cleared.
                    if let Expr::FnCall { name: callee, .. } = value {
                        if callee == "likeness_verify" {
                            ctx.token_bound = true;
                        }
                    }
                    match value {
                        Expr::HandleSource { origin, .. } => {
                            match ctx.origin_kind_map.get(origin).cloned() {
                                Some(kind) => {
                                    ctx.origin_kinds.insert(name.clone(), kind);
                                }
                                None => {
                                    ctx.origin_kinds.remove(name);
                                }
                            }
                        }
                        Expr::ProvBind { origin, .. } => {
                            match ctx.origin_kind_map.get(origin).cloned() {
                                Some(kind) => {
                                    ctx.origin_kinds.insert(name.clone(), kind);
                                }
                                None => {
                                    ctx.origin_kinds.remove(name);
                                }
                            }
                        }
                        Expr::Ident { name: src_name, .. } => {
                            match ctx.origin_kinds.get(src_name).cloned() {
                                Some(kind) => {
                                    ctx.origin_kinds.insert(name.clone(), kind);
                                }
                                None => {
                                    ctx.origin_kinds.remove(name);
                                }
                            }
                        }
                        _ => {
                            ctx.origin_kinds.remove(name);
                        }
                    }
                    // ── №729: the read-back domain tracking — a value
                    // from a paired Source (a direct call) or a
                    // domain-carrying chain (let / field / `to_string`)
                    // keeps the domain; ANY other binding drops it
                    // (fail-closed, the №387 alias posture).
                    {
                        let domain = if let Expr::FnCall { name: callee, .. } = value {
                            READBACK_PAIRS
                                .iter()
                                .find(|p| p.source == callee.as_str())
                                .map(|p| p.domain.to_string())
                                .or_else(|| readback_domain(value, ctx))
                        } else {
                            readback_domain(value, ctx)
                        };
                        match domain {
                            Some(d) => {
                                ctx.readback_domains.insert(name.clone(), d);
                            }
                            None => {
                                ctx.readback_domains.remove(name);
                            }
                        }
                    }
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                    ..
                }
                | Statement::EachWithIndex {
                    item_var: variable,
                    iterable,
                    body,
                    ..
                } => {
                    walk_expr(
                        iterable,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    // The iterant is bound for the body only (scope-local,
                    // №323 Each contract) — analyze the body with a fork.
                    let mut env_body = env.clone();
                    env_body.insert(
                        variable.clone(),
                        sink_arg_label(iterable, env, origin_labels),
                    );
                    let mut ctx_body = ctx.clone();
                    match media_arg_origin_kind(iterable, ctx) {
                        Some(kind) => {
                            ctx_body.origin_kinds.insert(variable.clone(), kind);
                        }
                        None => {
                            ctx_body.origin_kinds.remove(variable);
                        }
                    }
                    // ── №729: the iterant carries the paired Source's
                    // read-back domain (`each cj in cron_list()` → `cj`
                    // is the cron domain for the body scope; the fork is
                    // scope-local — the domain does not escape the loop).
                    // №532: if-let, not a wildcard match — a new Expr
                    // shape rides the fail-closed default (no domain).
                    {
                        let domain = if let Expr::FnCall { name: callee, .. } = iterable {
                            READBACK_PAIRS
                                .iter()
                                .find(|p| p.source == callee.as_str())
                                .map(|p| p.domain.to_string())
                        } else {
                            readback_domain(iterable, ctx)
                        };
                        match domain {
                            Some(d) => {
                                ctx_body.readback_domains.insert(variable.clone(), d);
                            }
                            None => {
                                ctx_body.readback_domains.remove(variable);
                            }
                        }
                    }
                    walk_stmts(
                        body,
                        container,
                        &mut env_body,
                        origin_labels,
                        &mut ctx_body,
                        check,
                        violations,
                    );
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk_expr(
                        condition,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    // Conservative single-pass join (ADR-0154 §8 D1): the
                    // body may raise labels; fold the raises back.
                    let mut env_body = env.clone();
                    // №387: the ctx fork is discarded (fail-closed) —
                    // loop-bound tokens/aliases do not escape the loop.
                    let mut ctx_body = ctx.clone();
                    walk_stmts(
                        body,
                        container,
                        &mut env_body,
                        origin_labels,
                        &mut ctx_body,
                        check,
                        violations,
                    );
                    for (k, v) in env_body {
                        let merged = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, merged);
                    }
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk_expr(
                        condition,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    // Branches fork from the entry env; merge on exit
                    // (componentwise, №323 rules). A closed merge (else
                    // present) drops the pre-branch value; an open merge
                    // keeps the entry env in the join.
                    let mut env_then = env.clone();
                    // №387: branch forks are conservative — branch-bound
                    // tokens/aliases do not escape (the entry ctx wins).
                    let mut ctx_then = ctx.clone();
                    walk_stmts(
                        then_body,
                        container,
                        &mut env_then,
                        origin_labels,
                        &mut ctx_then,
                        check,
                        violations,
                    );
                    let mut merged = env_then;
                    for (_, b) in else_ifs {
                        let mut env_b = env.clone();
                        let mut ctx_b = ctx.clone();
                        walk_stmts(
                            b,
                            container,
                            &mut env_b,
                            origin_labels,
                            &mut ctx_b,
                            check,
                            violations,
                        );
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut ctx_e = ctx.clone();
                        walk_stmts(
                            eb,
                            container,
                            &mut env_e,
                            origin_labels,
                            &mut ctx_e,
                            check,
                            violations,
                        );
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        // Open merge: the entry env survives.
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    walk_expr(
                        condition,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    let mut env_t = env.clone();
                    // №387: conservative fork (branch-local ctx discarded).
                    let mut ctx_t = ctx.clone();
                    walk_stmts(
                        body,
                        container,
                        &mut env_t,
                        origin_labels,
                        &mut ctx_t,
                        check,
                        violations,
                    );
                    for (k, v) in env_t {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                    walk_expr(value, container, env, origin_labels, ctx, check, violations);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk_expr(
                        scrutinee,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    let mut merged = env.clone();
                    for arm in arms {
                        let body = match arm {
                            MatchArm::Exact(_, b)
                            | MatchArm::StartsWith(_, b)
                            | MatchArm::Contains(_, b)
                            | MatchArm::Compare(_, _, b) => b,
                        };
                        let mut env_b = env.clone();
                        // №387: match-arm ctx forks are conservative.
                        let mut ctx_b = ctx.clone();
                        walk_stmts(
                            body,
                            container,
                            &mut env_b,
                            origin_labels,
                            &mut ctx_b,
                            check,
                            violations,
                        );
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut ctx_e = ctx.clone();
                        walk_stmts(
                            eb,
                            container,
                            &mut env_e,
                            origin_labels,
                            &mut ctx_e,
                            check,
                            violations,
                        );
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::Memorize(m) => walk_expr(
                    &m.value,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                ),
                Statement::Forget(f) => walk_expr(
                    &f.query,
                    container,
                    env,
                    origin_labels,
                    ctx,
                    check,
                    violations,
                ),
                Statement::Relate(r) => {
                    walk_expr(
                        &r.from,
                        container,
                        env,
                        origin_labels,
                        ctx,
                        check,
                        violations,
                    );
                    walk_expr(&r.to, container, env, origin_labels, ctx, check, violations);
                }
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let walk_container = |stmts: &[Statement],
                          container: &str,
                          params: &[crate::ast::Param],
                          violations: &mut Vec<SinkViolation>| {
        let mut env = seed.clone();
        for p in params {
            let l = match &p.label {
                Some(ann) => Label::parse(&ann.raw).unwrap_or_default(),
                None => Label::bottom(),
            };
            env.insert(p.name.clone(), l);
        }
        // №387: a fresh FlowCtx per container — the origin-kind
        // vocabulary rides inside (nested fns cannot capture locals).
        // №349: the duty-profile compile flags resolve ONCE per program
        // and ride the same ctx (the carrier is `profile duty`, №348).
        let mut ctx = FlowCtx {
            origin_kind_map: origin_kind_map.clone(),
            duty_materialization_denied,
            duty_surfaces_local_only,
            ..FlowCtx::default()
        };
        walk_stmts(
            stmts,
            container,
            &mut env,
            &origin_labels,
            &mut ctx,
            &check_calls,
            violations,
        );
    };

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_container(
                    &p.body,
                    &format!("pattern {}", p.name),
                    &p.params,
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_container(
                        &m.body,
                        &format!("tool {}.{}", t.name, m.name),
                        &m.params,
                        &mut violations,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_container(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        &[],
                        &mut violations,
                    );
                }
            }
            Declaration::Hook(h) => {
                walk_container(
                    &h.body,
                    &format!("hook {:?}", h.phase),
                    &[],
                    &mut violations,
                );
            }
            Declaration::Test(t) => {
                walk_container(
                    &t.body,
                    &format!("test \"{}\"", t.name),
                    &[],
                    &mut violations,
                );
            }
            _ => {}
        }
    }

    violations
}

// ── DenyEvent: handler scope + exhaustive matching (Наряд №392) ─────

/// The deny-event analyzer pass (№392 §1-3):
///
/// 1. Class validation — every `on_deny(...)` selector must be `*` or a
///    known sink-class word; duplicates are loud (two handlers for one
///    class is almost certainly a copy-paste mistake, and the silent
///    last-wins resolution would hide it).
/// 2. Handler scope — `deny_event()` / `deny_reason()` are the only
///    language surface of the runtime-constructed `DenyEvent`; outside an
///    `on_deny` body they are a compile error (the event cannot be
///    forged into existence — the runtime is the only constructor).
/// 3. Exhaustive matching — a `match` over `deny_reason()` inside a
///    handler is checked for completeness over the deny-reason enum
///    (`crate::deny::DENY_REASONS`); an incomplete match without an
///    `else` arm is a compile error listing every unhandled reason, and
///    a match arm naming an unknown reason is a compile error (a typo
///    can never silently match nothing).
///
/// Messages carry the `[DENY_` prefix — the run path (`run_program`)
/// blocks on it the way it blocks on distill_to errors (ADR-0117 §2-3
/// precedent).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn check_deny_events(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    // (1) class validation + duplicate detection.
    let mut seen_classes: HashMap<&str, u32> = HashMap::new();
    for decl in declarations {
        if let Declaration::OnDeny(d) = decl {
            if !crate::deny::is_valid_class(&d.class) {
                errors.push(SpannedError::at(
                    format!(
                        "[DENY_CLASS] unknown on_deny class '{}' — expected '*' or one of: {} (Naryad #392)",
                        d.class,
                        crate::deny::SINK_CLASSES.join(", ")
                    ),
                    d.span.clone(),
                ));
                continue;
            }
            if seen_classes
                .insert(d.class.as_str(), d.span.start_line)
                .is_some()
            {
                errors.push(SpannedError::at(
                    format!(
                        "[DENY_CLASS] duplicate on_deny handler for class '{}' — the last one would silently shadow this one (Naryad #392)",
                        d.class
                    ),
                    d.span.clone(),
                ));
            }
        }
    }

    // (2)+(3) walk every declaration: statements inside an on_deny body
    // run with in_handler = true, everything else with false.
    for decl in declarations {
        match decl {
            Declaration::OnDeny(d) => {
                walk_stmts_for_deny(&d.body, true, errors);
            }
            Declaration::Pattern(p) => walk_stmts_for_deny(&p.body, false, errors),
            // Learnable patterns are prompt templates — no statement body
            // to walk (the LLM call is the body).
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_stmts_for_deny(&m.body, false, errors);
                }
            }
            Declaration::Test(t) => walk_stmts_for_deny(&t.body, false, errors),
            Declaration::Hook(h) => walk_stmts_for_deny(&h.body, false, errors),
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_stmts_for_deny(&r.body, false, errors);
                }
            }
            Declaration::Flow(f) => {
                // Flow steps are declarative; pattern references resolve
                // at runtime — no deny builtins can hide in them.
                let _ = f;
            }
            _ => {}
        }
    }
}

/// Walk a statement list for the №392 deny checks (scope + exhaustive
/// matching), recursing into every nested statement body.
fn walk_stmts_for_deny(stmts: &[Statement], in_handler: bool, errors: &mut Vec<SpannedError>) {
    for stmt in stmts {
        walk_stmt_for_deny(stmt, in_handler, errors);
    }
}

fn walk_stmt_for_deny(stmt: &Statement, in_handler: bool, errors: &mut Vec<SpannedError>) {
    match stmt {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            walk_expr_for_deny(value, in_handler, errors);
        }
        Statement::Each { iterable, body, .. }
        | Statement::EachWithIndex { iterable, body, .. } => {
            walk_expr_for_deny(iterable, in_handler, errors);
            walk_stmts_for_deny(body, in_handler, errors);
        }
        Statement::While {
            condition, body, ..
        } => {
            walk_expr_for_deny(condition, in_handler, errors);
            walk_stmts_for_deny(body, in_handler, errors);
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_deny(condition, in_handler, errors);
            walk_stmts_for_deny(then_body, in_handler, errors);
            for (cond, body) in else_ifs {
                walk_expr_for_deny(cond, in_handler, errors);
                walk_stmts_for_deny(body, in_handler, errors);
            }
            if let Some(else_body) = else_body {
                walk_stmts_for_deny(else_body, in_handler, errors);
            }
        }
        Statement::IfThen {
            condition, body, ..
        } => {
            walk_expr_for_deny(condition, in_handler, errors);
            walk_stmts_for_deny(body, in_handler, errors);
        }
        Statement::Return { value, .. } => {
            walk_expr_for_deny(value, in_handler, errors);
        }
        Statement::ExprStmt { expr, .. } => {
            walk_expr_for_deny(expr, in_handler, errors);
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_deny(scrutinee, in_handler, errors);
            check_deny_match_exhaustiveness(scrutinee, arms, else_body.is_some(), stmt, errors);
            for arm in arms {
                walk_stmts_for_deny(arm.body(), in_handler, errors);
            }
            if let Some(else_body) = else_body {
                walk_stmts_for_deny(else_body, in_handler, errors);
            }
        }
        Statement::Memorize(m) => walk_expr_for_deny(&m.value, in_handler, errors),
        Statement::Forget(f) => walk_expr_for_deny(&f.query, in_handler, errors),
        Statement::Relate(r) => {
            walk_expr_for_deny(&r.from, in_handler, errors);
            walk_expr_for_deny(&r.to, in_handler, errors);
        }
        Statement::Break | Statement::Continue => {}
    }
}

/// (3) Exhaustive matching over the deny-reason enum: a Match whose
/// scrutinee is a bare `deny_reason()` call inside an on_deny handler.
/// An arm naming an unknown reason is a compile error; a match without
/// an `else` arm that does not cover every reason is a compile error
/// listing exactly the unhandled reasons.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn check_deny_match_exhaustiveness(
    scrutinee: &Expr,
    arms: &[crate::ast::MatchArm],
    has_else: bool,
    stmt: &Statement,
    errors: &mut Vec<SpannedError>,
) {
    let is_deny_reason_scrutinee = match scrutinee {
        Expr::FnCall { name, args, .. } => name == "deny_reason" && args.is_empty(),
        _ => false,
    };
    if !is_deny_reason_scrutinee {
        return;
    }
    let mut covered: Vec<String> = Vec::new();
    for arm in arms {
        if let crate::ast::MatchArm::Exact(reason, _) = arm {
            if !crate::deny::is_known_reason(reason) {
                errors.push(SpannedError::at_expr(
                    scrutinee,
                    format!(
                        "[DENY_MATCH_UNKNOWN] match over deny_reason() names '{}' — not a deny reason; known reasons: {} (Naryad #392)",
                        reason,
                        crate::deny::DENY_REASONS.join(", ")
                    ),
                ));
            }
            covered.push(reason.clone());
        }
        // starts_with / contains / compare arms on a reason word are
        // honest wildcards-in-parts: they may match unpredictably, so
        // they disable the exhaustiveness guarantee entirely unless an
        // else arm exists.
        if !matches!(arm, crate::ast::MatchArm::Exact(_, _)) {
            return;
        }
    }
    if has_else {
        return;
    }
    let uncovered = crate::deny::uncovered_reasons(&covered);
    if !uncovered.is_empty() {
        errors.push(SpannedError::at(
            format!(
                "[DENY_MATCH_EXHAUSTIVE] match over deny_reason() is not exhaustive — unhandled deny reasons: {} (add arms or an else arm) (Naryad #392)",
                uncovered.join(", ")
            ),
            stmt.span().clone(),
        ));
    }
}

/// (2) Handler scope: deny_event()/deny_reason() outside an on_deny body
/// are a compile error; inside, recursion continues (a handler may call
/// helpers? no — DenyEvent is handler-scoped, not importable, so nested
/// calls inside expressions of the handler body are fine but a pattern
/// called FROM the handler cannot read the event — the runtime gate
/// enforces the same rule loudly).
fn walk_expr_for_deny(expr: &Expr, in_handler: bool, errors: &mut Vec<SpannedError>) {
    match expr {
        Expr::FnCall { name, args, .. } => {
            if (name == "deny_event" || name == "deny_reason") && !in_handler {
                errors.push(SpannedError::at_expr(
                    expr,
                    format!(
                        "[DENY_HANDLER_SCOPE] {}() is only available inside an on_deny handler — the DenyEvent is runtime-constructed, it cannot be forged (Naryad #392)",
                        name
                    ),
                ));
            }
            for a in args {
                walk_expr_for_deny(a, in_handler, errors);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                walk_expr_for_deny(a, in_handler, errors);
            }
        }
        Expr::FieldAccess { object, .. } => {
            walk_expr_for_deny(object, in_handler, errors);
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr_for_deny(left, in_handler, errors);
            walk_expr_for_deny(right, in_handler, errors);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr_for_deny(condition, in_handler, errors);
            walk_expr_for_deny(then_branch, in_handler, errors);
            walk_expr_for_deny(else_branch, in_handler, errors);
        }
        Expr::List { items, .. } => {
            for i in items {
                walk_expr_for_deny(i, in_handler, errors);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            walk_expr_for_deny(object, in_handler, errors);
            walk_expr_for_deny(index, in_handler, errors);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_expr_for_deny(v, in_handler, errors);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_deny(condition, in_handler, errors);
            walk_stmts_for_deny(then_body, in_handler, errors);
            for (cond, body) in else_ifs {
                walk_expr_for_deny(cond, in_handler, errors);
                walk_stmts_for_deny(body, in_handler, errors);
            }
            if let Some(else_body) = else_body {
                walk_stmts_for_deny(else_body, in_handler, errors);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_deny(scrutinee, in_handler, errors);
            for arm in arms {
                walk_stmts_for_deny(arm.body(), in_handler, errors);
            }
            if let Some(else_body) = else_body {
                walk_stmts_for_deny(else_body, in_handler, errors);
            }
        }
        Expr::Try { expr: inner, .. } => {
            walk_expr_for_deny(inner, in_handler, errors);
        }
        Expr::HandleSource { .. } | Expr::ProvBind { .. } => {}
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
}

// ── Integrity: anti-injection decision gate (Наряд №327) ─────────────

/// One control-flow decision point whose deciding expression carries an
/// untrusted label (integrity axis — ADR-0154 §2.2).
#[derive(Debug, Clone)]
pub struct DecisionViolation {
    pub container: String,
    /// `"if" | "while" | "match"` — the kind of the decision point.
    pub kind: String,
    pub span: Span,
    /// The deciding expression's label.
    pub label: Label,
    /// The name of the untrusted source, when the deciding expression is
    /// a direct Source call; `<derived>` otherwise.
    pub source_name: String,
}

/// Name of the untrusted source behind a deciding expression: a direct
/// call of a №316 Source builtin names itself; everything else is
/// derived (the join poisoned the integrity — the exact origin is not
/// tracked at this precision; the boundary is documented).
fn decision_source_name(expr: &Expr, prov: &BTreeMap<String, String>) -> String {
    // Descend into the deciding expression to name the direct Source
    // call behind the untrusted label (the naryad: the message names
    // the untrusted SOURCE and the decision point). Variables bound to
    // a Source call carry their provenance through the bindings map.
    if let Expr::FnCall { name, .. } = expr {
        if matches!(
            classify(name).map(|c| c.role),
            Some(crate::builtins_classification::Role::Source)
        ) {
            return name.clone();
        }
    }
    if let Expr::Ident { name, .. } = expr {
        if let Some(origin) = prov.get(name) {
            return origin.clone();
        }
    }
    for sub in expr_operands(expr) {
        let found = decision_source_name(sub, prov);
        if found != "<derived>" {
            return found;
        }
    }
    "<derived>".to_string()
}

/// Direct sub-expressions of `expr` (one level).
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn expr_operands(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::BinaryOp { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => vec![then_branch.as_ref(), else_branch.as_ref()],
        Expr::FieldAccess { object, .. } => vec![object.as_ref()],
        Expr::IndexAccess { object, index, .. } => vec![object.as_ref(), index.as_ref()],
        Expr::Try { expr, .. } => vec![expr.as_ref()],
        Expr::List { items, .. } => items.iter().collect(),
        Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => args.iter().collect(),
        _ => vec![],
    }
}

/// The №327 anti-injection rule: data that DECIDES control flow must be
/// `trusted`. Untrusted data as DATA is legal — the gate fires only on
/// decision positions: `if`/`else if` conditions, `while` conditions,
/// `match` scrutinees. (Sink-target decisions are the №325 classes:
/// UNTRUSTED_EXEC_DECISION / UNTRUSTED_EGRESS_NETWORK.)
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn integrity_decision_violations(declarations: &[Declaration]) -> Vec<DecisionViolation> {
    let mut violations = Vec::new();

    // Seed environment mirrors the sink-clearance pass (№322 annotations
    // on params; entity initializers via the marker lексикон; otherwise
    // bottom).
    let mut seed: BTreeMap<String, Label> = BTreeMap::new();
    for decl in declarations {
        match decl {
            Declaration::EntitySimple(e) => {
                seed.insert(e.name.clone(), entity_seed_label(Some(&e.value)));
            }
            Declaration::EntityRecord(e) => {
                let strongest = e
                    .fields
                    .iter()
                    .map(|f| entity_seed_label(Some(&f.value)))
                    .fold(Label::bottom(), |a, b| a.join(&b));
                seed.insert(e.name.clone(), strongest);
            }
            _ => {}
        }
    }

    fn check_decision(
        expr: &Expr,
        kind: &str,
        container: &str,
        env: &BTreeMap<String, Label>,
        prov: &BTreeMap<String, String>,
        violations: &mut Vec<DecisionViolation>,
    ) {
        let label = sink_arg_label(expr, env, &BTreeMap::new());
        if label.integrity == crate::labels::Integrity::Untrusted {
            violations.push(DecisionViolation {
                container: container.to_string(),
                kind: kind.to_string(),
                span: expr.span().clone(),
                label,
                source_name: decision_source_name(expr, prov),
            });
        }
    }

    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        env: &mut BTreeMap<String, Label>,
        prov: &mut BTreeMap<String, String>,
        violations: &mut Vec<DecisionViolation>,
    ) {
        // Provenance: a variable bound to a DIRECT Source call keeps the
        // call's name; bindings derived from such a variable inherit it.
        #[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
        fn prov_of(value: &Expr, prov: &BTreeMap<String, String>) -> Option<String> {
            match value {
                Expr::FnCall { name, .. } => {
                    if matches!(
                        classify(name).map(|c| c.role),
                        Some(crate::builtins_classification::Role::Source)
                    ) {
                        return Some(name.clone());
                    }
                    // A wrapper call derives from its args' provenance.
                    for a in args_of(value) {
                        if let Some(o) = prov_of(a, prov) {
                            return Some(format!("{o} (via {name})"));
                        }
                    }
                    None
                }
                Expr::Ident { name, .. } => prov.get(name).cloned(),
                Expr::BinaryOp { left, right, .. } => {
                    prov_of(left, prov).or_else(|| prov_of(right, prov))
                }
                Expr::FieldAccess { object, .. } => {
                    if let Expr::Ident { name, .. } = object.as_ref() {
                        return prov.get(name).cloned();
                    }
                    prov_of(object, prov)
                }
                _ => None,
            }
        }
        #[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
        fn args_of(value: &Expr) -> Vec<&Expr> {
            match value {
                Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => {
                    args.iter().collect()
                }
                _ => vec![],
            }
        }
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { name, value, .. }
                | Statement::Assign { name, value, .. } => {
                    // №327 integrity pass: origin labels are Trusted, so
                    // the empty origins map is sound here (integrity is
                    // unaffected by the conf-bearing origin labels).
                    let label = sink_arg_label(value, env, &BTreeMap::new());
                    env.insert(name.clone(), label);
                    if let Some(origin) = prov_of(value, prov) {
                        prov.insert(name.clone(), origin);
                    } else {
                        prov.remove(name);
                    }
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                    ..
                }
                | Statement::EachWithIndex {
                    item_var: variable,
                    iterable,
                    body,
                    ..
                } => {
                    let it = sink_arg_label(iterable, env, &BTreeMap::new());
                    let mut env_body = env.clone();
                    env_body.insert(variable.clone(), it);
                    let mut prov_body = prov.clone();
                    if let Some(origin) = prov_of(iterable, prov) {
                        prov_body.insert(variable.clone(), origin);
                    }
                    walk_stmts(body, container, &mut env_body, &mut prov_body, violations);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    check_decision(condition, "while", container, env, prov, violations);
                    let mut env_body = env.clone();
                    let mut prov_body = prov.clone();
                    walk_stmts(body, container, &mut env_body, &mut prov_body, violations);
                    for (k, v) in env_body {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    check_decision(condition, "if", container, env, prov, violations);
                    let mut env_then = env.clone();
                    let mut prov_then = prov.clone();
                    walk_stmts(
                        then_body,
                        container,
                        &mut env_then,
                        &mut prov_then,
                        violations,
                    );
                    let mut merged = env_then;
                    for (cond, b) in else_ifs {
                        check_decision(cond, "if", container, env, prov, violations);
                        let mut env_b = env.clone();
                        let mut prov_b = prov.clone();
                        walk_stmts(b, container, &mut env_b, &mut prov_b, violations);
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut prov_e = prov.clone();
                        walk_stmts(eb, container, &mut env_e, &mut prov_e, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    check_decision(condition, "if", container, env, prov, violations);
                    let mut env_t = env.clone();
                    let mut prov_t = prov.clone();
                    walk_stmts(body, container, &mut env_t, &mut prov_t, violations);
                    for (k, v) in env_t {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::Return { .. } | Statement::ExprStmt { .. } => {}
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    check_decision(scrutinee, "match", container, env, prov, violations);
                    let mut merged = env.clone();
                    for arm in arms {
                        let b = match arm {
                            MatchArm::Exact(_, b)
                            | MatchArm::StartsWith(_, b)
                            | MatchArm::Contains(_, b)
                            | MatchArm::Compare(_, _, b) => b,
                        };
                        let mut env_b = env.clone();
                        let mut prov_b = prov.clone();
                        walk_stmts(b, container, &mut env_b, &mut prov_b, violations);
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut prov_e = prov.clone();
                        walk_stmts(eb, container, &mut env_e, &mut prov_e, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let walk_container = |stmts: &[Statement],
                          container: &str,
                          params: &[crate::ast::Param],
                          violations: &mut Vec<DecisionViolation>| {
        let mut env = seed.clone();
        let mut prov: BTreeMap<String, String> = BTreeMap::new();
        for p in params {
            let l = match &p.label {
                Some(ann) => Label::parse(&ann.raw).unwrap_or_default(),
                None => Label::bottom(),
            };
            env.insert(p.name.clone(), l);
        }
        walk_stmts(stmts, container, &mut env, &mut prov, violations);
    };

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_container(
                    &p.body,
                    &format!("pattern {}", p.name),
                    &p.params,
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_container(
                        &m.body,
                        &format!("tool {}.{}", t.name, m.name),
                        &m.params,
                        &mut violations,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_container(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        &[],
                        &mut violations,
                    );
                }
            }
            Declaration::Hook(h) => {
                walk_container(
                    &h.body,
                    &format!("hook {:?}", h.phase),
                    &[],
                    &mut violations,
                );
            }
            Declaration::Test(t) => {
                walk_container(
                    &t.body,
                    &format!("test \"{}\"", t.name),
                    &[],
                    &mut violations,
                );
            }
            _ => {}
        }
    }

    violations
}

/// №611 (P0 bugfix): the check-time DDL dry-run for schema-as-code
/// declarations. For every `schema { … }` block, each table's DDL is
/// rendered by the SSOT renderer (`ast::schema_table_ddl` — the same
/// function the interpreter apply path executes) and executed on an
/// in-memory SQLite connection. Any statement SQLite rejects becomes a
/// LOUD check error carrying the schema/table spans and the rendered
/// DDL — a broken schema now fails at `mlog check`, never as an
/// apply-time `SQL_ERROR` surprise (and never as a silent degradation,
/// the №611 references-loss class). No schema declarations → no
/// connection is opened at all (the zero-cost path for every program
/// without schema-as-code).
fn schema_ddl_dryrun(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    let has_schema = declarations
        .iter()
        .any(|d| matches!(d, Declaration::Schema(_)));
    if !has_schema {
        return;
    }
    let conn = match rusqlite::Connection::open_in_memory() {
        Ok(c) => c,
        Err(e) => {
            errors.push(SpannedError::at_line(
                format!(
                    "schema DDL dry-run: cannot open the in-memory SQLite connection: {}",
                    e
                ),
                0,
            ));
            return;
        }
    };
    for decl in declarations {
        if let Declaration::Schema(schema) = decl {
            for table in &schema.tables {
                let ddl = crate::ast::schema_table_ddl(table);
                if let Err(e) = conn.execute_batch(&ddl) {
                    errors.push(SpannedError::at(
                        format!(
                            "schema '{}' — the generated DDL for table '{}' is rejected by SQLite (check dry-run): {} [DDL: {}]",
                            schema.name, table.name, e, ddl
                        ),
                        table.span.clone(),
                    ));
                }
            }
        }
    }
}

/// Perform semantic analysis on a list of declarations (without executing them).
/// Validates:
///   - Entity types referenced in records exist
///   - Field initializers reference valid fields
///   - Patterns/learnables invoked in flows exist
///   - Flow branch targets are known patterns
///   - Duplicate entity/pattern/flow names
///   - Rule targets reference existing entities
///   - Adapt/mutate targets reference existing learnable patterns
///   - Relate/sandbox declarations are well-formed
///   - MlogServer middleware names are valid (Phase 6.1)
///   - Route methods are valid HTTP methods (Phase 6.1)
///   - Template return type is Html (Phase 6.2)
///   - Opaque types used in correct contexts (Phase 6.2–6.5)
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
pub fn check_program(declarations: &[Declaration]) -> AnalysisResult {
    // №765 (gh#1418, Wave 59 — the audit S-1b): the raw-position map is
    // seeded here too — the position-aware render arm in expr_label
    // reads it on the CHECK path (the audit entries seed for their own
    // path; re-seeding overwrites — each invocation is self-contained,
    // the seed_render_raw contract).
    crate::html_label::seed_render_raw(declarations);
    let mut result = AnalysisResult::default();
    let mut entity_types: HashSet<String> = HashSet::new();
    let mut entity_names: HashSet<String> = HashSet::new();
    let mut pattern_names: HashSet<String> = HashSet::new();
    let mut learnable_names: HashSet<String> = HashSet::new();
    let mut flow_names: HashSet<String> = HashSet::new();
    // Наряд №238 (Vision R4.1): vision declaration names (duplicate check).
    let mut vision_names: HashSet<String> = HashSet::new();
    let builtin_names = crate::builtins::builtin_name_set();
    let mut role_names: HashSet<String> = HashSet::new();
    let mut pattern_param_counts: HashSet<(String, usize)> = HashSet::new();
    // №617 (ledger gh#967 §3): entity names carrying a DECLARED opaque
    // type (Html/Query/Secret/Encrypted/Hash/Session) — the static fact
    // for the opaque-concat check. TW seeds ALL entities into
    // `self.variables` before any pattern body runs (execution.rs:77/88),
    // so the map is built over the WHOLE declaration list regardless of
    // source order — same lookup semantics the runtime Ident read has.
    let mut entity_opaque: HashMap<String, String> = HashMap::new();
    // №662 (ADR-0188 option (б), the owner's verdict gh#1123): the
    // template names (the №250 registration set — the same declarations
    // the compiler registers) and the pattern param types (RESOLVED
    // through the №119 alias map) — the two facts the (б) checker rules
    // read: the unknown-template refusal and the argument-position
    // coercion refusal.
    let mut template_names: HashSet<String> = HashSet::new();
    // №685 (gh#1252, В44): template name → the RESOLVED param type names
    // (positional, the render() argument order) — the argument-position
    // coercion rule for render() reads it (the №662 rule-1 twin for
    // templates: a String-shaped argument into an Html-typed parameter is
    // the implicit coercion the runtime accepts silently).
    let mut template_param_types_resolved: HashMap<String, Vec<String>> = HashMap::new();
    let mut pattern_param_types_resolved: HashMap<String, Vec<String>> = HashMap::new();
    // Наряд №181 (ADR-0117): reflex declarations — name → labels set.
    // Used to validate distill_to references and labels-non-empty rule.
    let mut reflex_decls: HashMap<String, Vec<String>> = HashMap::new();

    // Наряд №119: build type alias map and collect errors
    let (type_alias_map, alias_errors) = build_type_alias_map(declarations);
    for e in alias_errors {
        // build_type_alias_map returns Vec<String> (defined in ast.rs,
        // also consumed by interpreter/execution.rs). The errors don't
        // carry a span — `SpannedError::at_line(e, 0)` produces a
        // Span::unknown() which the LSP layer maps to line 0 (the
        // previous hardcoded fallback behavior).
        result.errors.push(SpannedError::at_line(e, 0));
    }
    let alias_names: HashSet<String> = type_alias_map.keys().cloned().collect();

    // Наряд №322 (ADR-0154): validate label annotations everywhere the
    // grammar allows them. This is what makes the annotation "visible to
    // semantics" — the label carrier (`ast::LabelAnn`) is parsed with a
    // span, and unknown words fail here, not silently later.
    for decl in declarations {
        validate_decl_labels(decl, &mut result.errors);
    }

    // Наряд №324 (ADR-0154 §9): effect trail in pattern signatures.
    // Validates the trail words (io|audit), computes the factual body
    // effects of every pattern/tool-method/learnable (interprocedural
    // fixpoint over the call graph — converges on recursion), and gates
    // every DECLARED trail: factual ⊑ declared, excess = loud error.
    // Patterns without a declared trail are ungated (zero delta).
    check_effect_trails(declarations, &mut result.errors);

    // №673 (gh#1200, В41): BREAK_OUTSIDE_LOOP — the break/continue edges
    // №658-a (crossing the value channel) and №658-b (outside any loop)
    // were «молча неверные» on the VM (the class-4 dictionary №653): the
    // TW refused at RUNTIME, the VM absorbed the signal (or compiled the
    // statement to nothing). The refusal shifts left to the semantic
    // gate — BEFORE execution, on BOTH backends (compile_program calls
    // the same shared walker). The runtime mirrors stay untouched as
    // backstops.
    result
        .errors
        .extend(break_outside_loop_violations(declarations));

    // №685 (gh#1252, В44): RAW_INSERT_NOT_HTML — the S-1 fail-closed gate:
    // a template parameter used in the raw insert `{{{ param }}}` must be
    // declared Html. The N9 raw form landed the value VERBATIM while every
    // layer answered «sanitized» — the audit's XSS scenario passed
    // check/run/serve silently. The refusal shifts left to the semantic
    // gate — BEFORE execution, on BOTH backends (compile_program calls the
    // same shared walker, the №673 contour shape).
    result
        .errors
        .extend(raw_insert_not_html_violations(declarations));

    // №695 (gh#1274, В46): SAFE_PIPE_NOT_HTML + UNSAFE_PIPE_FORM — the
    // ADR-0188 phase-2 gate: the safe stage `{{ param | safe }}` inserts
    // RAW, so its parameter must be declared Html (the №685 mirror), and
    // every unrecognized pipe form refuses LOUDLY (the N9 refusal
    // shifted left). The refusal is BEFORE execution on BOTH backends
    // (compile_program calls the same shared scan, the №673 contour
    // shape); the registration refusal stays as the runtime backstop.
    result.errors.extend(safe_pipe_violations(declarations));

    // №611 (P0 bugfix): the check-time DDL dry-run for schema-as-code.
    // `mlog check` previously did not validate the generated DDL at all —
    // a schema SQLite rejects (the №611 broken `REFERENCES parent(.)`
    // clause, reserved-word identifiers, …) came back green on check and
    // exploded only at apply time (or was silently degraded — the
    // №611 references loss itself). The dry-run renders each table
    // through the SAME SSOT renderer the interpreter apply uses
    // (ast::schema_table_ddl) and executes it on an in-memory SQLite
    // connection: DDL rejected by SQLite = a check error, never an
    // apply-time surprise.
    schema_ddl_dryrun(declarations, &mut result.errors);

    // Наряд №331 (ADR-0162 §2.5): media handles are opaque — any field
    // access on a media-typed expression is a COMPILE error; bytes are
    // reachable only through the sanctioned materialization sink
    // (media_save, №325-gated).
    check_media_handle_opacity(declarations, &mut result.errors);

    // Наряд №332 (ADR-0164): the origin chain — a media handle without
    // origin is not constructed (§7.4 rule); origin declarations are
    // validated loudly (unknown kinds/fields/labels).
    check_media_origin_chain(declarations, &mut result.errors);

    // Наряд №325 (ADR-0161): validate the compatibility profile shape —
    // unknown profile names / options / egress modes are loud errors
    // (a compat-profile mistake must never be a silent no-op).
    for decl in declarations {
        if let Declaration::Profile(p) = decl {
            if let Err(e) = crate::profile::validate(p) {
                result.errors.push(SpannedError::at(e, p.span.clone()));
            }
        }
    }

    // №801 (gh#1548, the gateway G2): validate the MCP policy — each
    // entry names an UPSTREAM tool (the `upstream.tool` form; a bare
    // name is the local-tool namespace whose policy compiles from the
    // body, №394) and carries a parsable Label annotation (ADR-0154 §3:
    // the conf word required, the integrity defaults trusted). A
    // miscalibrated class must never be a silent no-op — the gateway
    // makes denial decisions FROM these entries.
    for decl in declarations {
        if let Declaration::McpPolicy(p) = decl {
            for entry in &p.entries {
                if !entry.tool.contains('.') {
                    result.errors.push(
                        SpannedError::at(
                            format!(
                                "[MCP_POLICY] tool '{}' must be the UPSTREAM form `upstream.tool` — a bare name is the local-tool namespace (its policy compiles from the body, №394)",
                                entry.tool
                            ),
                            entry.span.clone(),
                        )
                        .with_kind(SemanticErrorKind::McpPolicyInvalid),
                    );
                }
                if let Err(e) = crate::labels::Label::parse(&entry.label) {
                    result.errors.push(
                        SpannedError::at(
                            format!(
                                "[MCP_POLICY] tool '{}': unknown label word '{}' — the label is a Label annotation (the conf word required: public|consented|private|poisoned; the integrity: trusted|untrusted; e.g. \"public, untrusted\")",
                                entry.tool, e.word
                            ),
                            entry.span.clone(),
                        )
                        .with_kind(SemanticErrorKind::McpPolicyInvalid),
                    );
                }
            }
        }
    }

    // Наряд №336 (ADR-0165 §2.4): the BackendSelect ladder companion
    // check — statically-visible ladders are verified against the №333
    // registry SSOT (unknown rung / class mismatch / duplicates), and a
    // `device { mode: production }` profile refuses unverifiable
    // (PendingNo334) rungs at BUILD time.
    check_backend_select_ladders(declarations, &mut result.errors);

    // №839 (ADR-0202 §2.3, wave 71 — W-A): the license-class × profile
    // permit rule — a statically-visible backend selection (a literal
    // ladder rung, a `vision` model field) whose license class the
    // program's profile does not permit is a LOUD refusal before run
    // (the dedicated `LICENSE_CLASS_NOT_ALLOWED` code; the permit
    // predicate is shared with the №333 audit gate).
    for v in license_class_violations(declarations) {
        result.errors.push(SpannedError::at(v.message, v.span));
    }

    // Наряд №392: deny-event handlers — class validation, handler scope
    // of deny_event()/deny_reason(), exhaustive matching over the deny
    // reasons. Messages carry the [DENY_ prefix the run path blocks on.
    check_deny_events(declarations, &mut result.errors);

    // №802 (gh#1549, волна 65, the C6 R-b 2 static check — the ADR-0198
    // queue): the try result and the `T|Unit` union are verified before
    // use. The refusal fires on `mlog check` and blocks the №523 run
    // gate and the serve/mcp-serve startup gates on BOTH backends (the
    // shared semantic pass); the suppression comment
    // `// allow(unchecked_result|unchecked_option): <reason>` is
    // honored by the source-level callers that hold the file text.
    check_unchecked_uses(declarations, &mut result.errors);

    // №581 (the audit d63cc1d X-1) → №584 (X-1 step 2) → №600 (the audit
    // 25b375e Y-1, release-block): the respond-terminality walk. The
    // early-answer surface keeps the style ADVISORY (a bare respond*
    // outside the route's tail chain → WARNING, the recommended form is
    // `return respond(...)`; the route-body compiler lowers it to
    // Call+Return — exact TW parity). The ONE form the lowering does not
    // reach — a bare respond* nested under a top-level block-form if/else
    // branch, discarded by the serve lane on BOTH backends — is a
    // BLOCKING RESPOND_SWALLOWED error again (fail-closed; №584 had
    // retired the №581 error for ALL forms at once, including this
    // non-working one — the audit's guard-bypass regression).
    check_respond_terminality(declarations, &mut result.warnings, &mut result.errors);

    // First pass: collect all declarations (names)
    for decl in declarations {
        match decl {
            Declaration::EntityType(e) => {
                if !entity_types.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity type: {}", e.name),
                    ));
                }
            }
            Declaration::EntityRecord(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity: {}", e.name),
                    ));
                }
            }
            Declaration::EntitySimple(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity: {}", e.name),
                    ));
                }
                // №617: the declared opaque-type fact (resolved through the
                // №119 alias map, the same resolution the runtime coercion
                // applies at execution.rs:85-89).
                let resolved = type_alias_map
                    .get(&e.type_name)
                    .map(|s| s.as_str())
                    .unwrap_or(&e.type_name);
                if crate::ast::is_opaque_type(resolved) {
                    entity_opaque.insert(e.name.clone(), resolved.to_string());
                    // №662 (ADR-0188 option (б), rule 1, the ASSIGNMENT
                    // position): a String-shaped initializer flowing into an
                    // opaque-typed entity is the IMPLICIT coercion the
                    // runtime accepted silently (№114) — the injection
                    // shape reaches the page. The explicit path (an Html
                    // producer — render()) stays legal; the runtime keeps
                    // the backstop for the conditionally-bound cases.
                    if expr_is_string_shaped(&e.value) {
                        result.errors.push(
                            SpannedError::at(
                                format!(
                                    "implicit String→{} coercion in the initializer of entity '{}' — build the value with an explicit {} producer (render)",
                                    resolved, e.name, resolved
                                ),
                                e.span.clone(),
                            )
                            .with_kind(SemanticErrorKind::ImplicitOpaqueCoercion),
                        );
                    }
                }
            }
            Declaration::Pattern(p) => {
                if !pattern_names.insert(p.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate pattern: {}", p.name),
                    ));
                }
                pattern_param_counts.insert((p.name.clone(), p.params.len()));
                // №662: the param types RESOLVED through the alias map —
                // the argument-position coercion rule reads them.
                pattern_param_types_resolved.insert(
                    p.name.clone(),
                    p.params
                        .iter()
                        .map(|pp| {
                            type_alias_map
                                .get(&pp.type_name)
                                .cloned()
                                .unwrap_or_else(|| pp.type_name.clone())
                        })
                        .collect(),
                );
            }
            Declaration::LearnablePattern(lp) => {
                if !learnable_names.insert(lp.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate learnable pattern: {}", lp.name),
                    ));
                }
                pattern_param_counts.insert((lp.name.clone(), lp.params.len()));
            }
            // Наряд №181: collect reflex declarations for cross-reference check.
            Declaration::Reflex(r) => {
                if reflex_decls
                    .insert(r.name.clone(), r.labels.clone())
                    .is_some()
                {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate reflex declaration: {}", r.name),
                    ));
                }
            }
            Declaration::Flow(f) => {
                if !flow_names.insert(f.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate flow: {}", f.name),
                    ));
                }
            }
            // Наряд №238 (Vision R4.1): duplicate vision declaration names
            // are a semantic error (same лекал as reflex/pattern/flow).
            Declaration::Vision(v) => {
                if !vision_names.insert(v.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate vision declaration: {}", v.name),
                    ));
                }
            }
            Declaration::Template(t) => {
                // №662: the template name collected — the №250 registration
                // set (the unknown-template refusal reads it). NOTE: the
                // DUPLICATE-template refusal is NOT added here — the
                // runtime registration is overwrite-idempotent (the №250
                // comment) and a duplicate error would be an unmandated
                // behavior change beyond the (б) letter.
                template_names.insert(t.name.clone());
                // №685: the positional param types (alias-resolved — the
                // same resolution pattern_param_types gets) — the render()
                // argument-position coercion rule reads it.
                template_param_types_resolved.insert(
                    t.name.clone(),
                    t.params
                        .iter()
                        .map(|p| {
                            type_alias_map
                                .get(&p.type_name)
                                .cloned()
                                .unwrap_or_else(|| p.type_name.clone())
                        })
                        .collect(),
                );
                // Templates are also callable as render targets
                pattern_names.insert(t.name.clone());
                if is_opaque_type(&t.return_type) && t.return_type != "Html" {
                    result.errors.push(with_line_prefix(decl, format!(
                        "template '{}' returns opaque type '{}' — only Html is supported as template return type",
                        t.name, t.return_type
                    )));
                }
            }
            _ => {}
        }
    }

    // Second pass: cross-reference validation
    for decl in declarations {
        match decl {
            // Наряд №181 (ADR-0117) Block 2: validate distill_to reference.
            // `distill_to: X` requires:
            //   1. X is declared as `reflex X { ... }` in the same scope
            //   2. If the pattern returns String, the reflex must have a
            //      non-empty `labels` list (closed-label enforcement —
            //      ADR-0117 §3 explicitly excludes free-form generation).
            Declaration::LearnablePattern(lp) => {
                if let Some(distill_target) = &lp.distill_to {
                    match reflex_decls.get(distill_target) {
                        None => {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "learnable pattern '{}': distill_to references '{}' \
                                     which is not declared as `reflex {} {{ ... }}`",
                                    lp.name, distill_target, distill_target
                                ),
                            ));
                        }
                        Some(labels) => {
                            // ADR-0117 §3 enforcement: String-returning patterns
                            // distilling to a reflex with empty labels is rejected
                            // (free-form generation is permanently out of scope).
                            if lp.return_type == "String" && labels.is_empty() {
                                result.errors.push(with_line_prefix(
                                    decl,
                                    format!(
                                        "learnable pattern '{}': distill_to '{}' has empty labels list. \
                                         Free-form text distillation is out of scope (ADR-0117 §3) — \
                                         reflex must declare a closed label set: \
                                         `reflex {} {{ labels: [\"a\", \"b\", ...] ... }}`",
                                        lp.name, distill_target, distill_target
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            Declaration::EntityRecord(e) => {
                if !entity_types.contains(&e.type_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!(
                            "entity '{}' references unknown type '{}'",
                            e.name, e.type_name
                        ),
                    ));
                }
                if let Some(fields) = get_type_fields(declarations, &e.type_name) {
                    for init in &e.fields {
                        if !fields.contains(&init.name.as_str()) {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "entity '{}' initializes unknown field '{}' on type '{}'",
                                    e.name, init.name, e.type_name
                                ),
                            ));
                        }
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let known_primitives = [
                    "String",
                    "Float",
                    "Bool",
                    "Html",
                    "Query",
                    "Secret",
                    "Encrypted",
                    "Hash",
                    "Session",
                ];
                // Наряд №119: resolve type alias before checking
                let resolved = type_alias_map
                    .get(&e.type_name)
                    .map(|s| s.as_str())
                    .unwrap_or(&e.type_name);
                if !known_primitives.contains(&resolved)
                    && !entity_types.contains(resolved)
                    && !alias_names.contains(&e.type_name)
                {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "entity '{}' uses undeclared type '{}' (may be a forward reference)",
                            e.name, e.type_name
                        ),
                    ));
                }
            }
            Declaration::Rule(r) => {
                if let Expr::Ident { name, .. } = &r.target {
                    if !entity_names.contains(name) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!("rule target '{}' references undefined entity", name),
                        ));
                    }
                }
            }
            Declaration::Adapt(a) => {
                if !learnable_names.contains(&a.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("adapt: learnable pattern '{}' not found", a.pattern_name),
                    ));
                }
            }
            Declaration::Mutate(m) => {
                if !learnable_names.contains(&m.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("mutate: learnable pattern '{}' not found", m.pattern_name),
                    ));
                }
                // №826 (ADR-0216): the `rollback_if` metric NAME must be
                // registered — a LOUD refusal at the check, never a silent
                // fallback (the reflex_train convention,
                // `builtins/reflex.rs`). `accuracy` is registered, so the
                // old literal form passes unchanged.
                if let Some(metric) = &m.rollback_metric {
                    if crate::nn::find_metric(metric).is_none() {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "mutate: unknown metric '{}'. Registered metrics: {}",
                                metric,
                                crate::nn::metric_names().join(", ")
                            ),
                        ));
                    }
                }
            }
            Declaration::Eval(e) => {
                if !learnable_names.contains(&e.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("eval: learnable pattern '{}' not found", e.pattern_name),
                    ));
                }
                if e.dataset.is_empty() {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "eval '{}': dataset is empty — eval will trivially pass",
                            e.pattern_name
                        ),
                    ));
                }
            }
            Declaration::Flow(f) => {
                for step in &f.pipeline {
                    let known = pattern_names.contains(step)
                        || learnable_names.contains(step)
                        || builtin_names.contains(step)
                        || step == "recall";
                    if !known {
                        let has_branch_def = f.branch_defs.iter().any(|(name, _)| name == step);
                        if !has_branch_def {
                            result.errors.push(with_line_prefix(decl, format!(
                                "flow '{}': pipeline step '{}' is not a known pattern, builtin, or branch definition",
                                f.name, step
                            )));
                        }
                    }
                }
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        if !pattern_names.contains(&branch.target)
                            && !learnable_names.contains(&branch.target)
                            && !builtin_names.contains(&branch.target)
                        {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "flow '{}': branch '{}' target '{}' is not a known pattern",
                                    f.name, branch.label, branch.target
                                ),
                            ));
                        }
                    }
                }
            }
            Declaration::Pattern(p) => {
                // Walk expression tree: check arity and undefined functions
                for stmt in &p.body {
                    check_stmt_exprs(
                        stmt,
                        &builtin_names,
                        &pattern_param_counts,
                        &learnable_names,
                        &mut result.errors,
                    );
                }
                // Наряд №264: static immutability enforcement — assignment to
                // a variable not bound with `let mut` is an error here, so
                // `mlog check` catches before any backend runs it. Previously
                // the tree-walking interpreter rejected such assignments at
                // RUNTIME (contract since №14, examples/p30_assign_immutable)
                // while `mlog check` passed and the VM compiled the
                // assignment silently — three backends, three answers.
                check_pattern_mutability(&p.body, &mut result.errors);
                // №617 (ledger gh#967 §3): the variable-scope walk — an
                // undefined variable read/assignment and an opaque-concat
                // refuse at check time on BOTH backends (the runtime stays
                // the backstop; the messages are the TW лекала verbatim).
                check_pattern_variables(
                    &p.body,
                    &p.params,
                    &type_alias_map,
                    &entity_opaque,
                    &entity_names,
                    &template_names,
                    &template_param_types_resolved,
                    &pattern_param_types_resolved,
                    &mut result.errors,
                );
            }
            // Phase 6.1: Validate mlogserver block
            Declaration::MlogServer(srv) => {
                // Validate middleware names
                for mw in &srv.middleware {
                    if !VALID_MIDDLEWARE.contains(&mw.as_str()) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "mlogserver: unknown middleware '{}'. Valid: {:?}",
                                mw, VALID_MIDDLEWARE
                            ),
                        ));
                    }
                }
                // Validate route methods and role references
                for route in &srv.routes {
                    if !VALID_METHODS.contains(&route.method.as_str()) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "route '{}': unknown HTTP method '{}'. Valid: {:?}",
                                route.path, route.method, VALID_METHODS
                            ),
                        ));
                    }
                    for role in &route.requires {
                        // Collect role names for cross-reference
                        role_names.insert(role.clone());
                    }
                }
                // Warn if no security_headers middleware
                if !srv.middleware.contains(&"security_headers".to_string()) {
                    result.warnings.push(
                        with_line_prefix(decl, "mlogserver: no 'security_headers' middleware — recommend adding it for OWASP compliance".to_string())
                    );
                }
                // Warn if POST routes but no csrf middleware
                let has_post = srv
                    .routes
                    .iter()
                    .any(|r| r.method == "POST" || r.method == "PUT" || r.method == "DELETE");
                if has_post && !srv.middleware.contains(&"csrf".to_string()) {
                    result.warnings.push(
                        with_line_prefix(decl, "mlogserver: has mutating routes but no 'csrf' middleware — recommend adding it".to_string())
                    );
                }
                // Наряд №264: route bodies get the same immutability walk as
                // pattern bodies — TW executes route bodies through
                // eval_statements (server.rs), so the `let mut` contract
                // applies there identically; the static pass now catches the
                // violation at `mlog check` time for both serve backends.
                // Наряд №523: route bodies get the same CALL walk as pattern
                // bodies too — an undefined function inside a route body was
                // invisible to the semantic pass (the serve startup gate
                // could not see it; only the VM route compiler refused, and
                // only on the VM backend). With the walk in place the serve
                // startup gate refuses on both backends; the same-import
                // caveat as pattern bodies applies (mlog check without
                // --root has never seen imported symbols on either).
                for route in &srv.routes {
                    for stmt in &route.body {
                        check_stmt_exprs(
                            stmt,
                            &builtin_names,
                            &pattern_param_counts,
                            &learnable_names,
                            &mut result.errors,
                        );
                    }
                    check_pattern_mutability(&route.body, &mut result.errors);
                    // №617: the same variable-scope walk for route bodies —
                    // no params (the TW route env starts empty, the entities
                    // resolve as globals, server.rs:2587), the flat let/each
                    // model identical to pattern bodies.
                    check_body_variables(
                        &route.body,
                        &entity_opaque,
                        &entity_names,
                        &template_names,
                        &template_param_types_resolved,
                        &pattern_param_types_resolved,
                        &mut result.errors,
                    );
                }
            }
            // Test declarations: no semantic checks needed (statements inside are checked by patterns)
            Declaration::Test(_) => {}
            // Наряд №238 (Vision R4.1): vision declaration validation
            // (Block 2). Parser already validated the field set and the
            // policy/profile enums — semantic does NOT re-validate enums
            // (наряд Block 1.3), it checks the model SSOT list and the
            // numeric contracts. All errors carry the declaration span and
            // name the field (Block 2.4 — no silent corrections).
            Declaration::Vision(v) => {
                // Block 2.1: model must be in the SSOT list
                // (`crate::vision::KNOWN_VISION_MODELS`, R4.1 = exactly
                // ["z-image-turbo"]).
                if !crate::vision::KNOWN_VISION_MODELS.contains(&v.model.as_str()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!(
                            "vision '{}': unknown model '{}' (known models: {})",
                            v.name,
                            v.model,
                            crate::vision::KNOWN_VISION_MODELS.join(", ")
                        ),
                    ));
                }
                // Block 2.2: steps >= 1. The value 8 is the recommended
                // distilled-NFE count for the z-image-turbo wedge (ADR-0122);
                // steps != 8 is an audit-warning, NOT an error.
                if v.steps < 1 {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("vision '{}': steps must be >= 1, got {}", v.name, v.steps),
                    ));
                } else if v.steps != 8 {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "vision '{}': steps = {} != 8 (recommended distilled-NFE for z-image-turbo)",
                            v.name, v.steps
                        ),
                    ));
                }
                // Block 2.2: width/height — multiples of 16, range
                // 256..=4096 (VAE latent constraint).
                for (field, value) in [("width", v.width), ("height", v.height)] {
                    if value % 16 != 0 || !(256..=4096).contains(&value) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "vision '{}': {} = {} must be a multiple of 16 in range 256..=4096 (VAE latent constraint)",
                                v.name, field, value
                            ),
                        ));
                    }
                }
                // Block 2.2: seed — any u64 is valid (parser guarantees the
                // range); no further check.
            }
            _ => {}
        }
    }

    // ── Наряд №98: Category A audit → compiler errors ──
    // SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION are structural security
    // invariants — a finding is never a legitimate false positive.
    // These run in `mlog check`/`run`/`serve`/`compile`, not just `mlog audit`.
    // Source text is not available here (we only have AST), so we pass ""
    // for line-number resolution; the check_id and message are sufficient.
    let cat_a_findings = audit_category_a(declarations, "");
    for finding in &cat_a_findings {
        match finding.severity {
            Severity::Error => {
                result.errors.push(SpannedError::at_line(
                    format!("[{}] {}", finding.check_id, finding.message),
                    finding.line,
                ));
            }
            Severity::Warning => {
                // check_html_injection currently emits Warning;
                // promote to Error per Наряд №98 Block 1 classification.
                result.errors.push(SpannedError::at_line(
                    format!("[{}] {}", finding.check_id, finding.message),
                    finding.line,
                ));
            }
            Severity::Info => {}
        }
    }

    // ── Нарjad №74: SVG/HTML security lint (ADR-0102) ──
    // AST-level analysis: detect potential injection vectors that could
    // bypass runtime escaping. See `svg_security_lint` docstring below.
    svg_security_lint(declarations, &mut result);

    // №474 (gh#742): stage 1 of the type system — the let-type inference,
    // WARN-ONLY (the №467 canon: "этап 1 (вывод let, warn-only)"). The
    // pass appends to `warnings` only — `errors` are structurally
    // untouched, no program is rejected, the runtime paths are unchanged.
    result
        .warnings
        .extend(crate::semantic_types::stage1_warnings(declarations));
    // №565 (gh#927; the audit 02.10 §7.2 W-3): the respond_html
    // TWO-ARGUMENT form is deprecated — the WARN-ONLY deprecation pass
    // (the №474 posture: warnings only, errors and both runtimes
    // untouched). See the pass docstring below.
    result
        .warnings
        .extend(respond_html_two_arg_warnings(declarations));

    // №799 (T2, gh#1488): the experimental-tier partition — the ONE
    // place. The check walk pushed [BUILTIN_EXPERIMENTAL] findings into
    // `errors` with the BuiltinExperimental kind (the walker threads
    // one vec); here they move to `warnings`: the blocking gates (№523)
    // never see them, `mlog check` stays exit-0, the human output
    // carries the class prefix. The parked kind NEVER partitions — it
    // blocks by construction (the C3 (a) tiers: parked = not callable
    // in this build, experimental = callable with a notice).
    let (experimental, rest): (Vec<_>, Vec<_>) = result
        .errors
        .into_iter()
        .partition(|e| e.kind == SemanticErrorKind::BuiltinExperimental);
    result.warnings.extend(experimental);
    result.errors = rest;

    result
}

// ── Наряд №74: SVG/HTML Security Lint ────────────────────────────────
//
// Walks the AST of every declaration and inspects all `Expr::FnCall`
// nodes. For each call to an SVG/HTML-emitting builtin (svg_text,
// svg_callout, svg_path, svg_canvas, svg_group, chart_*, diagram_*,
// html_response, escape_html), it inspects the string-literal arguments.
//
// Findings:
//
//   ERROR (potential bypass):
//     A string literal containing `<script`, `javascript:`, or `on\w+=`
//     is passed to a builtin that does NOT auto-escape that argument
//     (e.g. svg_path's `d` argument is structural and not escaped).
//     Also: a `<script` literal appearing in any string concatenation
//     that ends up in an HTML context.
//
//   WARNING (suspicious but auto-escaped):
//     A string literal containing `<script>`, `on\w+=`, etc. passed to
//     a builtin that DOES auto-escape (svg_text content, svg_callout
//     text). Runtime will escape it correctly, but the source intent
//     looks like an attempted injection — worth flagging for review.
//
// This is defense-in-depth: runtime escaping (escape_html_chars in
// svg_text, svg_callout) is the primary barrier. The lint catches the
// case where an attacker could bypass escaping by passing the payload
// to a non-escaping argument (svg_path d, svg_canvas viewbox, etc.).
//
// Whitelist: Google Fonts URLs ("https://fonts.googleapis.com/...")
// are explicitly permitted in href/src contexts — the only external
// resource allowed (matches the source repo's self_check.py rule).

/// Builtins whose string arguments are auto-escaped at runtime.
/// String literals with `<script>` here generate a WARNING (suspicious
/// but safe — runtime will escape).
///
/// chart_bar / chart_donut / chart_line / chart_area accept user labels
/// inside `data: List<Struct{label, value}>` at arg 0. Their labels are
/// escaped via escape_html_chars at runtime (defense-in-depth).
/// chart_scatter uses `List<Struct{x, y, label?}>` — same `label` key,
/// but optional and at a different struct position. The walker has a
/// special case (scan_chart_labels) that scans the list-of-structs
/// pattern by field NAME, so it works uniformly across all five shapes.
/// chart_boxplot uses `List<Struct{label, values}>` — same `label` key
/// in a list-of-structs, so scan_chart_labels covers it without changes.
/// chart_radar uses a DIFFERENT top-level shape (Struct{axes, series},
/// not List<Struct>), so it has its own scanner (scan_radar_labels).
/// chart_heatmap is intentionally NOT in this list — its data is purely
/// numeric (List<List<Float>>), there is no user text to scan.
const SVG_AUTO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_text",
    "svg_callout",
    "chart_bar",
    "chart_donut",
    "chart_line",
    "chart_scatter",
    "chart_area",
    "chart_radar",
    "chart_boxplot",
    // Наряд №81 Block 6: diagrams accept user text (label/title/description).
    //   diagram_tree / diagram_org_chart — recursive Struct{label, children}.
    //   diagram_flowchart — Struct{nodes: [{id,label}], edges: [{from,to,label?}]}.
    //   diagram_layers — List<Struct{label, description?}>.
    // All four escape text via escape_html_chars at runtime (svg.rs).
    // AST lint scans label literals as WARNINGs (defense-in-depth).
    "diagram_tree",
    "diagram_org_chart",
    "diagram_flowchart",
    "diagram_layers",
    // Наряд №82 Block 6: temporal & process diagrams.
    //   diagram_sequence  — Struct{actors: List<String>, messages: [{from,to,label?}]}.
    //     actors[] is a List<String> (NOT List<Struct>) — special scanner.
    //     messages[].label is the rendered text (from/to are idents, scanned
    //     defensively).
    //   diagram_timeline  — List<Struct{date, label, description?}> — flat
    //     list pattern, same shape as diagram_layers (3 string fields, not 2).
    //   diagram_gantt     — List<Struct{task, start, duration}> — task is
    //     the only string field; start/duration are floats, never rendered.
    //   diagram_process   — List<Struct{label, description?}> — identical
    //     shape to diagram_layers (reuses scan_layers_labels logic by name).
    //   diagram_loop      — List<Struct{label, description?}> — same shape
    //     as diagram_layers/diagram_process.
    // All five escape text via escape_html_chars at runtime (svg.rs).
    "diagram_sequence",
    "diagram_timeline",
    "diagram_gantt",
    "diagram_process",
    "diagram_loop",
    // Наряд №83 Block 6: sets & comparison diagrams.
    //   diagram_venn     — Struct{circles: [{label, value?}], overlap_label?}.
    //     circles[].label is rendered; overlap_label is a TOP-LEVEL field
    //     (NOT inside the list) — easy to forget, called out explicitly in
    //     the spec. Special scanner scan_venn_labels covers both.
    //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [{label, x, y}]}.
    //     BOTH axis labels are TOP-LEVEL fields (not in items[]) — same
    //     "easy to forget" category as overlap_label. items[].label is the
    //     only rendered text in the list (x/y are floats). Special scanner
    //     scan_quadrant_labels covers all three.
    //   diagram_pyramid  — List<Struct{label, value?}> — same flat shape
    //     as diagram_layers (label rendered, value is float). REUSES
    //     scan_layers_labels (no new scanner for an identical shape).
    //   diagram_nested   — List<Struct{label, value?}> — identical shape
    //     to diagram_pyramid. REUSES scan_layers_labels.
    //   diagram_medallion — List<Struct{icon?, label, value?}>. label is
    //     rendered; icon is a controlled enum (validated against the 10
    //     svg_icon names at runtime), NOT user free-form text — explicitly
    //     NOT scanned per the spec. value is float. New scanner
    //     scan_medallion_labels checks only `label`.
    "diagram_venn",
    "diagram_quadrant",
    "diagram_pyramid",
    "diagram_nested",
    "diagram_medallion",
    // Наряд №84 Block 7: data & state diagrams.
    //   diagram_er — Struct{entities: [{name, fields: List<String>}], relations: [{from,to,label?}]}.
    //     entities[].name and relations[].label are rendered as text.
    //     entities[].fields is a List<String> NESTED INSIDE a struct field —
    //     this is the THIRD nesting form encountered in the SVG suite
    //     (1st: top-level List<String> like diagram_sequence.actors in Н82;
    //      2nd: List<Struct> like diagram_layers in Н81;
    //      3rd: List<String> inside a struct field, here). The scanner
    //     scan_er_labels walks both levels: per-entity name + per-field string.
    //   diagram_state — Struct{states: List<String>, transitions: [{from,to,label?}], initial?}.
    //     states[] is List<String> (same as diagram_sequence.actors — scan
    //     each StringLit directly). transitions[].label is rendered.
    //     `initial` is a TOP-LEVEL String? field (like diagram_venn.overlap_label).
    //   diagram_swimlane — Struct{lanes: List<String>, steps: [{lane,label,order}]}.
    //     lanes[] is List<String>. steps[].label is rendered (lane is an
    //     identifier, scanned defensively). steps[].order is Float, skipped.
    //   diagram_data_flow / diagram_high_level / diagram_architecture —
    //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
    //     Same shape as diagram_flowchart (Н81) — REUSES scan_flowchart_labels
    //     (no new scanner for an identical shape, per spec: "переиспользовать
    //     сканер, не писать заново для каждой из трёх").
    //     For diagram_architecture, the `icon` field is a controlled enum
    //     (validated against svg_icon's 10 names at runtime) — NOT scanned,
    //     same decision as diagram_medallion.
    "diagram_er",
    "diagram_state",
    "diagram_swimlane",
    "diagram_data_flow",
    "diagram_high_level",
    "diagram_architecture",
];

/// Builtins whose string arguments are NOT auto-escaped (structural).
/// String literals with `<script>` here generate an ERROR (injection
/// vector — runtime cannot catch it).
///
/// [Наряд №92 Category 2] These 4 builtins are covered by this separate,
/// stricter lint path (ERROR not WARNING). Each has runtime validation
/// that rejects dangerous inputs:
///   svg_path          — d argument validated: rejects < and > chars
///   svg_canvas        — viewbox validated: must be exactly 4 numbers
///   svg_group         — transform escaped via escape_attr(); children
///                        are trusted prior-builtin outputs (not user text)
///   svg_sketchy_filter — id escaped via escape_attr()
const SVG_NO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_path",           // d is path-data mini-language, not escaped
    "svg_canvas",         // viewbox is structural, not escaped
    "svg_group",          // transform is structural, not escaped
    "svg_sketchy_filter", // id is structural, validated but not escaped as text
];

// [Наряд №92 Category 1] Builtins intentionally NOT in either
// SVG_AUTO_ESCAPE_BUILTINS or SVG_NO_ESCAPE_BUILTINS.
//
// None of these accept free-form user text that is inserted as-is
// into SVG markup. Each is excluded for a documented reason:
//
//   svg_rect / svg_circle / svg_line — attribute values only (fill,
//     stroke, color strings), escaped at runtime via escape_attr().
//     No text *content* inserted between XML tags.
//
//   svg_icon — name is a validated enum (10 known icon names, error
//     on unknown); color is an escaped attribute. No free-form text
//     injection surface.
//
//   diagram_style — returns a DiagramStyle Struct, NOT SVG markup.
//     Color strings are consumed and escaped by downstream builtins
//     (chart_*/diagram_*), which are in SVG_AUTO_ESCAPE_BUILTINS.
//
//   color_palette — returns a DiagramStyle Struct, NOT SVG markup.
//     intent/mode are validated enums; no SVG output at all.
//
//   svg_generate — kind/intent are validated enums. Procedural
//     background generation with no free-form text output.
//
//   svg_canvas_preset — preset_name is a validated enum; viewbox
//     validated as 4 numbers; children are trusted prior-builtin
//     outputs (same trust model as svg_group/svg_canvas children).
//
//   chart_heatmap — pure numeric grid (List<List<Float>>). No user
//     text at all. Documented exclusion per Наряд №79.
//
//   infographic_qa — read-only analysis of an SVG string, produces
//     no new markup. Documented exclusion per Наряд №89.
//
//   template_render — template is trusted code; data values escaped
//     at runtime via escape_html_chars. Documented exclusion per
//     Наряд №86.
//
//   НАРЯД №117 string builtins (trim_start, trim_end, truncate,
//     slugify, word_wrap, capitalize, title_case) — pure string
//     transformations. No SVG markup produced, no user text
//     inserted into HTML/SVG. Not applicable to SVG escape
//     classification.
//
//   НАРЯД №118 collection builtins (unique, chunk, sort) — pure
//     list transformations. No SVG/HTML markup produced.
//     Not applicable to SVG escape classification.

// Argument indices that are auto-escaped within SVG_AUTO_ESCAPE_BUILTINS.
// For svg_text: arg 2 (content). For svg_callout: arg 0 (text).
fn auto_escaped_arg_index(builtin: &str) -> Option<usize> {
    match builtin {
        "svg_text" => Some(2),
        "svg_callout" => Some(0),
        _ => None,
    }
}

/// Detect potential XSS/injection payload in a string literal.
/// Returns Some(reason) if the string looks dangerous.
fn detect_xss_payload(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    if lower.contains("<script") {
        return Some("contains <script> tag");
    }
    if lower.contains("javascript:") {
        return Some("contains javascript: URL");
    }
    // on<event>= attributes: onclick=, onload=, onerror=, etc.
    // Match `on` followed by 2+ letters followed by `=`
    //
    // N0 (gh#1209, PE-1): the `on` prefix must sit at a WORD BOUNDARY —
    // the byte before it must not be [A-Za-z0-9_-]. Without the boundary
    // `content=` in `<meta content=...>` matched (the `on` inside the
    // word) — the false positive that flagged every page carrying a
    // meta/content attribute. Optional spaces/tabs are allowed between
    // the handler name and `=`: `<div onLoad = "x">` is still an event
    // handler in HTML (before the fix the `=` was required immediately
    // after the letters and this slipped through).
    let bytes = s.as_bytes();
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'o' && bytes[i + 1] == b'n' {
            let at_boundary = i == 0 || !is_word_byte(bytes[i - 1]);
            if at_boundary {
                // Check that what follows is letters then (optional
                // spaces then) '='
                let mut j = i + 2;
                let mut letter_count = 0;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    letter_count += 1;
                    j += 1;
                }
                if letter_count >= 2 {
                    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'=' {
                        return Some("contains onX event handler attribute");
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Check if a URL is on the whitelist (Google Fonts only).
fn is_whitelisted_url(url: &str) -> bool {
    let trimmed = url.trim();
    // Allow Google Fonts CSS and font files
    if trimmed.starts_with("https://fonts.googleapis.com/") {
        return true;
    }
    if trimmed.starts_with("https://fonts.gstatic.com/") {
        return true;
    }
    // Allow data: URIs for SVG inline (common for icons)
    if trimmed.starts_with("data:image/svg+xml") {
        return true;
    }
    false
}

/// Scan chart_* `data` arg (arg 0) for XSS payloads in label string
/// literals. The data arg is a List literal of Struct literals:
///   chart_bar / chart_donut / chart_line / chart_area:
///     [{label: "...", value: 10.0}, ...]
///   chart_scatter:
///     [{x: 1.0, y: 2.0, label: "..."}, ...]   (label OPTIONAL)
///   chart_boxplot:
///     [{label: "...", values: [1.0, 2.0, ...]}, ...]
/// For each struct, we look up the `label` field BY NAME (not position).
/// This makes the scanner shape-agnostic: it catches <script> in label
/// regardless of whether label is the first, second, or third field, and
/// regardless of whether other fields (x, y, value, values) are present.
/// If the struct has no `label` field at all (legal for chart_scatter),
/// the scanner simply skips it — no label, no injection vector.
///
/// If the `label` value is a StringLit with an XSS payload, we emit a
/// WARNING (runtime escapes label text via escape_html_chars — this is
/// a defense-in-depth review hint, not a hard error).
fn scan_chart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for item in items {
            if let Expr::StructLit { fields, .. } = item {
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[].label string literal {} — runtime will escape, but review intent",
                                    fn_name, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Scan chart_radar `data` arg (arg 0) for XSS payloads. Radar has a
/// DIFFERENT top-level shape from other chart_* builtins — it's a
/// Struct with two List fields, not a List of Structs:
///   Struct {
///     axes:  List<String>,                  // scan each StringLit
///     series: List<Struct{name, values}>,   // scan each series.name
///   }
///
/// We look up `axes` and `series` by field name on the top-level
/// StructLit. For `axes`, the elements are StringLits directly (no
/// struct wrapping) — so we scan them in place. For `series`, each
/// element is a StructLit and we look up its `name` field by key
/// (same approach as scan_chart_labels, just one struct level deeper).
///
/// If any string literal carries an XSS payload, we emit a WARNING
/// (runtime escapes via escape_html_chars — defense-in-depth, not a
/// hard error). The warning identifies WHICH field is suspicious
/// (axes vs series[].name) so the caller can locate the input.
fn scan_radar_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // axes: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: axes_items, ..
        }) = fields.get("axes")
        {
            for axis in axes_items {
                if let Expr::StringLit { value: s, .. } = axis {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            axis,
                            format!(
                                "security: {} data.axes string literal {} — runtime will escape, but review intent",
                                fn_name, reason
                            ),
                        ));
                    }
                }
            }
        }
        // series: List<Struct{name, values}> — scan each series.name
        if let Some(Expr::List {
            items: series_items,
            ..
        }) = fields.get("series")
        {
            for item in series_items {
                if let Expr::StructLit {
                    fields: series_fields,
                    ..
                } = item
                {
                    if let Some(name_expr) = series_fields.get("name") {
                        if let Expr::StringLit { value: s, .. } = name_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    name_expr,
                                    format!(
                                        "security: {} data.series[].name string literal {} — runtime will escape, but review intent",
                                        fn_name, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №81 Block 6 — recursive scanner for `diagram_tree` /
/// `diagram_org_chart` data shapes.
///
/// Both functions accept `Struct { label, title?, children }` where
/// `children` is `List<Struct>` of the SAME shape — i.e. the nesting
/// depth is unbounded. The existing `scan_chart_labels` only scans
/// ONE level (it iterates a List<Struct> literal and looks up `label`
/// on each element). It does NOT recurse into a `children` field —
/// which means an injection at depth ≥ 2 would slip past it.
///
/// This function walks the recursive struct: at each level we look up
/// `label` (and `title` if present) as StringLit fields and check for
/// XSS payloads. We then descend into the `children` List and recurse
/// on each child Struct.
///
/// `path` is used in the warning message so the caller can locate the
/// offending node (e.g. "root.children[1].children[2].label") — this
/// is essential because the recursive structure can be deeply nested
/// and a generic "label contains <script>" warning would be useless.
///
/// `allow_title` controls whether `title` is scanned (diagram_org_chart
/// allows it; diagram_tree ignores the field even if present, but we
/// still scan it defensively — better to over-warn than miss a payload
/// at the call site that uses diagram_tree but actually provides title).
fn scan_tree_labels_recursive(
    fn_name: &str,
    arg: &Expr,
    path: &str,
    allow_title: bool,
    result: &mut AnalysisResult,
) {
    if let Expr::StructLit { fields, .. } = arg {
        // Scan `label` (always present per spec — required field)
        if let Some(label_expr) = fields.get("label") {
            if let Expr::StringLit { value: s, .. } = label_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        label_expr,
                        format!(
                            "security: {} data.{}.label string literal {} — runtime will escape, but review intent",
                            fn_name, path, reason
                        ),
                    ));
                }
            }
        }
        // Scan `title` (org chart only — but scan defensively regardless
        // of allow_title, since a malicious caller could supply title to
        // diagram_tree too; the runtime still escapes it via svg_text if
        // it ends up in the output. Better to over-warn.)
        if allow_title {
            if let Some(title_expr) = fields.get("title") {
                if let Expr::StringLit { value: s, .. } = title_expr {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            title_expr,
                            format!(
                                "security: {} data.{}.title string literal {} — runtime will escape, but review intent",
                                fn_name, path, reason
                            ),
                        ));
                    }
                }
            }
        } else {
            // Even for diagram_tree (allow_title=false), if a title is
            // present we should warn — it's not used, but its presence
            // in literal form is suspicious intent.
            if let Some(title_expr) = fields.get("title") {
                if let Expr::StringLit { value: s, .. } = title_expr {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            title_expr,
                            format!(
                                "security: {} data.{}.title string literal {} — field not used by diagram_tree but review intent",
                                fn_name, path, reason
                            ),
                        ));
                    }
                }
            }
        }
        // Recurse into children (if present)
        if let Some(Expr::List {
            items: child_items, ..
        }) = fields.get("children")
        {
            for (i, child) in child_items.iter().enumerate() {
                let child_path = format!("{}.children[{}]", path, i);
                scan_tree_labels_recursive(fn_name, child, &child_path, allow_title, result);
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_flowchart` data shape.
///
/// Flowchart data has TWO independent lists, each containing user text:
///   Struct {
///     nodes: List<Struct{id, label}>,
///     edges: List<Struct{from, to, label?}>,
///   }
///
/// We scan `nodes[].label` and `edges[].label` separately (the `id`,
/// `from`, `to` fields are identifiers used for graph topology, not
/// rendered text — but we scan them defensively too, since a literal
/// `<script>` in an id field is still suspicious intent even if it
/// wouldn't reach the output).
///
/// Both lists are scanned to WARNINGs (runtime escapes label text via
/// escape_html_chars — defense-in-depth, same as chart_*).
fn scan_flowchart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // nodes: List<Struct{id, label}>
        if let Some(Expr::List {
            items: node_items, ..
        }) = fields.get("nodes")
        {
            for (i, item) in node_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: node_fields,
                    ..
                } = item
                {
                    // Scan id (defensive — not rendered, but suspicious if payload)
                    if let Some(id_expr) = node_fields.get("id") {
                        if let Expr::StringLit { value: s, .. } = id_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    id_expr,
                                    format!(
                                        "security: {} data.nodes[{}].id string literal {} — field not rendered but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // Scan label (rendered — primary injection vector)
                    if let Some(label_expr) = node_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.nodes[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // edges: List<Struct{from, to, label?}>
        if let Some(Expr::List {
            items: edge_items, ..
        }) = fields.get("edges")
        {
            for (i, item) in edge_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: edge_fields,
                    ..
                } = item
                {
                    // from/to are identifiers (defensive scan)
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = edge_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.edges[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    // label is rendered as edge midpoint text
                    if let Some(label_expr) = edge_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.edges[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_layers` data shape.
///
/// Flat list of structs:
///   List<Struct{label, description?}>
///
/// Both `label` and `description` are rendered as text — both must be
/// scanned. Uses the same per-struct pattern as scan_chart_labels but
/// checks TWO fields instead of one.
fn scan_layers_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // label (always rendered)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // description (rendered, optional)
                if let Some(description_expr) = fields.get("description") {
                    if let Expr::StringLit { value: s, .. } = description_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                description_expr,
                                format!(
                                    "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_sequence` data shape.
///
/// Sequence has TWO independent lists, each containing user text:
///   Struct {
///     actors:   List<String>,                         // scan each StringLit directly
///     messages: List<Struct{from, to, label?}>,       // scan messages[].label
///   }
///
/// `actors` is a `List<String>` (NOT `List<Struct>`) — this is the
/// special case called out in the narazd spec: "список строк — не забыть,
/// это другая форма, чем везде остальные". The scanner walks each
/// StringLit in actors[] directly (no struct unwrap), unlike
/// scan_chart_labels/scan_layers_labels which expect StructLit elements.
///
/// `messages[].label` is the only rendered text field in messages;
/// `from`/`to` are identifier strings used for actor lookup, not
/// rendered — but we scan them defensively (same pattern as
/// scan_flowchart_labels for edges[].from/to).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_sequence_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // actors: List<String> — scan each StringLit directly (no struct
        // unwrap, this is the spec's "list of strings, not list of structs"
        // special case).
        if let Some(Expr::List {
            items: actor_items, ..
        }) = fields.get("actors")
        {
            for (i, actor) in actor_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = actor {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            actor,
                            format!(
                                "security: {} data.actors[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // messages: List<Struct{from, to, label?}> — scan from/to
        // defensively (identifiers, not rendered) and label as primary
        // (rendered edge label at midpoint).
        if let Some(Expr::List {
            items: msg_items, ..
        }) = fields.get("messages")
        {
            for (i, item) in msg_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: msg_fields, ..
                } = item
                {
                    // from/to (defensive — identifiers, not rendered)
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = msg_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.messages[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    // label (rendered as edge midpoint text)
                    if let Some(label_expr) = msg_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.messages[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_gantt` data shape.
///
/// Gantt is a flat list of structs with exactly ONE string field:
///   List<Struct{task: String, start: Float, duration: Float}>
///
/// Only `task` is rendered (as the bar label). `start`/`duration` are
/// floats used for geometry — never reach SVG output. We scan `task`
/// as a WARNING (runtime escapes via escape_html_chars — same as
/// scan_chart_labels scans `label`, just under a different field name).
///
/// Implementation note: this is essentially scan_chart_labels with the
/// field name changed from "label" to "task". We could parametrize
/// scan_chart_labels to take a field name, but the existing call sites
/// all use "label" — keeping a separate function preserves the
/// self-documenting nature of each scanner and avoids changing
/// behavior for the five existing chart_* builtins.
fn scan_gantt_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // task is the only string field; start/duration are floats
                if let Some(task_expr) = fields.get("task") {
                    if let Expr::StringLit { value: s, .. } = task_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                task_expr,
                                format!(
                                    "security: {} data[{}].task string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_timeline` data shape.
///
/// Timeline is a flat list of structs with THREE string fields:
///   List<Struct{date: String, label: String, description?: String}>
///
/// All three are rendered as text (`date` and `label` always, `description`
/// when present). All three must be scanned. Same per-struct walk pattern
/// as scan_layers_labels — extended to check `date` as the first field.
fn scan_timeline_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // date (always rendered, above/below the dot)
                if let Some(date_expr) = fields.get("date") {
                    if let Expr::StringLit { value: s, .. } = date_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                date_expr,
                                format!(
                                    "security: {} data[{}].date string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // label (always rendered)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // description (rendered, optional)
                if let Some(description_expr) = fields.get("description") {
                    if let Expr::StringLit { value: s, .. } = description_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                description_expr,
                                format!(
                                    "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_venn` data shape.
///
/// Venn has a nested structure with a TOP-LEVEL string field:
///   Struct {
///     circles:      List<Struct{label: String, value?: Float}>,
///     overlap_label: String?,                          ← TOP-LEVEL, not in list
///   }
///
/// `overlap_label` is the spec-called-out "easy to forget" case: it's a
/// field on the outer Struct, not an element of `circles[]`. A scanner
/// that only walks List elements would miss it entirely. We scan it
/// separately as a top-level StringLit field.
///
/// `circles[].label` is rendered inside each circle (offset from center).
/// `circles[].value` is a Float — never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_venn_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // circles: List<Struct{label, value?}> — scan each circle's label
        if let Some(Expr::List {
            items: circle_items,
            ..
        }) = fields.get("circles")
        {
            for (i, item) in circle_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: circle_fields,
                    ..
                } = item
                {
                    if let Some(label_expr) = circle_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.circles[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // overlap_label — TOP-LEVEL field (the spec's "don't forget" case).
        // This is NOT inside the circles list; a scanner that only walks
        // list elements would miss it.
        if let Some(overlap_expr) = fields.get("overlap_label") {
            if let Expr::StringLit { value: s, .. } = overlap_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        overlap_expr,
                        format!(
                            "security: {} data.overlap_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_quadrant` data shape.
///
/// Quadrant has TWO top-level string fields PLUS a nested list:
///   Struct {
///     x_axis_label: String,                            ← TOP-LEVEL
///     y_axis_label: String,                            ← TOP-LEVEL
///     items: List<Struct{label: String, x: Float, y: Float}>,
///   }
///
/// Both `x_axis_label` and `y_axis_label` are TOP-LEVEL fields (the spec
/// calls them out as "поля верхнего уровня, не элементы списка, легко
/// забыть"). A scanner that only walks List elements would miss BOTH.
///
/// `items[].label` is rendered next to each point marker. `items[].x`
/// and `items[].y` are Floats — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_quadrant_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // x_axis_label — top-level, easy to forget
        if let Some(x_axis_expr) = fields.get("x_axis_label") {
            if let Expr::StringLit { value: s, .. } = x_axis_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        x_axis_expr,
                        format!(
                            "security: {} data.x_axis_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
        // y_axis_label — top-level, easy to forget
        if let Some(y_axis_expr) = fields.get("y_axis_label") {
            if let Expr::StringLit { value: s, .. } = y_axis_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        y_axis_expr,
                        format!(
                            "security: {} data.y_axis_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
        // items: List<Struct{label, x, y}> — scan each item's label
        if let Some(Expr::List {
            items: item_list, ..
        }) = fields.get("items")
        {
            for (i, item) in item_list.iter().enumerate() {
                if let Expr::StructLit {
                    fields: item_fields,
                    ..
                } = item
                {
                    if let Some(label_expr) = item_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.items[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_medallion` data shape.
///
/// Medallion is a flat list of structs:
///   List<Struct{icon: String?, label: String, value?: Float}>
///
/// `label` is rendered below each medallion — primary scan target.
/// `icon` is a CONTROLLED ENUM (validated at runtime against the 10
/// known svg_icon names) — NOT user free-form text, explicitly NOT
/// scanned per the spec ("icon не сканировать как текст — это controlled
/// enum-подобное значение"). If a user passes `<script>` as an icon name,
/// the runtime rejects it with an "unknown icon name" error before any
/// SVG output is produced.
/// `value` is a Float — never rendered as text, skipped.
///
/// This scanner is essentially scan_chart_labels (single `label` field
/// per struct). We keep it as a separate function for self-documenting
/// naming and to leave room for medallion-specific extensions later.
fn scan_medallion_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // label — rendered below the medallion (primary target)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // NOTE: `icon` is intentionally NOT scanned here. It is a
                // controlled enum (one of 10 known svg_icon names), validated
                // at runtime via icon_path_data() before any SVG output.
                // Scanning it as text would produce false positives for
                // legitimate icon names that happen to contain angle brackets
                // (none currently do, but the principle holds).
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_er` data shape.
///
/// ER has two lists, each with rendered text:
///   Struct {
///     entities:  List<Struct{name: String, fields: List<String>}>,
///     relations: List<Struct{from: String, to: String, label?: String}>,
///   }
///
/// The novel case here is `entities[].fields` — a `List<String>` NESTED
/// INSIDE a struct field. This is the THIRD nesting form in the SVG
/// suite (after top-level `List<String>` in diagram_sequence.actors,
/// and `List<Struct>` everywhere else). A scanner that only walks one
/// level of struct fields would miss `fields[]` entirely — each field
/// name is rendered as a separate line inside the entity box, so an
/// injection in `fields[2]` would reach the SVG output.
///
/// We walk per-entity: scan `name` (rendered in the header bar), then
/// iterate `fields[]` and scan each `StringLit` element directly (no
/// struct unwrap — same approach as scan_sequence_labels.actors).
///
/// `relations[].label` is rendered at the connector midpoint (same as
/// flowchart edges). `relations[].from` and `.to` are entity-name
/// identifiers, scanned defensively (not rendered, but suspicious if
/// they contain a payload).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_er_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // entities: List<Struct{name, fields: List<String>}>
        if let Some(Expr::List {
            items: entity_items,
            ..
        }) = fields.get("entities")
        {
            for (i, item) in entity_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: entity_fields,
                    ..
                } = item
                {
                    // name — rendered in the entity header bar (primary target)
                    if let Some(name_expr) = entity_fields.get("name") {
                        if let Expr::StringLit { value: s, .. } = name_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    name_expr,
                                    format!(
                                        "security: {} data.entities[{}].name string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // fields: List<String> NESTED inside a struct field —
                    // the third nesting form. Each StringLit is rendered
                    // as a separate line inside the entity box.
                    if let Some(Expr::List {
                        items: field_items, ..
                    }) = entity_fields.get("fields")
                    {
                        for (j, f_item) in field_items.iter().enumerate() {
                            if let Expr::StringLit { value: s, .. } = f_item {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        f_item,
                                        format!(
                                            "security: {} data.entities[{}].fields[{}] string literal {} — runtime will escape, but review intent",
                                            fn_name, i, j, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        // relations: List<Struct{from, to, label?}> — same shape as
        // diagram_flowchart.edges. Scan from/to defensively, label as primary.
        if let Some(Expr::List {
            items: rel_items, ..
        }) = fields.get("relations")
        {
            for (i, item) in rel_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: rel_fields, ..
                } = item
                {
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = rel_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.relations[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(label_expr) = rel_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.relations[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_state` data shape.
///
/// State has TWO independent text sources plus a top-level optional:
///   Struct {
///     states:      List<String>,                              // scan each StringLit
///     transitions: List<Struct{from, to, label?}>,            // scan transitions[].label
///     initial:     String?,                                   // TOP-LEVEL field, like venn.overlap_label
///   }
///
/// `states[]` is `List<String>` (same form as diagram_sequence.actors —
/// scan each StringLit directly, no struct unwrap).
/// `transitions[].label` is rendered at the connector midpoint (self-
/// loops render the label above the loop arc — same scan rule applies).
/// `transitions[].from`/`.to` are state-name identifiers, scanned
/// defensively (not rendered).
/// `initial` is a TOP-LEVEL `String?` field — easy to forget, called
/// out explicitly in the spec. We scan it separately at the outer
/// StructLit level (not inside any list).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_state_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // states: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: state_items, ..
        }) = fields.get("states")
        {
            for (i, state) in state_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = state {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            state,
                            format!(
                                "security: {} data.states[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // transitions: List<Struct{from, to, label?}> — same pattern as
        // diagram_flowchart.edges / diagram_sequence.messages.
        if let Some(Expr::List {
            items: trans_items, ..
        }) = fields.get("transitions")
        {
            for (i, item) in trans_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: trans_fields,
                    ..
                } = item
                {
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = trans_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.transitions[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(label_expr) = trans_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.transitions[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // initial — TOP-LEVEL String? field (the spec's "easy to forget" case,
        // same category as diagram_venn.overlap_label). It's rendered as the
        // entry-arrow target state name, but ALSO drawn near the initial node.
        if let Some(initial_expr) = fields.get("initial") {
            if let Expr::StringLit { value: s, .. } = initial_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        initial_expr,
                        format!(
                            "security: {} data.initial string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_swimlane` data shape.
///
/// Swimlane has TWO lists, each containing user text:
///   Struct {
///     lanes: List<String>,                              // scan each StringLit
///     steps: List<Struct{lane, label, order}>,          // scan steps[].label
///   }
///
/// `lanes[]` is `List<String>` (same form as diagram_sequence.actors
/// and diagram_state.states — scan each StringLit directly).
/// `steps[].label` is rendered inside each step pill (primary target).
/// `steps[].lane` is a lane-name identifier (defensive scan — not
/// rendered, but suspicious if it carries a payload).
/// `steps[].order` is a Float — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_swimlane_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // lanes: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: lane_items, ..
        }) = fields.get("lanes")
        {
            for (i, lane) in lane_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = lane {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            lane,
                            format!(
                                "security: {} data.lanes[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // steps: List<Struct{lane, label, order}> — scan lane defensively,
        // label as primary. order is Float, skipped.
        if let Some(Expr::List {
            items: step_items, ..
        }) = fields.get("steps")
        {
            for (i, item) in step_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: step_fields,
                    ..
                } = item
                {
                    // lane — defensive (identifier, not rendered as free text)
                    if let Some(lane_expr) = step_fields.get("lane") {
                        if let Expr::StringLit { value: s, .. } = lane_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    lane_expr,
                                    format!(
                                        "security: {} data.steps[{}].lane string literal {} — field not rendered but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // label — rendered inside the step pill (primary target)
                    if let Some(label_expr) = step_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.steps[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Walk an expression and run the security check on every FnCall node.
fn walk_expr_for_svg_security(expr: &Expr, result: &mut AnalysisResult, ctx: &str) {
    match expr {
        Expr::FnCall { name, args, .. } => {
            // Check string-literal arguments to SVG builtins
            if SVG_AUTO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                if let Some(content_idx) = auto_escaped_arg_index(name) {
                    if let Some(content_expr) = args.get(content_idx) {
                        if let Expr::StringLit { value: s, .. } = content_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    content_expr,
                                    format!(
                                        "security: {}({} arg) string literal {} — runtime will escape, but review intent",
                                        name, content_idx + 1, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
                // Special case for chart_* builtins: their `label` field
                // lives inside `data: List<Struct{...}>` at arg 0. The list
                // element shape varies:
                //   chart_bar / chart_donut / chart_line / chart_area:
                //     {label, value}  — label first
                //   chart_scatter:
                //     {x, y, label?}  — label third, optional
                //   chart_boxplot:
                //     {label, values} — label first, but values is a List
                // scan_chart_labels looks up `label` BY NAME, so it works
                // uniformly across all these list-of-struct shapes.
                //
                // chart_radar has a DIFFERENT top-level shape
                // (Struct{axes, series}, not List<Struct>), so it gets
                // its own scanner (scan_radar_labels) that walks both
                // the `axes` List<String> and the `series[].name` field.
                //
                // chart_heatmap is intentionally NOT scanned — its data
                // is `List<List<Float>>` (pure numeric, no user text).
                // Runtime escapes label text via escape_html_chars, so
                // these are WARNINGs (suspicious intent) not ERRORs.
                if name == "chart_bar"
                    || name == "chart_donut"
                    || name == "chart_line"
                    || name == "chart_area"
                    || name == "chart_scatter"
                    || name == "chart_boxplot"
                {
                    scan_chart_labels(name, args, result);
                }
                if name == "chart_radar" {
                    scan_radar_labels(name, args, result);
                }
                // Наряд №81 Block 6: diagram scanners.
                //   diagram_tree / diagram_org_chart — recursive Struct,
                //     need scan_tree_labels_recursive (NOT the flat
                //     scan_chart_labels — that one only goes one level
                //     deep and would miss injections at children[2].label
                //     or deeper).
                //   diagram_flowchart — two independent lists (nodes,
                //     edges), each with its own label field. Scanned
                //     separately by scan_flowchart_labels.
                //   diagram_layers — flat list, label + description.
                //     scan_layers_labels checks both fields.
                if name == "diagram_tree" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", false, result);
                    }
                }
                if name == "diagram_org_chart" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", true, result);
                    }
                }
                if name == "diagram_flowchart" {
                    scan_flowchart_labels(name, args, result);
                }
                if name == "diagram_layers" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №82 Block 6: temporal & process diagram scanners.
                //   diagram_sequence — Struct{actors: List<String>, messages: ...}.
                //     actors is List<String> (special case — direct StringLit
                //     scan, no StructLit unwrap). messages[].label is rendered.
                //     Scanned by scan_sequence_labels.
                //   diagram_timeline — flat List<Struct{date, label, description?}>.
                //     3 string fields — scan_timeline_labels checks all three.
                //   diagram_gantt — flat List<Struct{task, start, duration}>.
                //     Only `task` is rendered — scan_gantt_labels checks it.
                //   diagram_process / diagram_loop — same shape as diagram_layers
                //     (List<Struct{label, description?}>) — REUSES
                //     scan_layers_labels (no need to write a new scanner for
                //     an identical shape — "не писать заново" per spec).
                if name == "diagram_sequence" {
                    scan_sequence_labels(name, args, result);
                }
                if name == "diagram_timeline" {
                    scan_timeline_labels(name, args, result);
                }
                if name == "diagram_gantt" {
                    scan_gantt_labels(name, args, result);
                }
                if name == "diagram_process" || name == "diagram_loop" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №83 Block 6: sets & comparison diagram scanners.
                //   diagram_venn — Struct{circles: [{label, value?}], overlap_label?}.
                //     Special scanner: overlap_label is a TOP-LEVEL field
                //     (not in circles[]), easy to forget — scan_venn_labels
                //     checks both the nested circles[].label and the top-level
                //     overlap_label in one pass.
                //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [...]}.
                //     BOTH axis labels are top-level fields. scan_quadrant_labels
                //     checks both axis labels + items[].label.
                //   diagram_pyramid / diagram_nested — same flat shape as
                //     diagram_layers (List<Struct{label, value?}>) — REUSES
                //     scan_layers_labels (no new scanner for identical shape).
                //   diagram_medallion — List<Struct{icon?, label, value?}>.
                //     Only `label` is scanned; `icon` is a controlled enum
                //     (validated at runtime against 10 known svg_icon names),
                //     NOT free-form text — explicitly NOT scanned per spec.
                if name == "diagram_venn" {
                    scan_venn_labels(name, args, result);
                }
                if name == "diagram_quadrant" {
                    scan_quadrant_labels(name, args, result);
                }
                if name == "diagram_pyramid" || name == "diagram_nested" {
                    scan_layers_labels(name, args, result);
                }
                if name == "diagram_medallion" {
                    scan_medallion_labels(name, args, result);
                }
                // Наряд №84 Block 7: data & state diagram scanners.
                //   diagram_er — Struct{entities:[{name, fields:[String]}],
                //     relations:[{from,to,label?}]}. Special scanner
                //     scan_er_labels walks the nested List<String> inside
                //     each entity (the third nesting form — see comment
                //     on the scanner for why this needs bespoke handling).
                //   diagram_state — Struct{states:[String], transitions:[...],
                //     initial?}. scan_state_labels scans states[] as a direct
                //     List<String>, transitions[].label as rendered text, and
                //     `initial` as a TOP-LEVEL String? field (overlap_label form).
                //   diagram_swimlane — Struct{lanes:[String], steps:[{lane,label,order}]}.
                //     scan_swimlane_labels scans lanes[] directly + steps[].label.
                //   diagram_data_flow / diagram_high_level / diagram_architecture —
                //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
                //     Same shape as diagram_flowchart (Н81) — REUSES
                //     scan_flowchart_labels (no new scanner for an identical
                //     shape, per spec). The `icon` field on diagram_architecture
                //     nodes is a controlled enum (validated against svg_icon's
                //     10 names at runtime), NOT free-form text — explicitly
                //     NOT scanned, same decision as diagram_medallion.
                if name == "diagram_er" {
                    scan_er_labels(name, args, result);
                }
                if name == "diagram_state" {
                    scan_state_labels(name, args, result);
                }
                if name == "diagram_swimlane" {
                    scan_swimlane_labels(name, args, result);
                }
                if name == "diagram_data_flow"
                    || name == "diagram_high_level"
                    || name == "diagram_architecture"
                {
                    scan_flowchart_labels(name, args, result);
                }
            }
            if SVG_NO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                // Check ALL string-literal args — any of them could be an injection vector
                for (i, arg) in args.iter().enumerate() {
                    if let Expr::StringLit { value: s, .. } = arg {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.errors.push(SpannedError::at_expr(
                                arg,
                                format!(
                                    "security: {}({} arg) string literal {} — this builtin does NOT auto-escape this argument; potential injection vector",
                                    name, i + 1, reason
                                ),
                            ));
                        }
                        // For svg_canvas, check viewbox arg specifically
                        if name == "svg_canvas" && i == 2 {
                            // viewbox should be 4 numbers — any other format is suspicious
                            let parts: Vec<&str> = s.split_whitespace().collect();
                            if parts.len() != 4 || parts.iter().any(|p| p.parse::<f64>().is_err()) {
                                result.warnings.push(SpannedError::at_expr(
                                    arg,
                                    format!(
                                        "security: svg_canvas viewbox argument should be 4 numbers, got {:?}",
                                        s
                                    ),
                                ));
                            }
                        }
                    } else {
                        // Recurse into non-literal expressions (concat, list, etc.)
                        // to catch <script> hidden inside e.g. "M 10 10 " + "<script>"
                        walk_expr_for_svg_security(arg, result, ctx);
                    }
                }
            }
            // Check for external URLs in non-whitelisted contexts
            // (e.g. svg_icon color, svg_text fill) — these are colors, not URLs,
            // but if someone passes "javascript:..." as a color, that's suspicious
            for (i, arg) in args.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = arg {
                    if s.starts_with("javascript:") || s.starts_with("data:text/html") {
                        result.errors.push(SpannedError::at_expr(
                            arg,
                            format!(
                                "security: {}({} arg) contains potentially dangerous URL scheme: {:?}",
                                name,
                                i + 1,
                                s
                            ),
                        ));
                    }
                }
            }
            // Recurse into arguments
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
            let _ = function;
        }
        Expr::BinaryOp {
            left: lhs,
            right: rhs,
            ..
        } => {
            // String concatenation: walk BOTH sides to scan all string literals.
            // If a StringLit appears inside a concat that's an arg to an SVG
            // no-escape builtin, the surrounding FnCall walker has already
            // recursed into us (via the `else` branch). Here we additionally
            // scan the immediate StringLit children of BinaryOp for XSS payloads
            // — this catches concat expressions where the FnCall walker didn't
            // flag them because the payload is buried inside a BinaryOp.
            if let Expr::StringLit { value: s, .. } = lhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    // This is a WARNING only — we don't know if this concat
                    // feeds an SVG context. The FnCall walker will escalate
                    // to ERROR if appropriate.
                    result.warnings.push(SpannedError::at_expr(
                        lhs.as_ref(),
                        format!(
                            "security: string literal in concatenation {} — review usage",
                            reason
                        ),
                    ));
                }
            }
            if let Expr::StringLit { value: s, .. } = rhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        rhs.as_ref(),
                        format!(
                            "security: string literal in concatenation {} — review usage",
                            reason
                        ),
                    ));
                }
            }
            walk_expr_for_svg_security(lhs, result, ctx);
            walk_expr_for_svg_security(rhs, result, ctx);
        }
        Expr::IfElse {
            condition: cond,
            then_branch: then_e,
            else_branch: else_e,
            ..
        } => {
            walk_expr_for_svg_security(cond, result, ctx);
            walk_expr_for_svg_security(then_e, result, ctx);
            walk_expr_for_svg_security(else_e, result, ctx);
        }
        Expr::List { items, .. } => {
            for item in items {
                walk_expr_for_svg_security(item, result, ctx);
            }
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_expr_for_svg_security(v, result, ctx);
            }
        }
        Expr::FieldAccess { object: inner, .. } => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::IndexAccess {
            object: inner,
            index: idx,
            ..
        } => {
            walk_expr_for_svg_security(inner, result, ctx);
            walk_expr_for_svg_security(idx, result, ctx);
        }
        Expr::Try { expr: inner, .. } => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        // №369: match-as-expression — walk scrutinee + arm bodies + else.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(scrutinee, result, ctx);
            for arm in arms {
                for s in arm.body() {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(eb) = else_body {
                for s in eb {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        // №332 (ADR-0164): perception constructions — no SVG surface;
        // ProvBind delegates to the inner construction.
        Expr::HandleSource { .. } => {}
        Expr::ProvBind { inner, .. } => walk_expr_for_svg_security(inner, result, ctx),
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
    let _ = ctx;
}

/// Walk a statement and run security check on every expression it contains.
fn walk_stmt_for_svg_security(stmt: &Statement, result: &mut AnalysisResult, ctx: &str) {
    match stmt {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            walk_expr_for_svg_security(value, result, ctx);
        }
        Statement::Return { value: e, .. } => walk_expr_for_svg_security(e, result, ctx),
        Statement::ExprStmt { expr: e, .. } => walk_expr_for_svg_security(e, result, ctx),
        Statement::Each { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::IfThen {
            condition: cond,
            body,
            ..
        } => {
            walk_expr_for_svg_security(cond, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(scrutinee, result, ctx);
            for arm in arms {
                match arm {
                    MatchArm::Compare(_, e, body) => {
                        walk_expr_for_svg_security(e, result, ctx);
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::Break | Statement::Continue => {}
        // Наряд №266: memory statements carry user-controlled expressions —
        // scan them like any other expression carrier.
        Statement::Memorize(m) => walk_expr_for_svg_security(&m.value, result, ctx),
        Statement::Forget(f) => walk_expr_for_svg_security(&f.query, result, ctx),
        Statement::Relate(r) => {
            walk_expr_for_svg_security(&r.from, result, ctx);
            walk_expr_for_svg_security(&r.to, result, ctx);
        }
    }
}

/// Walk all declarations and run the SVG/HTML security lint.
#[allow(clippy::wildcard_enum_match_arm)] // №532 tail: report/parse-only catch-all, no security decision reads this arm (docs/wildcard-tail-ledger.md)
fn svg_security_lint(declarations: &[Declaration], result: &mut AnalysisResult) {
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let ctx = format!("pattern {}", p.name);
                for s in &p.body {
                    walk_stmt_for_svg_security(s, result, &ctx);
                }
            }
            Declaration::Flow(f) => {
                let ctx = format!("flow {}", f.name);
                walk_expr_for_svg_security(&f.source, result, &ctx);
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        // Branch bodies can contain expressions in pipeline steps
                        let _ = branch;
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let ctx = format!("entity {}", e.name);
                walk_expr_for_svg_security(&e.value, result, &ctx);
            }
            Declaration::EntityRecord(e) => {
                let ctx = format!("entity {}", e.name);
                for f in &e.fields {
                    walk_expr_for_svg_security(&f.value, result, &ctx);
                }
            }
            Declaration::MlogServer(srv) => {
                let ctx = "mlogserver".to_string();
                for route in &srv.routes {
                    for s in &route.body {
                        walk_stmt_for_svg_security(s, result, &ctx);
                    }
                }
            }
            Declaration::Template(t) => {
                // Template body is a raw string with {{ var }} placeholders,
                // not parsed statements. We do a simple text scan for XSS payloads.
                let ctx = format!("template {}", t.name);
                if let Some(reason) = detect_xss_payload(&t.body) {
                    result.errors.push(SpannedError::at_decl(
                        decl,
                        format!(
                            "security: template '{}' body {} — templates are raw HTML, no auto-escaping",
                            t.name, reason
                        ),
                    ));
                }
                let _ = ctx;
            }
            _ => {}
        }
    }
    // Suppress unused warning for is_whitelisted_url (kept for future URL-context checks)
    let _ = is_whitelisted_url("https://fonts.googleapis.com/css");
}

/// Helper: extract field names from an EntityType declaration.
fn get_type_fields<'a>(declarations: &'a [Declaration], type_name: &str) -> Option<Vec<&'a str>> {
    for decl in declarations {
        if let Declaration::EntityType(e) = decl {
            if e.name == type_name {
                return Some(e.fields.iter().map(|f| f.name.as_str()).collect());
            }
        }
    }
    None
}

/// №565 (gh#927; the audit 02.10 §7.2 W-3): the respond_html TWO-ARGUMENT
/// form is DEPRECATED — its sense depends on the CONTENT of the first
/// argument (a first token 100..=599 reads as an HTTP status, anything
/// else as a document title: `respond_html("200 причин выбрать нас",
/// body)` served HTTP 200 and silently dropped the title — and a DB- or
/// LLM-sourced string would decide the response's meaning at runtime).
/// One release carries this WARN-ONLY pass (the №474 posture: warnings
/// only, errors structurally untouched, no runtime change): every 2-arg
/// call whose first argument is NOT a string literal is announced with
/// the migration path (respond_html_status / respond_html_doc). A
/// LITERAL first argument keeps working silently this release (the
/// audit's exact wording); the removal comes no earlier than one
/// release with the warning in place. The walk mirrors the call walk's
/// surfaces (pattern bodies, route bodies, flows, entities) — the same
/// sites the arity checker sees.
fn respond_html_two_arg_warnings(declarations: &[Declaration]) -> Vec<SpannedError> {
    // The №532 report-only catch-alls: the walker reads only the variants
    // that can CARRY a call (patterns, entities, flows, route bodies; the
    // FnCall/operator/block/list/struct/match expressions); the wildcard
    // tails cover the literal/structural variants that carry no calls —
    // documented in docs/wildcard-tail-ledger.md.
    #[allow(clippy::wildcard_enum_match_arm)] // the Declaration tail (Import/Test/Reflex/... carry no callable surface this pass reads)
    fn walk_decls(declarations: &[Declaration], out: &mut Vec<SpannedError>) {
        for decl in declarations {
            match decl {
                Declaration::Pattern(p) => walk_stmts(&p.body, out),
                Declaration::EntitySimple(e) => walk_expr(&e.value, out),
                Declaration::EntityRecord(e) => {
                    for f in &e.fields {
                        walk_expr(&f.value, out);
                    }
                }
                Declaration::Flow(f) => {
                    walk_expr(&f.source, out);
                    for (_, branches) in &f.branch_defs {
                        for b in branches {
                            walk_expr(&b.condition.target, out);
                            walk_expr(&b.condition.threshold, out);
                        }
                    }
                }
                Declaration::MlogServer(srv) => {
                    for route in &srv.routes {
                        walk_stmts(&route.body, out);
                    }
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::wildcard_enum_match_arm)] // the Expr tail (StringLit/FloatLit/BoolLit/Ident/HandleSource carry no sub-expressions)
    fn walk_expr(expr: &Expr, out: &mut Vec<SpannedError>) {
        if let Expr::FnCall { name, args, .. } = expr {
            if name == "respond_html" && args.len() == 2 {
                let literal_first = matches!(&args[0], Expr::StringLit { .. });
                if !literal_first {
                    out.push(SpannedError::at_expr(
                        expr,
                        "deprecated: respond_html(a, b) reads its sense from the first \
                         argument's CONTENT (a number-like string becomes an HTTP status, \
                         anything else a document title). Name the form explicitly: \
                         respond_html_status(status, body) or respond_html_doc(title, body) \
                         — №565, removal no earlier than one release with this warning",
                    ));
                }
            }
            for arg in args {
                walk_expr(arg, out);
            }
            return;
        }
        match expr {
            Expr::BinaryOp { left, right, .. } => {
                walk_expr(left, out);
                walk_expr(right, out);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                walk_expr(condition, out);
                walk_expr(then_branch, out);
                walk_expr(else_branch, out);
            }
            Expr::FieldAccess { object, .. } => walk_expr(object, out),
            Expr::IndexAccess { object, index, .. } => {
                walk_expr(object, out);
                walk_expr(index, out);
            }
            Expr::Try { expr: inner, .. } => walk_expr(inner, out),
            Expr::List { items, .. } => {
                for i in items {
                    walk_expr(i, out);
                }
            }
            Expr::StructLit { fields, .. } => {
                for v in fields.values() {
                    walk_expr(v, out);
                }
            }
            Expr::BlockIfElse {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                walk_expr(condition, out);
                walk_stmts(then_body, out);
                for (cond, body) in else_ifs {
                    walk_expr(cond, out);
                    walk_stmts(body, out);
                }
                if let Some(body) = else_body {
                    walk_stmts(body, out);
                }
            }
            Expr::MatchExpr {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                walk_expr(scrutinee, out);
                for arm in arms {
                    match arm {
                        MatchArm::Compare(_, e, body) => {
                            walk_expr(e, out);
                            walk_stmts(body, out);
                        }
                        MatchArm::Exact(_, body)
                        | MatchArm::StartsWith(_, body)
                        | MatchArm::Contains(_, body) => walk_stmts(body, out),
                    }
                }
                if let Some(body) = else_body {
                    walk_stmts(body, out);
                }
            }
            Expr::ProvBind { inner, .. } => walk_expr(inner, out),
            _ => {}
        }
    }

    fn walk_stmts(stmts: &[Statement], out: &mut Vec<SpannedError>) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                    walk_expr(value, out)
                }
                Statement::Return { value: e, .. } | Statement::ExprStmt { expr: e, .. } => {
                    walk_expr(e, out)
                }
                Statement::Each { iterable, body, .. }
                | Statement::EachWithIndex { iterable, body, .. } => {
                    walk_expr(iterable, out);
                    walk_stmts(body, out);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk_expr(condition, out);
                    walk_stmts(body, out);
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk_expr(condition, out);
                    walk_stmts(then_body, out);
                    for (cond, body) in else_ifs {
                        walk_expr(cond, out);
                        walk_stmts(body, out);
                    }
                    if let Some(body) = else_body {
                        walk_stmts(body, out);
                    }
                }
                Statement::IfThen {
                    condition: cond,
                    body,
                    ..
                } => {
                    walk_expr(cond, out);
                    walk_stmts(body, out);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk_expr(scrutinee, out);
                    for arm in arms {
                        match arm {
                            MatchArm::Compare(_, e, body) => {
                                walk_expr(e, out);
                                walk_stmts(body, out);
                            }
                            MatchArm::Exact(_, body)
                            | MatchArm::StartsWith(_, body)
                            | MatchArm::Contains(_, body) => walk_stmts(body, out),
                        }
                    }
                    if let Some(body) = else_body {
                        walk_stmts(body, out);
                    }
                }
                Statement::Memorize(m) => walk_expr(&m.value, out),
                Statement::Forget(f) => walk_expr(&f.query, out),
                Statement::Relate(r) => {
                    walk_expr(&r.from, out);
                    walk_expr(&r.to, out);
                }
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let mut out = Vec::new();
    walk_decls(declarations, &mut out);
    out
}

/// Walk an expression tree, checking FnCall arity and detecting undefined functions.
fn check_expr_calls(
    expr: &Expr,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    if let Expr::FnCall { name, args, .. } = expr {
        let is_known = builtin_names.contains(name)
            || pattern_param_counts.iter().any(|(n, _)| n == name)
            || learnable_names.contains(name);

        const INTERCEPTED_FUNCTIONS: &[&str] = &["recall_top_k"];

        if !is_known && !INTERCEPTED_FUNCTIONS.contains(&name.as_str()) {
            // №799 (T2, gh#1488): a PARKED builtin's name is KNOWN to the
            // registry source even when the row does not compile into
            // THIS build (the feature gate, the C3 (a) tiers) — the
            // refusal names the feature and the parking basis instead of
            // the lie "undefined". The finding blocks the run/serve
            // gates like every semantic finding (the exemption list
            // stays empty; №523).
            if let Some(parked) = crate::builtins::parked::parked_spec(name) {
                errors.push(
                    SpannedError::at_expr(expr, crate::builtins::parked::refusal_message(parked))
                        .with_kind(SemanticErrorKind::BuiltinParked),
                );
            } else {
                // Наряд №165: span taken from the FnCall expression itself —
                // the call site is the natural place to point the squiggle.
                // Наряд №523: the finding carries the structured
                // UNDEFINED_FUNCTION kind — the blocking gates classify by
                // the enum, never by the message text.
                errors.push(
                    SpannedError::at_expr(
                        expr,
                        format!(
                            "undefined: function '{}' is not a builtin, pattern, or learnable",
                            name
                        ),
                    )
                    .with_kind(SemanticErrorKind::UndefinedFunction),
                );
            }
        }

        // Check builtin arity
        if builtin_names.contains(name) {
            // №523: DYNAMIC-ARITY builtins — the registry's (min..=max)
            // spec cannot state their real contract, so the static check
            // stays silent here and the builtin itself remains the loud
            // runtime validator:
            //   * forget — two real surfaces: the №72 memory form
            //     forget(query, days?) [1..2, served by the TW/VM
            //     intercepts in memory_ops.rs] and the registry form
            //     forget(handle, key, grant, dry_run?) [3..4]. The union
            //     is non-contiguous; one spec cannot say it.
            //   * render — the template's parameter list is DATA (№115);
            //     the 1-arg form is the №448 taint-lift surface. Unbounded.
            // The №523 run gate surfaced both drifts (examples/p72,
            // examples/leak/ok_448_*). One named place, structured by
            // name — the same posture as is_exempt_from_blocking.
            let dynamic_arity = matches!(name.as_str(), "forget" | "render");
            if !dynamic_arity {
                if let Err(e) = crate::builtins::check_builtin_arity(name, args.len()) {
                    errors.push(SpannedError::at_expr(expr, e));
                }
            }

            // №799 (T2, gh#1488): the experimental tier warns on CHECK —
            // never an error (the exit-0 semantics). The finding rides
            // the errors vec with its OWN kind and check_program
            // partitions it into `warnings` at the tail (the ONE place):
            // the blocking gates never see it, the human output carries
            // the [BUILTIN_EXPERIMENTAL] class prefix.
            if let Some(spec) = crate::builtins::registry::spec_by_name(name) {
                if spec.tier == "experimental" {
                    errors.push(
                        SpannedError::at_expr(
                            expr,
                            format!(
                                "[BUILTIN_EXPERIMENTAL] {} is experimental — it may change \
                                 or refuse without notice (tier: experimental)",
                                name
                            ),
                        )
                        .with_kind(SemanticErrorKind::BuiltinExperimental),
                    );
                }
            }
        }

        // Check pattern param count
        for (pname, pcount) in pattern_param_counts {
            if *pname == *name && !builtin_names.contains(name) {
                if args.len() != *pcount {
                    errors.push(SpannedError::at_expr(
                        expr,
                        format!(
                            "function '{}' expects {} argument(s), got {}",
                            name,
                            pcount,
                            args.len()
                        ),
                    ));
                }
                break;
            }
        }

        // Recurse into arguments
        for arg in args {
            check_expr_calls(
                arg,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::BinaryOp {
        left,
        op: _op,
        right,
        ..
    } = expr
    {
        check_expr_calls(
            left,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            right,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::IfElse {
        condition: cond,
        then_branch: then_br,
        else_branch: else_br,
        ..
    } = expr
    {
        check_expr_calls(
            cond,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            then_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            else_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::List { items, .. } = expr {
        for item in items {
            check_expr_calls(
                item,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::IndexAccess {
        object: inner,
        index: idx,
        ..
    } = expr
    {
        check_expr_calls(
            inner,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            idx,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::StructLit { fields, .. } = expr {
        for v in fields.values() {
            check_expr_calls(
                v,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    }
}

/// Extract expressions from a statement and walk them for arity/undefined checks.
fn check_stmt_exprs(
    stmt: &Statement,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    match stmt {
        Statement::LetBinding { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Assign { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Return { value: expr, .. } => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::ExprStmt { expr, .. } => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Each { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in then_body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
            for (cond, body) in else_ifs {
                check_expr_calls(
                    cond,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
                for s in body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::IfThen {
            condition: cond,
            body,
            ..
        } => {
            check_expr_calls(
                cond,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_expr_calls(
                scrutinee,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                    MatchArm::Compare(_, expr, body) => {
                        check_expr_calls(
                            expr,
                            builtin_names,
                            pattern_param_counts,
                            learnable_names,
                            errors,
                        );
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::Break | Statement::Continue => {}
        // Наряд №266: memory statements' expressions go through the same
        // call/arity/undefined-function checks as every other statement.
        Statement::Memorize(m) => {
            check_expr_calls(
                &m.value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Forget(f) => {
            check_expr_calls(
                &f.query,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Relate(r) => {
            check_expr_calls(
                &r.from,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            check_expr_calls(
                &r.to,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    }
}

// ── Наряд №264: static immutability enforcement ─────────────────────
//
// Contract (№14, REFERENCE.md §3.2, examples/p30_assign_*): a variable is
// immutable unless bound with `let mut`; assignment to an immutable
// variable is an error "cannot assign to immutable variable: x (use
// 'let mut x' to make it mutable)". Until this pass the contract lived
// ONLY in the tree-walking interpreter's runtime (execution.rs
// eval_statements_cf) — `mlog check` passed violating programs and the
// VM compiled them silently.
//
// The walk mirrors TW's EXACT mutability model so `mlog check` never
// diverges from `mlog run`:
//   - a flat `HashSet<String>` of mutable names is threaded through ALL
//     nested blocks and never popped (execution.rs:946 passes the same
//     `mutable_vars` down into if/while/each/match bodies — a `let mut`
//     inside a block stays in effect for the rest of the function);
//   - pattern params are NOT mutable (they never enter mutable_vars);
//   - `each` / `each i, item` loop variables are NOT mutable (they are
//     env.insert-ed per iteration, never added to mutable_vars);
//   - the mutability check happens BEFORE the variable is resolved, so an
//     assignment to a never-declared name reports the same immutability
//     message (TW behavior, execution.rs:989-992) — not a separate
//     "undefined variable" diagnostic.

/// The error text is the TW лекало verbatim (execution.rs:991) —
/// examples/p30_assign_immutable.error is matched against the TW channel
/// and must keep matching whichever backend surfaces the contract.
pub(crate) fn immutability_error_text(name: &str) -> String {
    format!(
        "cannot assign to immutable variable: {} (use 'let mut {}' to make it mutable)",
        name, name
    )
}

/// Entry point: check one statement list (pattern body / route body) with
/// an initially-empty mutable set (params are immutable; route bodies have
/// no params at all).
fn check_pattern_mutability(body: &[Statement], errors: &mut Vec<SpannedError>) {
    let mut mutable: HashSet<String> = HashSet::new();
    check_stmts_mutability(body, &mut mutable, errors);
}

fn check_stmts_mutability(
    stmts: &[Statement],
    mutable: &mut HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    for stmt in stmts {
        check_stmt_mutability(stmt, mutable, errors);
    }
}

fn check_stmt_mutability(
    stmt: &Statement,
    mutable: &mut HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    match stmt {
        Statement::LetBinding {
            name,
            mutable: is_mut,
            ..
        } => {
            // `let` (mutable or not) rebinds the name in the flat env;
            // only `let mut` makes it assignable. TW: execution.rs:977-988.
            if *is_mut {
                mutable.insert(name.clone());
            }
        }
        Statement::Assign { name, span, .. } => {
            if !mutable.contains(name) {
                errors.push(SpannedError::at(
                    immutability_error_text(name),
                    span.clone(),
                ));
            }
            // The assigned value expression is walked by check_stmt_exprs.
        }
        // Recurse into every nested block with the SAME set — flat
        // function-level model, mirrors eval_statements_cf exactly.
        Statement::Each { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::EachWithIndex { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::While { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::IfThen { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::IfElseBlock {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_stmts_mutability(then_body, mutable, errors);
            for (_, body) in else_ifs {
                check_stmts_mutability(body, mutable, errors);
            }
            if let Some(else_body) = else_body {
                check_stmts_mutability(else_body, mutable, errors);
            }
        }
        Statement::Match {
            arms, else_body, ..
        } => {
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        check_stmts_mutability(body, mutable, errors);
                    }
                    MatchArm::Compare(_, _, body) => {
                        check_stmts_mutability(body, mutable, errors);
                    }
                }
            }
            if let Some(else_body) = else_body {
                check_stmts_mutability(else_body, mutable, errors);
            }
        }
        Statement::Return { .. } | Statement::ExprStmt { .. } => {}
        // Наряд №266: memory statements never assign to variables — no
        // mutability implications.
        Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
        Statement::Break | Statement::Continue => {}
    }
}

// ── №617 (ledger gh#967 §3): the variable-scope walk + the static opaque-concat check ─────────────
//
// Three ledger-§3 tests (tests/phase19_22_constraints.rs) hung #[ignore]:
// (1) an opaque-typed operand in a `+` concat was refused only at RUNTIME
// (execution.rs eval_binop — "cannot concatenate opaque type …"), never at
// check time; (2) a read of an undefined variable passed `mlog check` and
// exploded at runtime; (3) an assignment to an undefined variable passed
// check silently (the №264 immutability pass resolves only `let mut`
// facts — it deliberately reports the immutability message for a
// never-declared name, the TW runtime behavior — it does NOT decide
// definedness). This pass closes all three, backend-agnostic (AST-level):
// the run/serve startup gate refuses on BOTH backends before any backend
// compiles anything.
//
// The scope model is the TW env model EXACTLY (eval_statements_cf):
//   - ONE flat set of bound names threaded through ALL nested blocks and
//     NEVER popped (execution.rs threads the same `env` into every block;
//     a `let` inside a branch stays visible for the rest of the body);
//   - pattern params are pre-bound; ROUTE bodies have no params (the TW
//     route env starts empty, server.rs:2587);
//   - entity declarations resolve as GLOBALS on every Ident read
//     (execution.rs: `env.get(name).or_else(self.variables.get(name))`)
//     — the map is built over the whole declaration list regardless of
//     source order;
//   - `each x in …` / `each i, item in …` loop variables bind like `let`
//     (env.insert per iteration, never popped);
//   - a LetBinding re-binds (shadows) the name — any inherited opaque
//     fact is replaced (env.insert semantics).
//
// The opaque facts tracked statically are the DECLARED ones only: entity
// type annotations (coerced by №114 at runtime), pattern param type
// annotations (resolved through the №119 alias map), and the let-chain
// (a binding whose initializer is a bare Ident of an opaque binding).
// Builtins returning opaque VALUES (escape_html → Html, query() → Query)
// are NOT inferred here — the registry's stage-0 Type carries opaque
// FAMILIES, not the concrete runtime type names the runtime guard
// prints; inventing a name would stamp an honest gap. The runtime guard
// stays the complete backstop (fail-closed) — this pass shifts the
// declared-fact class LEFT, it does not claim the whole class.
//
// Known honest divergence (loud, per the naryad's boundary rule): an
// assignment to a NEVER-declared name reports
// "assignment to undefined variable: n" at CHECK time (the ledger §3
// contract) while the TW runtime reports the №264 immutability message
// for the same program (the mutability check precedes resolution there —
// execution.rs:989-992). Both refuse fail-closed; the static diagnostic
// fires first and names the actual defect.

/// The flat variable scope threaded through one body (the TW env лекало).
struct VarScope {
    /// Every name a bare `Ident` may legally read.
    bound: HashSet<String>,
    /// №617: name → the DECLARED opaque type name (Html/Secret/…) — the
    /// static fact the opaque-concat check refuses on.
    opaque: HashMap<String, String>,
    /// №662 (ADR-0188 (б)): the program's template names (the №250
    /// registration set, collected from the checked AST declarations) —
    /// the unknown-template refusal reads it. Cloned at the body entry
    /// (once per body — the sets are small); no lifetime threading
    /// through the 55-site walk.
    templates: HashSet<String>,
    /// №662 (ADR-0188 (б)): pattern name → the RESOLVED param type names
    /// (the argument-position coercion rule reads it).
    pattern_param_types: HashMap<String, Vec<String>>,
    /// №685 (gh#1252, В44): template name → the RESOLVED positional param
    /// type names — the render() argument-position coercion rule (the
    /// №662 rule-1 twin for templates) reads it.
    template_param_types: HashMap<String, Vec<String>>,
}

/// Entry point for PATTERN bodies: params pre-bound, ALL entities as
/// globals (both kinds — the runtime resolves any entity name), the
/// opaque map carries only the opaque-typed subset.
#[allow(clippy::too_many_arguments)]
fn check_pattern_variables(
    body: &[Statement],
    params: &[crate::ast::Param],
    type_alias_map: &HashMap<String, String>,
    entity_opaque: &HashMap<String, String>,
    entity_names: &HashSet<String>,
    templates: &HashSet<String>,
    template_param_types: &HashMap<String, Vec<String>>,
    pattern_param_types: &HashMap<String, Vec<String>>,
    errors: &mut Vec<SpannedError>,
) {
    let mut scope = VarScope {
        bound: HashSet::new(),
        opaque: HashMap::new(),
        templates: templates.clone(),
        template_param_types: template_param_types.clone(),
        pattern_param_types: pattern_param_types.clone(),
    };
    for p in params {
        scope.bound.insert(p.name.clone());
        let resolved = type_alias_map
            .get(&p.type_name)
            .map(|s| s.as_str())
            .unwrap_or(&p.type_name);
        if crate::ast::is_opaque_type(resolved) {
            scope.opaque.insert(p.name.clone(), resolved.to_string());
        }
    }
    for name in entity_names {
        scope.bound.insert(name.clone());
    }
    for (name, ty) in entity_opaque {
        scope.opaque.insert(name.clone(), ty.clone());
    }
    check_stmts_vars(body, &mut scope, errors);
}

/// Entry point for ROUTE bodies: no params (the TW route env starts
/// empty — server.rs:2587), entities resolve as globals.
fn check_body_variables(
    body: &[Statement],
    entity_opaque: &HashMap<String, String>,
    entity_names: &HashSet<String>,
    templates: &HashSet<String>,
    template_param_types: &HashMap<String, Vec<String>>,
    pattern_param_types: &HashMap<String, Vec<String>>,
    errors: &mut Vec<SpannedError>,
) {
    let mut scope = VarScope {
        bound: HashSet::new(),
        opaque: HashMap::new(),
        templates: templates.clone(),
        template_param_types: template_param_types.clone(),
        pattern_param_types: pattern_param_types.clone(),
    };
    for name in entity_names {
        scope.bound.insert(name.clone());
    }
    for (name, ty) in entity_opaque {
        scope.opaque.insert(name.clone(), ty.clone());
    }
    check_stmts_vars(body, &mut scope, errors);
}

/// №662 (ADR-0188 option (б), rule 1): the String-SHAPED value — a
/// literal or a `+` chain (the concat shape the injection rides). The
/// deeper inference (an Ident of a String-typed binding, a call's return
/// type) stays out — the same depth limit the №617 walk documents.
fn expr_is_string_shaped(expr: &Expr) -> bool {
    // №510 posture: no wildcard — every Expr variant is named, a new
    // variant forces this list to grow consciously.
    match expr {
        Expr::StringLit { .. } => true,
        Expr::BinaryOp {
            op: crate::ast::BinOp::Add,
            ..
        } => true,
        Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::FieldAccess { .. }
        | Expr::FnCall { .. }
        | Expr::QualifiedCall { .. }
        | Expr::BinaryOp { .. }
        | Expr::IfElse { .. }
        | Expr::List { .. }
        | Expr::IndexAccess { .. }
        | Expr::StructLit { .. }
        | Expr::BlockIfElse { .. }
        | Expr::MatchExpr { .. }
        | Expr::Try { .. }
        | Expr::HandleSource { .. }
        | Expr::ProvBind { .. } => false,
    }
}

fn check_stmts_vars(stmts: &[Statement], scope: &mut VarScope, errors: &mut Vec<SpannedError>) {
    for stmt in stmts {
        check_stmt_vars(stmt, scope, errors);
    }
}

fn check_stmt_vars(stmt: &Statement, scope: &mut VarScope, errors: &mut Vec<SpannedError>) {
    match stmt {
        Statement::LetBinding { name, value, .. } => {
            check_expr_vars(value, scope, errors);
            // The binding overwrites any inherited fact (env.insert — the
            // flat shadowing semantics of the runtime).
            scope.opaque.remove(name);
            // The honest static opaque facts for the new binding: a bare
            // Ident of an opaque binding (the let-chain; deeper inference
            // stays out — see the module comment).
            if let Expr::Ident { name: src, .. } = value {
                if let Some(t) = scope.opaque.get(src) {
                    let t = t.clone();
                    scope.opaque.insert(name.clone(), t);
                }
            }
            scope.bound.insert(name.clone());
        }
        Statement::Assign {
            name, value, span, ..
        } => {
            check_expr_vars(value, scope, errors);
            if !scope.bound.contains(name) {
                // The ledger §3 contract: the assignment target must be
                // bound. (The TW runtime reports the №264 immutability
                // message for this shape — the divergence is documented
                // and loud, both refusals are fail-closed.)
                errors.push(
                    SpannedError::at(
                        format!("assignment to undefined variable: {}", name),
                        span.clone(),
                    )
                    .with_kind(SemanticErrorKind::UndefinedVariable),
                );
            }
            // The value expression is walked by check_expr_vars above.
        }
        Statement::Return { value: expr, .. } | Statement::ExprStmt { expr, .. } => {
            check_expr_vars(expr, scope, errors);
        }
        Statement::Each {
            variable,
            iterable,
            body,
            ..
        } => {
            check_expr_vars(iterable, scope, errors);
            scope.bound.insert(variable.clone());
            scope.opaque.remove(variable);
            check_stmts_vars(body, scope, errors);
        }
        Statement::EachWithIndex {
            index_var,
            item_var,
            iterable,
            body,
            ..
        } => {
            check_expr_vars(iterable, scope, errors);
            scope.bound.insert(index_var.clone());
            scope.bound.insert(item_var.clone());
            scope.opaque.remove(index_var);
            scope.opaque.remove(item_var);
            check_stmts_vars(body, scope, errors);
        }
        Statement::While {
            condition, body, ..
        } => {
            check_expr_vars(condition, scope, errors);
            check_stmts_vars(body, scope, errors);
        }
        Statement::IfThen {
            condition, body, ..
        } => {
            check_expr_vars(condition, scope, errors);
            check_stmts_vars(body, scope, errors);
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_expr_vars(condition, scope, errors);
            check_stmts_vars(then_body, scope, errors);
            for (cond, body) in else_ifs {
                check_expr_vars(cond, scope, errors);
                check_stmts_vars(body, scope, errors);
            }
            if let Some(else_body) = else_body {
                check_stmts_vars(else_body, scope, errors);
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_expr_vars(scrutinee, scope, errors);
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        check_stmts_vars(body, scope, errors);
                    }
                    MatchArm::Compare(_, expr, body) => {
                        check_expr_vars(expr, scope, errors);
                        check_stmts_vars(body, scope, errors);
                    }
                }
            }
            if let Some(else_body) = else_body {
                check_stmts_vars(else_body, scope, errors);
            }
        }
        Statement::Memorize(m) => check_expr_vars(&m.value, scope, errors),
        Statement::Forget(f) => check_expr_vars(&f.query, scope, errors),
        Statement::Relate(r) => {
            check_expr_vars(&r.from, scope, errors);
            check_expr_vars(&r.to, scope, errors);
        }
        Statement::Break | Statement::Continue => {}
    }
}

/// The NAME-REFERENCE argument positions (№617): the runtime treats a
/// bare `Ident` in these slots as a compile-time reference to a
/// top-level declaration — NEVER a variable read (execution.rs: render
/// №115 pushes the template name as a String; reflex_train/predict/
/// save/metrics/generate №179b/№180/№187/№193 resolve the model name
/// via reflex_names → Value::Reflex). The variable walk skips exactly
/// these slots; an undeclared reference is the runtime's own loud error
/// ("model 'X' not declared"), unchanged.
fn is_name_reference_builtin(name: &str) -> bool {
    matches!(
        name,
        "render"
            | crate::reflex_ops::NAME_REFLEX_TRAIN
            | crate::reflex_ops::NAME_REFLEX_PREDICT
            | crate::reflex_ops::NAME_REFLEX_SAVE
            | crate::reflex_ops::NAME_REFLEX_METRICS
            | crate::reflex_ops::NAME_REFLEX_GENERATE
            // №825 (ADR-0200): attest_verify(Model) — the same bare-Ident
            // name-reference form (the reflex-group marshaling shape; the
            // runtime resolves the model through reflex_names → Value::Reflex).
            | crate::builtins::NAME_ATTEST_VERIFY
    )
}

/// The reserved expression/statement keywords (№617): a bare `Ident`
/// node carrying one of these names is a PARSE ARTIFACT of the tolerated
/// legacy shapes (the self-host parser's `while if … {` / dangling-`else`
/// chains — self-host/parser.mlog), never a user variable reference: the
/// grammar reserves the words, so no legal program reads a variable
/// spelled `if`/`then`/… and the runtime never evaluates the artifact
/// nodes (parser.mlog runs green on main — the naryad_197 evidence).
/// The variable walk skips exactly these names; a REAL undefined
/// variable is never spelled with a reserved word, so the exemption
/// cannot mask the ledger §3 defect class. The artifact itself (the
/// expression parser accepting stray keyword tokens as bare Idents) is
/// a separate parser finding — recorded in the PR, not silently
/// swallowed here.
fn is_reserved_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "then"
            | "else"
            | "while"
            | "each"
            | "in"
            | "match"
            | "return"
            | "let"
            | "mut"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "and"
            | "or"
            | "not"
            | "try"
    )
}

fn check_expr_vars(expr: &Expr, scope: &mut VarScope, errors: &mut Vec<SpannedError>) {
    match expr {
        Expr::Ident { name, span } => {
            // №617: the keyword-ident exemption — see is_reserved_keyword.
            if is_reserved_keyword(name) {
                return;
            }
            if !scope.bound.contains(name) {
                // The TW runtime message verbatim (execution.rs), the
                // runtime's stable №479 code attached — the static refusal
                // and the runtime refusal share one class signature.
                errors.push(
                    SpannedError::at(format!("undefined variable: {}", name), span.clone())
                        .with_kind(SemanticErrorKind::UndefinedVariable),
                );
            }
        }
        Expr::FieldAccess { object, .. } => check_expr_vars(object, scope, errors),
        Expr::FnCall { name, args, span } => {
            // №662 (ADR-0188 option (б), rule 2): the unknown-template
            // refusal — a `render("Name", …)` whose statically-resolvable
            // name (a literal, or a bare Ident — the name-reference slot)
            // is absent from the collected template declarations. The
            // runtime message is mirrored verbatim (http.rs builtin_render);
            // a dynamic expr name stays the runtime's backstop.
            if name == "render" {
                if let Some(first) = args.first() {
                    // №510 posture: no wildcard — every Expr variant is
                    // named. ONLY a String literal is statically resolvable:
                    // a bare Ident is a RUNTIME-evaluated name (the leak
                    // suite's render(raw) passes the variable holding the
                    // name — the CI-verified false positive №662 fixed) —
                    // every non-literal form stays the runtime's backstop.
                    let wanted = match first {
                        Expr::StringLit { value, .. } => Some(value.clone()),
                        // a bare Ident = a RUNTIME-evaluated name (the
                        // ok_448 leak-flow shape) — the backstop, never a
                        // static refusal
                        Expr::Ident { .. } => None,
                        Expr::FloatLit { .. }
                        | Expr::BoolLit { .. }
                        | Expr::FieldAccess { .. }
                        | Expr::FnCall { .. }
                        | Expr::QualifiedCall { .. }
                        | Expr::BinaryOp { .. }
                        | Expr::IfElse { .. }
                        | Expr::List { .. }
                        | Expr::IndexAccess { .. }
                        | Expr::StructLit { .. }
                        | Expr::BlockIfElse { .. }
                        | Expr::MatchExpr { .. }
                        | Expr::Try { .. }
                        | Expr::HandleSource { .. }
                        | Expr::ProvBind { .. } => None,
                    };
                    if let Some(wanted) = wanted {
                        if !scope.templates.contains(&wanted) {
                            errors.push(
                                SpannedError::at(
                                    format!(
                                        "render(): unknown template '{}' — declare it with `template {}(...) -> Html {{ ... }}`",
                                        wanted, wanted
                                    ),
                                    span.clone(),
                                )
                                .with_kind(SemanticErrorKind::UnknownTemplate),
                            );
                        }
                        // №685 (gh#1252, В44): the argument-position
                        // coercion rule for render() — the №662 rule-1
                        // twin for templates. A String-SHAPED argument
                        // flowing into an Html-typed parameter (positional,
                        // args[1..] ↔ params) is the implicit coercion the
                        // runtime accepts silently (builtin_render takes
                        // Value::String for any param) — the exact hole the
                        // S-1 scenario rode for a raw parameter. The static
                        // refusal names the explicit producer; the runtime
                        // stays the backstop.
                        if let Some(param_types) = scope.template_param_types.get(&wanted) {
                            for (i, arg) in args.iter().skip(1).enumerate() {
                                if let Some(pt) = param_types.get(i) {
                                    if pt == "Html" && expr_is_string_shaped(arg) {
                                        errors.push(
                                            SpannedError::at(
                                                format!(
                                                    "implicit String→Html coercion in argument {} of render('{}') — build the value with an explicit Html producer (render)",
                                                    i + 1,
                                                    wanted
                                                ),
                                                arg.span().clone(),
                                            )
                                            .with_kind(SemanticErrorKind::ImplicitOpaqueCoercion),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // №662 (ADR-0188 option (б), rule 1, the ARGUMENT position):
            // a String-shaped argument flowing into a pattern param
            // declared with an opaque type — the implicit coercion the
            // runtime accepted silently (№114).
            if let Some(param_types) = scope.pattern_param_types.get(name) {
                for (i, arg) in args.iter().enumerate() {
                    if let Some(pt) = param_types.get(i) {
                        if crate::ast::is_opaque_type(pt) && expr_is_string_shaped(arg) {
                            errors.push(
                                SpannedError::at(
                                    format!(
                                        "implicit String→{} coercion in argument {} of {} — build the value with an explicit {} producer (render)",
                                        pt,
                                        i + 1,
                                        name,
                                        pt
                                    ),
                                    arg.span().clone(),
                                )
                                .with_kind(SemanticErrorKind::ImplicitOpaqueCoercion),
                            );
                        }
                    }
                }
            }
            for (i, arg) in args.iter().enumerate() {
                // №617: the name-reference slots (render/reflex_*) — a bare
                // Ident there is a declaration reference, not a variable.
                if i == 0 && is_name_reference_builtin(name) {
                    continue;
                }
                check_expr_vars(arg, scope, errors);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for arg in args {
                check_expr_vars(arg, scope, errors);
            }
        }
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            // №617: the static twin of the runtime concat guard —
            // execution.rs eval_binop refuses Add on an opaque VALUE with
            // this exact message. The static check fires on the DECLARED
            // opaque fact before the program runs.
            if matches!(op, crate::ast::BinOp::Add) {
                for side in [left, right] {
                    if let Expr::Ident { name, span } = side.as_ref() {
                        if let Some(t) = scope.opaque.get(name) {
                            errors.push(
                                SpannedError::at(
                                    format!("cannot concatenate opaque type {}", t),
                                    span.clone(),
                                )
                                .with_kind(SemanticErrorKind::OpaqueConcat),
                            );
                        }
                    }
                }
            }
            check_expr_vars(left, scope, errors);
            check_expr_vars(right, scope, errors);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_expr_vars(condition, scope, errors);
            check_expr_vars(then_branch, scope, errors);
            check_expr_vars(else_branch, scope, errors);
        }
        Expr::List { items, .. } => {
            for item in items {
                check_expr_vars(item, scope, errors);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            check_expr_vars(object, scope, errors);
            check_expr_vars(index, scope, errors);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_expr_vars(v, scope, errors);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_expr_vars(condition, scope, errors);
            check_stmts_vars(then_body, scope, errors);
            for (cond, body) in else_ifs {
                check_expr_vars(cond, scope, errors);
                check_stmts_vars(body, scope, errors);
            }
            if let Some(else_body) = else_body {
                check_stmts_vars(else_body, scope, errors);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_expr_vars(scrutinee, scope, errors);
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        check_stmts_vars(body, scope, errors);
                    }
                    MatchArm::Compare(_, expr, body) => {
                        check_expr_vars(expr, scope, errors);
                        check_stmts_vars(body, scope, errors);
                    }
                }
            }
            if let Some(else_body) = else_body {
                check_stmts_vars(else_body, scope, errors);
            }
        }
        Expr::Try { expr: inner, .. } => check_expr_vars(inner, scope, errors),
        Expr::ProvBind { inner, .. } => check_expr_vars(inner, scope, errors),
        // Literals carry no idents; HandleSource's `origin` names a
        // declared origin (the ADR-0164 rule), not a variable.
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::HandleSource { .. } => {}
    }
}

// ── №802 (gh#1549, волна 65, P2 — the C6 R-b 2 static check) ─────────
//
// ADR-0198 (№792) accepted the error model: `Result<T>` is the type of
// the `try` result (`Struct{ok, value, error}`, ADR-0142), `Option<T>`
// is the spelling of `T|Unit` — the ALIASES of the existing shapes.
// Nothing stopped a program from reading `r.value` without ever
// checking `r.ok`, or from using a `T|Unit` value where `T` is
// required — the failure surfaced only at RUNTIME. This pass shifts
// BOTH refusals left to the shared semantic pass (backend-independent
// by construction — the №673 contour shape): every `mlog check`, the
// №523 run gate and the serve/mcp-serve startup gates refuse the
// class on BOTH backends.
//
// The shape (the registry-task wording, verbatim):
//   1. the flow-sensitive check: `.value` on a try-result outside a
//      branch where `.ok` was proven — [UNCHECKED_RESULT]; a `T|Unit`
//      value used in a `T` position without the Unit handling —
//      [UNCHECKED_OPTION];
//   2. CONSERVATIVE: when the guard cannot be proven, the error fires,
//      and the explicit suppression comment
//      `// allow(unchecked_result): <reason>` (visible at review)
//      is the sanctioned escape — a BARE allow (no reason) is NOT a
//      suppression (the honest-spelling rule, pinned);
//   3. the Result-ness comes ONLY from the `try` binding (a row
//      struct's `.value` field never fires — the p7-compat negative).
//
// The guard grammar the walker PROVES (everything else is
// "unsure" → error → suppress):
//   - `if r.ok { ... }` — the then-branch is checked;
//   - `if r.ok == true` / `true == r.ok` — the then-branch;
//   - `if r.ok == false { return ... }` — the code AFTER the if is
//     checked (the early-exit dominance; a top-level return/break/
//     continue in the branch proves the exit);
//   - `if not(r.ok) { return ... }` — the same dominance form;
//   - `A and B` — the right side is walked under A's then-facts
//     (`and` short-circuits on BOTH backends — the execution.rs fact);
//   - `match r.ok { "true" then { ... } }` — the "true" arm is checked;
//   - `if type_of(v) == "Unit" { return ... }` — after the if the
//     Unit arm is excluded; `type_of(v) != "Unit"` — the then-branch.
// The Option T-positions the walker refuses: the arithmetic/ordering
// binary operands (Add/Sub/Mul/Div/Gt/Lt/Ge/Le), the `each` iterable,
// the string-key index access, and the declared-pattern arguments
// whose parameter spelling is a precise (non-union, non-Fluid) type.
// `to_string` is the sanctioned total projection — never a T position.
// The builtin ARGUMENT positions carry no param types in the registry
// (the №467 stage-0 SSOT records only the returns) — that lane stays
// out of this naryad's refusal surface, honestly (recorded in the
// report; widening it is a registry decision, not a walker guess).

/// №802: the stable code of the unchecked try-result read.
pub const CODE_UNCHECKED_RESULT: &str = "UNCHECKED_RESULT";

/// №802: the stable code of the unchecked `T|Unit` use.
pub const CODE_UNCHECKED_OPTION: &str = "UNCHECKED_OPTION";

/// The declared surface of the plain-name callables: the RAW return
/// spelling (the union arms live in the string — the №740 erasure
/// keeps `Type::Union` a flat tag) and the RAW parameter spellings.
struct UncheckedCallables {
    returns_raw: std::collections::HashMap<String, String>,
    params_raw: std::collections::HashMap<String, Vec<String>>,
}

#[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the collect maps only the plain-name callables' declared surfaces (pattern/learnable); every other declaration contributes no callable — a future callable-bearing declaration joins this match at the next audit
impl UncheckedCallables {
    fn collect(declarations: &[Declaration]) -> Self {
        let mut map = Self {
            returns_raw: std::collections::HashMap::new(),
            params_raw: std::collections::HashMap::new(),
        };
        for d in declarations {
            match d {
                Declaration::Pattern(p) => {
                    map.returns_raw
                        .insert(p.name.clone(), p.return_type.clone());
                    map.params_raw.insert(
                        p.name.clone(),
                        p.params.iter().map(|q| q.type_name.clone()).collect(),
                    );
                }
                Declaration::LearnablePattern(l) => {
                    map.returns_raw
                        .insert(l.name.clone(), l.return_type.clone());
                    map.params_raw.insert(
                        l.name.clone(),
                        l.params.iter().map(|q| q.type_name.clone()).collect(),
                    );
                }
                _ => {}
            }
        }
        map
    }
}

/// The TOP-LEVEL arms of a union spelling — the `|` split that respects
/// the angle-bracket nesting (`List<Float|String|Unit>` has ONE
/// top-level arm, the List head; its inner unions are element facts,
/// not the variable's shape).
fn union_arms_top(raw: &str) -> Vec<String> {
    let mut arms = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in raw.chars() {
        match ch {
            '<' => {
                depth += 1;
                cur.push(ch);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            '|' if depth == 0 => {
                arms.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    arms.push(cur.trim().to_string());
    arms
}

/// The ADR-0198 Option shape: a MULTI-ARM union whose top-level arms
/// include `Unit`. A bare `Unit` return is not an Option (nothing to
/// unwrap); a parameterized `List<...|Unit>` is not a top-level Option.
fn raw_is_option(raw: &str) -> bool {
    let arms = union_arms_top(raw);
    arms.len() > 1 && arms.iter().any(|a| a.eq_ignore_ascii_case("Unit"))
}

/// Whether the builtin's registry row returns an Option shape (the
/// `return_type_raw` SSOT — the same literal `from_path` consumed).
fn builtin_returns_option(name: &str) -> bool {
    crate::builtins::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .map(|s| raw_is_option(s.return_type_raw))
        .unwrap_or(false)
}

/// The flow state of one straight-line block.
#[derive(Clone, Default)]
struct UncheckedFlow {
    /// var → the `.ok` guard proven? (present = bound to a try result)
    try_vars: std::collections::HashMap<String, bool>,
    /// var → the Unit arm excluded? (present = bound to an Option shape)
    opt_vars: std::collections::HashMap<String, bool>,
}

/// The guard facts one condition proves for the branches of its `if`.
#[derive(Default, Clone)]
struct GuardFacts {
    try_then: Vec<String>,
    try_else: Vec<String>,
    opt_then: Vec<String>,
    opt_else: Vec<String>,
    /// The then-branch proves the NEGATED side (`not(r.ok)`,
    /// `r.ok == false`, `type_of(v) == "Unit"`) — a top-level early exit
    /// there makes the code AFTER the if carry the else-facts.
    negated: bool,
}

impl GuardFacts {
    fn is_empty(&self) -> bool {
        self.try_then.is_empty()
            && self.try_else.is_empty()
            && self.opt_then.is_empty()
            && self.opt_else.is_empty()
    }
}

/// Mark the tracked variables the names name (an untracked name's
/// guard is a no-op — the honesty of the narrow surface).
fn apply_facts(flow: &mut UncheckedFlow, try_names: &[String], opt_names: &[String]) {
    for n in try_names {
        if flow.try_vars.contains_key(n) {
            flow.try_vars.insert(n.clone(), true);
        }
    }
    for n in opt_names {
        if flow.opt_vars.contains_key(n) {
            flow.opt_vars.insert(n.clone(), true);
        }
    }
}

/// The eq/ne guard shapes shared by both orientations
/// (`r.ok == true` / `true == r.ok`).
fn eq_guard_facts(probe: &Expr, lit: &Expr, eq: bool) -> Option<GuardFacts> {
    match (probe, lit) {
        (Expr::FieldAccess { object, field, .. }, Expr::BoolLit { value, .. }) if field == "ok" => {
            let Expr::Ident { name, .. } = object.as_ref() else {
                return None;
            };
            let true_side = *value == eq;
            Some(if true_side {
                GuardFacts {
                    try_then: vec![name.clone()],
                    ..Default::default()
                }
            } else {
                GuardFacts {
                    try_else: vec![name.clone()],
                    negated: true,
                    ..Default::default()
                }
            })
        }
        (Expr::FnCall { name, args, .. }, Expr::StringLit { value, .. })
            if name == "type_of" && args.len() == 1 && value == "Unit" =>
        {
            let Expr::Ident { name: v, .. } = &args[0] else {
                return None;
            };
            Some(if eq {
                // type_of(v) == "Unit": the then-branch IS the Unit arm
                GuardFacts {
                    opt_else: vec![v.clone()],
                    negated: true,
                    ..Default::default()
                }
            } else {
                // type_of(v) != "Unit": the then-branch excludes Unit
                GuardFacts {
                    opt_then: vec![v.clone()],
                    ..Default::default()
                }
            })
        }
        _ => None,
    }
}

/// The guard facts of one condition (the PROVEN shapes only).
#[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the proven guard shapes are enumerated; every future Expr variant lands in the catch-all as UNPROVABLE (the conservative posture — an unrecognized condition never marks a branch checked), never as silently proven
fn guard_facts(cond: &Expr) -> GuardFacts {
    match cond {
        Expr::FieldAccess { object, field, .. } if field == "ok" => {
            if let Expr::Ident { name, .. } = object.as_ref() {
                GuardFacts {
                    try_then: vec![name.clone()],
                    ..Default::default()
                }
            } else {
                GuardFacts::default()
            }
        }
        Expr::BinaryOp {
            left,
            op: BinOp::And,
            ..
        } => {
            // `A and B`: the then-branch proves A's then-facts (B was
            // evaluated too, which presupposes A on BOTH backends).
            guard_facts(left)
        }
        Expr::BinaryOp {
            left,
            op: BinOp::Or,
            right,
            ..
        } => {
            // `A or B`: the else-branch proves BOTH arms false.
            let mut a = guard_facts(left);
            let b = guard_facts(right);
            a.try_else.extend(b.try_else);
            a.opt_else.extend(b.opt_else);
            a
        }
        Expr::BinaryOp {
            left,
            op: op @ (BinOp::Eq | BinOp::Ne),
            right,
            ..
        } => {
            let eq = matches!(op, BinOp::Eq);
            for (probe, lit) in [
                (left.as_ref(), right.as_ref()),
                (right.as_ref(), left.as_ref()),
            ] {
                if let Some(f) = eq_guard_facts(probe, lit, eq) {
                    return f;
                }
            }
            GuardFacts::default()
        }
        Expr::FnCall { name, args, .. } if name == "not" && args.len() == 1 => {
            let mut f = guard_facts(&args[0]);
            if !f.is_empty() {
                std::mem::swap(&mut f.try_then, &mut f.try_else);
                std::mem::swap(&mut f.opt_then, &mut f.opt_else);
                f.negated = true;
            }
            f
        }
        // №802 tail: everything unenumerated is honestly "unprovable"
        // (an unrecognized condition never marks a branch checked).
        _ => GuardFacts::default(),
    }
}

/// A top-level return/break/continue in the branch body proves the
/// branch exits (the dominance fact for the negated-guard form).
fn branch_exits(body: &[Statement]) -> bool {
    body.iter().any(|st| {
        matches!(
            st,
            Statement::Return { .. } | Statement::Break | Statement::Continue
        )
    })
}

/// Rebind one variable per its initializer (the runtime-follow rule:
/// a fresh binding carries what the initializer proves — a non-try,
/// non-union initializer drops the old knowledge).
#[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the initializer shapes that PROVE a type are enumerated (Try/Ident-copy/FnCall); everything else (literals, field/index access, struct literals…) honestly proves nothing — the variable stays untracked, the conservative direction
fn bind_unchecked(
    name: &str,
    value: &Expr,
    flow: &mut UncheckedFlow,
    callables: &UncheckedCallables,
) {
    flow.try_vars.remove(name);
    flow.opt_vars.remove(name);
    match value {
        Expr::Try { .. } => {
            flow.try_vars.insert(name.to_string(), false);
        }
        Expr::Ident { name: src, .. } => {
            if let Some(checked) = flow.try_vars.get(src).copied() {
                flow.try_vars.insert(name.to_string(), checked);
            } else if let Some(handled) = flow.opt_vars.get(src).copied() {
                flow.opt_vars.insert(name.to_string(), handled);
            }
        }
        Expr::FnCall { name: callee, .. } => {
            let declared_option = callables
                .returns_raw
                .get(callee)
                .map(|raw| raw_is_option(raw))
                .unwrap_or(false);
            if declared_option || builtin_returns_option(callee) {
                flow.opt_vars.insert(name.to_string(), false);
            }
        }
        _ => {}
    }
}

/// The precise-parameter predicate: the declared spelling parses to a
/// comparable precise type (labels erased, Fluid/Union/Unknown silent).
fn declared_param_precise(raw: &str) -> bool {
    #[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the precise arm is the negation of the three silent shapes (Fluid/Unknown/Union) plus the label erasure — a future Type variant defaults to PRECISE (fires), the fail-closed direction
    fn comparable(ty: &crate::builtins::sig_types::Type) -> bool {
        match ty {
            crate::builtins::sig_types::Type::Labeled(inner, _) => comparable(inner),
            crate::builtins::sig_types::Type::Fluid
            | crate::builtins::sig_types::Type::Unknown
            | crate::builtins::sig_types::Type::Union => false,
            _ => true,
        }
    }
    comparable(&crate::builtins::sig_types::parse_type(raw))
}

#[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the leaf variants (StringLit/FloatLit/BoolLit/Ident/HandleSource) land in the catch-all as no-ops — they carry no sub-expressions and cannot hide a read; a future CONTAINER variant must join the walk loudly
fn walk_expr_unchecked(
    expr: &Expr,
    flow: &UncheckedFlow,
    callables: &UncheckedCallables,
    errors: &mut Vec<SpannedError>,
) {
    match expr {
        Expr::FieldAccess {
            object,
            field,
            span,
        } => {
            if let Expr::Ident { name, .. } = object.as_ref() {
                if field == "value" {
                    if flow.try_vars.get(name) == Some(&false) {
                        errors.push(SpannedError::at(
                            format!(
                                "[UNCHECKED_RESULT] the .value of '{}' is read outside a proven .ok guard — the failed try's value is Unit (the ADR-0142 Struct{{ok, value, error}}): check `if r.ok {{ ... }}` (or early-exit on `r.ok == false`) before reading .value, or suppress with a reason: // allow(unchecked_result): <reason> (naryad №802, ADR-0198)",
                                name
                            ),
                            span.clone(),
                        ));
                    }
                    return; // the Ident object needs no descent
                }
            }
            walk_expr_unchecked(object, flow, callables, errors);
        }
        Expr::IndexAccess {
            object,
            index,
            span,
        } => {
            if let (Expr::Ident { name, .. }, Expr::StringLit { value: key, .. }) =
                (object.as_ref(), index.as_ref())
            {
                if key == "value" && flow.try_vars.get(name) == Some(&false) {
                    errors.push(SpannedError::at(
                        format!(
                            "[UNCHECKED_RESULT] the [\"value\"] read of '{}' is the same unchecked getfield (the failed try's value is Unit): check the .ok guard first, or suppress with a reason: // allow(unchecked_result): <reason> (naryad №802)",
                            name
                        ),
                        span.clone(),
                    ));
                    return;
                }
                if flow.opt_vars.get(name) == Some(&false) {
                    errors.push(SpannedError::at(
                        format!(
                            "[UNCHECKED_OPTION] '{}' may be Unit (the union spelling carries a Unit arm — the ADR-0198 Option<T>) and is indexed where a List is required: handle the Unit arm first (`if type_of(v) == \"Unit\" {{ ... }}`), or suppress with a reason: // allow(unchecked_option): <reason> (naryad №802)",
                            name
                        ),
                        span.clone(),
                    ));
                    return;
                }
            }
            walk_expr_unchecked(object, flow, callables, errors);
            walk_expr_unchecked(index, flow, callables, errors);
        }
        Expr::BinaryOp {
            left,
            op,
            right,
            span,
        } => {
            match op {
                BinOp::And => {
                    walk_expr_unchecked(left, flow, callables, errors);
                    let facts = guard_facts(left);
                    if facts.is_empty() {
                        walk_expr_unchecked(right, flow, callables, errors);
                    } else {
                        let mut rf = flow.clone();
                        apply_facts(&mut rf, &facts.try_then, &facts.opt_then);
                        walk_expr_unchecked(right, &rf, callables, errors);
                    }
                }
                BinOp::Eq | BinOp::Ne => {
                    walk_expr_unchecked(left, flow, callables, errors);
                    walk_expr_unchecked(right, flow, callables, errors);
                }
                _ => {
                    // The arithmetic/ordering T positions: a Unit arm
                    // refuses loudly at runtime on BOTH backends.
                    walk_expr_unchecked(left, flow, callables, errors);
                    walk_expr_unchecked(right, flow, callables, errors);
                    for operand in [left.as_ref(), right.as_ref()] {
                        if let Expr::Ident { name, .. } = operand {
                            if flow.opt_vars.get(name) == Some(&false) {
                                errors.push(SpannedError::at(
                                    format!(
                                        "[UNCHECKED_OPTION] '{}' may be Unit (the union spelling carries a Unit arm — the ADR-0198 Option<T>) and is used where a value is required ('{}'): handle the Unit arm first (`if type_of(v) == \"Unit\" {{ ... }}`), project through to_string, or suppress with a reason: // allow(unchecked_option): <reason> (naryad №802)",
                                        name, op
                                    ),
                                    operand.span().clone(),
                                ));
                            }
                        }
                    }
                    let _ = span;
                }
            }
        }
        Expr::FnCall { name, args, .. } => {
            for a in args {
                walk_expr_unchecked(a, flow, callables, errors);
            }
            // The declared-parameter T positions: a union-with-Unit
            // argument in a precise (non-union) declared param.
            if let Some(params) = callables.params_raw.get(name) {
                for (i, a) in args.iter().enumerate() {
                    let Some(p) = params.get(i) else { continue };
                    if !declared_param_precise(p) {
                        continue;
                    }
                    if let Expr::Ident { name: v, .. } = a {
                        if flow.opt_vars.get(v) == Some(&false) {
                            errors.push(SpannedError::at(
                                format!(
                                    "[UNCHECKED_OPTION] '{}' may be Unit (the union spelling carries a Unit arm) and is passed to the declared '{}' parameter of '{}': handle the Unit arm first, or suppress with a reason: // allow(unchecked_option): <reason> (naryad №802)",
                                    v, p, name
                                ),
                                a.span().clone(),
                            ));
                        }
                    }
                }
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                walk_expr_unchecked(a, flow, callables, errors);
            }
        }
        Expr::List { items, .. } => {
            for i in items {
                walk_expr_unchecked(i, flow, callables, errors);
            }
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_expr_unchecked(v, flow, callables, errors);
            }
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr_unchecked(condition, flow, callables, errors);
            let facts = guard_facts(condition);
            let mut then_flow = flow.clone();
            apply_facts(&mut then_flow, &facts.try_then, &facts.opt_then);
            walk_expr_unchecked(then_branch, &then_flow, callables, errors);
            let mut else_flow = flow.clone();
            apply_facts(&mut else_flow, &facts.try_else, &facts.opt_else);
            walk_expr_unchecked(else_branch, &else_flow, callables, errors);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            // The expression-form block-if: the branches are walked for
            // their findings, but the outer flow is NOT rejoined (the
            // expression context holds an immutable flow; a branch may
            // not run at all — the conservative posture, the stage-1
            // cloned-env discipline).
            let ei: Vec<(&Expr, &Vec<Statement>)> =
                else_ifs.iter().map(|(c, b)| (c.as_ref(), b)).collect();
            let mut branch_flow = flow.clone();
            walk_if_block_unchecked(
                condition,
                then_body,
                &ei,
                else_body.as_ref(),
                &mut branch_flow,
                callables,
                errors,
            );
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_unchecked(scrutinee, flow, callables, errors);
            let ok_probe = match_arm_ok_var(scrutinee);
            for arm in arms {
                let mut af = flow.clone();
                if let (Some(v), MatchArm::Exact(s, _)) = (ok_probe.as_ref(), arm) {
                    if s == "true" {
                        apply_facts(&mut af, std::slice::from_ref(v), &[]);
                    }
                }
                for st in arm.body() {
                    walk_stmt_unchecked(st, &mut af, callables, errors);
                }
            }
            if let Some(eb) = else_body {
                let mut ef = flow.clone();
                walk_stmts_unchecked(eb, &mut ef, callables, errors);
            }
        }
        Expr::Try { expr, .. } => {
            walk_expr_unchecked(expr, flow, callables, errors);
        }
        Expr::ProvBind { inner, .. } => {
            walk_expr_unchecked(inner, flow, callables, errors);
        }
        // №802 tail: the leaf shapes carry no sub-expressions and no
        // access sites — nothing to walk; a future CONTAINER variant
        // must join the walk loudly.
        _ => {}
    }
}

/// The try-var an `r.ok` probe names (the match-on-ok guard support).
fn match_arm_ok_var(scrutinee: &Expr) -> Option<String> {
    if let Expr::FieldAccess { object, field, .. } = scrutinee {
        if field == "ok" {
            if let Expr::Ident { name, .. } = object.as_ref() {
                return Some(name.clone());
            }
        }
    }
    None
}

/// The shared if/else-block walk (the statement form and the
/// BlockIfElse expression share the exact branch discipline). The
/// else-if chain arrives as borrowed pairs — the two AST forms carry
/// `(Expr, Vec<Statement>)` vs `(Box<Expr>, Vec<Statement>)`.
fn walk_if_block_unchecked(
    condition: &Expr,
    then_body: &[Statement],
    else_ifs: &[(&Expr, &Vec<Statement>)],
    else_body: Option<&Vec<Statement>>,
    flow: &mut UncheckedFlow,
    callables: &UncheckedCallables,
    errors: &mut Vec<SpannedError>,
) {
    walk_expr_unchecked(condition, flow, callables, errors);
    let facts = guard_facts(condition);
    let mut then_flow = flow.clone();
    apply_facts(&mut then_flow, &facts.try_then, &facts.opt_then);
    walk_stmts_unchecked(then_body, &mut then_flow, callables, errors);
    let mut else_flow = flow.clone();
    apply_facts(&mut else_flow, &facts.try_else, &facts.opt_else);
    for (cond, body) in else_ifs {
        // The else-if chain: each condition is evaluated in the
        // pre-state (conservative — the earlier branches may have
        // run); its body carries its own guard facts.
        walk_expr_unchecked(cond, flow, callables, errors);
        let ef = guard_facts(cond);
        let mut chain_flow = flow.clone();
        apply_facts(&mut chain_flow, &ef.try_then, &ef.opt_then);
        walk_stmts_unchecked(body, &mut chain_flow, callables, errors);
    }
    match else_body {
        Some(eb) => {
            walk_stmts_unchecked(eb, &mut else_flow, callables, errors);
            if else_ifs.is_empty() {
                // The intersection join: a variable is checked after
                // the if/else only when BOTH branches proved it.
                for (k, checked) in flow.try_vars.clone() {
                    let t = then_flow.try_vars.get(&k).copied();
                    let e = else_flow.try_vars.get(&k).copied();
                    if checked {
                        flow.try_vars.insert(k, t == Some(true) && e == Some(true));
                    }
                }
                for (k, handled) in flow.opt_vars.clone() {
                    let t = then_flow.opt_vars.get(&k).copied();
                    let e = else_flow.opt_vars.get(&k).copied();
                    if !handled {
                        flow.opt_vars.insert(k, t == Some(true) && e == Some(true));
                    }
                }
            }
        }
        None => {
            // No else: NO join (the if may skip; the branch's knowledge
            // must not leak). The negated-guard dominance applies —
            // `if r.ok == false { return … }` / `if type_of(v) == "Unit"
            // { return … }` make the code AFTER the if carry the
            // else-facts.
            if facts.negated && branch_exits(then_body) && else_ifs.is_empty() {
                apply_facts(flow, &facts.try_else, &facts.opt_else);
            }
        }
    }
}

fn walk_stmt_unchecked(
    st: &Statement,
    flow: &mut UncheckedFlow,
    callables: &UncheckedCallables,
    errors: &mut Vec<SpannedError>,
) {
    match st {
        Statement::LetBinding { name, value, .. } => {
            walk_expr_unchecked(value, flow, callables, errors);
            bind_unchecked(name, value, flow, callables);
        }
        Statement::Assign { name, value, .. } => {
            walk_expr_unchecked(value, flow, callables, errors);
            bind_unchecked(name, value, flow, callables);
        }
        Statement::IfThen {
            condition, body, ..
        } => {
            walk_expr_unchecked(condition, flow, callables, errors);
            let facts = guard_facts(condition);
            let mut body_flow = flow.clone();
            apply_facts(&mut body_flow, &facts.try_then, &facts.opt_then);
            walk_stmts_unchecked(body, &mut body_flow, callables, errors);
            // The negated-guard dominance: `if not(r.ok) { return ... }`
            // makes the code AFTER the if carry the else-facts.
            if facts.negated && branch_exits(body) {
                apply_facts(flow, &facts.try_else, &facts.opt_else);
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let ei: Vec<(&Expr, &Vec<Statement>)> = else_ifs.iter().map(|(c, b)| (c, b)).collect();
            walk_if_block_unchecked(
                condition,
                then_body,
                &ei,
                else_body.as_ref(),
                flow,
                callables,
                errors,
            );
        }
        Statement::While {
            condition, body, ..
        } => {
            walk_expr_unchecked(condition, flow, callables, errors);
            let mut body_flow = flow.clone();
            walk_stmts_unchecked(body, &mut body_flow, callables, errors);
            // The loop may run zero times: the body's knowledge does
            // not leak (the stage-1 discipline).
        }
        Statement::Each {
            variable,
            iterable,
            body,
            ..
        }
        | Statement::EachWithIndex {
            item_var: variable,
            iterable,
            body,
            ..
        } => {
            // The iterable is a T position (a List is required).
            if let Expr::Ident { name, .. } = iterable {
                if flow.opt_vars.get(name) == Some(&false) {
                    errors.push(SpannedError::at(
                        format!(
                            "[UNCHECKED_OPTION] '{}' may be Unit (the union spelling carries a Unit arm) and is iterated where a List is required: handle the Unit arm first, or suppress with a reason: // allow(unchecked_option): <reason> (naryad №802)",
                            name
                        ),
                        iterable.span().clone(),
                    ));
                }
            }
            let _ = variable;
            walk_expr_unchecked(iterable, flow, callables, errors);
            let mut body_flow = flow.clone();
            walk_stmts_unchecked(body, &mut body_flow, callables, errors);
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_unchecked(scrutinee, flow, callables, errors);
            let ok_probe = match_arm_ok_var(scrutinee);
            for arm in arms {
                let mut af = flow.clone();
                if let (Some(v), MatchArm::Exact(s, _)) = (ok_probe.as_ref(), arm) {
                    if s == "true" {
                        apply_facts(&mut af, std::slice::from_ref(v), &[]);
                    }
                }
                for st in arm.body() {
                    walk_stmt_unchecked(st, &mut af, callables, errors);
                }
            }
            if let Some(eb) = else_body {
                let mut ef = flow.clone();
                walk_stmts_unchecked(eb, &mut ef, callables, errors);
            }
        }
        Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
            walk_expr_unchecked(value, flow, callables, errors);
        }
        Statement::Memorize(m) => walk_expr_unchecked(&m.value, flow, callables, errors),
        Statement::Forget(f) => walk_expr_unchecked(&f.query, flow, callables, errors),
        Statement::Relate(r) => {
            walk_expr_unchecked(&r.from, flow, callables, errors);
            walk_expr_unchecked(&r.to, flow, callables, errors);
        }
        Statement::Break | Statement::Continue => {}
    }
}

fn walk_stmts_unchecked(
    stmts: &[Statement],
    flow: &mut UncheckedFlow,
    callables: &UncheckedCallables,
    errors: &mut Vec<SpannedError>,
) {
    for st in stmts {
        walk_stmt_unchecked(st, flow, callables, errors);
    }
}

/// №802: the entry — walk every statement-bearing declaration body.
#[allow(clippy::wildcard_enum_match_arm)] // №802 tail: the walker descends only into the statement-bearing bodies (the deny-pass coverage set); the silent arm is the documented no-body set (learnable templates, flows, entity/schema declaratives) — a future declaration with statements JOINS this match at the next audit
pub fn check_unchecked_uses(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    let callables = UncheckedCallables::collect(declarations);
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut flow = UncheckedFlow::default();
                walk_stmts_unchecked(&p.body, &mut flow, &callables, errors);
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let mut flow = UncheckedFlow::default();
                    walk_stmts_unchecked(&m.body, &mut flow, &callables, errors);
                }
            }
            Declaration::Test(t) => {
                let mut flow = UncheckedFlow::default();
                walk_stmts_unchecked(&t.body, &mut flow, &callables, errors);
            }
            Declaration::Hook(h) => {
                let mut flow = UncheckedFlow::default();
                walk_stmts_unchecked(&h.body, &mut flow, &callables, errors);
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    let mut flow = UncheckedFlow::default();
                    walk_stmts_unchecked(&r.body, &mut flow, &callables, errors);
                }
            }
            Declaration::OnDeny(d) => {
                let mut flow = UncheckedFlow::default();
                walk_stmts_unchecked(&d.body, &mut flow, &callables, errors);
            }
            // Learnable patterns are prompt templates (no statement
            // body); flows are declarative; everything else carries no
            // user statements.
            _ => {}
        }
    }
}

/// №802: the suppression filter — removes the [UNCHECKED_RESULT] /
/// [UNCHECKED_OPTION] findings the explicit allow-comments sanction.
///
/// The MECHANISM is textual by necessity: the parser's expression
/// spans are the historical `Span::unknown()` across the board
/// (expr.rs carries zero `from_pest` calls — the recorded hole; the
/// span propagation is its own movement), so an error cannot name its
/// line. The filter instead matches the access TEXT: an allow-comment
/// line `// allow(unchecked_result): <reason>` sanctions the read of
/// the named variable ON THAT LINE (the trailing form) or ON THE NEXT
/// LINE (the dedicated-comment form). The reason must be non-empty —
/// a bare allow is NOT a suppression. The per-(code, variable) QUOTA
/// caps over-suppression: a comment's weight is the number of reads
/// on its ONE access line — several reads on the same line are
/// sanctioned as a unit, two adjacent access lines need two comments
/// (one comment cannot launder two lines). The
/// comments sit in the program text (visible at review, the naryad
/// wording); the multi-file honest limitation (an imported module's
/// identical access text) is recorded in the naryad report.
pub fn filter_suppressed_unchecked(errors: Vec<SpannedError>, source: &str) -> Vec<SpannedError> {
    /// The (code, variable) a finding names — `None` for foreign messages.
    fn unchecked_ref_of(message: &str) -> Option<(&'static str, String)> {
        let code = if message.contains("[UNCHECKED_RESULT]") {
            "unchecked_result"
        } else if message.contains("[UNCHECKED_OPTION]") {
            "unchecked_option"
        } else {
            return None;
        };
        // The first quoted name after the marker: 'r'.
        let start = message.match_indices('\'').next()?.0;
        let rest = &message[start + 1..];
        let end = rest.find('\'')?;
        let var = &rest[..end];
        if var.is_empty() || !var.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        Some((code, var.to_string()))
    }

    /// The access probe: HOW MANY reads of the variable the
    /// allow-comment on line `idx` sanctions. The preferred access
    /// line is the comment's OWN line (the trailing form); only when
    /// it carries no probe does the NEXT line count (the dedicated
    /// comment form) — one comment is one access LINE, so two
    /// adjacent access lines cannot share one comment (the
    /// multi-line laundering stays closed), while several reads on
    /// the SAME line are sanctioned as a unit (the specimen shape:
    /// `str(outer.value.ok)` + `outer.value.error.code` on one line).
    fn comment_weight(lines: &[&str], idx: usize, code: &str, var: &str) -> usize {
        fn probe_count(ln: &str, code: &str, var: &str) -> usize {
            if code == "unchecked_result" {
                let dot = format!("{}.value", var);
                if ln.contains(&dot) {
                    return ln.matches(&dot).count();
                }
                ln.matches(&format!("{}[", var)).count()
            } else {
                word_count(ln, var)
            }
        }
        if idx < lines.len() {
            let same = probe_count(lines[idx], code, var);
            if same > 0 {
                return same;
            }
        }
        if idx + 1 < lines.len() {
            probe_count(lines[idx + 1], code, var)
        } else {
            0
        }
    }

    /// The word-boundary occurrence count.
    fn word_count(line: &str, word: &str) -> usize {
        let chars: Vec<char> = line.chars().collect();
        let w: Vec<char> = word.chars().collect();
        if w.is_empty() || w.len() > chars.len() {
            return 0;
        }
        let mut n = 0;
        for i in 0..=(chars.len() - w.len()) {
            if chars[i..i + w.len()] == w[..] {
                let before_ok = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
                let j = i + w.len();
                let after_ok = j >= chars.len() || !(chars[j].is_alphanumeric() || chars[j] == '_');
                if before_ok && after_ok {
                    n += 1;
                }
            }
        }
        n
    }

    let lines: Vec<&str> = source.lines().collect();
    // The allow-comment lines: (line index, code) — the reason must be
    // non-empty (a bare allow is not a suppression).
    let token_of = |code: &str| {
        if code == "unchecked_result" {
            "allow(unchecked_result):"
        } else {
            "allow(unchecked_option):"
        }
    };
    let allow_lines: Vec<(usize, &'static str)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, ln)| {
            for code in ["unchecked_result", "unchecked_option"] {
                let token = token_of(code);
                if let Some(pos) = ln.find(token) {
                    if !ln[pos + token.len()..].trim().is_empty() {
                        return Some((i, code));
                    }
                }
            }
            None
        })
        .collect();

    // The suppression quota per (code, variable): the number of
    // matching allow-comments (one comment sanctions ONE site).
    let mut quota: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    let mut used: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    let mut kept = Vec::with_capacity(errors.len());
    for err in errors {
        match unchecked_ref_of(&err.message) {
            Some((code, var)) => {
                let key = (code.to_string(), var.clone());
                let q = *quota.entry(key.clone()).or_insert_with(|| {
                    allow_lines
                        .iter()
                        .filter(|(_i, c)| *c == code)
                        .map(|(i, _)| comment_weight(&lines, *i, code, &var))
                        .sum()
                });
                let u = used.entry(key).or_insert(0);
                if *u < q {
                    *u += 1; // the comment spends its sanction
                    continue;
                }
                kept.push(err);
            }
            None => kept.push(err),
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_program() {
        let source = r#"
            entity greeting: String = "Hello, Metalogos!"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_undefined_type() {
        let source = r#"
            entity m: UnknownType = { text: "hi" }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown type")));
    }

    #[test]
    fn test_adapt_target_not_found() {
        let source = r#"
            adapt NonExistent add_example("in", "out")
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("not found")));
    }

    #[test]
    fn test_duplicate_pattern() {
        let source = r#"
            pattern Foo(x: String) -> String { return x }
            pattern Foo(y: String) -> String { return y }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("duplicate pattern")));
    }

    // ── Phase 6 semantic tests ────────────────────────────────

    #[test]
    fn test_mlogserver_valid() {
        let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_mlogserver_unknown_middleware() {
        let source = r#"
mlogserver {
  middleware: [bogus_middleware]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown middleware")));
    }

    #[test]
    fn test_mlogserver_invalid_method() {
        let source = r#"
mlogserver {
  route "/" method=INVALID { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown HTTP method")));
    }

    #[test]
    fn test_mlogserver_warns_no_security_headers() {
        let source = r#"
mlogserver {
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok()); // Only warning, not error
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("security_headers")));
    }

    #[test]
    fn test_mlogserver_warns_no_csrf_with_post() {
        let source = r#"
mlogserver {
  middleware: [session, security_headers]
  route "/login" method=POST { return "OK" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert!(result.warnings.iter().any(|w| w.message.contains("csrf")));
    }

    #[test]
    fn test_template_valid() {
        let source = r#"
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_wrong_return_type() {
        let source = r#"
template Page(title: String) -> Secret {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("only Html is supported")));
    }

    // ── Наряд №238 (Vision R4.1): vision declaration semantic ─────────

    /// Valid plan-§3 example — no errors, no warnings (no false positives).
    #[test]
    fn test_vision_valid_program_ok() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
        assert!(
            result.warnings.is_empty(),
            "unexpected warnings: {:?}",
            result.warnings
        );
    }

    /// Block 3.2 negative: unknown model — semantic error naming the model
    /// and the known set.
    #[test]
    fn test_vision_unknown_model_is_semantic_error() {
        let source = r#"
vision "poster" {
  model: "flux-2"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        let hit = result
            .errors
            .iter()
            .find(|e| e.message.contains("unknown model 'flux-2'"))
            .expect("expected unknown-model error");
        assert!(
            hit.message.contains("z-image-turbo"),
            "got: {}",
            hit.message
        );
    }

    /// Block 3.2 negative: duplicate vision declaration name.
    #[test]
    fn test_vision_duplicate_name_is_semantic_error() {
        let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 1 policy: safe profile: fp16 }
vision "poster" { model: "z-image-turbo" steps: 8 width: 512 height: 512 seed: 2 policy: safe profile: fp8 }
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("duplicate vision declaration: poster")));
    }

    /// Block 3.2 negative: width not a multiple of 16.
    #[test]
    fn test_vision_width_not_multiple_of_16_is_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1025
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        let hit = result
            .errors
            .iter()
            .find(|e| e.message.contains("width = 1025 must be a multiple of 16"))
            .expect("expected width-contract error");
        assert!(
            hit.message.contains("vision 'poster'"),
            "got: {}",
            hit.message
        );
    }

    /// Block 3.2 negative: steps = 0 — semantic error (steps >= 1).
    #[test]
    fn test_vision_steps_zero_is_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 0
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("steps must be >= 1, got 0")));
    }

    /// Block 2.2: steps != 8 (but >= 1) is an audit-WARNING, not an error.
    #[test]
    fn test_vision_steps_not_8_is_warning_not_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 12
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(
            result.is_ok(),
            "steps=12 must not be an error; got: {:?}",
            result.errors
        );
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("steps = 12 != 8")));
    }
}
