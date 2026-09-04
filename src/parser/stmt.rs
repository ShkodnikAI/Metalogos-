use pest::iterators::Pair;

use super::*;

pub(super) fn parse_params(pair: Pair<Rule>) -> Vec<Param> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::param)
        .map(|p| {
            let span = Span::from_pest(p.as_span());
            let children = children_of(&p);
            Param {
                span,
                name: find_child_str(&children, Rule::IDENT).unwrap_or_default(),
                type_name: find_child_str(&children, Rule::type_name).unwrap_or_default(),
            }
        })
        .collect()
}

pub(super) fn parse_pattern_body(pair: Pair<Rule>) -> Result<Vec<Statement>, ParseError> {
    pair.into_inner()
        .filter(|s| s.as_rule() == Rule::statement)
        .map(|s| parse_single_statement(s))
        .collect::<Result<_, _>>()
}

/// Parse a single statement from its rule pair.
pub(super) fn parse_single_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let children = children_of(&pair);
    // statement = { match_stmt | if_block_stmt | each_stmt | ... }
    // Наряд №14: match_stmt is now a proper AST statement
    if let Some(m_pair) = children.iter().find(|c| c.as_rule() == Rule::match_stmt) {
        return match parse_match_stmt(m_pair.clone()) {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        };
    }
    // NOTE: match_stmt previously was parsed as a regular expression — now has full AST support.
    if let Some(ib_pair) = children.iter().find(|c| c.as_rule() == Rule::if_block_stmt) {
        parse_if_block_stmt(ib_pair.clone())
    } else if let Some(each_pair) = children.iter().find(|c| c.as_rule() == Rule::each_stmt) {
        let each_children: Vec<Pair<Rule>> = each_pair.clone().into_inner().collect();
        // each_stmt: IDENT [COMMA IDENT] "in" expression { body }
        // "in" is a string literal — not in parse tree. Use index after IDENTs.
        let idents: Vec<String> = each_children
            .iter()
            .filter(|c| c.as_rule() == Rule::IDENT)
            .map(|c| pair_str(c))
            .collect();
        // After idents (1 or 2) and optional COMMA, next non-ignored child is the expression
        let ident_count = idents.len();
        let expr_idx = each_children
            .iter()
            .position(|c| c.as_rule() == Rule::expression)
            .unwrap_or_else(|| {
                // Fallback: skip IDENTs and COMMA, take next
                ident_count + idents.len().min(1)
            });
        let iterable = parse_expression(each_children[expr_idx].clone())?;
        let body: Vec<Statement> = each_children[expr_idx + 1..]
            .iter()
            .filter(|c| c.as_rule() == Rule::statement)
            .map(|c| parse_single_statement(c.clone()))
            .collect::<Result<_, _>>()?;
        if idents.len() == 2 {
            // each i, item in list { ... } — index + value
            // Desugar into: let _each_list = <iterable>; let i = 0; while i < len(_each_list) { let item = _each_list[i]; <body>; i = i + 1 }
            // We create a synthetic Each that the interpreter will handle
            let index_var = idents[0].clone();
            let item_var = idents[1].clone();
            Ok(Statement::EachWithIndex {
                index_var,
                item_var,
                iterable,
                body,
                span: Span::unknown(),
            })
        } else {
            let variable = idents[0].clone();
            Ok(Statement::Each {
                variable,
                iterable,
                body,
                span: Span::unknown(),
            })
        }
    } else if let Some(while_pair) = children.iter().find(|c| c.as_rule() == Rule::while_stmt) {
        let while_children: Vec<Pair<Rule>> = while_pair.clone().into_inner().collect();
        // children: expression(condition), statement*(body)
        let condition = parse_expression(while_children[0].clone())?;
        let body: Vec<Statement> = while_children[1..]
            .iter()
            .filter(|c| c.as_rule() == Rule::statement)
            .map(|c| parse_single_statement(c.clone()))
            .collect::<Result<_, _>>()?;
        Ok(Statement::While {
            condition,
            body,
            span: Span::unknown(),
        })
    } else if let Some(lb_pair) = children.iter().find(|c| c.as_rule() == Rule::let_binding) {
        let lb_children = children_of(lb_pair);
        let name = find_child_str(&lb_children, Rule::IDENT).unwrap_or_default();
        // Наряд №173b: let_binding can now contain match_expr (for `let x = match y { ... }`)
        // in addition to regular expression. Try expression first, then match_expr.
        let mutable = lb_children.iter().any(|c| c.as_rule() == Rule::MUT_KW);
        if let Some(expr) = find_child(&lb_children, Rule::expression) {
            Ok(Statement::LetBinding {
                name,
                value: parse_expression(expr)?,
                mutable,
                span: Span::unknown(),
            })
        } else if let Some(me_pair) = find_child(&lb_children, Rule::match_expr) {
            // Наряд №173b: match as expression in let_binding.
            // Parse it as a match_stmt (same grammar), then extract
            // the scrutinee expression and wrap it as a let-binding value.
            // The interpreter's eval_statements handles Match by executing
            // the matched arm; the let-binding captures the last expression
            // value from the matched arm's body.
            let match_stmt = parse_match_stmt(me_pair.clone())?;
            // Extract the scrutinee from the match statement to use as
            // the let-binding's value expression. The actual match logic
            // runs as a side-effect during expression evaluation.
            match match_stmt {
                Statement::Match { scrutinee, .. } => Ok(Statement::LetBinding {
                    name,
                    value: scrutinee,
                    mutable,
                    span: Span::unknown(),
                }),
                _ => unreachable!("parse_match_stmt returned non-Match"),
            }
        } else {
            Err(pair_error(
                &pair,
                "GRAMMAR INVARIANT: expected Rule::expression or Rule::match_expr in let_binding",
            ))
        }
    } else if let Some(ae_pair) = children
        .iter()
        .find(|c| c.as_rule() == Rule::assign_or_expr)
    {
        // assign_or_expr = { IDENT ~ ASSIGN ~ expression | expression }
        let ae_children: Vec<Pair<Rule>> = ae_pair.clone().into_inner().collect();
        // Check if this is an assignment (has IDENT + expression) or expression
        let has_assign = ae_children.iter().any(|c| c.as_rule() == Rule::ASSIGN);
        if has_assign {
            let name = pair_str(&ae_children[0]); // IDENT is first
            let expr = ae_children
                .iter()
                .find(|c| c.as_rule() == Rule::expression)
                .cloned()
                .ok_or_else(|| {
                    pair_error(
                        &pair,
                        "GRAMMAR INVARIANT: assign_or_expr assignment must have expression",
                    )
                })?;
            Ok(Statement::Assign {
                name,
                value: parse_expression(expr)?,
                span: Span::unknown(),
            })
        } else {
            // Expression statement (function call, etc.)
            let expr = ae_children
                .iter()
                .find(|c| c.as_rule() == Rule::expression)
                .cloned()
                .ok_or_else(|| {
                    pair_error(
                        &pair,
                        "GRAMMAR INVARIANT: assign_or_expr expression must have expression",
                    )
                })?;
            Ok(Statement::ExprStmt {
                expr: parse_expression(expr)?,
                span: Span::unknown(),
            })
        }
    } else if let Some(rs_pair) = children.iter().find(|c| c.as_rule() == Rule::return_stmt) {
        let rs_children = children_of(rs_pair);
        let expr = find_child(&rs_children, Rule::expression).ok_or_else(|| {
            pair_error(
                &pair,
                "GRAMMAR INVARIANT: expected Rule::expression in return_stmt",
            )
        })?;
        Ok(Statement::Return {
            value: parse_expression(expr)?,
            span: Span::unknown(),
        })
    } else if let Some(_br_pair) = children.iter().find(|c| c.as_rule() == Rule::break_stmt) {
        Ok(Statement::Break)
    } else if let Some(_co_pair) = children.iter().find(|c| c.as_rule() == Rule::continue_stmt) {
        Ok(Statement::Continue)
    } else if let Some(it_pair) = children.iter().find(|c| c.as_rule() == Rule::if_then_stmt) {
        // if_then_stmt with optional else: "if expr then { ... } [else if expr then { ... }]* [else { ... }]"
        let it_children: Vec<Pair<Rule>> = it_pair.clone().into_inner().collect();
        let condition = it_children
            .iter()
            .find(|c| c.as_rule() == Rule::expression)
            .cloned()
            .map(|c| parse_expression(c))
            .unwrap_or(Ok(Expr::BoolLit {
                value: true,
                span: Span::unknown(),
            }))?;
        let body: Vec<Statement> = it_children
            .iter()
            .filter(|c| c.as_rule() == Rule::statement)
            .map(|c| parse_single_statement(c.clone()))
            .collect::<Result<_, _>>()?;

        // Check for else_if_then_block and else blocks
        let mut else_ifs = Vec::new();
        let mut else_body: Option<Vec<Statement>> = None;
        let mut in_else = false;

        for child in &it_children {
            match child.as_rule() {
                Rule::else_if_then_block => {
                    let ei_children = children_of(child);
                    let ei_condition = ei_children
                        .iter()
                        .find(|c| c.as_rule() == Rule::expression)
                        .map(|c| parse_expression(c.clone()))
                        .unwrap_or(Ok(Expr::BoolLit {
                            value: true,
                            span: Span::unknown(),
                        }))?;
                    let ei_body: Vec<Statement> = ei_children
                        .iter()
                        .filter(|c| c.as_rule() == Rule::statement)
                        .map(|c| parse_single_statement(c.clone()))
                        .collect::<Result<_, _>>()?;
                    else_ifs.push((ei_condition, ei_body));
                }
                // Наряд M2: else_block is a named rule — its statements are
                // children of else_block, NOT direct children of if_then_stmt.
                // The old in_else flag and text "else" detector are dead code.
                Rule::else_block => {
                    let eb: Vec<Statement> = children_of(child)
                        .iter()
                        .filter(|c| c.as_rule() == Rule::statement)
                        .map(|c| parse_single_statement(c.clone()))
                        .collect::<Result<_, _>>()?;
                    else_body = Some(eb);
                }
                Rule::statement => {
                    // Dead path: statements inside else_block are handled above.
                    // Kept per наряд restriction "не удалять".
                    if in_else {
                        if else_body.is_none() {
                            else_body = Some(Vec::new());
                        }
                        if let Some(ref mut eb) = else_body {
                            eb.push(parse_single_statement(child.clone())?)
                        }
                    }
                }
                _ => {
                    // Dead path: text-based "else" detector never matches else_block node.
                    // Kept per наряд restriction "не удалять".
                    if child.as_str().trim() == "else" {
                        in_else = true;
                    }
                }
            }
        }

        if else_ifs.is_empty() && else_body.is_none() {
            Ok(Statement::IfThen {
                condition: Box::new(condition),
                body,
                span: Span::unknown(),
            })
        } else {
            Ok(Statement::IfElseBlock {
                condition,
                then_body: body,
                else_ifs,
                else_body,
                span: Span::unknown(),
            })
        }
    } else if let Some(es_pair) = children.iter().find(|c| c.as_rule() == Rule::expr_stmt) {
        // Legacy expr_stmt fallback (shouldn't normally be reached with assign_or_expr)
        let expr = es_pair
            .clone()
            .into_inner()
            .find(|c| c.as_rule() == Rule::expression)
            .ok_or_else(|| {
                pair_error(
                    &pair,
                    "GRAMMAR INVARIANT: expr_stmt must contain expression",
                )
            })?;
        Ok(Statement::ExprStmt {
            expr: parse_expression(expr)?,
            span: Span::unknown(),
        })
    } else if let Some(as_pair) = children.iter().find(|c| c.as_rule() == Rule::assign_stmt) {
        // Legacy assign_stmt fallback
        let as_children = children_of(as_pair);
        let name = find_child_str(&as_children, Rule::IDENT).unwrap_or_default();
        let expr = find_child(&as_children, Rule::expression).ok_or_else(|| {
            pair_error(
                &pair,
                "GRAMMAR INVARIANT: expected Rule::expression in assign_stmt",
            )
        })?;
        Ok(Statement::Assign {
            name,
            value: parse_expression(expr)?,
            span: Span::unknown(),
        })
    } else {
        // Fallback: unrecognized statement — return proper parse error with position
        Err(pair_error(&pair, pair.as_str()))
    }
}

/// Parse a match statement: `match expr { "val" then { stmts } ... else { stmts } }` (Наряд №14)
pub(super) fn parse_match_stmt(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    use crate::ast::{CompareOp as AstCmp, MatchArm};
    let children: Vec<Pair<Rule>> = pair.into_inner().collect();

    // First child is the scrutinee expression
    let scrutinee = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .unwrap_or(Ok(Expr::StringLit {
            value: String::new(),
            span: Span::unknown(),
        }))?;

    // Parse match arms
    let mut arms = Vec::new();
    let mut else_body: Option<Vec<Statement>> = None;

    for child in &children {
        match child.as_rule() {
            Rule::match_arm_exact => {
                let arm_children: Vec<Pair<Rule>> = child.clone().into_inner().collect();
                let value = arm_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::STRING_LITERAL)
                    .map(|c| unescape_string(c.as_str()))
                    .unwrap_or_default();
                let body = arm_children
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                arms.push(MatchArm::Exact(value, body));
            }
            Rule::match_arm_starts => {
                let arm_children: Vec<Pair<Rule>> = child.clone().into_inner().collect();
                let prefix = arm_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::STRING_LITERAL)
                    .map(|c| unescape_string(c.as_str()))
                    .unwrap_or_default();
                let body = arm_children
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                arms.push(MatchArm::StartsWith(prefix, body));
            }
            Rule::match_arm_contains => {
                let arm_children: Vec<Pair<Rule>> = child.clone().into_inner().collect();
                let substr = arm_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::STRING_LITERAL)
                    .map(|c| unescape_string(c.as_str()))
                    .unwrap_or_default();
                let body = arm_children
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                arms.push(MatchArm::Contains(substr, body));
            }
            Rule::match_arm_compare => {
                let arm_children: Vec<Pair<Rule>> = child.clone().into_inner().collect();
                // First non-statement child is the operator (from compare_op rule)
                let op_str = arm_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::compare_op)
                    .map(|c| c.as_str().trim())
                    .unwrap_or("==");
                let op = match op_str {
                    ">=" => AstCmp::Ge,
                    "<=" => AstCmp::Le,
                    "!=" => AstCmp::Ne,
                    ">" => AstCmp::Gt,
                    "<" => AstCmp::Lt,
                    "==" => AstCmp::Eq,
                    _ => AstCmp::Eq,
                };
                let expr = arm_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::expression)
                    .map(|c| parse_expression(c.clone()))
                    .unwrap_or(Ok(Expr::FloatLit {
                        value: 0.0,
                        span: Span::unknown(),
                    }))?;
                let body = arm_children
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                arms.push(MatchArm::Compare(op, expr, body));
            }
            Rule::match_else => {
                let else_children: Vec<Pair<Rule>> = child.clone().into_inner().collect();
                else_body = Some(
                    else_children
                        .iter()
                        .filter(|c| c.as_rule() == Rule::statement)
                        .map(|c| parse_single_statement(c.clone()))
                        .collect::<Result<_, _>>()?,
                );
            }
            _ => {}
        }
    }

    Ok(Statement::Match {
        scrutinee,
        arms,
        else_body,
        span: Span::unknown(),
    })
}

/// Наряд №14 P0-3: Parse block if/else as expression.
/// `if cond { stmts } else if cond { stmts } else { stmts }` → Expr::BlockIfElse
/// Parse a block-style if statement: `if expr { stmts } else if expr { stmts } else { stmts }`
pub(super) fn parse_if_block_stmt(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let children = children_of(&pair);
    // Grammar now has else_block as a named rule, so children are:
    // [expression, statement*(then), else_if_block*, else_block?]
    let condition = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .unwrap_or(Ok(Expr::BoolLit {
            value: true,
            span: Span::unknown(),
        }))?;

    let mut then_body = Vec::new();
    let mut else_ifs = Vec::new();
    let mut else_body: Option<Vec<Statement>> = None;
    let mut in_else = false;

    for child in &children {
        match child.as_rule() {
            Rule::statement => {
                if in_else {
                    if let Some(ref mut eb) = else_body {
                        eb.push(parse_single_statement(child.clone())?)
                    }
                } else {
                    then_body.push(parse_single_statement(child.clone())?)
                }
            }
            Rule::else_if_block => {
                in_else = false;
                let ei_children = children_of(child);
                let ei_condition = ei_children
                    .iter()
                    .find(|c| c.as_rule() == Rule::expression)
                    .map(|c| parse_expression(c.clone()))
                    .unwrap_or(Ok(Expr::BoolLit {
                        value: true,
                        span: Span::unknown(),
                    }))?;
                let ei_body: Vec<Statement> = ei_children
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                else_ifs.push((ei_condition, ei_body));
            }
            Rule::else_block => {
                // Наряд M2: extract statements from else_block node directly.
                let eb: Vec<Statement> = children_of(child)
                    .iter()
                    .filter(|c| c.as_rule() == Rule::statement)
                    .map(|c| parse_single_statement(c.clone()))
                    .collect::<Result<_, _>>()?;
                else_body = Some(eb);
            }
            _ => {}
        }
    }

    Ok(Statement::IfElseBlock {
        condition,
        then_body,
        else_ifs,
        else_body,
        span: Span::unknown(),
    })
}

// ── Flow ──────────────────────────────────────────────────────────────
// flow_decl = { "flow" ~ IDENT ~ "{" ~ flow_pipeline ~ branch_def* ~ "}" }
// flow_pipeline = { "input" ":" type_name "=" expression ~ flow_step* ~ ARROW ~ "output" }
// flow_step     = { ARROW ~ (checkpoint_call | step_ident) }
// checkpoint_call = { "checkpoint" ~ "(" ~ STRING_LITERAL ~ ")" }
// branch_def    = { step_ident ~ "{" ~ branch* ~ "}" }
