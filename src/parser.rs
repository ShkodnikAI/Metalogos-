// ── Pest → AST conversion for METALOGOS M1 ────────────────────────────

use pest::iterators::Pair;
use pest::Parser as _; // brings trait method .parse() into scope
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct MlogParser;

pub type ParseError = pest::error::Error<crate::parser::Rule>;

/// Parse a .mlog source string into a list of declarations.
pub fn parse(source: &str) -> Result<Vec<crate::ast::Declaration>, ParseError> {
    let pairs = MlogParser::parse(Rule::program, source)?;
    let mut declarations = Vec::new();

    for pair in pairs {
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::entity_decl => declarations.push(parse_entity_decl(inner_pair)),
                Rule::pattern_decl => declarations.push(parse_pattern_decl(inner_pair)),
                Rule::flow_decl => declarations.push(parse_flow_decl(inner_pair)),
                _ => {}
            }
        }
    }

    Ok(declarations)
}

fn parse_entity_decl(pair: Pair<Rule>) -> crate::ast::Declaration {
    let mut inner = pair.into_inner();
    let name = expect_ident(inner.next().unwrap());
    // skip ':'
    let type_name_val = expect_ident(inner.next().unwrap());
    // skip '='
    let value = parse_expression(inner.next().unwrap());
    crate::ast::Declaration::Entity(crate::ast::EntityDecl {
        name,
        type_name: type_name_val,
        value,
    })
}

fn parse_pattern_decl(pair: Pair<Rule>) -> crate::ast::Declaration {
    let mut inner = pair.into_inner();
    let name = expect_ident(inner.next().unwrap());

    // params
    let params_pair = inner.next().unwrap();
    let params_vec = parse_params(params_pair);

    // skip '->'
    let return_type_val = expect_ident(inner.next().unwrap());

    // body: statements inside {}
    let body_pair = inner.next().unwrap();
    let body_vec = parse_pattern_body(body_pair);

    crate::ast::Declaration::Pattern(crate::ast::PatternDecl {
        name,
        params: params_vec,
        return_type: return_type_val,
        body: body_vec,
    })
}

fn parse_params(pair: Pair<Rule>) -> Vec<crate::ast::Param> {
    let mut result = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::param {
            let mut inner = p.into_inner();
            let pname = expect_ident(inner.next().unwrap());
            let ptype = expect_ident(inner.next().unwrap());
            result.push(crate::ast::Param {
                name: pname,
                type_name: ptype,
            });
        }
    }
    result
}

fn parse_pattern_body(pair: Pair<Rule>) -> Vec<crate::ast::Statement> {
    let mut stmts = Vec::new();
    for s in pair.into_inner() {
        if s.as_rule() == Rule::statement {
            let mut inner = s.into_inner();
            let expr = parse_expression(inner.next().unwrap());
            stmts.push(crate::ast::Statement::Return(expr));
        }
    }
    stmts
}

fn parse_flow_decl(pair: Pair<Rule>) -> crate::ast::Declaration {
    let mut inner = pair.into_inner();
    let name = expect_ident(inner.next().unwrap());

    // flow_body
    let body_pair = inner.next().unwrap();
    let body_inner = body_pair.into_inner();

    // children of flow_body:
    //   "input" (anonymous), ":" (anonymous), type_name → IDENT, "=" (anonymous),
    //   expression, (ARROW, step_ident)* pairs, ARROW, "output" (anonymous)
    let mut steps = Vec::new();
    let mut source: Option<crate::ast::Expr> = None;
    let mut input_type: String = String::new();

    for child in body_inner {
        match child.as_rule() {
            Rule::expression => {
                source = Some(parse_expression(child));
            }
            Rule::ARROW => {
                // ARROW followed by either step_ident (pipeline step) or "output" (terminator)
                // We need to look at what comes next — but we're iterating, so peek is tricky.
                // Instead: ARROW before step_ident is a pipeline step; ARROW before "output" is the final arrow.
                // We'll collect ARROWs and resolve after.
            }
            Rule::step_ident => {
                steps.push(child.as_str().to_string());
            }
            Rule::IDENT => {
                // Could be "input" type name or other IDENTs
                if input_type.is_empty() {
                    // First IDENT in flow_body is the type_name after "input:"
                    input_type = child.as_str().to_string();
                }
            }
            _ => {
                // Skip "input", ":", "=", "output" (anonymous string literals)
            }
        }
    }

    crate::ast::Declaration::Flow(crate::ast::FlowDecl {
        name,
        input_type,
        source: source.unwrap(),
        pipeline: steps,
    })
}

fn parse_expression(pair: Pair<Rule>) -> crate::ast::Expr {
    match pair.as_rule() {
        Rule::expression => {
            // expression = binary_expr; unwrap one level
            let inner = pair.into_inner().next().unwrap();
            parse_expression(inner)
        }
        Rule::binary_expr => {
            // binary_expr = { unary_expr ~ (binop ~ unary_expr)* }
            // Children: [unary_expr, binop, unary_expr, binop, unary_expr, ...]
            let children: Vec<Pair<Rule>> = pair.into_inner().collect();
            if children.is_empty() {
                return crate::ast::Expr::StringLit(String::new());
            }

            let mut left = parse_expression(children[0].clone());
            let mut i = 1;
            while i + 1 < children.len() {
                let op_pair = &children[i];
                let right_pair = &children[i + 1];

                let op = match op_pair.as_rule() {
                    Rule::binop => match op_pair.as_str().trim() {
                        "+" => crate::ast::BinOp::Add,
                        "-" => crate::ast::BinOp::Sub,
                        "*" => crate::ast::BinOp::Mul,
                        "/" => crate::ast::BinOp::Div,
                        _ => { i += 2; continue; }
                    },
                    _ => { i += 1; continue; }
                };

                let right = parse_expression(right_pair.clone());
                left = crate::ast::Expr::BinaryOp(Box::new(left), op, Box::new(right));
                i += 2;
            }
            left
        }
        Rule::unary_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_expression(inner)
        }
        Rule::call_expr => {
            let children: Vec<Pair<Rule>> = pair.into_inner().collect();
            let fname = expect_ident(children[0].clone());

            // children[1] is "(", then args, then ")"
            let mut args = Vec::new();
            for child in children.iter().skip(1) {
                if child.as_rule() == Rule::expression_list {
                    for arg_pair in child.clone().into_inner() {
                        if arg_pair.as_rule() == Rule::expression {
                            args.push(parse_expression(arg_pair));
                        }
                    }
                } else if child.as_rule() == Rule::expression {
                    args.push(parse_expression(child.clone()));
                }
                // skip "(", ")" anonymous
            }
            crate::ast::Expr::FnCall(fname, args)
        }
        Rule::primary_expr => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::STRING_LITERAL => {
                    let s = inner.as_str();
                    crate::ast::Expr::StringLit(s[1..s.len() - 1].to_string())
                }
                Rule::IDENT => crate::ast::Expr::Ident(inner.as_str().to_string()),
                _ => crate::ast::Expr::Ident(inner.as_str().to_string()),
            }
        }
        _ => crate::ast::Expr::StringLit(pair.as_str().to_string()),
    }
}

fn expect_ident(pair: Pair<Rule>) -> String {
    pair.as_str().to_string()
}
