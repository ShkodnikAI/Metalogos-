use pest::iterators::Pair;

use super::*;

/// Наряд №14 P0-3: Parse block if/else as expression.
/// `if cond { stmts } else if cond { stmts } else { stmts }` → Expr::BlockIfElse
pub(super) fn parse_block_if_else_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let children: Vec<Pair<Rule>> = pair.into_inner().collect();
    let condition = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .unwrap_or(Ok(Expr::BoolLit(true)))?;

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
                    .unwrap_or(Ok(Expr::BoolLit(true)))?;
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

    Ok(Expr::BlockIfElse {
        condition: Box::new(condition),
        then_body,
        else_ifs: else_ifs
            .into_iter()
            .map(|(c, b)| (Box::new(c), b))
            .collect(),
        else_body,
    })
}

/// Parse a block-style if statement: `if expr { stmts } else if expr { stmts } else { stmts }`

// ── Expressions ─────────────────────────────────────────────────────

pub(super) fn parse_binop(pair: &Pair<Rule>) -> Result<BinOp, ParseError> {
    match pair.as_str().trim() {
        "and" => Ok(BinOp::And),
        "or" => Ok(BinOp::Or),
        "+" => Ok(BinOp::Add),
        "-" => Ok(BinOp::Sub),
        "*" => Ok(BinOp::Mul),
        "/" => Ok(BinOp::Div),
        ">=" => Ok(BinOp::Ge),
        "<=" => Ok(BinOp::Le),
        ">" => Ok(BinOp::Gt),
        "<" => Ok(BinOp::Lt),
        "==" => Ok(BinOp::Eq),
        "!=" => Ok(BinOp::Ne),
        _ => Err(pair_error(
            pair,
            "GRAMMAR INVARIANT: unknown binary operator",
        ))?,
    }
}

pub(super) fn parse_expression(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expression => {
            let span = pair.as_span();
            let inner = pair.into_inner().next().ok_or_else(|| {
                pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "GRAMMAR INVARIANT: expression must have inner content"
                            .to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            Ok(parse_expression(inner)?)
        }
        Rule::or_expr => {
            let children = children_of(&pair);
            if children.is_empty() {
                return Ok(Expr::StringLit(String::new()));
            }
            let mut left = parse_expression(children[0].clone())?;
            let mut i = 1;
            while i + 1 < children.len() {
                if children[i].as_rule() == Rule::OR_KW {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::Or,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            Ok(left)
        }
        Rule::and_expr => {
            let children = children_of(&pair);
            if children.is_empty() {
                return Ok(Expr::StringLit(String::new()));
            }
            let mut left = parse_expression(children[0].clone())?;
            let mut i = 1;
            while i + 1 < children.len() {
                if children[i].as_rule() == Rule::AND_KW {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::And,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            Ok(left)
        }
        Rule::compare_expr => {
            let children = children_of(&pair);
            if children.is_empty() {
                return Ok(Expr::StringLit(String::new()));
            }
            let mut left = parse_expression(children[0].clone())?;
            let mut i = 1;
            while i + 1 < children.len() {
                if children[i].as_rule() == Rule::compare_op {
                    let op = parse_binop(&children[i])?;
                    left = Expr::BinaryOp(
                        Box::new(left),
                        op,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            Ok(left)
        }
        Rule::add_expr => {
            let children = children_of(&pair);
            if children.is_empty() {
                return Ok(Expr::StringLit(String::new()));
            }
            let mut left = parse_expression(children[0].clone())?;
            let mut i = 1;
            while i + 1 < children.len() {
                let rule = children[i].as_rule();
                if rule == Rule::PLUS {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::Add,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else if rule == Rule::MINUS {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::Sub,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            Ok(left)
        }
        Rule::mul_expr => {
            let children = children_of(&pair);
            if children.is_empty() {
                return Ok(Expr::StringLit(String::new()));
            }
            let mut left = parse_expression(children[0].clone())?;
            let mut i = 1;
            while i + 1 < children.len() {
                let rule = children[i].as_rule();
                if rule == Rule::STAR {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::Mul,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else if rule == Rule::SLASH {
                    left = Expr::BinaryOp(
                        Box::new(left),
                        BinOp::Div,
                        Box::new(parse_expression(children[i + 1].clone())?),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            Ok(left)
        }
        Rule::unary_expr => {
            let span = pair.as_span();
            let inner = pair.into_inner().next().ok_or_else(|| {
                pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "GRAMMAR INVARIANT: unary_expr must have inner content"
                            .to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            Ok(parse_expression(inner)?)
        }
        // Наряд №14 P1-4: try expression
        Rule::try_expr => {
            let span = pair.as_span();
            let inner = pair.into_inner().next().ok_or_else(|| {
                pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "GRAMMAR INVARIANT: try_expr must have inner content".to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            let expr = parse_expression(inner)?;
            Ok(Expr::Try(Box::new(expr)))
        }
        Rule::if_else_expr => {
            let children = children_of(&pair);
            // if_else_expr = { "if" ~ expression ~ "then" ~ expression ~ "else" ~ expression }
            let exprs: Vec<Expr> = children
                .iter()
                .filter(|c| c.as_rule() == Rule::expression)
                .map(|c| parse_expression(c.clone()))
                .collect::<Result<_, _>>()?;
            // Should have exactly 3 expression children: cond, then, else
            let cond = exprs[0].clone();
            let then_br = exprs[1].clone();
            let else_br = exprs[2].clone();
            Ok(Expr::IfElse(
                Box::new(cond),
                Box::new(then_br),
                Box::new(else_br),
            ))
        }
        Rule::access_expr => {
            let children = children_of(&pair);
            // access_expr = { primary_expr ~ postfix_op* }
            // postfix_op is silent — children are flattened: IDENT and LBRACKET...RBRACKET
            let mut base = parse_expression(children[0].clone())?; // primary_expr
            let mut i = 1;
            while i < children.len() {
                let child = &children[i];
                if child.as_rule() == Rule::IDENT {
                    // Field access: payload.chat_id
                    base = Expr::FieldAccess(Box::new(base), pair_str(child));
                } else if child.as_rule() == Rule::LBRACKET {
                    // Index access: items[0]
                    // LBRACKET is atomic (@{...}) — cannot drill into it.
                    // The expression is the NEXT child, RBRACKET is after that.
                    if i + 2 < children.len() {
                        let expr_child = &children[i + 1];
                        if expr_child.as_rule() == Rule::expression {
                            base = Expr::IndexAccess(
                                Box::new(base),
                                Box::new(parse_expression(expr_child.clone())?),
                            );
                        }
                    }
                }
                i += 1;
            }
            Ok(base)
        }
        Rule::qualified_call_expr => {
            let children = children_of(&pair);
            // qualified_call_expr = { IDENT ~ "." ~ IDENT ~ "(" ~ expression_list? ~ ")" }
            let idents: Vec<String> = children
                .iter()
                .filter(|c| c.as_rule() == Rule::IDENT)
                .map(|c| pair_str(c))
                .collect();
            let module = idents.first().cloned().unwrap_or_default();
            let function = idents.get(1).cloned().unwrap_or_default();
            let mut args = Vec::new();
            for child in children.iter() {
                if child.as_rule() == Rule::expression_list {
                    for ap in child.clone().into_inner() {
                        if ap.as_rule() == Rule::expression {
                            args.push(parse_expression(ap)?);
                        }
                    }
                }
            }
            Ok(Expr::QualifiedCall {
                module,
                function,
                args,
            })
        }
        Rule::call_expr => {
            let children = children_of(&pair);
            let fname = pair_str(&children[0]);
            // Handle integer literals as function names (e.g., if INT is matched as call_expr)
            if fname == "true" || fname == "false" {
                // Bool literal parsed as call_expr — shouldn't happen but handle gracefully
                return Ok(Expr::BoolLit(fname == "true"));
            }
            let mut args = Vec::new();
            for child in children.iter().skip(1) {
                match child.as_rule() {
                    Rule::expression_list => {
                        for ap in child.clone().into_inner() {
                            if ap.as_rule() == Rule::expression {
                                args.push(parse_expression(ap)?);
                            }
                        }
                    }
                    Rule::expression => args.push(parse_expression(child.clone())?),
                    _ => {}
                }
            }
            Ok(Expr::FnCall(fname, args))
        }
        Rule::unary_minus => {
            let children: Vec<_> = pair.clone().into_inner().collect();
            // children: [MINUS, unary_expr] — without @ atomic rule
            let inner_expr = if children.len() >= 2 {
                parse_expression(children[1].clone())?
            } else {
                Expr::FloatLit(0.0)
            };
            // Negate: 0.0 - val
            Ok(Expr::BinaryOp(
                Box::new(Expr::FloatLit(0.0)),
                BinOp::Sub,
                Box::new(inner_expr),
            ))
        }
        Rule::primary_expr => {
            let span = pair.as_span();
            let inner = pair.into_inner().next().ok_or_else(|| {
                pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "GRAMMAR INVARIANT: primary_expr must have inner content"
                            .to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            match inner.as_rule() {
                Rule::paren_expr => {
                    // Наряд M1: parenthesized grouping — unwrap inner expression
                    let inner_span = inner.as_span();
                    let inner_expr = inner.into_inner().next().ok_or_else(|| {
                        pest::error::Error::new_from_pos(
                            pest::error::ErrorVariant::CustomError {
                                message: "GRAMMAR INVARIANT: paren_expr must have inner content"
                                    .to_string(),
                            },
                            inner_span.start_pos(),
                        )
                    })?;
                    Ok(parse_expression(inner_expr)?)
                }
                Rule::block_if_else_expr => {
                    // Наряд №14 P0-3: block if/else as expression
                    Ok(parse_block_if_else_expr(inner)?)
                }
                Rule::struct_literal => {
                    let mut fields = std::collections::HashMap::new();
                    for child in inner.clone().into_inner() {
                        if child.as_rule() == Rule::struct_field_init {
                            let field_children: Vec<_> = child.clone().into_inner().collect();
                            // struct_field_init = { IDENT ~ COLON ~ expression }
                            let name = pair_str(&field_children[0]);
                            let value = parse_expression(
                                field_children
                                    .last()
                                    .cloned()
                                    .ok_or_else(|| {
                                        pest::error::Error::new_from_pos(
                                            pest::error::ErrorVariant::CustomError {
                                                message: "GRAMMAR INVARIANT: struct_field_init must have expression".to_string(),
                                            },
                                            child.as_span().start_pos(),
                                        )
                                    })?,
                            )?;
                            fields.insert(name, value);
                        }
                    }
                    Ok(Expr::StructLit(fields))
                }
                Rule::list_literal => {
                    let mut items = Vec::new();
                    for child in inner.clone().into_inner() {
                        match child.as_rule() {
                            Rule::expression_list => {
                                // expression_list contains nested expression pairs
                                for expr_pair in child.into_inner() {
                                    if expr_pair.as_rule() == Rule::expression {
                                        items.push(parse_expression(expr_pair)?);
                                    }
                                }
                            }
                            Rule::expression => {
                                items.push(parse_expression(child)?);
                            }
                            _ => {}
                        }
                    }
                    Ok(Expr::List(items))
                }
                Rule::BOOL_LITERAL => Ok(Expr::BoolLit(inner.as_str() == "true")),
                Rule::STRING_LITERAL => Ok(Expr::StringLit(unescape_string(inner.as_str()))),
                Rule::MULTILINE_STRING => {
                    // Triple-quoted string: content between """ delimiters, raw (no escape processing)
                    Ok(Expr::StringLit(inner.as_str().to_string()))
                }
                Rule::FLOAT_LITERAL => Ok(Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0))),
                Rule::INT => Ok(Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0))),
                Rule::IDENT => Ok(Expr::Ident(inner.as_str().to_string())),
                _ => Ok(Expr::Ident(inner.as_str().to_string())),
            }
        }
        _ => Ok(Expr::StringLit(pair.as_str().to_string())),
    }
}
