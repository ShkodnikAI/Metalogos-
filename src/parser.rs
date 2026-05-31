// ── Pest → AST conversion for METALOGOS M1+M2 ──────────────────────

use pest::iterators::Pair;
use pest::Parser as _;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct MlogParser;

pub type ParseError = pest::error::Error<Rule>;

use crate::ast::*;

/// Parse a .mlog source string into a list of declarations.
pub fn parse(source: &str) -> Result<Vec<Declaration>, ParseError> {
    let pairs = MlogParser::parse(Rule::program, source)?;
    let mut declarations = Vec::new();

    for pair in pairs {
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::entity_type_decl => declarations.push(parse_entity_type_decl(inner_pair)),
                Rule::entity_record_decl => declarations.push(parse_entity_record_decl(inner_pair)),
                Rule::entity_simple_decl => declarations.push(parse_entity_simple_decl(inner_pair)),
                Rule::rule_decl => declarations.push(parse_rule_decl(inner_pair)),
                Rule::memorize_decl => declarations.push(parse_memorize_decl(inner_pair)),
                Rule::forget_decl => declarations.push(parse_forget_decl(inner_pair)),
                Rule::fluid_decl => declarations.push(parse_fluid_decl(inner_pair)),
                Rule::adapt_decl => declarations.push(parse_adapt_decl(inner_pair)),
                Rule::learn_decl => declarations.push(parse_learn_decl(inner_pair)),
                Rule::learnable_pattern_decl => declarations.push(parse_learnable_pattern_decl(inner_pair)),
                Rule::pattern_decl => declarations.push(parse_pattern_decl(inner_pair)),
                Rule::flow_decl => declarations.push(parse_flow_decl(inner_pair)),
                _ => {}
            }
        }
    }

    Ok(declarations)
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a string from a pair (works for IDENT, type_name, COLON, etc.).
fn pair_str(pair: &Pair<Rule>) -> String {
    pair.as_str().to_string()
}

/// Collect inner pairs into a Vec for easy index access.
fn children_of<'a>(pair: &'a Pair<'a, Rule>) -> Vec<Pair<'a, Rule>> {
    pair.clone().into_inner().collect()
}

/// Find the first child matching a rule, return its string.
fn find_child_str(children: &[Pair<Rule>], rule: Rule) -> Option<String> {
    children.iter().find(|c| c.as_rule() == rule).map(|c| pair_str(c))
}

/// Find the first child matching a rule and return it.
fn find_child<'a>(children: &'a [Pair<'a, Rule>], rule: Rule) -> Option<Pair<'a, Rule>> {
    children.iter().find(|c| c.as_rule() == rule).cloned()
}

// ── Entity: struct type ─────────────────────────────────────────────

fn parse_entity_type_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let fields: Vec<FieldDecl> = children.iter()
        .filter(|c| c.as_rule() == Rule::field_decl)
        .map(|c| parse_field_decl(c.clone()))
        .collect();
    Declaration::EntityType(EntityTypeDecl { name, fields })
}

fn parse_field_decl(pair: Pair<Rule>) -> FieldDecl {
    let children = children_of(&pair);
    // Children: IDENT, COLON, type_name, [ASSIGN, literal]
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let default = find_child(&children, Rule::literal)
        .map(|lit| parse_literal_to_expr(&lit));
    FieldDecl { name, type_name, default }
}

/// Convert a literal pair (STRING_LITERAL, FLOAT_LITERAL, or IDENT) to an Expr.
fn parse_literal_to_expr(pair: &Pair<Rule>) -> Expr {
    let inner = pair.clone().into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::STRING_LITERAL => Expr::StringLit(inner.as_str()[1..inner.as_str().len()-1].to_string()),
        Rule::FLOAT_LITERAL => Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0)),
        Rule::INT => Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0)),
        Rule::IDENT => Expr::Ident(inner.as_str().to_string()),
        _ => Expr::StringLit(pair.as_str().to_string()),
    }
}

// ── Entity: struct instance ─────────────────────────────────────────

fn parse_entity_record_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: IDENT, type_name, field_init, ...
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let fields: Vec<FieldInit> = children.iter()
        .filter(|c| c.as_rule() == Rule::field_init)
        .map(|c| parse_field_init(c.clone()))
        .collect();
    Declaration::EntityRecord(EntityRecordDecl { name, type_name, fields })
}

fn parse_field_init(pair: Pair<Rule>) -> FieldInit {
    let children = children_of(&pair);
    // Children: IDENT, COLON, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).unwrap();
    let value = parse_expression(expr_pair);
    FieldInit { name, value }
}

// ── Entity: simple (M1) ──────────────────────────────────────────────

fn parse_entity_simple_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: IDENT, type_name, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).unwrap();
    let value = parse_expression(expr_pair);
    Declaration::EntitySimple(EntitySimpleDecl { name, type_name, value })
}

// ── Rule ──────────────────────────────────────────────────────────────

fn parse_rule_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: condition (contains/compare), assignment, [INT]
    let condition_pair = &children[0];
    let condition = parse_condition(condition_pair.clone());

    // assignment = { IDENT ~ "." ~ IDENT ~ "=" ~ expression }
    // Children: [IDENT(target), IDENT(field), expression(value)]
    let assignment_children = children_of(&children[1]);
    let target = Expr::Ident(pair_str(&assignment_children[0]));
    let field = pair_str(&assignment_children[1]);
    let value = parse_expression(assignment_children[2].clone());

    let priority = find_child_str(&children, Rule::INT)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    Declaration::Rule(RuleDecl { condition, target, field, value, priority })
}

fn parse_condition(pair: Pair<Rule>) -> Condition {
    match pair.as_rule() {
        Rule::contains_condition => {
            let children = children_of(&pair);
            // Children: expression, CONTAINS_KW, expression
            let left = parse_expression(children[0].clone());
            let right = parse_expression(children[2].clone());
            Condition::Contains { left, right }
        }
        Rule::compare_condition => {
            let children = children_of(&pair);
            // Children: expression, compare_op, expression
            Condition::Compare {
                left: parse_expression(children[0].clone()),
                op: parse_compare_op(&children[1]),
                right: parse_expression(children[2].clone()),
            }
        }
        _ => unreachable!("expected condition, got {:?}", pair.as_rule()),
    }
}

fn parse_compare_op(pair: &Pair<Rule>) -> CompareOp {
    match pair.as_str().trim() {
        ">" => CompareOp::Gt,
        "<" => CompareOp::Lt,
        ">=" => CompareOp::Ge,
        "<=" => CompareOp::Le,
        "==" => CompareOp::Eq,
        _ => unreachable!("unexpected compare op: {}", pair.as_str()),
    }
}

// ── Fluid Types (Phase 1) ──────────────────────────────────────────

fn parse_fluid_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // fluid_decl = { FLUID_KW ~ IDENT ~ "=" ~ fluid_branch ~ ("or" ~ fluid_branch)* }
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let variants: Vec<FluidVariant> = children.iter()
        .filter(|c| c.as_rule() == Rule::fluid_branch)
        .map(|c| parse_fluid_branch(c.clone()))
        .collect();

    Declaration::Fluid(FluidDecl { name, variants })
}

fn parse_fluid_branch(pair: Pair<Rule>) -> FluidVariant {
    let children = children_of(&pair);
    // fluid_branch = { type_name ~ LBRACKET ~ expression ~ RBRACKET ~ LBRACKET ~ FLOAT_LITERAL ~ RBRACKET }
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();

    let exprs: Vec<Pair<Rule>> = children.iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();
    let value = if !exprs.is_empty() { parse_expression(exprs[0].clone()) } else { Expr::StringLit(String::new()) };

    let floats: Vec<&Pair<Rule>> = children.iter()
        .filter(|c| c.as_rule() == Rule::FLOAT_LITERAL)
        .collect();
    // The last FLOAT_LITERAL is the confidence (value may also be a float)
    let confidence = floats.last()
        .map(|f| f.as_str().parse().unwrap_or(0.0))
        .unwrap_or(0.0);

    FluidVariant { type_name, value, confidence }
}

// ── Adapt (M5) ──────────────────────────────────────────────────

fn parse_adapt_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // adapt_decl = { ADAPT_KW ~ IDENT ~ ADD_EXAMPLE_KW ~ "(" ~ expression ~ COMMA ~ expression ~ ")" }
    // Children: IDENT(pattern_name), "(", expression(input), ",", expression(output), ")"
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let exprs: Vec<Pair<Rule>> = children.iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();

    let input_example = if exprs.len() >= 1 { parse_expression(exprs[0].clone()) } else { Expr::StringLit(String::new()) };
    let output_example = if exprs.len() >= 2 { parse_expression(exprs[1].clone()) } else { Expr::StringLit(String::new()) };

    Declaration::Adapt(AdaptDecl { pattern_name, input_example, output_example })
}

// ── Learn (Phase 2.3) ──────────────────────────────────────────

fn parse_learn_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // learn_decl = { LEARN_KW ~ IDENT ~ "with" ~ "{" ~ learn_param_list? ~ COMMA? ~ "}" }
    // Children: IDENT(pattern_name), [learn_param_list, ...]
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let mut hyperparams: Vec<(String, Expr)> = Vec::new();
    if let Some(list_pair) = find_child(&children, Rule::learn_param_list) {
        for child in list_pair.clone().into_inner() {
            if child.as_rule() == Rule::learn_param {
                let param_children = children_of(&child);
                let name = find_child_str(&param_children, Rule::IDENT).unwrap_or_default();
                if let Some(expr_pair) = find_child(&param_children, Rule::expression) {
                    let value = parse_expression(expr_pair);
                    hyperparams.push((name, value));
                }
            }
        }
    }

    Declaration::Learn(LearnDecl { pattern_name, hyperparams })
}

// ── Memorize (M4) ──────────────────────────────────────────────────

fn parse_memorize_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: expression, ["with", "priority", "=", FLOAT_LITERAL]
    let value = find_child(&children, Rule::expression).unwrap();
    let value = parse_expression(value);

    let priority = find_child(&children, Rule::FLOAT_LITERAL)
        .map(|f| f.as_str().parse().unwrap_or(0.5))
        .unwrap_or(0.5);

    Declaration::Memorize(MemorizeDecl { value, priority })
}

fn parse_forget_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: expression, INT, "days"
    let query = find_child(&children, Rule::expression).unwrap();
    let query = parse_expression(query);

    let days = find_child(&children, Rule::INT)
        .map(|i| i.as_str().parse().unwrap_or(30))
        .unwrap_or(30);

    Declaration::Forget(ForgetDecl { query, days })
}

// ── Learnable Pattern (M3) ────────────────────────────────────────────

fn parse_learnable_pattern_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: IDENT, [params], ARROW, type_name, learnable_body
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();

    // Extract prompt from learnable_body -> prompt_line -> expression
    let mut prompt = String::new();
    if let Some(body_pair) = find_child(&children, Rule::learnable_body) {
        let body_children = children_of(&body_pair);
        if let Some(pl_pair) = body_children.iter().find(|c| c.as_rule() == Rule::prompt_line) {
            let pl_children = children_of(pl_pair);
            if let Some(expr_pair) = pl_children.iter().find(|c| c.as_rule() == Rule::expression) {
                if let Expr::StringLit(s) = parse_expression(expr_pair.clone()) {
                    prompt = s;
                }
            }
        }
    }

    Declaration::LearnablePattern(LearnablePatternDecl {
        name,
        params,
        return_type,
        prompt,
    })
}

// ── Pattern ──────────────────────────────────────────────────────────

fn parse_pattern_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // Children: IDENT, params, ARROW, type_name, pattern_body
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let body = find_child(&children, Rule::pattern_body)
        .map(|p| parse_pattern_body(p))
        .unwrap_or_default();
    Declaration::Pattern(PatternDecl { name, params, return_type, body })
}

fn parse_params(pair: Pair<Rule>) -> Vec<Param> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::param)
        .map(|p| {
            let children = children_of(&p);
            Param {
                name: find_child_str(&children, Rule::IDENT).unwrap_or_default(),
                type_name: find_child_str(&children, Rule::type_name).unwrap_or_default(),
            }
        })
        .collect()
}

fn parse_pattern_body(pair: Pair<Rule>) -> Vec<Statement> {
    pair.into_inner()
        .filter(|s| s.as_rule() == Rule::statement)
        .map(|s| {
            let children = children_of(&s);
            // statement = { "return" ~ expression }
            let expr = find_child(&children, Rule::expression).unwrap();
            Statement::Return(parse_expression(expr))
        })
        .collect()
}

// ── Flow ──────────────────────────────────────────────────────────────
// flow_decl = { "flow" ~ IDENT ~ "{" ~ flow_pipeline ~ branch_def* ~ "}" }
// flow_pipeline = { "input" ":" ~ type_name "=" ~ expression ~ (ARROW ~ step_ident)* ~ ARROW ~ "output" }
// branch_def    = { step_ident ~ "{" ~ branch* ~ "}" }

fn parse_flow_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    // children: IDENT, flow_pipeline, [branch_def, ...]
    let pipeline_pair = find_child(&children, Rule::flow_pipeline).unwrap();
    let pipeline_children = children_of(&pipeline_pair);

    let mut input_type = String::new();
    let mut source: Option<Expr> = None;
    let mut pipeline_steps: Vec<String> = Vec::new();

    // Walk pipeline children: type_name, expression, (ARROW, step_ident)*, ARROW
    let mut i = 0;
    // First: type_name
    if i < pipeline_children.len() && pipeline_children[i].as_rule() == Rule::type_name {
        let tn_inner = children_of(&pipeline_children[i]);
        input_type = pair_str(&tn_inner[0]);
        i += 1;
    }
    // Second: expression (source)
    if i < pipeline_children.len() && pipeline_children[i].as_rule() == Rule::expression {
        source = Some(parse_expression(pipeline_children[i].clone()));
        i += 1;
    }
    // Remaining: (ARROW, step_ident)* pairs, final ARROW (-> output)
    while i < pipeline_children.len() {
        if pipeline_children[i].as_rule() == Rule::ARROW {
            i += 1;
            if i < pipeline_children.len() && pipeline_children[i].as_rule() == Rule::step_ident {
                pipeline_steps.push(pair_str(&pipeline_children[i]));
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // Parse branch_def blocks after the pipeline
    let mut branch_defs: Vec<(String, Vec<Branch>)> = Vec::new();
    for child in &children {
        if child.as_rule() == Rule::branch_def {
            let bd_children = children_of(child);
            let step_name = pair_str(&bd_children[0]); // step_ident
            let branches: Vec<Branch> = bd_children.iter()
                .filter(|c| c.as_rule() == Rule::branch)
                .map(|c| parse_branch(c.clone()))
                .collect();
            branch_defs.push((step_name, branches));
        }
    }

    Declaration::Flow(FlowDecl {
        name,
        input_type,
        source: source.unwrap(),
        pipeline: pipeline_steps,
        branch_defs,
    })
}

fn parse_branch(pair: Pair<Rule>) -> Branch {
    let children = children_of(&pair);
    // branch = { IDENT ~ "(" ~ branch_condition ~ ")" ~ ARROW ~ step_ident }
    let label = pair_str(&children[0]);
    let cond_pair = find_child(&children, Rule::branch_condition).unwrap();
    let target = pair_str(&children.last().unwrap()); // step_ident is last
    Branch {
        label,
        condition: parse_branch_condition(cond_pair),
        target,
    }
}

fn parse_branch_condition(pair: Pair<Rule>) -> BranchCondition {
    let children = children_of(&pair);
    // branch_condition = { IDENT ~ "." ~ IDENT ~ compare_op ~ expression }
    // Children: [IDENT(target), IDENT(field), compare_op, expression(threshold)]
    BranchCondition {
        target: Expr::Ident(pair_str(&children[0])),
        field: pair_str(&children[1]),
        op: parse_compare_op(&children[2]),
        threshold: parse_expression(children[3].clone()),
    }
}

// ── Expressions ─────────────────────────────────────────────────────

fn parse_expression(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::expression => {
            let inner = pair.into_inner().next().unwrap();
            parse_expression(inner)
        }
        Rule::binary_expr => {
            let children = children_of(&pair);
            if children.is_empty() { return Expr::StringLit(String::new()); }
            let mut left = parse_expression(children[0].clone());
            let mut i = 1;
            while i + 1 < children.len() {
                if children[i].as_rule() == Rule::binop {
                    let op = match children[i].as_str().trim() {
                        "+" => BinOp::Add,
                        "-" => BinOp::Sub,
                        _ => { i += 2; continue; }
                    };
                    left = Expr::BinaryOp(
                        Box::new(left), op,
                        Box::new(parse_expression(children[i + 1].clone())),
                    );
                    i += 2;
                } else {
                    i += 1;
                }
            }
            left
        }
        Rule::unary_expr => {
            parse_expression(pair.into_inner().next().unwrap())
        }
        Rule::field_expr => {
            let children = children_of(&pair);
            // field_expr = { primary_expr ~ "." ~ IDENT }
            Expr::FieldAccess(
                Box::new(parse_expression(children[0].clone())),
                pair_str(&children[1]),
            )
        }
        Rule::call_expr => {
            let children = children_of(&pair);
            let fname = pair_str(&children[0]);
            let mut args = Vec::new();
            for child in children.iter().skip(1) {
                match child.as_rule() {
                    Rule::expression_list => {
                        for ap in child.clone().into_inner() {
                            if ap.as_rule() == Rule::expression {
                                args.push(parse_expression(ap));
                            }
                        }
                    }
                    Rule::expression => args.push(parse_expression(child.clone())),
                    _ => {}
                }
            }
            Expr::FnCall(fname, args)
        }
        Rule::primary_expr => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::STRING_LITERAL => {
                    let s = inner.as_str();
                    Expr::StringLit(s[1..s.len()-1].to_string())
                }
                Rule::FLOAT_LITERAL => {
                    Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0))
                }
                Rule::INT => Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0)),
                Rule::IDENT => Expr::Ident(inner.as_str().to_string()),
                _ => Expr::Ident(inner.as_str().to_string()),
            }
        }
        _ => Expr::StringLit(pair.as_str().to_string()),
    }
}
