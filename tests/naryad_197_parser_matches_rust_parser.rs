// ── tests/naryad_197_parser_matches_rust_parser.rs ─────────────────
// Наряд №197, Block 2 (Contract 2): structural equivalence.
//
// For a representative sample of existing .mlog files (covering the subset
// supported by Block 1), verify that the AST produced by `self-host/parser.mlog`
// is structurally equivalent to the AST produced by the original Rust parser
// (`src/parser/`).
//
// "Structural equivalence" means: same nodes, same fields, same order. It
// does NOT mean byte-exact internal representation — the two parsers are
// written in different languages (Metalogos vs Rust) and use different
// intermediate structures. The test normalises both to a single S-expr
// string format and compares them.
//
// The format (produced by parser.mlog and mirrored by this test's
// `rust_ast_to_sexpr` helper) is:
//
//   Declarations:
//     (PATTERN name=N params=[(PARAM p1 T1) (PARAM p2 T2)] ret=R body=[stmts])
//     (ENTITY_SIMPLE name=N type=T value=expr)
//     (FLOW name=N input_type=T source=expr pipeline=[S1 S2])
//     (IMPORT path=P [alias=A])
//
//   Statements:
//     (LET name expr) | (LET_MUT name expr) | (ASSIGN name expr)
//     (RETURN expr) | (EXPR_STMT expr) | (BREAK) | (CONTINUE)
//     (IF_BLOCK cond then_body [(cond body)...] else_body)
//     (IF_THEN cond body) | (WHILE cond body)
//     (EACH var iterable body) | (EACH_IDX idx var iterable body)
//
//   Expressions:
//     (STRING value) — value is escaped (parens, brackets, backslash, \n, \t)
//     (NUMBER value) — value is the literal token text (e.g. "42.0", "3.14")
//     (BOOL true|false) | (IDENT name)
//     (CALL name args...) | (QCALL base_expr func args...)
//     (FIELD obj field) | (INDEX obj idx)
//     (LIST items...) | (STRUCT (k:v)...)
//     (BINOP op left right) | (UNARY_MINUS expr) | (PAREN expr)
//     (IFELSE cond then else)

use std::process::Command;
use std::sync::OnceLock;

use metalogos::ast::{Declaration, Expr, FluidDecl, Param, PatternDecl, Statement, TypeAliasDecl};

static MLOG_BIN: OnceLock<String> = OnceLock::new();

fn mlog_bin() -> &'static str {
    MLOG_BIN.get_or_init(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("target")
            .join("debug")
            .join("mlog")
            .to_string_lossy()
            .into_owned()
    })
}

fn manifest_dir() -> &'static str {
    static DIR: OnceLock<String> = OnceLock::new();
    DIR.get_or_init(|| std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
}

// ── Rust AST → S-expr serializer (mirrors parser.mlog's output format) ──

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_float(f: f64) -> String {
    // Match parser.mlog's number formatting: the lexer emits the raw token
    // text, which is always a decimal like "42.0" or "3.14" (FLOAT_LITERAL
    // per grammar.pest requires DIGIT+ ~ DOT_CHAR ~ DIGIT+). Use {:?} which
    // gives the same "42.0" form for integer-valued floats.
    format!("{:?}", f)
}

fn binop_str(op: &metalogos::ast::BinOp) -> &'static str {
    use metalogos::ast::BinOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Gt => ">",
        Lt => "<",
        Ge => ">=",
        Le => "<=",
        Eq => "==",
        Ne => "!=",
        And => "and",
        Or => "or",
    }
}

fn expr_to_sexpr(e: &Expr) -> String {
    use metalogos::ast::Expr::*;
    match e {
        StringLit { value, .. } => format!("(STRING {})", escape_str(value)),
        FloatLit { value, .. } => format!("(NUMBER {})", format_float(*value)),
        BoolLit { value, .. } => format!("(BOOL {})", value),
        Ident { name, .. } => format!("(IDENT {})", name),
        FieldAccess { object, field, .. } => {
            format!("(FIELD {} {})", expr_to_sexpr(object), field)
        }
        FnCall { name, args, .. } => {
            let args_str = args.iter().map(expr_to_sexpr).collect::<Vec<_>>().join(" ");
            format!("(CALL {} {})", name, args_str)
        }
        QualifiedCall {
            module,
            function,
            args,
            ..
        } => {
            let args_str = args.iter().map(expr_to_sexpr).collect::<Vec<_>>().join(" ");
            format!("(QCALL (IDENT {}) {} {})", module, function, args_str)
        }
        BinaryOp {
            left, op, right, ..
        } => {
            format!(
                "(BINOP {} {} {})",
                binop_str(op),
                expr_to_sexpr(left),
                expr_to_sexpr(right)
            )
        }
        IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => format!(
            "(IFELSE {} {} {})",
            expr_to_sexpr(condition),
            expr_to_sexpr(then_branch),
            expr_to_sexpr(else_branch)
        ),
        List { items, .. } => {
            let items_str = items
                .iter()
                .map(expr_to_sexpr)
                .collect::<Vec<_>>()
                .join(" ");
            format!("(LIST {})", items_str)
        }
        IndexAccess { object, index, .. } => {
            format!("(INDEX {} {})", expr_to_sexpr(object), expr_to_sexpr(index))
        }
        StructLit { fields, .. } => {
            // Order fields by key for deterministic comparison.
            // parser.mlog parses fields in source order; HashMap iteration
            // is non-deterministic. Sort by key to get a stable comparison.
            let mut entries: Vec<(&String, &Expr)> = fields.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let fields_str = entries
                .iter()
                .map(|(k, v)| format!("({}:{})", k, expr_to_sexpr(v)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("(STRUCT {})", fields_str)
        }
        BlockIfElse { .. } => "(UNSUPPORTED block_if_else_expr)".to_string(),
        Try { expr, .. } => format!("(TRY {})", expr_to_sexpr(expr)),
    }
}

fn stmt_to_sexpr(s: &Statement) -> String {
    use metalogos::ast::Statement::*;
    match s {
        LetBinding {
            name,
            value,
            mutable,
            ..
        } => {
            let tag = if *mutable { "LET_MUT" } else { "LET" };
            format!("({} {} {})", tag, name, expr_to_sexpr(value))
        }
        Assign { name, value, .. } => {
            format!("(ASSIGN {} {})", name, expr_to_sexpr(value))
        }
        Each {
            variable,
            iterable,
            body,
            ..
        } => format!(
            "(EACH {} {} {})",
            variable,
            expr_to_sexpr(iterable),
            body.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
        ),
        EachWithIndex {
            index_var,
            item_var,
            iterable,
            body,
            ..
        } => format!(
            "(EACH_IDX {} {} {} {})",
            index_var,
            item_var,
            expr_to_sexpr(iterable),
            body.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
        ),
        While {
            condition, body, ..
        } => format!(
            "(WHILE {} {})",
            expr_to_sexpr(condition),
            body.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
        ),
        IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let then_str = then_body
                .iter()
                .map(stmt_to_sexpr)
                .collect::<Vec<_>>()
                .join(" ");
            let else_if_str = else_ifs
                .iter()
                .map(|(c, b)| {
                    format!(
                        "({} {})",
                        expr_to_sexpr(c),
                        b.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            let else_str = match else_body {
                Some(b) => b.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" "),
                None => "(UNIT)".to_string(),
            };
            format!(
                "(IF_BLOCK {} {} {} {})",
                expr_to_sexpr(condition),
                then_str,
                else_if_str,
                else_str
            )
        }
        IfThen {
            condition, body, ..
        } => format!(
            "(IF_THEN {} {})",
            expr_to_sexpr(condition),
            body.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
        ),
        Return { value, .. } => format!("(RETURN {})", expr_to_sexpr(value)),
        ExprStmt { expr, .. } => format!("(EXPR_STMT {})", expr_to_sexpr(expr)),
        Match { .. } => "(UNSUPPORTED match)".to_string(),
        Break => "(BREAK)".to_string(),
        Continue => "(CONTINUE)".to_string(),
    }
}

fn params_to_sexpr(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| format!("(PARAM {} {})", p.name, p.type_name))
        .collect::<Vec<_>>()
        .join(" ")
}

fn decl_to_sexpr(d: &Declaration) -> Option<String> {
    use Declaration::*;
    match d {
        Pattern(PatternDecl {
            name,
            params,
            return_type,
            body,
            ..
        }) => {
            let body_str = body.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ");
            Some(format!(
                "(PATTERN name={} params=[{}] ret={} body=[{}])",
                name,
                params_to_sexpr(params),
                return_type,
                body_str
            ))
        }
        EntitySimple(es) => Some(format!(
            "(ENTITY_SIMPLE name={} type={} value={})",
            es.name,
            es.type_name,
            expr_to_sexpr(&es.value)
        )),
        Flow(flow) => {
            let pipeline = flow.pipeline.join(" ");
            Some(format!(
                "(FLOW name={} input_type={} source={} pipeline=[{}])",
                flow.name,
                flow.input_type,
                expr_to_sexpr(&flow.source),
                pipeline
            ))
        }
        Import(imp) => {
            // parser.mlog emits `(IMPORT path=P)` without alias if no alias.
            match &imp.alias {
                Some(a) => Some(format!("(IMPORT path={} alias={})", imp.path, a)),
                None => Some(format!("(IMPORT path={})", imp.path)),
            }
        }
        // Declarations outside the supported subset (Block 1) are NOT
        // emitted by parser.mlog (it skips them via SkipDecl). Return None
        // to filter them out so they don't break the comparison.
        _ => None,
    }
}

fn rust_ast_to_sexpr(decls: &[Declaration]) -> String {
    decls
        .iter()
        .filter_map(decl_to_sexpr)
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Run parser.mlog on a file, return its AST output ────────────────

fn run_parser_mlog(target_file: &str) -> String {
    let parser_path = std::path::Path::new(manifest_dir()).join("self-host/parser.mlog");
    let output = Command::new(mlog_bin())
        .arg("run")
        .arg(&parser_path)
        .env("MLOG_PARSE_TARGET", target_file)
        .output()
        .expect("failed to spawn mlog process");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "mlog process failed parsing {}:\nstderr: {}",
            target_file, stderr
        );
    }

    String::from_utf8_lossy(&output.stdout).to_string()
}

// ── Representative sample of .mlog files ────────────────────────────
// These cover the Block 1 subset: pattern/entity_simple/flow/import
// declarations, let/let_mut/assign/if-then/if-else-block/while/each/
// return/expr-stmt statements, and the full expression grammar
// (or/and/compare/add/mul, unary-minus, calls, field access, lists,
// structs, if-else expressions, all literals).
//
// Files with constructs OUTSIDE Block 1 (reflex_*, mlogserver, template,
// schema, rule, learnable_pattern, etc.) are intentionally excluded —
// parser.mlog skips unsupported declarations via SkipDecl, and a
// structural comparison would have to account for that. The files here
// are all Block 1-clean.

fn sample_files() -> Vec<&'static str> {
    vec![
        "examples/m1_hello.mlog",
        "examples/p5_let_bindings.mlog",
        "examples/p5_strings.mlog",
        "examples/p5_while.mlog",
        "examples/p5_each.mlog",
        "examples/p5_if_else.mlog",
        "examples/p5_unary_minus.mlog",
        "examples/p5_let_if.mlog",
        "examples/p5_let_ifelse.mlog",
        "self-host/std/string.mlog",
        "self-host/std/math.mlog",
        "self-host/std/collections.mlog",
    ]
}

#[test]
fn naryad_197_parser_matches_rust_parser() {
    let project_dir = std::path::Path::new(manifest_dir());

    for sample in sample_files() {
        let path = project_dir.join(sample);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));

        // Get parser.mlog's output.
        let mlog_ast = run_parser_mlog(sample);
        let mlog_ast = mlog_ast.trim_end();

        // Get Rust parser's AST, converted to the same S-expr format.
        let rust_decls = metalogos::parser::parse(&source)
            .unwrap_or_else(|e| panic!("Rust parser failed on {}: {}", sample, e));
        let rust_ast = rust_ast_to_sexpr(&rust_decls);

        // Normalize both sides: strip trailing whitespace per line.
        let mlog_normalized: Vec<&str> = mlog_ast.lines().filter(|l| !l.is_empty()).collect();
        let rust_normalized: Vec<&str> = rust_ast.lines().filter(|l| !l.is_empty()).collect();

        assert_eq!(
            mlog_normalized.len(),
            rust_normalized.len(),
            "Declaration count mismatch for {}:\nparser.mlog produced {} decls, Rust parser produced {}.\n\
             parser.mlog output:\n{}\n\nRust output:\n{}",
            sample,
            mlog_normalized.len(),
            rust_normalized.len(),
            mlog_ast,
            rust_ast
        );

        // Compare each declaration line.
        for (i, (m, r)) in mlog_normalized
            .iter()
            .zip(rust_normalized.iter())
            .enumerate()
        {
            assert_eq!(
                m, r,
                "Declaration {} mismatch for {}:\n\
                 parser.mlog:  {}\n\
                 Rust parser: {}",
                i, sample, m, r
            );
        }
    }
}

// ── Used imports (avoid clippy warning) ─────────────────────────────
#[allow(dead_code)]
fn _unused_imports() {
    let _ = (
        TypeAliasDecl {
            span: metalogos::ast::Span::unknown(),
            alias: String::new(),
            target: String::new(),
        },
        FluidDecl {
            span: metalogos::ast::Span::unknown(),
            name: String::new(),
            variants: vec![],
        },
    );
}
