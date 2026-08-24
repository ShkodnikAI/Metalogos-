use pest::iterators::Pair;
use std::collections::HashMap;

use super::*;

// ── MlogServer (Phase 6.1) ─────────────────────────────────────

pub(super) fn parse_mlogserver_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let _children = children_of(&pair);
    let body_children: Vec<Pair<Rule>> = pair
        .clone()
        .into_inner()
        .filter(|c| c.as_rule() == Rule::mlogserver_body)
        .flat_map(|c| c.into_inner())
        .collect();

    let port: u16 = body_children
        .iter()
        .find(|c| c.as_rule() == Rule::mlogserver_port)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let host: Option<String> = body_children
        .iter()
        .find(|c| c.as_rule() == Rule::mlogserver_host)
        .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
        .map(|s| s.trim_matches('"').to_string());

    let mut middleware = Vec::new();
    for child in &body_children {
        if child.as_rule() == Rule::mlogserver_middleware {
            if let Some(il) = child
                .clone()
                .into_inner()
                .find(|c| c.as_rule() == Rule::ident_list)
            {
                middleware = il
                    .clone()
                    .into_inner()
                    .filter(|c| c.as_rule() == Rule::IDENT)
                    .map(|c| pair_str(&c))
                    .collect();
            }
        }
    }

    let routes: Vec<RouteDecl> = body_children
        .iter()
        .filter(|c| c.as_rule() == Rule::route_decl)
        .map(|c| parse_route_decl(c.clone()))
        .collect::<Result<_, _>>()?;

    Ok(Declaration::MlogServer(MlogServerDecl {
        port,
        host,
        middleware,
        routes,
    }))
}

pub(super) fn parse_route_decl(pair: Pair<Rule>) -> Result<RouteDecl, ParseError> {
    let children: Vec<Pair<Rule>> = pair.clone().into_inner().collect();
    let path = children
        .iter()
        .find(|c| c.as_rule() == Rule::STRING_LITERAL)
        .map(|c| {
            let s = c.as_str();
            s[1..s.len() - 1].to_string()
        })
        .unwrap_or_default();

    let method = children
        .iter()
        .filter(|c| c.as_rule() == Rule::IDENT)
        .map(|c| pair_str(c))
        .next() // first IDENT after STRING_LITERAL is the HTTP method
        .unwrap_or_else(|| "GET".to_string());

    let mut requires = Vec::new();
    for child in &children {
        if child.as_rule() == Rule::route_requires {
            if let Some(il) = child
                .clone()
                .into_inner()
                .find(|c| c.as_rule() == Rule::ident_list)
            {
                requires = il
                    .clone()
                    .into_inner()
                    .filter(|c| c.as_rule() == Rule::IDENT)
                    .map(|c| pair_str(&c))
                    .collect();
            }
        }
    }

    let body: Vec<Statement> = pair
        .clone()
        .into_inner()
        .filter(|c| c.as_rule() == Rule::statement)
        .map(|c| parse_single_statement(c))
        .collect::<Result<_, _>>()?;

    Ok(RouteDecl {
        path,
        method,
        requires,
        body,
    })
}

// ── Template (Phase 6.2) ─────────────────────────────────────

/// Pre-process source to handle template bodies containing `}` (HTML, CSS, JS).
/// Extracts template bodies using balanced brace counting, replaces with safe placeholders,
/// and returns a mapping of placeholder -> actual body content.
/// Uses char_indices() for Unicode-safe byte positioning.
/// Parse a template declaration, restoring the pre-processed body.
pub(super) fn parse_template_decl_with_body(
    pair: Pair<Rule>,
    bodies: &HashMap<String, String>,
) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();

    // The grammar captured the placeholder in template_body_raw
    let body = pair
        .clone()
        .into_inner()
        .find(|c| c.as_rule() == Rule::template_body_raw)
        .map(|c| {
            let placeholder = c.as_str().trim().to_string();
            // Look up the placeholder in our pre-processed bodies map
            if let Some(actual_body) = bodies.get(&placeholder) {
                actual_body.clone()
            } else {
                // Fallback: use the raw text (for templates without } in body)
                placeholder
            }
        })
        .unwrap_or_default();

    Declaration::Template(TemplateDecl {
        name,
        params,
        return_type,
        body,
    })
}

// ── DB (Phase 6.3) ─────────────────────────────────────

pub(super) fn parse_db_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children: Vec<Pair<Rule>> = pair
        .into_inner()
        .filter(|c| c.as_rule() == Rule::db_body)
        .flat_map(|c| c.into_inner())
        .collect();

    let url = children
        .iter()
        .find(|c| c.as_rule() == Rule::db_url)
        .and_then(|c| {
            let c_children = children_of(c);
            find_child(&c_children, Rule::expression).and_then(|e| parse_expression(e).ok())
        });

    let pool_size = children
        .iter()
        .find(|c| c.as_rule() == Rule::db_pool)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok());

    let migrate = children
        .iter()
        .find(|c| c.as_rule() == Rule::db_migrate)
        .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
        .map(|s| s[1..s.len() - 1].to_string());

    Ok(Declaration::Db(DbDecl {
        url,
        pool_size,
        migrate,
    }))
}

// ── Schema (Problem C: schema-as-code) ──────────────────────────────

// ── Schema (Problem C: schema-as-code) ──────────────────────────────

pub(super) fn parse_schema_decl(pair: Pair<Rule>) -> Declaration {
    let mut name = String::new();
    let mut tables = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IDENT => name = child.as_str().to_string(),
            Rule::schema_table => {
                tables.push(parse_schema_table(child));
            }
            _ => {}
        }
    }

    Declaration::Schema(SchemaDecl { name, tables })
}

// ── Skill Index (Problem A: tiered skill index) ──────────────────────────

// ── Skill Index (Problem A: tiered skill index) ──────────────────────────

pub(super) fn parse_skill_index_decl(pair: Pair<Rule>) -> Declaration {
    let mut name = String::new();
    let mut tiers = Vec::new();
    let mut budget: Option<f64> = None;
    let mut truncation: Option<TruncationMode> = None;

    // skill_index_decl = { "skill_index" ~ IDENT ~ LBRACE ~ skill_index_body ~ RBRACE }
    // skill_index_body  = { skill_tier* ~ skill_budget? ~ (COMMA ~ skill_truncation)? }
    // The tiers, budget, truncation are nested inside skill_index_body.
    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IDENT => name = child.as_str().to_string(),
            Rule::skill_index_body => {
                // Drill into the body wrapper
                for body_child in child.into_inner() {
                    match body_child.as_rule() {
                        Rule::skill_tier => {
                            tiers.push(parse_skill_tier(body_child));
                        }
                        Rule::skill_budget => {
                            let budget_str = body_child.as_str();
                            if let Some(start) = budget_str.find(' ') {
                                let num_part = &budget_str[start + 1..];
                                if let Some(end) = num_part.find(' ') {
                                    if let Ok(val) = num_part[..end].trim().parse::<f64>() {
                                        budget = Some(val);
                                    }
                                }
                            }
                        }
                        Rule::skill_truncation => {
                            let trunc_str = body_child.as_str();
                            if let Some(pos) = trunc_str.find(':') {
                                let mode = trunc_str[pos + 1..].trim();
                                if mode == "whole_skill_only" {
                                    truncation = Some(TruncationMode::WholeSkillOnly);
                                } else {
                                    truncation = Some(TruncationMode::TruncateAtBoundary);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Declaration::SkillIndex(SkillIndexDecl {
        name,
        tiers,
        budget,
        truncation,
    })
}

pub(super) fn parse_skill_tier(pair: Pair<Rule>) -> SkillTier {
    let mut level: u32 = 0;
    let mut mode = String::new();
    let mut skills = Vec::new();
    let mut rules = Vec::new();

    // skill_tier_mode is a silent rule (_{}) in grammar.pest, so it won't
    // appear as a child Pair. Extract mode from the raw pair text instead.
    let raw = pair.as_str();
    if raw.contains("when_matches") {
        mode = "when_matches".to_string();
    } else if raw.contains("always") {
        mode = "always".to_string();
    }

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::INT => {
                level = child.as_str().parse().unwrap_or(0);
            }
            Rule::skill_tier_content => {
                // skill_tier_content = { tier_always_list | tier_matches_list }
                for content_child in child.into_inner() {
                    match content_child.as_rule() {
                        Rule::tier_always_list => {
                            for str_child in content_child.into_inner() {
                                if str_child.as_rule() == Rule::STRING_LITERAL {
                                    let s = str_child.as_str();
                                    skills.push(s[1..s.len() - 1].to_string());
                                } else if str_child.as_rule() == Rule::string_list {
                                    for lit in str_child.into_inner() {
                                        if lit.as_rule() == Rule::STRING_LITERAL {
                                            let s = lit.as_str();
                                            skills.push(s[1..s.len() - 1].to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Rule::tier_matches_list => {
                            for rule_child in content_child.into_inner() {
                                if rule_child.as_rule() == Rule::tier_match_rule {
                                    let mut skill_name = String::new();
                                    let mut triggers = Vec::new();
                                    for inner in rule_child.into_inner() {
                                        match inner.as_rule() {
                                            Rule::tier_match_skill => {
                                                for innermost in inner.into_inner() {
                                                    if innermost.as_rule() == Rule::STRING_LITERAL {
                                                        let s = innermost.as_str();
                                                        skill_name = s[1..s.len() - 1].to_string();
                                                    }
                                                }
                                            }
                                            Rule::tier_match_triggers => {
                                                for trig_child in inner.into_inner() {
                                                    if trig_child.as_rule() == Rule::STRING_LITERAL
                                                    {
                                                        let s = trig_child.as_str();
                                                        triggers
                                                            .push(s[1..s.len() - 1].to_string());
                                                    } else if trig_child.as_rule()
                                                        == Rule::string_list
                                                    {
                                                        for lit in trig_child.into_inner() {
                                                            if lit.as_rule() == Rule::STRING_LITERAL
                                                            {
                                                                let s = lit.as_str();
                                                                triggers.push(
                                                                    s[1..s.len() - 1].to_string(),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    rules.push(SkillTriggerRule {
                                        skill: skill_name,
                                        triggers,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    SkillTier {
        level,
        mode,
        skills,
        rules,
    }
}

pub(super) fn parse_schema_table(pair: Pair<Rule>) -> SchemaTable {
    let mut table_name = String::new();
    let mut columns = Vec::new();

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IDENT => table_name = child.as_str().to_string(),
            Rule::schema_column => {
                columns.push(parse_schema_column(child));
            }
            _ => {}
        }
    }

    SchemaTable {
        name: table_name,
        columns,
    }
}

pub(super) fn parse_schema_column(pair: Pair<Rule>) -> SchemaColumn {
    let mut col_name = String::new();
    let mut col_type = String::new();
    let mut modifiers = Vec::new();
    let mut default_val: Option<String> = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::IDENT => col_name = child.as_str().to_string(),
            Rule::schema_col_type => col_type = child.as_str().to_string(),
            Rule::schema_modifiers => {
                // Iterate into the modifiers group to get individual schema_modifier pairs
                for mod_child in child.into_inner() {
                    let mod_str = mod_child.as_str();
                    if mod_str == "primary_key" {
                        modifiers.push(ColumnModifier::PrimaryKey);
                    } else if mod_str == "auto_increment" {
                        modifiers.push(ColumnModifier::AutoIncrement);
                    } else if mod_str == "nullable" {
                        modifiers.push(ColumnModifier::Nullable);
                    } else if mod_str.starts_with("references") {
                        // Parse references(table.field)
                        let inner: Vec<&str> = mod_str
                            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                            .collect();
                        let idents: Vec<&str> = inner
                            .iter()
                            .filter(|s| !s.is_empty() && **s != "references")
                            .copied()
                            .collect();
                        if idents.len() >= 2 {
                            modifiers.push(ColumnModifier::References(
                                idents[0].to_string(),
                                idents[1].to_string(),
                            ));
                        }
                    }
                }
            }
            Rule::schema_default => {
                let default_str = child.as_str();
                // Extract content between parentheses
                if let Some(start) = default_str.find('(') {
                    if let Some(end) = default_str.rfind(')') {
                        default_val = Some(default_str[start + 1..end].to_string());
                    }
                }
            }
            _ => {}
        }
    }

    SchemaColumn {
        name: col_name,
        col_type,
        modifiers,
        default: default_val,
    }
}

// ── Memory Config (Phase 7.6) ──────────────────────────────────────

// ── Memory Config (Phase 7.6) ──────────────────────────────────────

pub(super) fn parse_memory_decl(pair: Pair<Rule>) -> Declaration {
    let children: Vec<Pair<Rule>> = pair
        .into_inner()
        .filter(|c| c.as_rule() == Rule::memory_body)
        .flat_map(|c| c.into_inner())
        .collect();

    let persist = children
        .iter()
        .find(|c| c.as_rule() == Rule::memory_persist)
        .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
        .map(|s| s[1..s.len() - 1].to_string());

    Declaration::Memory(MemoryDecl { persist })
}

// ── Import (Phase 5.4) ─────────────────────────────────────

// ── Import (Phase 5.4) ─────────────────────────────────────

pub(super) fn parse_import_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // import_decl = { IMPORT_KW ~ import_path ~ (AS_KW ~ IDENT)? }
    let path = find_child_str(&children, Rule::import_path).unwrap_or_default();
    let alias = find_child_str(&children, Rule::IDENT);
    Declaration::Import(ImportDecl { path, alias })
}

// ── Entity: struct type ─────────────────────────────────────────────

// ── Entity: struct type ─────────────────────────────────────────────

pub(super) fn parse_entity_type_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let fields: Vec<FieldDecl> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::field_decl)
        .map(|c| parse_field_decl(c.clone()))
        .collect();
    Declaration::EntityType(EntityTypeDecl { name, fields })
}

pub(super) fn parse_field_decl(pair: Pair<Rule>) -> FieldDecl {
    let children = children_of(&pair);
    // Children: IDENT, COLON, type_name, [ASSIGN, literal]
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let default =
        find_child(&children, Rule::literal).and_then(|lit| parse_literal_to_expr(&lit).ok());
    FieldDecl {
        name,
        type_name,
        default,
    }
}

// ── Entity: struct instance ─────────────────────────────────────────

/// Process escape sequences in a string literal (without outer quotes).
pub(super) fn parse_entity_record_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, type_name, field_init, ...
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let fields: Vec<FieldInit> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::field_init)
        .map(|c| parse_field_init(c.clone()))
        .collect::<Result<_, _>>()?;
    Ok(Declaration::EntityRecord(EntityRecordDecl {
        name,
        type_name,
        fields,
    }))
}

pub(super) fn parse_field_init(pair: Pair<Rule>) -> Result<FieldInit, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, COLON, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::expression in field_init",
        )
    })?;
    let value = parse_expression(expr_pair)?;
    Ok(FieldInit { name, value })
}

// ── Entity: simple (M1) ──────────────────────────────────────────────

// ── Entity: simple (M1) ──────────────────────────────────────────────

pub(super) fn parse_entity_simple_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, type_name, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::expression in entity_simple_decl",
        )
    })?;
    let value = parse_expression(expr_pair)?;
    Ok(Declaration::EntitySimple(EntitySimpleDecl {
        name,
        type_name,
        value,
    }))
}

// ── Rule ──────────────────────────────────────────────────────────────

// ── Rule ──────────────────────────────────────────────────────────────

pub(super) fn parse_rule_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: condition (contains/compare), assignment, [INT]
    let condition_pair = &children[0];
    let condition = parse_condition(condition_pair.clone())?;

    // assignment = { IDENT ~ "." ~ IDENT ~ "=" ~ expression }
    // Children: [IDENT(target), IDENT(field), expression(value)]
    let assignment_children = children_of(&children[1]);
    let target = Expr::Ident { name: pair_str(&assignment_children[0]), span: Span::unknown() };
    let field = pair_str(&assignment_children[1]);
    let value = parse_expression(assignment_children[2].clone())?;

    let priority = find_child_str(&children, Rule::INT)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    Ok(Declaration::Rule(RuleDecl {
        condition,
        target,
        field,
        value,
        priority,
    }))
}

pub(super) fn parse_condition(pair: Pair<Rule>) -> Result<Condition, ParseError> {
    match pair.as_rule() {
        Rule::contains_condition => {
            let children = children_of(&pair);
            // Children: expression, CONTAINS_KW, expression
            let left = parse_expression(children[0].clone())?;
            let right = parse_expression(children[2].clone())?;
            Ok(Condition::Contains { left, right })
        }
        Rule::compare_condition => {
            let children = children_of(&pair);
            // Children: expression, compare_op, expression
            Ok(Condition::Compare {
                left: parse_expression(children[0].clone())?,
                op: parse_compare_op(&children[1])?,
                right: parse_expression(children[2].clone())?,
            })
        }
        _ => Err(pair_error(
            &pair,
            "GRAMMAR INVARIANT: unknown condition type",
        ))?,
    }
}

pub(super) fn parse_compare_op(pair: &Pair<Rule>) -> Result<CompareOp, ParseError> {
    match pair.as_str().trim() {
        ">" => Ok(CompareOp::Gt),
        "<" => Ok(CompareOp::Lt),
        ">=" => Ok(CompareOp::Ge),
        "<=" => Ok(CompareOp::Le),
        "==" => Ok(CompareOp::Eq),
        _ => Err(pair_error(
            pair,
            "GRAMMAR INVARIANT: unknown compare operator",
        ))?,
    }
}

// ── Fluid Types (Phase 1) ──────────────────────────────────────────

// ── Fluid Types (Phase 1) ──────────────────────────────────────────

pub(super) fn parse_fluid_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // fluid_decl = { FLUID_KW ~ IDENT ~ "=" ~ fluid_branch ~ ("or" ~ fluid_branch)* }
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let variants: Vec<FluidVariant> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::fluid_branch)
        .map(|c| parse_fluid_branch(c.clone()))
        .collect::<Result<_, _>>()?;

    Ok(Declaration::Fluid(FluidDecl { name, variants }))
}

pub(super) fn parse_fluid_branch(pair: Pair<Rule>) -> Result<FluidVariant, ParseError> {
    let children = children_of(&pair);
    // fluid_branch = { type_name ~ LBRACKET ~ expression ~ RBRACKET ~ LBRACKET ~ FLOAT_LITERAL ~ RBRACKET }
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();

    let exprs: Vec<Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();
    let value = if !exprs.is_empty() {
        parse_expression(exprs[0].clone())?
    } else {
        Expr::StringLit { value: String::new(), span: Span::unknown() }
    };

    let floats: Vec<&Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::FLOAT_LITERAL)
        .collect();
    // The last FLOAT_LITERAL is the confidence (value may also be a float)
    let confidence = floats
        .last()
        .map(|f| f.as_str().parse().unwrap_or(0.0))
        .unwrap_or(0.0);

    Ok(FluidVariant {
        type_name,
        value,
        confidence,
    })
}

// ── Adapt (M5) ──────────────────────────────────────────────────

// ── Adapt (M5) ──────────────────────────────────────────────────

pub(super) fn parse_adapt_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // adapt_decl = { ADAPT_KW ~ IDENT ~ ADD_EXAMPLE_KW ~ "(" ~ expression ~ COMMA ~ expression ~ ")" }
    // Children: IDENT(pattern_name), "(", expression(input), ",", expression(output), ")"
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let exprs: Vec<Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();

    let input_example = if !exprs.is_empty() {
        parse_expression(exprs[0].clone())?
    } else {
        Expr::StringLit { value: String::new(), span: Span::unknown() }
    };
    let output_example = if exprs.len() >= 2 {
        parse_expression(exprs[1].clone())?
    } else {
        Expr::StringLit { value: String::new(), span: Span::unknown() }
    };

    Ok(Declaration::Adapt(AdaptDecl {
        pattern_name,
        input_example,
        output_example,
    }))
}

// ── Relate (knowledge graph edge) ──────────────────────────────

// ── Relate (knowledge graph edge) ──────────────────────────────

pub(super) fn parse_relate_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // relate_decl = { RELATE_KW ~ expression ~ "to" ~ expression ~ "as" ~ expression }
    // Children: expression(from), expression(to), expression(relation)
    let exprs: Vec<Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();
    let from = if !exprs.is_empty() {
        parse_expression(exprs[0].clone())?
    } else {
        Expr::StringLit { value: String::new(), span: Span::unknown() }
    };
    let to = if exprs.len() >= 2 {
        parse_expression(exprs[1].clone())?
    } else {
        Expr::StringLit { value: String::new(), span: Span::unknown() }
    };

    // Extract relation string from third expression
    let relation = if exprs.len() >= 3 {
        match parse_expression(exprs[2].clone())? {
            Expr::StringLit { value: s, .. } => s,
            _ => String::new(),
        }
    } else {
        String::new()
    };

    Ok(Declaration::Relate(RelateDecl { from, to, relation }))
}

// ── Hook (ADR-0045) ──────────────────────────────────────────────────

// ── Hook (ADR-0045) ──────────────────────────────────────────────────

pub(super) fn parse_hook_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // hook_decl = { HOOK_KW ~ hook_kind ~ "{" ~ statement* ~ "}" }
    let phase = children
        .iter()
        .find(|c| c.as_rule() == Rule::hook_kind)
        .map(|c| {
            let kind_children = children_of(c);
            if kind_children
                .iter()
                .any(|kc| kc.as_rule() == Rule::BEFORE_PATTERN_KW)
            {
                HookPhase::BeforePattern
            } else if kind_children
                .iter()
                .any(|kc| kc.as_rule() == Rule::AFTER_PATTERN_KW)
            {
                HookPhase::AfterPattern
            } else if kind_children
                .iter()
                .any(|kc| kc.as_rule() == Rule::ON_SESSION_START_KW)
            {
                HookPhase::OnSessionStart
            } else if kind_children
                .iter()
                .any(|kc| kc.as_rule() == Rule::ON_WRITE_KW)
            {
                HookPhase::OnWrite
            } else {
                HookPhase::OnSessionEnd
            }
        })
        .unwrap_or(HookPhase::BeforePattern);

    let body: Vec<Statement> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::statement)
        .map(|c| parse_single_statement(c.clone()))
        .collect::<Result<_, _>>()?;

    Ok(Declaration::Hook(HookDecl { phase, body }))
}

// ── Sandbox (P2) ────────────────────────────────────────────────

// ── Sandbox (P2) ────────────────────────────────────────────────

pub(super) fn parse_sandbox_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // sandbox_decl = { SANDBOX_KW ~ IDENT ~ "{" ~ sandbox_body "}" }
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let mut allowed = Vec::new();
    let mut forbidden = Vec::new();
    let mut timeout: i64 = 30;

    if let Some(body_pair) = find_child(&children, Rule::sandbox_body) {
        let body_children = children_of(&body_pair);
        // Extract allowed list
        if let Some(al_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::sandbox_allowed)
        {
            let al_children = children_of(al_pair);
            if let Some(il_pair) = al_children.iter().find(|c| c.as_rule() == Rule::ident_list) {
                allowed = il_pair
                    .clone()
                    .into_inner()
                    .filter(|c| c.as_rule() == Rule::IDENT)
                    .map(|c| pair_str(&c))
                    .collect();
            }
        }
        // Extract forbidden list
        if let Some(fb_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::sandbox_forbidden)
        {
            let fb_children = children_of(fb_pair);
            if let Some(il_pair) = fb_children.iter().find(|c| c.as_rule() == Rule::ident_list) {
                forbidden = il_pair
                    .clone()
                    .into_inner()
                    .filter(|c| c.as_rule() == Rule::IDENT)
                    .map(|c| pair_str(&c))
                    .collect();
            }
        }
        // Extract timeout
        if let Some(to_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::sandbox_timeout)
        {
            if let Some(int_val) = find_child_str(&children_of(to_pair), Rule::INT) {
                timeout = int_val.parse().unwrap_or(30);
            }
        }
    }

    Declaration::Sandbox(SandboxDecl {
        name,
        allowed,
        forbidden,
        timeout,
    })
}

// ── Mutate (P2) ─────────────────────────────────────────────────

// ── Mutate (P2) ─────────────────────────────────────────────────

pub(super) fn parse_mutate_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // mutate_decl = { MUTATE_KW ~ IDENT ~ "{" ~ mutate_body "}" }
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let mut new_examples: Vec<(Expr, Expr)> = Vec::new();
    let mut rollback_threshold: Option<f64> = None;
    let mut rollback_op: Option<CompareOp> = None;

    if let Some(body_pair) = find_child(&children, Rule::mutate_body) {
        let body_children = children_of(&body_pair);
        // Extract add_example pairs
        for ae_pair in body_children
            .iter()
            .filter(|c| c.as_rule() == Rule::mutate_add_example)
        {
            let ae_children = children_of(ae_pair);
            let exprs: Vec<Expr> = ae_children
                .iter()
                .filter(|c| c.as_rule() == Rule::expression)
                .map(|c| parse_expression(c.clone()))
                .collect::<Result<_, _>>()?;
            if exprs.len() >= 2 {
                new_examples.push((exprs[0].clone(), exprs[1].clone()));
            }
        }
        // Extract rollback_if condition
        if let Some(rb_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::mutate_rollback)
        {
            let rb_children = children_of(rb_pair);
            // Find compare_op and FLOAT_LITERAL
            if let Some(op_pair) = rb_children.iter().find(|c| c.as_rule() == Rule::compare_op) {
                rollback_op = Some(parse_compare_op(op_pair)?);
            }
            if let Some(float_pair) = rb_children
                .iter()
                .find(|c| c.as_rule() == Rule::FLOAT_LITERAL)
            {
                rollback_threshold = Some(float_pair.as_str().parse().unwrap_or(0.0));
            }
        }
    }

    Ok(Declaration::Mutate(MutateDecl {
        pattern_name,
        new_examples,
        rollback_threshold,
        rollback_op,
    }))
}

// ── Conversation Config (ADR-0053) ──────────────────────────────────

// ── Conversation Config (ADR-0053) ──────────────────────────────────

pub(super) fn parse_conversation_decl(pair: Pair<Rule>) -> Declaration {
    let children: Vec<Pair<Rule>> = pair
        .into_inner()
        .filter(|c| c.as_rule() == Rule::conversation_body)
        .flat_map(|c| c.into_inner())
        .collect();

    let ttl = children
        .iter()
        .find(|c| c.as_rule() == Rule::conversation_ttl)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);

    let max_messages = children
        .iter()
        .find(|c| c.as_rule() == Rule::conversation_max_messages)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    let compress_after = children
        .iter()
        .find(|c| c.as_rule() == Rule::conversation_compress_after)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    Declaration::Conversation(ConversationDecl {
        ttl,
        max_messages,
        compress_after,
    })
}

// ── Context Budget (sqz-inspired P3) ──────────────────────────────

// ── Context Budget (sqz-inspired P3) ──────────────────────────────

pub(super) fn parse_context_budget_decl(pair: Pair<Rule>) -> Declaration {
    use crate::ast::ContextBudgetDecl;
    let children: Vec<Pair<Rule>> = pair
        .into_inner()
        .filter(|c| c.as_rule() == Rule::context_budget_body)
        .flat_map(|c| c.into_inner())
        .collect();

    let pattern_name = children
        .iter()
        .find(|c| c.as_rule() == Rule::context_budget_pattern)
        .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
        .map(|s| s[1..s.len() - 1].to_string())
        .unwrap_or_default();

    let limit = children
        .iter()
        .find(|c| c.as_rule() == Rule::context_budget_limit)
        .and_then(|c| {
            let expr_children = children_of(c);
            for p in &expr_children {
                let s = p.as_str();
                if s == "limit" || s == ":" {
                    continue;
                }
                if let Ok(v) = s.parse::<f64>() {
                    return Some(v);
                }
            }
            None
        });

    Declaration::ContextBudget(ContextBudgetDecl {
        pattern_name,
        limit,
    })
}

// ── LLM Config (Наряд №4: Smart LLM Routing) ──────────────────────────

// ── LLM Config (Наряд №4: Smart LLM Routing) ──────────────────────────

pub(super) fn parse_llm_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children: Vec<Pair<Rule>> = pair
        .into_inner()
        .filter(|c| c.as_rule() == Rule::llm_body)
        .flat_map(|c| c.into_inner())
        .collect();

    // Parse providers list
    let mut providers = Vec::new();
    if let Some(pl_pair) = children.iter().find(|c| c.as_rule() == Rule::llm_providers) {
        // Flatten: llm_provider_list -> llm_provider_entry*
        let entries: Vec<Pair<Rule>> = pl_pair
            .clone()
            .into_inner()
            .flat_map(|c| {
                if c.as_rule() == Rule::llm_provider_entry {
                    vec![c]
                } else if c.as_rule() == Rule::llm_provider_list {
                    c.into_inner()
                        .filter(|inner| inner.as_rule() == Rule::llm_provider_entry)
                        .collect()
                } else {
                    vec![]
                }
            })
            .collect();

        for entry_pair in entries {
            let entry_children = children_of(&entry_pair);

            let alias = entry_children
                .iter()
                .find(|c| c.as_rule() == Rule::llm_provider_alias)
                .and_then(|c| find_child_str(&children_of(c), Rule::IDENT))
                .unwrap_or_default();

            let provider = entry_children
                .iter()
                .find(|c| c.as_rule() == Rule::llm_provider_name)
                .and_then(|c| find_child_str(&children_of(c), Rule::IDENT))
                .unwrap_or_default();

            let key = entry_children
                .iter()
                .find(|c| c.as_rule() == Rule::llm_provider_key)
                .and_then(|c| {
                    find_child(&children_of(c), Rule::expression)
                        .and_then(|e| parse_expression(e).ok())
                });

            let url = entry_children
                .iter()
                .find(|c| c.as_rule() == Rule::llm_provider_url)
                .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
                .map(|s| s[1..s.len() - 1].to_string());

            providers.push(LlmProviderEntry {
                alias,
                provider,
                key,
                url,
            });
        }
    }

    // Parse default_model
    let default_model = children
        .iter()
        .find(|c| c.as_rule() == Rule::llm_default_model)
        .and_then(|c| {
            find_child(&children_of(c), Rule::expression).and_then(|e| {
                parse_expression(e).ok().and_then(|expr| {
                    if let Expr::StringLit { value: s, .. } = expr {
                        Some(s)
                    } else {
                        None
                    }
                })
            })
        });

    // Parse failover mode
    let failover = children
        .iter()
        .find(|c| c.as_rule() == Rule::llm_failover)
        .and_then(|c| find_child_str(&children_of(c), Rule::IDENT));

    // Parse circuit_breaker
    let circuit_breaker = children
        .iter()
        .find(|c| c.as_rule() == Rule::llm_circuit_breaker)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    // Parse timeout
    let timeout = children
        .iter()
        .find(|c| c.as_rule() == Rule::llm_timeout)
        .and_then(|c| find_child_str(&children_of(c), Rule::INT))
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    Ok(Declaration::LlmConfig(LlmConfigDecl {
        providers,
        default_model,
        failover,
        circuit_breaker,
        timeout,
    }))
}

// ── Tool Abstraction (ADR-0054) ──────────────────────────────────────

// ── Tool Abstraction (ADR-0054) ──────────────────────────────────────

pub(super) fn parse_tool_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // tool_decl = { TOOL_KW ~ IDENT ~ "{" ~ tool_method* ~ "}" }
    // First IDENT is the tool name; subsequent children are tool_method nodes.
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let methods: Vec<ToolMethod> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::tool_method)
        .map(|c| parse_tool_method(c.clone()))
        .collect::<Result<_, _>>()?;

    Ok(Declaration::Tool(ToolDecl { name, methods }))
}

pub(super) fn parse_tool_method(pair: Pair<Rule>) -> Result<ToolMethod, ParseError> {
    let children = children_of(&pair);
    // tool_method = { IDENT ~ "(" ~ params? ~ ")" ~ ARROW ~ type_name ~ LBRACE ~ statement* ~ RBRACE }
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let body: Vec<Statement> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::statement)
        .map(|c| parse_single_statement(c.clone()))
        .collect::<Result<_, _>>()?;
    Ok(ToolMethod {
        name,
        params,
        return_type,
        body,
    })
}

// ── Eval Harness (ADR-0050) ──────────────────────────────────────────

// ── Eval Harness (ADR-0050) ──────────────────────────────────────────

pub(super) fn parse_eval_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // eval_decl = { EVAL_KW ~ IDENT ~ "{" ~ eval_body ~ "}" }
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let mut dataset: Vec<(String, String)> = Vec::new();
    let mut metric = "accuracy".to_string();
    let mut threshold: f64 = 0.8;

    if let Some(body_pair) = find_child(&children, Rule::eval_body) {
        let body_children = children_of(&body_pair);

        // Extract dataset: [("input", "expected"), ...]
        if let Some(ds_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::eval_dataset)
        {
            // eval_example is nested inside eval_example_list (if present)
            // or directly in eval_dataset when there's a single example.
            // Flatten both levels to collect all eval_example pairs.
            let examples: Vec<Pair<Rule>> = ds_pair
                .clone()
                .into_inner()
                .flat_map(|c| {
                    if c.as_rule() == Rule::eval_example {
                        vec![c]
                    } else if c.as_rule() == Rule::eval_example_list {
                        c.into_inner()
                            .filter(|inner| inner.as_rule() == Rule::eval_example)
                            .collect()
                    } else {
                        vec![]
                    }
                })
                .collect();
            for ex_pair in examples {
                let strings: Vec<Pair<Rule>> = ex_pair
                    .clone()
                    .into_inner()
                    .filter(|c| c.as_rule() == Rule::STRING_LITERAL)
                    .collect();
                let input = if !strings.is_empty() {
                    unescape_string(strings[0].as_str())
                } else {
                    String::new()
                };
                let expected = if strings.len() >= 2 {
                    unescape_string(strings[1].as_str())
                } else {
                    String::new()
                };
                dataset.push((input, expected));
            }
        }

        // Extract metric: accuracy (or future metrics)
        if let Some(m_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::eval_metric)
        {
            let m_children = children_of(m_pair);
            metric =
                find_child_str(&m_children, Rule::IDENT).unwrap_or_else(|| "accuracy".to_string());
        }

        // Extract threshold: 0.8
        if let Some(t_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::eval_threshold)
        {
            let t_children = children_of(t_pair);
            threshold = find_child_str(&t_children, Rule::FLOAT_LITERAL)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.8);
        }
    }

    Declaration::Eval(EvalDecl {
        pattern_name,
        dataset,
        metric,
        threshold,
    })
}

// ── Memorize (M4) ──────────────────────────────────────────────────

// ── Memorize (M4) ──────────────────────────────────────────────────

pub(super) fn parse_memorize_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: expression, ["with", "priority", "=", FLOAT_LITERAL]
    let value = find_child(&children, Rule::expression).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::expression in memorize_decl",
        )
    })?;
    let value = parse_expression(value)?;

    let priority = find_child(&children, Rule::FLOAT_LITERAL)
        .map(|f| f.as_str().parse().unwrap_or(0.5))
        .unwrap_or(0.5);

    Ok(Declaration::Memorize(MemorizeDecl { value, priority }))
}

pub(super) fn parse_forget_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: expression, INT, "days"
    let query = find_child(&children, Rule::expression).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::expression in forget_decl",
        )
    })?;
    let query = parse_expression(query)?;

    let days = find_child(&children, Rule::INT)
        .map(|i| i.as_str().parse().unwrap_or(30))
        .unwrap_or(30);

    Ok(Declaration::Forget(ForgetDecl { query, days }))
}

// ── Learnable Pattern (M3) ────────────────────────────────────────────

// ── Learnable Pattern (M3) ────────────────────────────────────────────

pub(super) fn parse_learnable_pattern_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, [params], ARROW, type_name, learnable_body
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();

    // Extract prompt, context, model, max_tokens, cache, cache_ttl from learnable_body
    let mut prompt = String::new();
    let mut context: Option<ContextMode> = None;
    let mut model: Option<String> = None;
    let mut max_tokens: Option<u32> = None;
    let mut cache = false;
    let mut cache_ttl: u64 = 3600; // default 1 hour
    let mut conversation: Option<String> = None;
    let mut context_strategy: ContextStrategy = ContextStrategy::None;
    let mut max_context_tokens: usize = 2000; // default 2000

    if let Some(body_pair) = find_child(&children, Rule::learnable_body) {
        let body_children = children_of(&body_pair);

        // Extract prompt from prompt_line -> expression
        if let Some(pl_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::prompt_line)
        {
            let pl_children = children_of(pl_pair);
            if let Some(expr_pair) = pl_children.iter().find(|c| c.as_rule() == Rule::expression) {
                if let Expr::StringLit { value: s, .. } = parse_expression(expr_pair.clone())? {
                    prompt = s;
                }
            }
        }

        // Extract context: supports recall(...), auto, none, or string literal
        // NOTE: context_line uses _{ } (silent choice), so the inner rule appears
        // directly in body_children, NOT wrapped in a context_line node.
        let ctx_pair = body_children.iter().find(|c| {
            matches!(
                c.as_rule(),
                Rule::context_recall_line
                    | Rule::context_auto_line
                    | Rule::context_none_line
                    | Rule::context_literal_line
            )
        });
        if let Some(inner_pair) = ctx_pair {
            match inner_pair.as_rule() {
                Rule::context_recall_line => {
                    // context: recall(query_expr, limit=N)
                    let inner_children = children_of(inner_pair);
                    let exprs: Vec<Expr> = inner_children
                        .iter()
                        .filter(|c| c.as_rule() == Rule::expression)
                        .cloned()
                        .map(|p| parse_expression(p))
                        .collect::<Result<_, _>>()?;
                    if !exprs.is_empty() {
                        let limit = if exprs.len() >= 2 {
                            if let Expr::FloatLit { value: n, .. } = exprs[1].clone() {
                                Some(n as usize)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        context = Some(ContextMode::Recall(exprs[0].clone(), limit));
                    }
                }
                Rule::context_auto_line => {
                    // context: auto → recall(first_param, limit=5)
                    context = Some(ContextMode::Auto);
                }
                Rule::context_none_line => {
                    // context: none → explicitly no context
                    context = Some(ContextMode::None);
                }
                Rule::context_literal_line => {
                    // context: "some string" or context: expr
                    let inner_children = children_of(inner_pair);
                    if let Some(expr_pair) = inner_children
                        .iter()
                        .find(|c| c.as_rule() == Rule::expression)
                    {
                        if let Expr::StringLit { value: s, .. } = parse_expression(expr_pair.clone())? {
                            context = Some(ContextMode::Literal(s));
                        }
                    }
                }
                _ => {}
            }
        }

        // Extract conversation: "current" or conversation: <expr> (ADR-0053)
        if let Some(conv_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::conversation_line)
        {
            let conv_children = children_of(conv_pair);
            if let Some(expr_pair) = conv_children
                .iter()
                .find(|c| c.as_rule() == Rule::expression)
            {
                if let Expr::StringLit { value: s, .. } = parse_expression(expr_pair.clone())? {
                    conversation = Some(s);
                }
            }
        }

        // Extract model: "haiku" (ADR-0048)
        if let Some(m_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::model_line)
        {
            let m_children = children_of(m_pair);
            if let Some(expr_pair) = m_children.iter().find(|c| c.as_rule() == Rule::expression) {
                if let Expr::StringLit { value: s, .. } = parse_expression(expr_pair.clone())? {
                    model = Some(s);
                }
            }
        }

        // Extract max_tokens: N
        if let Some(mt_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::max_tokens_line)
        {
            let mt_children = children_of(mt_pair);
            if let Some(expr_pair) = mt_children.iter().find(|c| c.as_rule() == Rule::expression) {
                if let Expr::FloatLit { value: n, .. } = parse_expression(expr_pair.clone())? {
                    max_tokens = Some(n as u32);
                }
            }
        }

        // Extract cache: true/false
        if let Some(c_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::cache_line)
        {
            let c_str = c_pair.as_str().trim();
            // cache_line = { "cache" ~ ":" ~ BOOL_LITERAL }
            // Extract the boolean value after the colon
            if let Some(colon_pos) = c_str.find(':') {
                let val_str = c_str[colon_pos + 1..].trim();
                cache = val_str == "true";
            }
        }

        // Extract cache_ttl: N.minutes
        if let Some(ttl_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::cache_ttl_line)
        {
            let ttl_children = children_of(ttl_pair);
            // cache_ttl_line = { "cache_ttl" ~ ":" ~ expression ~ "." ~ IDENT }
            let exprs: Vec<Expr> = ttl_children
                .iter()
                .filter(|c| c.as_rule() == Rule::expression)
                .cloned()
                .map(|p| parse_expression(p))
                .collect::<Result<_, _>>()?;
            let unit_ident = ttl_children
                .iter()
                .filter(|c| c.as_rule() == Rule::IDENT)
                .map(|c| pair_str(c))
                .next()
                .unwrap_or_default();
            if let Some(Expr::FloatLit { value: n, .. }) = exprs.first() {
                let n_val = *n as u64;
                cache_ttl = match unit_ident.as_str() {
                    "seconds" | "second" => n_val,
                    "minutes" | "minute" => n_val * 60,
                    "hours" | "hour" => n_val * 3600,
                    "days" | "day" => n_val * 86400,
                    _ => n_val * 60, // default to minutes
                };
            }
        }

        // Extract context_strategy: none | auto | compress (ADR-0055)
        if let Some(cs_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::context_strategy_line)
        {
            let cs_str = cs_pair.as_str().trim();
            // context_strategy_line = { "context_strategy" ~ ":" ~ context_strategy_value }
            if let Some(colon_pos) = cs_str.find(':') {
                let val_str = cs_str[colon_pos + 1..].trim();
                context_strategy = match val_str {
                    "auto" => ContextStrategy::Auto,
                    "compress" => ContextStrategy::Compress,
                    _ => ContextStrategy::None,
                };
            }
        }

        // Extract max_context_tokens: N (ADR-0055)
        if let Some(mct_pair) = body_children
            .iter()
            .find(|c| c.as_rule() == Rule::max_context_tokens_line)
        {
            if let Some(int_val) = find_child_str(&children_of(mct_pair), Rule::INT) {
                max_context_tokens = int_val.parse().unwrap_or(2000);
            }
        }
    }

    Ok(Declaration::LearnablePattern(LearnablePatternDecl {
        name,
        params,
        return_type,
        prompt,
        context,
        context_strategy,
        max_context_tokens,
        model,
        max_tokens,
        cache,
        cache_ttl,
        conversation,
    }))
}

// ── Pattern ──────────────────────────────────────────────────────────

// ── Pattern ──────────────────────────────────────────────────────────

pub(super) fn parse_pattern_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, params, ARROW, type_name, pattern_body
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let body = find_child(&children, Rule::pattern_body)
        .map(|p| parse_pattern_body(p))
        .transpose()?
        .unwrap_or_default();
    Ok(Declaration::Pattern(PatternDecl {
        name,
        params,
        return_type,
        body,
    }))
}

// ── Flow ──────────────────────────────────────────────────────────────
// flow_decl = { "flow" ~ IDENT ~ "{" ~ flow_pipeline ~ branch_def* ~ "}" }
// flow_pipeline = { "input" ":" type_name "=" expression ~ flow_step* ~ ARROW ~ "output" }
// flow_step     = { ARROW ~ (checkpoint_call | step_ident) }
// checkpoint_call = { "checkpoint" ~ "(" ~ STRING_LITERAL ~ ")" }
// branch_def    = { step_ident ~ "{" ~ branch* ~ "}" }

pub(super) fn parse_flow_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    // children: IDENT, flow_pipeline, [branch_def, ...]
    let pipeline_pair = find_child(&children, Rule::flow_pipeline).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::flow_pipeline in flow_decl",
        )
    })?;
    let pipeline_children = children_of(&pipeline_pair);

    let mut input_type = String::new();
    let mut source: Option<Expr> = None;
    let mut pipeline_steps: Vec<String> = Vec::new();
    let mut checkpoints: HashMap<String, usize> = HashMap::new();

    // Walk pipeline children: type_name, expression, flow_step*, ARROW
    let mut i = 0;
    // First: type_name
    if i < pipeline_children.len() && pipeline_children[i].as_rule() == Rule::type_name {
        let tn_inner = children_of(&pipeline_children[i]);
        input_type = pair_str(&tn_inner[0]);
        i += 1;
    }
    // Second: expression (source)
    if i < pipeline_children.len() && pipeline_children[i].as_rule() == Rule::expression {
        source = Some(parse_expression(pipeline_children[i].clone())?);
        i += 1;
    }
    // Remaining: flow_step* then final ARROW -> output
    while i < pipeline_children.len() {
        if pipeline_children[i].as_rule() == Rule::flow_step {
            let step_children = children_of(&pipeline_children[i]);
            // flow_step = { ARROW ~ (checkpoint_call | step_ident) }
            for sc in &step_children {
                if sc.as_rule() == Rule::step_ident {
                    pipeline_steps.push(pair_str(sc));
                } else if sc.as_rule() == Rule::checkpoint_call {
                    // checkpoint("name") — maps to the PRECEDING step index
                    let full = sc.as_str(); // e.g., 'checkpoint("mid")'
                                            // Extract the checkpoint name between quotes
                    let cp_name = if let Some(s) = full.find('"') {
                        if let Some(e) = full.rfind('"') {
                            if e > s + 1 {
                                full[s + 1..e].to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    // Checkpoint fires AFTER the last pipeline step added
                    let step_idx = if pipeline_steps.is_empty() {
                        0
                    } else {
                        pipeline_steps.len() - 1
                    };
                    checkpoints.insert(cp_name, step_idx);
                }
            }
            i += 1;
        } else if pipeline_children[i].as_rule() == Rule::ARROW {
            // Final ARROW before "output" — skip
            i += 1;
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
            let branches: Vec<Branch> = bd_children
                .iter()
                .filter(|c| c.as_rule() == Rule::branch)
                .map(|c| parse_branch(c.clone()))
                .collect::<Result<_, _>>()?;
            branch_defs.push((step_name, branches));
        }
    }

    Ok(Declaration::Flow(FlowDecl {
        name: name.clone(),
        input_type,
        source: source.unwrap_or_else(|| Expr::StringLit { value: String::new(), span: Span::unknown() }),
        pipeline: pipeline_steps.clone(),
        branch_defs,
        checkpoints: checkpoints.clone(),
    }))
}

pub(super) fn parse_branch(pair: Pair<Rule>) -> Result<Branch, ParseError> {
    let children = children_of(&pair);
    // branch = { IDENT ~ "(" ~ branch_condition ~ ")" ~ ARROW ~ step_ident }
    let label = pair_str(&children[0]);
    let cond_pair = find_child(&children, Rule::branch_condition).ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected Rule::branch_condition in branch",
        )
    })?;
    let target = pair_str(children.last().ok_or_else(|| {
        pair_error(
            &pair,
            "GRAMMAR INVARIANT: expected step_ident at end of branch",
        )
    })?);
    Ok(Branch {
        label,
        condition: parse_branch_condition(cond_pair)?,
        target,
    })
}

pub(super) fn parse_branch_condition(pair: Pair<Rule>) -> Result<BranchCondition, ParseError> {
    let children = children_of(&pair);
    // branch_condition = { IDENT ~ "." ~ IDENT ~ compare_op ~ expression }
    // Children: [IDENT(target), IDENT(field), compare_op, expression(threshold)]
    Ok(BranchCondition {
        target: Expr::Ident { name: pair_str(&children[0]), span: Span::unknown() },
        field: pair_str(&children[1]),
        op: parse_compare_op(&children[2])?,
        threshold: parse_expression(children[3].clone())?,
    })
}

// ── Type Alias (Наряд №119) ─────────────────────────────────────

/// `type Token = Secret`
pub(super) fn parse_type_alias_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let alias = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let target = find_child_str(&children, Rule::type_name).unwrap_or_default();
    Declaration::TypeAlias(TypeAliasDecl {
        alias: alias.to_string(),
        target: target.to_string(),
    })
}

// ── Test Declaration (Наряд №120) ─────────────────────────────────

/// `test "name" { <statements> }`
pub(super) fn parse_test_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::STRING_LITERAL)
        .map(|s| unescape_string(s.trim_matches('"')))
        .unwrap_or_default();
    let body: Vec<Statement> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::statement)
        .filter_map(|s| parse_single_statement(s.clone()).ok())
        .collect();
    Declaration::Test(TestDecl { name, body })
}

// ── Expressions ─────────────────────────────────────────────────────
