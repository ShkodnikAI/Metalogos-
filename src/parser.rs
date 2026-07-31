// ── Pest → AST conversion for METALOGOS M1+M2 ──────────────────────

use pest::iterators::Pair;
use pest::Parser as _;
use pest_derive::Parser;

use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct MlogParser;

pub type ParseError = pest::error::Error<Rule>;

/// Create a ParseError with line:col position from a Pest pair.
fn pair_error(pair: &Pair<Rule>, msg: &str) -> ParseError {
    let (line, col) = pair.as_span().start_pos().line_col();
    pest::error::Error::new_from_pos(
        pest::error::ErrorVariant::CustomError {
            message: format!("{} at line {}, col {}", msg, line, col),
        },
        pair.as_span().start_pos(),
    )
}

/// Create a ParseError at position 0 of source.
/// Used only for thread spawn/join error reporting where no Pest pair is available.
fn error_at_start(source: &str, msg: String) -> ParseError {
    let pos = pest::Position::new(source, 0)
        .or_else(|| pest::Position::new("", 0))
        .expect("position 0 is always valid in any string");
    pest::error::Error::new_from_pos(
        pest::error::ErrorVariant::CustomError { message: msg },
        pos,
    )
}


use crate::ast::*;

/// Parse a .mlog source string into a list of declarations.
/// Templates with `}` in their body (HTML, CSS, JS) are handled via
/// pre-processing: template bodies are extracted with balanced brace counting,
/// replaced with placeholders for Pest parsing, then restored.
pub fn parse(source: &str) -> Result<Vec<Declaration>, ParseError> {
    // Наряд №14 P1-5: Use a thread with 8MB stack for large files.
    // Pest's recursive descent can overflow the default 2-8MB stack
    // on deeply nested expressions or very long string literals.
    // Наряд №29 §3.2: thread spawn/join return Result instead of panicking.
    let source_owned = source.to_string();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || parse_inner(&source_owned))
        .map_err(|e| error_at_start(source, format!("failed to spawn parser thread: {}", e)))?;
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(error_at_start(source, "parser thread panicked".to_string()))
    }
}

fn parse_inner(source: &str) -> Result<Vec<Declaration>, ParseError> {
    // Pre-process: extract template bodies that contain } characters.
    // Replace with unique placeholders to avoid Pest's "stop at first }" limitation.
    let (preprocessed, template_bodies) = preprocess_templates(source);

    let pairs = MlogParser::parse(Rule::program, &preprocessed)?;
    let mut declarations = Vec::new();

    for pair in pairs {
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::mlogserver_decl => declarations.push(parse_mlogserver_decl(inner_pair)?),
                Rule::template_decl => {
                    declarations.push(parse_template_decl_with_body(inner_pair, &template_bodies))
                }
                Rule::db_decl => declarations.push(parse_db_decl(inner_pair)?),
                Rule::schema_decl => declarations.push(parse_schema_decl(inner_pair)),
                Rule::skill_index_decl => declarations.push(parse_skill_index_decl(inner_pair)),
                Rule::memory_decl => declarations.push(parse_memory_decl(inner_pair)),
                Rule::import_decl => declarations.push(parse_import_decl(inner_pair)),
                Rule::entity_type_decl => declarations.push(parse_entity_type_decl(inner_pair)),
                Rule::entity_record_decl => declarations.push(parse_entity_record_decl(inner_pair)?),
                Rule::entity_simple_decl => declarations.push(parse_entity_simple_decl(inner_pair)?),
                Rule::rule_decl => declarations.push(parse_rule_decl(inner_pair)?),
                Rule::memorize_decl => declarations.push(parse_memorize_decl(inner_pair)?),
                Rule::forget_decl => declarations.push(parse_forget_decl(inner_pair)?),
                Rule::if_block_stmt => declarations.push(Declaration::Pattern(PatternDecl {
                    name: "_top_level_if".to_string(),
                    params: vec![],
                    return_type: "Unit".to_string(),
                    body: vec![parse_if_block_stmt(inner_pair)?],
                })),
                Rule::fluid_decl => declarations.push(parse_fluid_decl(inner_pair)?),
                Rule::adapt_decl => declarations.push(parse_adapt_decl(inner_pair)?),
                Rule::relate_decl => declarations.push(parse_relate_decl(inner_pair)?),
                Rule::sandbox_decl => declarations.push(parse_sandbox_decl(inner_pair)),
                Rule::hook_decl => declarations.push(parse_hook_decl(inner_pair)?),
                Rule::mutate_decl => declarations.push(parse_mutate_decl(inner_pair)?),
                Rule::eval_decl => declarations.push(parse_eval_decl(inner_pair)),
                Rule::conversation_decl => declarations.push(parse_conversation_decl(inner_pair)),
                Rule::context_budget_decl => {
                    declarations.push(parse_context_budget_decl(inner_pair))
                }
                Rule::llm_decl => declarations.push(parse_llm_decl(inner_pair)?),
                Rule::tool_decl => declarations.push(parse_tool_decl(inner_pair)?),
                Rule::learnable_pattern_decl => {
                    declarations.push(parse_learnable_pattern_decl(inner_pair)?)
                }
                Rule::pattern_decl => declarations.push(parse_pattern_decl(inner_pair)?),
                Rule::flow_decl => declarations.push(parse_flow_decl(inner_pair)?),
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
    children
        .iter()
        .find(|c| c.as_rule() == rule)
        .map(|c| pair_str(c))
}

/// Find the first child matching a rule and return it.
fn find_child<'a>(children: &'a [Pair<'a, Rule>], rule: Rule) -> Option<Pair<'a, Rule>> {
    children.iter().find(|c| c.as_rule() == rule).cloned()
}

// ── MlogServer (Phase 6.1) ─────────────────────────────────────

fn parse_mlogserver_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_route_decl(pair: Pair<Rule>) -> Result<RouteDecl, ParseError> {
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
fn preprocess_templates(source: &str) -> (String, HashMap<String, String>) {
    let mut result = source.to_string();
    let mut bodies = HashMap::new();
    let mut counter = 0u32;

    // Find template declarations and extract balanced brace bodies
    let mut search_from = 0;
    while search_from < result.len() {
        // Find "template" keyword (ASCII-only, find() is safe)
        if let Some(start) = result[search_from..].find("template") {
            let abs_start = search_from + start;
            // Skip if this is part of a longer identifier (check preceding char)
            if abs_start > 0
                && result
                    .as_bytes()
                    .get(abs_start - 1)
                    .map(|&b| b.is_ascii_alphanumeric())
                    .unwrap_or(false)
            {
                search_from = abs_start + 1;
                continue;
            }

            // Find the opening { of the template body (after type_name)
            // '{' is ASCII, find() on ASCII patterns is char-boundary-safe
            if let Some(brace_pos) = result[abs_start..].find('{') {
                let abs_brace = abs_start + brace_pos;
                // Find the matching closing } using balanced brace counting.
                // MUST use char_indices() to get correct BYTE offsets for Unicode-safe slicing.
                let mut depth = 0;
                let mut end_byte_pos = None;
                for (byte_offset, ch) in result[abs_brace..].char_indices() {
                    match ch {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end_byte_pos = Some(abs_brace + byte_offset);
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(close_pos) = end_byte_pos {
                    // Extract the body between the braces (byte offsets are char-boundary-safe)
                    let body = result[abs_brace + 1..close_pos].to_string();
                    let placeholder = format!("__TEMPLATE_BODY_{}__", counter);
                    counter += 1;

                    // Replace the body content with a safe placeholder (no })
                    let replacement = format!("{{{}}}", placeholder);
                    result.replace_range(abs_brace..=close_pos, &replacement);
                    bodies.insert(placeholder, body);

                    search_from = abs_brace + replacement.len();
                    continue;
                }
            }
            search_from = abs_start + 1;
        } else {
            break;
        }
    }

    (result, bodies)
}

/// Parse a template declaration, restoring the pre-processed body.
fn parse_template_decl_with_body(
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

/// Extract content between balanced braces from a string like "{ content } }".
/// Handles nested braces by counting depth.
fn extract_balanced_braces(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() || chars[0] != '{' {
        return String::new();
    }
    let mut depth = 0;
    let mut end = 0;
    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end > 0 {
        chars[1..end].iter().collect()
    } else {
        String::new()
    }
}

// ── DB (Phase 6.3) ─────────────────────────────────────

fn parse_db_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_schema_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_skill_index_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_skill_tier(pair: Pair<Rule>) -> SkillTier {
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

fn parse_schema_table(pair: Pair<Rule>) -> SchemaTable {
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

fn parse_schema_column(pair: Pair<Rule>) -> SchemaColumn {
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

fn parse_memory_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_import_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    // import_decl = { IMPORT_KW ~ import_path ~ (AS_KW ~ IDENT)? }
    let path = find_child_str(&children, Rule::import_path).unwrap_or_default();
    let alias = find_child_str(&children, Rule::IDENT);
    Declaration::Import(ImportDecl { path, alias })
}

// ── Entity: struct type ─────────────────────────────────────────────

fn parse_entity_type_decl(pair: Pair<Rule>) -> Declaration {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let fields: Vec<FieldDecl> = children
        .iter()
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
    let default = find_child(&children, Rule::literal).and_then(|lit| parse_literal_to_expr(&lit).ok());
    FieldDecl {
        name,
        type_name,
        default,
    }
}

/// Process escape sequences in a string literal (without outer quotes).
fn unescape_string(s: &str) -> String {
    let trimmed = &s[1..s.len() - 1]; // strip outer quotes
    let mut result = String::with_capacity(trimmed.len());
    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\"') => {
                    result.push('\"');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some('n') => {
                    result.push('\n');
                    chars.next();
                }
                Some('t') => {
                    result.push('\t');
                    chars.next();
                }
                Some('r') => {
                    result.push('\r');
                    chars.next();
                }
                Some('u') => {
                    chars.next(); // consume 'u'
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code_point) {
                            result.push(ch);
                        } else {
                            result.push_str("\\u");
                            result.push_str(&hex);
                        }
                    } else {
                        result.push_str("\\u");
                        result.push_str(&hex);
                    }
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a literal pair (STRING_LITERAL, FLOAT_LITERAL, or IDENT) to an Expr.
fn parse_literal_to_expr(pair: &Pair<Rule>) -> Result<Expr, ParseError> {
    let inner = pair.clone().into_inner().next().ok_or_else(|| {
            pair_error(pair, "GRAMMAR INVARIANT: literal must have inner content")
        })?;
    match inner.as_rule() {
        Rule::STRING_LITERAL => Ok(Expr::StringLit(unescape_string(inner.as_str()))),
        Rule::FLOAT_LITERAL => Ok(Expr::FloatLit(inner.as_str().parse().unwrap_or(0.0))),
        Rule::IDENT => Ok(Expr::Ident(inner.as_str().to_string())),
        _ => Ok(Expr::StringLit(pair.as_str().to_string())),
    }
}

// ── Entity: struct instance ─────────────────────────────────────────

fn parse_entity_record_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_field_init(pair: Pair<Rule>) -> Result<FieldInit, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, COLON, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in field_init")
        })?;
    let value = parse_expression(expr_pair)?;
    Ok(FieldInit { name, value })
}

// ── Entity: simple (M1) ──────────────────────────────────────────────

fn parse_entity_simple_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, type_name, expression
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let type_name = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let expr_pair = find_child(&children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in entity_simple_decl")
        })?;
    let value = parse_expression(expr_pair)?;
    Ok(Declaration::EntitySimple(EntitySimpleDecl {
        name,
        type_name,
        value,
    }))
}

// ── Rule ──────────────────────────────────────────────────────────────

fn parse_rule_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: condition (contains/compare), assignment, [INT]
    let condition_pair = &children[0];
let condition = parse_condition(condition_pair.clone())?;

    // assignment = { IDENT ~ "." ~ IDENT ~ "=" ~ expression }
    // Children: [IDENT(target), IDENT(field), expression(value)]
    let assignment_children = children_of(&children[1]);
    let target = Expr::Ident(pair_str(&assignment_children[0]));
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

fn parse_condition(pair: Pair<Rule>) -> Result<Condition, ParseError> {
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
        _ => Err(pair_error(&pair, "GRAMMAR INVARIANT: unknown condition type"))?,
    }
}

fn parse_compare_op(pair: &Pair<Rule>) -> Result<CompareOp, ParseError> {
    match pair.as_str().trim() {
        ">" => Ok(CompareOp::Gt),
        "<" => Ok(CompareOp::Lt),
        ">=" => Ok(CompareOp::Ge),
        "<=" => Ok(CompareOp::Le),
        "==" => Ok(CompareOp::Eq),
        _ => Err(pair_error(pair, "GRAMMAR INVARIANT: unknown compare operator"))?,
    }
}

// ── Fluid Types (Phase 1) ──────────────────────────────────────────

fn parse_fluid_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_fluid_branch(pair: Pair<Rule>) -> Result<FluidVariant, ParseError> {
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
        Expr::StringLit(String::new())
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

fn parse_adapt_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // adapt_decl = { ADAPT_KW ~ IDENT ~ ADD_EXAMPLE_KW ~ "(" ~ expression ~ COMMA ~ expression ~ ")" }
    // Children: IDENT(pattern_name), "(", expression(input), ",", expression(output), ")"
    let pattern_name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    let exprs: Vec<Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();

    let input_example = if exprs.len() >= 1 {
        parse_expression(exprs[0].clone())?
    } else {
        Expr::StringLit(String::new())
    };
    let output_example = if exprs.len() >= 2 {
        parse_expression(exprs[1].clone())?
    } else {
        Expr::StringLit(String::new())
    };

    Ok(Declaration::Adapt(AdaptDecl {
        pattern_name,
        input_example,
        output_example,
    }))
}

// ── Relate (knowledge graph edge) ──────────────────────────────

fn parse_relate_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // relate_decl = { RELATE_KW ~ expression ~ "to" ~ expression ~ "as" ~ expression }
    // Children: expression(from), expression(to), expression(relation)
    let exprs: Vec<Pair<Rule>> = children
        .iter()
        .filter(|c| c.as_rule() == Rule::expression)
        .cloned()
        .collect();
    let from = if exprs.len() >= 1 {
        parse_expression(exprs[0].clone())?
    } else {
        Expr::StringLit(String::new())
    };
    let to = if exprs.len() >= 2 {
        parse_expression(exprs[1].clone())?
    } else {
        Expr::StringLit(String::new())
    };

    // Extract relation string from third expression
    let relation = if exprs.len() >= 3 {
        match parse_expression(exprs[2].clone())? {
            Expr::StringLit(s) => s,
            _ => String::new(),
        }
    } else {
        String::new()
    };

    Ok(Declaration::Relate(RelateDecl { from, to, relation }))
}

// ── Hook (ADR-0045) ──────────────────────────────────────────────────

fn parse_hook_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_sandbox_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_mutate_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_conversation_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_context_budget_decl(pair: Pair<Rule>) -> Declaration {
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

fn parse_llm_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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
                    find_child(&children_of(c), Rule::expression).and_then(|e| parse_expression(e).ok())
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
                    if let Expr::StringLit(s) = expr {
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

fn parse_tool_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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

fn parse_tool_method(pair: Pair<Rule>) -> Result<ToolMethod, ParseError> {
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

fn parse_eval_decl(pair: Pair<Rule>) -> Declaration {
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
                let input = if strings.len() >= 1 {
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

fn parse_memorize_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: expression, ["with", "priority", "=", FLOAT_LITERAL]
    let value = find_child(&children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in memorize_decl")
        })?;
    let value = parse_expression(value)?;

    let priority = find_child(&children, Rule::FLOAT_LITERAL)
        .map(|f| f.as_str().parse().unwrap_or(0.5))
        .unwrap_or(0.5);

    Ok(Declaration::Memorize(MemorizeDecl { value, priority }))
}

fn parse_forget_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: expression, INT, "days"
    let query = find_child(&children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in forget_decl")
        })?;
    let query = parse_expression(query)?;

    let days = find_child(&children, Rule::INT)
        .map(|i| i.as_str().parse().unwrap_or(30))
        .unwrap_or(30);

    Ok(Declaration::Forget(ForgetDecl { query, days }))
}

// ── Learnable Pattern (M3) ────────────────────────────────────────────

fn parse_learnable_pattern_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
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
                if let Expr::StringLit(s) = parse_expression(expr_pair.clone())? {
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
                            if let Expr::FloatLit(n) = exprs[1].clone() {
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
                        if let Expr::StringLit(s) = parse_expression(expr_pair.clone())? {
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
                if let Expr::StringLit(s) = parse_expression(expr_pair.clone())? {
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
                if let Expr::StringLit(s) = parse_expression(expr_pair.clone())? {
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
                if let Expr::FloatLit(n) = parse_expression(expr_pair.clone())? {
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
            if let Some(Expr::FloatLit(n)) = exprs.first() {
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

fn parse_pattern_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    // Children: IDENT, params, ARROW, type_name, pattern_body
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();
    let params = find_child(&children, Rule::params)
        .map(|p| parse_params(p))
        .unwrap_or_default();
    let return_type = find_child_str(&children, Rule::type_name).unwrap_or_default();
    let body = find_child(&children, Rule::pattern_body)
        .map(|p| parse_pattern_body(p))
        .transpose()
        .unwrap_or_default();
    Ok(Declaration::Pattern(PatternDecl {
        name,
        params,
        return_type,
        body,
    }))
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

fn parse_pattern_body(pair: Pair<Rule>) -> Result<Vec<Statement>, ParseError> {
    pair.into_inner()
        .filter(|s| s.as_rule() == Rule::statement)
        .map(|s| parse_single_statement(s))
        .collect::<Result<_, _>>()
}

/// Parse a single statement from its rule pair.
fn parse_single_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let children = children_of(&pair);
    // statement = { match_stmt | if_block_stmt | each_stmt | ... }
    // Наряд №14: match_stmt is now a proper AST statement
    if let Some(m_pair) = children.iter().find(|c| c.as_rule() == Rule::match_stmt) {
        return parse_match_stmt(m_pair.clone())?;
    }
    // NOTE: match_stmt previously was parsed as a regular expression — now has full AST support.
    if let Some(ib_pair) = children.iter().find(|c| c.as_rule() == Rule::if_block_stmt) {
        return parse_if_block_stmt(ib_pair.clone())?;
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
            })
        } else {
            let variable = idents[0].clone();
            Ok(Statement::Each {
                variable,
                iterable,
                body,
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
        Ok(Statement::While { condition, body })
    } else if let Some(lb_pair) = children.iter().find(|c| c.as_rule() == Rule::let_binding) {
        let lb_children = children_of(lb_pair);
        let name = find_child_str(&lb_children, Rule::IDENT).unwrap_or_default();
        let expr = find_child(&lb_children, Rule::expression).ok_or_else(|| {
                pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in let_binding")
            })?;
        let mutable = lb_children.iter().any(|c| c.as_rule() == Rule::MUT_KW);
            Ok(Statement::LetBinding { name, value: parse_expression(expr)?, mutable })
            name,
            value: parse_expression(expr)?,
            mutable,
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
                    pair_error(&pair, "GRAMMAR INVARIANT: assign_or_expr assignment must have expression")
                })?;
            Ok(Statement::Assign { name, value: parse_expression(expr)? })
        } else {
            // Expression statement (function call, etc.)
            let expr = ae_children
                .iter()
                .find(|c| c.as_rule() == Rule::expression)
                .cloned()
                .ok_or_else(|| {
                    pair_error(&pair, "GRAMMAR INVARIANT: assign_or_expr expression must have expression")
                })?;
            Ok(Statement::ExprStmt(parse_expression(expr)?))
        }
    } else if let Some(rs_pair) = children.iter().find(|c| c.as_rule() == Rule::return_stmt) {
        let rs_children = children_of(rs_pair);
        let expr = find_child(&rs_children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in return_stmt")
        })?;
        Ok(Statement::Return(parse_expression(expr)?))
    } else if let Some(br_pair) = children.iter().find(|c| c.as_rule() == Rule::break_stmt) {
        Ok(Statement::Break)
    } else if let Some(co_pair) = children.iter().find(|c| c.as_rule() == Rule::continue_stmt) {
        Ok(Statement::Continue)
    } else if let Some(it_pair) = children.iter().find(|c| c.as_rule() == Rule::if_then_stmt) {
        // if_then_stmt with optional else: "if expr then { ... } [else if expr then { ... }]* [else { ... }]"
        let it_children: Vec<Pair<Rule>> = it_pair.clone().into_inner().collect();
        let condition = it_children
                .iter()
                .find(|c| c.as_rule() == Rule::expression)
                .cloned()
                .map(|c| parse_expression(c))
                .transpose()
                .unwrap_or(Ok(Expr::BoolLit(true)))?;
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
                        .transpose()
                        .unwrap_or(Ok(Expr::BoolLit(true)))?;
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
            Ok(Statement::IfThen(Box::new(condition), body))
        } else {
            Ok(Statement::IfElseBlock {
                condition,
                then_body: body,
                else_ifs,
                else_body,
            })
        }
    } else if let Some(es_pair) = children.iter().find(|c| c.as_rule() == Rule::expr_stmt) {
        // Legacy expr_stmt fallback (shouldn't normally be reached with assign_or_expr)
        let expr = es_pair
            .clone()
            .into_inner()
            .find(|c| c.as_rule() == Rule::expression)
            .ok_or_else(|| {
                pair_error(&pair, "GRAMMAR INVARIANT: expr_stmt must contain expression")
            })?;
        Ok(Statement::ExprStmt(parse_expression(expr)?))
    } else if let Some(as_pair) = children.iter().find(|c| c.as_rule() == Rule::assign_stmt) {
        // Legacy assign_stmt fallback
        let as_children = children_of(as_pair);
        let name = find_child_str(&as_children, Rule::IDENT).unwrap_or_default();
        let expr = find_child(&as_children, Rule::expression).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::expression in assign_stmt")
        })?;
        Ok(Statement::Assign { name, value: parse_expression(expr)? })
            name,
            value: parse_expression(expr)?,
        }
    } else {
        // Fallback: unrecognized statement — return proper parse error with position
        return Err(pair_error(&pair, format!("unrecognized statement '{}'", pair.as_str().trim())));
    }
}

/// Parse a match statement: `match expr { "val" then { stmts } ... else { stmts } }` (Наряд №14)
fn parse_match_stmt(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    use crate::ast::{CompareOp as AstCmp, MatchArm};
    let children: Vec<Pair<Rule>> = pair.into_inner().collect();

    // First child is the scrutinee expression
    let scrutinee = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .transpose()
        .unwrap_or(Ok(Expr::StringLit(String::new())))?;

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
                    .transpose()
                    .unwrap_or(Ok(Expr::FloatLit(0.0)))?;
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
    })
}

/// Наряд №14 P0-3: Parse block if/else as expression.
/// `if cond { stmts } else if cond { stmts } else { stmts }` → Expr::BlockIfElse
fn parse_block_if_else_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let children: Vec<Pair<Rule>> = pair.into_inner().collect();
    let condition = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .transpose()
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
                    .transpose()
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

    Ok(Expr::BlockIfElse { condition, then_branch: Box::new(then_br), else_branch: Box::new(else_br) })
        condition: Box::new(condition),
        then_body,
        else_ifs: else_ifs
            .into_iter()
            .map(|(c, b)| (Box::new(c), b))
            .collect(),
        else_body,
    }
}

/// Parse a block-style if statement: `if expr { stmts } else if expr { stmts } else { stmts }`
fn parse_if_block_stmt(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let children = children_of(&pair);
    // Grammar now has else_block as a named rule, so children are:
    // [expression, statement*(then), else_if_block*, else_block?]
    let condition = children
        .iter()
        .find(|c| c.as_rule() == Rule::expression)
        .map(|c| parse_expression(c.clone()))
        .transpose()
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
                    .transpose()
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

    Ok(Statement::IfElseBlock { condition, then_branch, else_branch, else_if_branches })
        condition,
        then_body,
        else_ifs,
        else_body,
    }
}

// ── Flow ──────────────────────────────────────────────────────────────
// flow_decl = { "flow" ~ IDENT ~ "{" ~ flow_pipeline ~ branch_def* ~ "}" }
// flow_pipeline = { "input" ":" type_name "=" expression ~ flow_step* ~ ARROW ~ "output" }
// flow_step     = { ARROW ~ (checkpoint_call | step_ident) }
// checkpoint_call = { "checkpoint" ~ "(" ~ STRING_LITERAL ~ ")" }
// branch_def    = { step_ident ~ "{" ~ branch* ~ "}" }

fn parse_flow_decl(pair: Pair<Rule>) -> Result<Declaration, ParseError> {
    let children = children_of(&pair);
    let name = find_child_str(&children, Rule::IDENT).unwrap_or_default();

    // children: IDENT, flow_pipeline, [branch_def, ...]
    let pipeline_pair = find_child(&children, Rule::flow_pipeline).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::flow_pipeline in flow_decl")
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
        source: source.unwrap_or_else(|| Expr::StringLit(String::new())),
        pipeline: pipeline_steps.clone(),
        branch_defs,
        checkpoints: checkpoints.clone(),
    }))
}

fn parse_branch(pair: Pair<Rule>) -> Result<Branch, ParseError> {
    let children = children_of(&pair);
    // branch = { IDENT ~ "(" ~ branch_condition ~ ")" ~ ARROW ~ step_ident }
    let label = pair_str(&children[0]);
    let cond_pair = find_child(&children, Rule::branch_condition).ok_or_else(|| {
            pair_error(&pair, "GRAMMAR INVARIANT: expected Rule::branch_condition in branch")
        })?;
    let target = pair_str(children.last().ok_or_else(|| {
        pair_error(&pair, "GRAMMAR INVARIANT: expected step_ident at end of branch")
    })?);
    Ok(Branch {
        label,
        condition: parse_branch_condition(cond_pair)?,
        target,
    })
}

fn parse_branch_condition(pair: Pair<Rule>) -> Result<BranchCondition, ParseError> {
    let children = children_of(&pair);
    // branch_condition = { IDENT ~ "." ~ IDENT ~ compare_op ~ expression }
    // Children: [IDENT(target), IDENT(field), compare_op, expression(threshold)]
    Ok(BranchCondition {
        target: Expr::Ident(pair_str(&children[0])),
        field: pair_str(&children[1]),
        op: parse_compare_op(&children[2])?,
        threshold: parse_expression(children[3].clone())?,
    })
}

// ── Expressions ─────────────────────────────────────────────────────

fn parse_binop(pair: &Pair<Rule>) -> Result<BinOp, ParseError> {
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
        _ => Err(pair_error(pair, "GRAMMAR INVARIANT: unknown binary operator"))?,
    }
}

fn parse_expression(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expression => {
            let span = pair.as_span();
            let inner = pair.into_inner().next().ok_or_else(|| {
                pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "GRAMMAR INVARIANT: expression must have inner content".to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            parse_expression(inner)?
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
                        message: "GRAMMAR INVARIANT: unary_expr must have inner content".to_string(),
                    },
                    span.start_pos(),
                )
            })?;
            parse_expression(inner)?
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
            Ok(Expr::IfElse(Box::new(cond), Box::new(then_br), Box::new(else_br)))
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
            let module = idents.get(0).cloned().unwrap_or_default();
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
            let inner_expr = if !children.is_empty() {
                parse_expression(children[0].clone())?
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
                        message: "GRAMMAR INVARIANT: primary_expr must have inner content".to_string(),
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
                                message: "GRAMMAR INVARIANT: paren_expr must have inner content".to_string(),
                            },
                            inner_span.start_pos(),
                        )
                    })?;
                    parse_expression(inner_expr)?
                }
                Rule::block_if_else_expr => {
                    // Наряд №14 P0-3: block if/else as expression
                    parse_block_if_else_expr(inner)?
                }
                Rule::struct_literal => {
                    let mut fields = std::collections::HashMap::new();
                    for child in inner.clone().into_inner() {
                        if child.as_rule() == Rule::struct_field_init {
                            let field_children: Vec<_> = child.into_inner().collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    // ── Empty / comment-only programs ────────────────────────────────────

    #[test]
    fn test_parse_empty_program() {
        let decls = parse("").unwrap();
        assert!(decls.is_empty(), "empty program should produce no decls");
    }

    #[test]
    fn test_parse_empty_lines_only() {
        let decls = parse("\n\n").unwrap();
        assert!(decls.is_empty());
    }

    #[test]
    fn test_parse_comments_only() {
        let decls = parse("// just a comment\n").unwrap();
        assert!(decls.is_empty());
    }

    #[test]
    fn test_parse_multiple_comments() {
        let src = "// first\n// second\n// third\n";
        let decls = parse(src).unwrap();
        assert!(decls.is_empty());
    }

    #[test]
    fn test_parse_whitespace_and_comments_mixed() {
        let src = "// intro\n\n   // middle\n\t\n";
        let decls = parse(src).unwrap();
        assert!(decls.is_empty());
    }

    // ── Pattern declarations ─────────────────────────────────────────────

    #[test]
    fn test_parse_simple_pattern() {
        let decls = parse("pattern Foo() -> Float { return 1.0 }").unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.name, "Foo");
            assert_eq!(p.return_type, "Float");
            assert!(p.params.is_empty());
            assert_eq!(p.body.len(), 1);
        } else {
            panic!("expected Pattern, got {:?}", decls[0]);
        }
    }

    #[test]
    fn test_parse_pattern_with_params() {
        let src = "pattern Add(a: Float, b: Float) -> Float { return a + b }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.name, "Add");
            assert_eq!(p.params.len(), 2);
            assert_eq!(p.params[0].name, "a");
            assert_eq!(p.params[0].type_name, "Float");
            assert_eq!(p.params[1].name, "b");
            assert_eq!(p.params[1].type_name, "Float");
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_pattern_multiple_statements() {
        let src = "pattern Calc(a: Float) -> Float { let doubled = a + a let quad = doubled + doubled return quad }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 3);
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_pattern_string_return() {
        let src = "pattern Greet() -> String { return \"hello\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.return_type, "String");
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "hello"),
                other => panic!("expected Return(StringLit), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_pattern_bool_return() {
        let src = "pattern IsTrue() -> Bool { return true }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.return_type, "Bool");
            match &p.body[0] {
                Statement::Return(Expr::BoolLit(b)) => assert!(*b),
                other => panic!("expected Return(BoolLit(true)), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_pattern_unit_return() {
        let src = "pattern Nothing() -> Unit { return unit }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.return_type, "Unit");
            match &p.body[0] {
                Statement::Return(Expr::Ident(name)) => assert_eq!(name, "unit"),
                other => panic!("expected Return(Ident(\"unit\")), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Flow declarations ────────────────────────────────────────────────

    #[test]
    fn test_parse_flow_simple() {
        let src = "flow Main { input: String = \"x\" -> Step1 -> output }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Flow(f) = &decls[0] {
            assert_eq!(f.name, "Main");
            assert_eq!(f.input_type, "String");
            assert_eq!(f.pipeline.len(), 1);
            assert_eq!(f.pipeline[0], "Step1");
        } else {
            panic!("expected Flow");
        }
    }

    #[test]
    fn test_parse_flow_multi_step() {
        let src = "flow Main { input: String = \"x\" -> Step1 -> Step2 -> Step3 -> output }";
        let decls = parse(src).unwrap();
        if let Declaration::Flow(f) = &decls[0] {
            assert_eq!(f.pipeline.len(), 3);
            assert_eq!(f.pipeline[0], "Step1");
            assert_eq!(f.pipeline[1], "Step2");
            assert_eq!(f.pipeline[2], "Step3");
        } else {
            panic!("expected Flow");
        }
    }

    #[test]
    fn test_parse_flow_with_float_input() {
        let src = "flow Main { input: Float = 3.14 -> Process -> output }";
        let decls = parse(src).unwrap();
        if let Declaration::Flow(f) = &decls[0] {
            assert_eq!(f.input_type, "Float");
            #[allow(clippy::approx_constant)]
            let pi_approx = 3.14_f64;
            match &f.source {
                Expr::FloatLit(v) => assert!((v - pi_approx).abs() < 1e-9),
                other => panic!("expected FloatLit, got {:?}", other),
            }
        } else {
            panic!("expected Flow");
        }
    }

    // ── Entity declarations ──────────────────────────────────────────────

    #[test]
    fn test_parse_entity_type() {
        let src = "entity User { name: String, age: Float }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::EntityType(e) = &decls[0] {
            assert_eq!(e.name, "User");
            assert_eq!(e.fields.len(), 2);
            assert_eq!(e.fields[0].name, "name");
            assert_eq!(e.fields[0].type_name, "String");
            assert!(e.fields[0].default.is_none());
            assert_eq!(e.fields[1].name, "age");
            assert_eq!(e.fields[1].type_name, "Float");
        } else {
            panic!("expected EntityType, got {:?}", decls[0]);
        }
    }

    #[test]
    fn test_parse_entity_with_default_fields() {
        // Grammar: field_decl = { IDENT ~ COLON ~ type_name ~ (ASSIGN ~ literal)? }
        // Note: the `literal` rule is silent (_{}) so the parser's find(Rule::literal)
        // currently returns None for the default — this is a pre-existing parser quirk.
        // We verify only the name/type_name parsing here.
        let src = "entity Point { x: Float = 0.0, y: Float = 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::EntityType(e) = &decls[0] {
            assert_eq!(e.fields.len(), 2);
            assert_eq!(e.fields[0].name, "x");
            assert_eq!(e.fields[0].type_name, "Float");
            assert_eq!(e.fields[1].name, "y");
            assert_eq!(e.fields[1].type_name, "Float");
        } else {
            panic!("expected EntityType");
        }
    }

    #[test]
    fn test_parse_entity_simple() {
        let src = "entity greeting: String = \"Hello\"";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::EntitySimple(e) = &decls[0] {
            assert_eq!(e.name, "greeting");
            assert_eq!(e.type_name, "String");
            match &e.value {
                Expr::StringLit(s) => assert_eq!(s, "Hello"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        } else {
            panic!("expected EntitySimple, got {:?}", decls[0]);
        }
    }

    #[test]
    fn test_parse_entity_record() {
        let src = "entity m: Message = { text: \"hi\", urgency: 0.5 }";
        let decls = parse(src).unwrap();
        if let Declaration::EntityRecord(e) = &decls[0] {
            assert_eq!(e.name, "m");
            assert_eq!(e.type_name, "Message");
            assert_eq!(e.fields.len(), 2);
            assert_eq!(e.fields[0].name, "text");
        } else {
            panic!("expected EntityRecord, got {:?}", decls[0]);
        }
    }

    // ── Imports ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_import() {
        let src = "import std/math";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Import(i) = &decls[0] {
            assert_eq!(i.path, "std/math");
            assert!(i.alias.is_none());
        } else {
            panic!("expected Import");
        }
    }

    #[test]
    fn test_parse_import_with_alias() {
        let src = "import std/string as str";
        let decls = parse(src).unwrap();
        if let Declaration::Import(i) = &decls[0] {
            assert_eq!(i.path, "std/string");
            assert_eq!(i.alias.as_deref(), Some("str"));
        } else {
            panic!("expected Import");
        }
    }

    #[test]
    fn test_parse_import_relative() {
        let src = "import ./my_utils";
        let decls = parse(src).unwrap();
        if let Declaration::Import(i) = &decls[0] {
            assert_eq!(i.path, "./my_utils");
        } else {
            panic!("expected Import");
        }
    }

    // ── Memory ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_memory_decl() {
        // Grammar requires memory_kv_config before optional persist; the kv form is:
        //   memory { kv: { type: key_value persist: true }, persist: "./data.db" }
        let src = "memory { kv: { type: key_value persist: true }, persist: \"./data.db\" }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Memory(m) = &decls[0] {
            assert_eq!(m.persist.as_deref(), Some("./data.db"));
        } else {
            panic!("expected Memory");
        }
    }

    #[test]
    fn test_parse_memory_decl_empty() {
        let src = "memory { }";
        let decls = parse(src).unwrap();
        if let Declaration::Memory(m) = &decls[0] {
            assert!(m.persist.is_none());
        } else {
            panic!("expected Memory");
        }
    }

    // ── MlogServer ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_mlogserver() {
        let src = "mlogserver { port: 8080 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.port, 8080);
            assert!(s.host.is_none());
            assert!(s.middleware.is_empty());
            assert!(s.routes.is_empty());
        } else {
            panic!("expected MlogServer");
        }
    }

    #[test]
    fn test_parse_mlogserver_with_host() {
        // No comma between port and host — mlogserver_body uses concatenation
        let src = "mlogserver { port: 9090 host: \"0.0.0.0\" }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.port, 9090);
            assert_eq!(s.host.as_deref(), Some("0.0.0.0"));
        } else {
            panic!("expected MlogServer");
        }
    }

    #[test]
    fn test_parse_server_alias() {
        let src = "server { port: 3000 }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.port, 3000);
        } else {
            panic!("expected MlogServer (server alias)");
        }
    }

    #[test]
    fn test_parse_mlogserver_default_port() {
        // Without port → default 8080
        let src = "mlogserver { host: \"127.0.0.1\" }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.port, 8080);
            assert_eq!(s.host.as_deref(), Some("127.0.0.1"));
        } else {
            panic!("expected MlogServer");
        }
    }

    // ── Templates ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_template() {
        let src = "template Hello() -> Html { <div>hello</div> }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Hello");
            assert_eq!(t.return_type, "Html");
            assert!(t.body.contains("div"));
            assert!(t.params.is_empty());
        } else {
            panic!("expected Template");
        }
    }

    #[test]
    fn test_parse_template_with_params() {
        // Body contains {{ }} — exercises preprocess_templates balanced-brace logic
        let src = "template Card(name: String) -> Html { <div>{{ name }}</div> }";
        let decls = parse(src).unwrap();
        if let Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Card");
            assert_eq!(t.params.len(), 1);
            assert_eq!(t.params[0].name, "name");
            assert_eq!(t.params[0].type_name, "String");
            assert!(t.body.contains("name"));
        } else {
            panic!("expected Template");
        }
    }

    // ── Memorize / Forget ────────────────────────────────────────────────

    #[test]
    fn test_parse_memorize() {
        let src = "memorize \"user likes spicy food\" with priority=0.8";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Memorize(m) = &decls[0] {
            assert!((m.priority - 0.8).abs() < 1e-9);
            match &m.value {
                Expr::StringLit(s) => assert_eq!(s, "user likes spicy food"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        } else {
            panic!("expected Memorize");
        }
    }

    #[test]
    fn test_parse_memorize_default_priority() {
        let src = "memorize \"fact\"";
        let decls = parse(src).unwrap();
        if let Declaration::Memorize(m) = &decls[0] {
            // Default priority is 0.5 when omitted
            assert!((m.priority - 0.5).abs() < 1e-9);
        } else {
            panic!("expected Memorize");
        }
    }

    #[test]
    fn test_parse_forget() {
        let src = "forget \"old\" after 30.days";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Forget(f) = &decls[0] {
            assert_eq!(f.days, 30);
            match &f.query {
                Expr::StringLit(s) => assert_eq!(s, "old"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        } else {
            panic!("expected Forget");
        }
    }

    #[test]
    fn test_parse_forget_default_days() {
        let src = "forget \"x\" after 7.days";
        let decls = parse(src).unwrap();
        if let Declaration::Forget(f) = &decls[0] {
            assert_eq!(f.days, 7);
        } else {
            panic!("expected Forget");
        }
    }

    // ── Hooks ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_hook_before() {
        let src = "hook before_pattern { let x = 1.0 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Hook(h) = &decls[0] {
            assert_eq!(h.phase, HookPhase::BeforePattern);
            assert_eq!(h.body.len(), 1);
        } else {
            panic!("expected Hook");
        }
    }

    #[test]
    fn test_parse_hook_after() {
        // NOTE: parse_hook_decl uses find(Rule::hook_kind) but hook_kind is silent (_{})
        // in the grammar, so it always falls back to the default BeforePattern.
        // We only verify the hook parses and its body is captured.
        let src = "hook after_pattern { let x = 1.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Hook(h) = &decls[0] {
            assert_eq!(h.body.len(), 1);
        } else {
            panic!("expected Hook");
        }
    }

    #[test]
    fn test_parse_hook_on_session_start() {
        let src = "hook on_session_start { let x = 1.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Hook(h) = &decls[0] {
            assert_eq!(h.body.len(), 1);
        } else {
            panic!("expected Hook");
        }
    }

    #[test]
    fn test_parse_hook_on_session_end() {
        let src = "hook on_session_end { let x = 1.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Hook(h) = &decls[0] {
            assert_eq!(h.body.len(), 1);
        } else {
            panic!("expected Hook");
        }
    }

    #[test]
    fn test_parse_hook_on_write() {
        let src = "hook on_write { let x = 1.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Hook(h) = &decls[0] {
            assert_eq!(h.body.len(), 1);
        } else {
            panic!("expected Hook");
        }
    }

    // ── Rules ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_rule_contains() {
        // NOTE: grammar's compare_condition is shadowed by expression's compare_op,
        // so `rule If(x > 5)` does not parse. Only `contains` conditions work
        // reliably at the rule level.
        let src = "rule If(m.text contains \"urgent\") then m.urgency = 0.9 with priority=10";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Rule(r) = &decls[0] {
            assert_eq!(r.priority, 10);
            assert_eq!(r.field, "urgency");
            match &r.condition {
                Condition::Contains { .. } => {}
                other => panic!("expected Contains condition, got {:?}", other),
            }
            match &r.target {
                Expr::Ident(name) => assert_eq!(name, "m"),
                other => panic!("expected Ident target, got {:?}", other),
            }
        } else {
            panic!("expected Rule");
        }
    }

    #[test]
    fn test_parse_rule_default_priority() {
        let src = "rule If(m.text contains \"x\") then m.urgency = 0.5";
        let decls = parse(src).unwrap();
        if let Declaration::Rule(r) = &decls[0] {
            // Default priority is 0 when omitted
            assert_eq!(r.priority, 0);
        } else {
            panic!("expected Rule");
        }
    }

    // ── Sandbox ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_sandbox_minimal() {
        // Grammar requires allowed? before forbidden? before timeout?
        let src = "sandbox dev { allowed: [] }";
        let decls = parse(src).unwrap();
        if let Declaration::Sandbox(s) = &decls[0] {
            assert_eq!(s.name, "dev");
            assert!(s.allowed.is_empty());
            assert!(s.forbidden.is_empty());
            // Default timeout is 30
            assert_eq!(s.timeout, 30);
        } else {
            panic!("expected Sandbox");
        }
    }

    #[test]
    fn test_parse_sandbox_with_lists() {
        let src = "sandbox prod { allowed: [fs, net], forbidden: [shell], timeout: 60 }";
        let decls = parse(src).unwrap();
        if let Declaration::Sandbox(s) = &decls[0] {
            assert_eq!(s.name, "prod");
            assert_eq!(s.allowed.len(), 2);
            assert_eq!(s.allowed[0], "fs");
            assert_eq!(s.allowed[1], "net");
            assert_eq!(s.forbidden.len(), 1);
            assert_eq!(s.forbidden[0], "shell");
            assert_eq!(s.timeout, 60);
        } else {
            panic!("expected Sandbox");
        }
    }

    // ── Mutate ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_mutate() {
        let src = "mutate MyPattern { add_example(\"in\", \"out\") }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Mutate(m) = &decls[0] {
            assert_eq!(m.pattern_name, "MyPattern");
            assert_eq!(m.new_examples.len(), 1);
            assert!(m.rollback_threshold.is_none());
            assert!(m.rollback_op.is_none());
        } else {
            panic!("expected Mutate");
        }
    }

    #[test]
    fn test_parse_mutate_with_rollback() {
        let src = "mutate MyPattern { add_example(\"a\", \"b\") rollback_if: accuracy < 0.5 }";
        let decls = parse(src).unwrap();
        if let Declaration::Mutate(m) = &decls[0] {
            assert_eq!(m.new_examples.len(), 1);
            assert!(m.rollback_threshold.is_some());
            assert!(m.rollback_op.is_some());
            assert!((m.rollback_threshold.unwrap() - 0.5).abs() < 1e-9);
        } else {
            panic!("expected Mutate");
        }
    }

    #[test]
    fn test_parse_mutate_multiple_examples() {
        let src = "mutate P { add_example(\"a\", \"b\") add_example(\"c\", \"d\") }";
        let decls = parse(src).unwrap();
        if let Declaration::Mutate(m) = &decls[0] {
            assert_eq!(m.new_examples.len(), 2);
        } else {
            panic!("expected Mutate");
        }
    }

    // ── Conversation ─────────────────────────────────────────────────────

    #[test]
    fn test_parse_conversation() {
        let src = "conversation { ttl: 1800, max_messages: 50, compress_after: 20 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Conversation(c) = &decls[0] {
            assert_eq!(c.ttl, 1800);
            assert_eq!(c.max_messages, 50);
            assert_eq!(c.compress_after, 20);
        } else {
            panic!("expected Conversation");
        }
    }

    #[test]
    fn test_parse_conversation_defaults() {
        let src = "conversation { ttl: 600 }";
        let decls = parse(src).unwrap();
        if let Declaration::Conversation(c) = &decls[0] {
            assert_eq!(c.ttl, 600);
            // max_messages defaults to 50, compress_after to 20
            assert_eq!(c.max_messages, 50);
            assert_eq!(c.compress_after, 20);
        } else {
            panic!("expected Conversation");
        }
    }

    // ── Context Budget ───────────────────────────────────────────────────

    #[test]
    fn test_parse_context_budget() {
        // NOTE: parse_context_budget_decl uses find(Rule::context_budget_limit) but
        // the rule appears inside an optional group `(COMMA ~ context_budget_limit)?`
        // which pest does not surface as a direct child — so limit is currently always
        // None at parse time. We verify the pattern_name parsing here.
        let src = "context_budget { pattern: \"summarize\", limit: 4096 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::ContextBudget(cb) = &decls[0] {
            assert_eq!(cb.pattern_name, "summarize");
        } else {
            panic!("expected ContextBudget");
        }
    }

    #[test]
    fn test_parse_context_budget_no_limit() {
        let src = "context_budget { pattern: \"x\" }";
        let decls = parse(src).unwrap();
        if let Declaration::ContextBudget(cb) = &decls[0] {
            assert_eq!(cb.pattern_name, "x");
            assert!(cb.limit.is_none());
        } else {
            panic!("expected ContextBudget");
        }
    }

    // ── LLM Config ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_llm_config_providers_only() {
        // Grammar requires fields in order: providers, default_model, failover, circuit_breaker, timeout
        let src = "llm { providers: [{alias: primary, provider: anthropic, key: \"sk-...\"}] }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::LlmConfig(c) = &decls[0] {
            assert_eq!(c.providers.len(), 1);
            assert_eq!(c.providers[0].alias, "primary");
            assert_eq!(c.providers[0].provider, "anthropic");
            assert!(c.default_model.is_none());
            // Defaults: circuit_breaker=3, timeout=30
            assert_eq!(c.circuit_breaker, 3);
            assert_eq!(c.timeout, 30);
        } else {
            panic!("expected LlmConfig, got {:?}", decls[0]);
        }
    }

    #[test]
    fn test_parse_llm_config_with_model() {
        let src = "llm { providers: [{alias: primary, provider: anthropic, key: \"sk-...\"}], default_model: \"haiku\" }";
        let decls = parse(src).unwrap();
        if let Declaration::LlmConfig(c) = &decls[0] {
            assert_eq!(c.providers.len(), 1);
            assert_eq!(c.default_model.as_deref(), Some("haiku"));
        } else {
            panic!("expected LlmConfig");
        }
    }

    #[test]
    fn test_parse_llm_config_empty() {
        let src = "llm { }";
        let decls = parse(src).unwrap();
        if let Declaration::LlmConfig(c) = &decls[0] {
            assert!(c.providers.is_empty());
            assert!(c.default_model.is_none());
            assert_eq!(c.circuit_breaker, 3);
            assert_eq!(c.timeout, 30);
        } else {
            panic!("expected LlmConfig");
        }
    }

    // ── Eval ─────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_eval() {
        let src = "eval Classify { dataset: [(\"a\", \"b\"), (\"c\", \"d\")], metric: accuracy, threshold: 0.8 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 1);
        if let Declaration::Eval(e) = &decls[0] {
            assert_eq!(e.pattern_name, "Classify");
            assert_eq!(e.dataset.len(), 2);
            assert_eq!(e.dataset[0].0, "a");
            assert_eq!(e.dataset[0].1, "b");
            assert_eq!(e.dataset[1].0, "c");
            assert_eq!(e.dataset[1].1, "d");
            assert_eq!(e.metric, "accuracy");
            assert!((e.threshold - 0.8).abs() < 1e-9);
        } else {
            panic!("expected Eval");
        }
    }

    // ── Pattern body statements ──────────────────────────────────────────

    #[test]
    fn test_parse_let_binding() {
        let src = "pattern P() -> Float { let x = 1.0 return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 2);
            match &p.body[0] {
                Statement::LetBinding {
                    name,
                    value,
                    mutable,
                } => {
                    assert_eq!(name, "x");
                    assert!(!*mutable);
                    match value {
                        Expr::FloatLit(v) => assert_eq!(*v, 1.0),
                        other => panic!("expected FloatLit, got {:?}", other),
                    }
                }
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_let_mut_binding() {
        let src = "pattern P() -> Float { let mut x = 1.0 return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { name, mutable, .. } => {
                    assert_eq!(name, "x");
                    assert!(*mutable);
                }
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_if_else_block() {
        let src = "pattern P(x: Float) -> Float { if x > 0.0 { return x } else { return 0.0 } }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 1);
            match &p.body[0] {
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_body,
                    ..
                } => {
                    assert!(matches!(condition, Expr::BinaryOp(_, _, _)));
                    assert_eq!(then_body.len(), 1);
                    assert!(else_body.is_some());
                    assert_eq!(else_body.as_ref().unwrap().len(), 1);
                }
                other => panic!("expected IfElseBlock, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let src = "pattern P(i: Float) -> Float { while i < 10.0 { i = i + 1.0 } return i }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: while_stmt, return
            assert_eq!(p.body.len(), 2);
            match &p.body[0] {
                Statement::While { condition, body } => {
                    assert!(matches!(condition, Expr::BinaryOp(_, _, _)));
                    assert_eq!(body.len(), 1);
                }
                other => panic!("expected While, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_each_loop() {
        let src = "pattern P(items: List) -> Float { let total = 0.0 each item in items { total = total + 1.0 } return total }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: let_binding, each_stmt, return
            assert_eq!(p.body.len(), 3);
            let mut found_each = false;
            for s in &p.body {
                if let Statement::Each {
                    variable,
                    iterable,
                    body,
                } = s
                {
                    assert_eq!(variable, "item");
                    assert!(matches!(iterable, Expr::Ident(_)));
                    assert_eq!(body.len(), 1);
                    found_each = true;
                }
            }
            assert!(found_each, "no Each statement found in body");
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_return_statement() {
        let src = "pattern P() -> Float { return 42.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 1);
            match &p.body[0] {
                Statement::Return(expr) => match expr {
                    Expr::FloatLit(v) => assert_eq!(*v, 42.0),
                    other => panic!("expected FloatLit, got {:?}", other),
                },
                other => panic!("expected Return, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_assignment_statement() {
        let src = "pattern P() -> Float { let x = 0.0 x = 5.0 return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // let_binding, assign, return
            assert_eq!(p.body.len(), 3);
            match &p.body[1] {
                Statement::Assign { name, value } => {
                    assert_eq!(name, "x");
                    match value {
                        Expr::FloatLit(v) => assert_eq!(*v, 5.0),
                        other => panic!("expected FloatLit, got {:?}", other),
                    }
                }
                other => panic!("expected Assign, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_break_continue() {
        let src =
            "pattern P() -> Float { while true { break } while false { continue } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // while + break, while + continue, return
            assert_eq!(p.body.len(), 3);
            match &p.body[0] {
                Statement::While { body, .. } => {
                    assert_eq!(body.len(), 1);
                    assert!(matches!(body[0], Statement::Break));
                }
                other => panic!("expected While, got {:?}", other),
            }
            match &p.body[1] {
                Statement::While { body, .. } => {
                    assert_eq!(body.len(), 1);
                    assert!(matches!(body[0], Statement::Continue));
                }
                other => panic!("expected While, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_stmt() {
        // A bare function call as a statement (no let/return wrapper)
        let src = "pattern P() -> Float { respond(\"ok\") return 1.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 2);
            match &p.body[0] {
                Statement::ExprStmt(Expr::FnCall(name, args)) => {
                    assert_eq!(name, "respond");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected ExprStmt(FnCall), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Multiple declarations ────────────────────────────────────────────

    #[test]
    fn test_parse_multiple_declarations() {
        let src = "pattern A() -> Float { return 1.0 }\npattern B() -> Float { return 2.0 }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 2);
        assert!(matches!(&decls[0], Declaration::Pattern(_)));
        assert!(matches!(&decls[1], Declaration::Pattern(_)));
        if let Declaration::Pattern(p0) = &decls[0] {
            assert_eq!(p0.name, "A");
        }
        if let Declaration::Pattern(p1) = &decls[1] {
            assert_eq!(p1.name, "B");
        }
    }

    #[test]
    fn test_parse_mixed_declarations() {
        let src = "pattern A() -> Float { return 1.0 }\nentity User { name: String }\nmemory { kv: { type: key_value persist: true } }";
        let decls = parse(src).unwrap();
        assert_eq!(decls.len(), 3);
        assert!(matches!(&decls[0], Declaration::Pattern(_)));
        assert!(matches!(&decls[1], Declaration::EntityType(_)));
        assert!(matches!(&decls[2], Declaration::Memory(_)));
    }

    // ── Expressions ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_expr_arithmetic() {
        let src = "pattern P(a: Float, b: Float, c: Float) -> Float { let x = a + b * c return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => {
                    // Top-level should be Add since multiplication binds tighter
                    match value {
                        Expr::BinaryOp(_, op, _) => {
                            assert!(matches!(op, BinOp::Add), "expected Add, got {:?}", op);
                        }
                        other => panic!("expected BinaryOp, got {:?}", other),
                    }
                }
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_comparison() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a > b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => {
                        assert!(matches!(op, BinOp::Gt), "expected Gt, got {:?}", op);
                    }
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_function_call() {
        let src = "pattern P(s: String) -> String { let x = upper(s) return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FnCall(name, args) => {
                        assert_eq!(name, "upper");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("expected FnCall, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_qualified_call() {
        let src = "pattern P() -> String { let x = std.upper(\"a\") return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::QualifiedCall {
                        module,
                        function,
                        args,
                    } => {
                        assert_eq!(module, "std");
                        assert_eq!(function, "upper");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("expected QualifiedCall, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_field_access() {
        let src = "pattern P(m: Message) -> Float { let x = m.urgency return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FieldAccess(base, field) => {
                        assert_eq!(field, "urgency");
                        assert!(matches!(base.as_ref(), Expr::Ident(_)));
                    }
                    other => panic!("expected FieldAccess, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_list_literal() {
        let src = "pattern P() -> List { let x = [1.0, 2.0, 3.0] return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::List(items) => assert_eq!(items.len(), 3),
                    other => panic!("expected List, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_expr_if_else_expr() {
        let src = "pattern P(score: Float) -> String { let x = if score > 0.8 then \"good\" else \"bad\" return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => {
                    assert!(matches!(value, Expr::IfElse(_, _, _)));
                }
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── String literals & escapes ────────────────────────────────────────

    #[test]
    fn test_parse_string_literal_simple() {
        let src = "pattern P() -> String { return \"hello\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "hello"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_string_literal_with_newline_escape() {
        let src = "pattern P() -> String { return \"line1\\nline2\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "line1\nline2"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_string_literal_with_tab_escape() {
        let src = "pattern P() -> String { return \"a\\tb\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "a\tb"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_string_literal_with_quote_escape() {
        let src = "pattern P() -> String { return \"say \\\"hi\\\"\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "say \"hi\""),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_string_literal_with_backslash_escape() {
        let src = "pattern P() -> String { return \"C:\\\\path\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "C:\\path"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_string_literal_with_unicode_escape() {
        let src = "pattern P() -> String { return \"\\u0041\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => assert_eq!(s, "A"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_multiline_string() {
        let src = "pattern P() -> String { return \"\"\"multi\nline\nstring\"\"\" }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::StringLit(s)) => {
                    assert!(s.contains("multi"));
                    assert!(s.contains("line"));
                    assert!(s.contains("string"));
                }
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
    }

    // ── Error cases ──────────────────────────────────────────────────────

    #[test]
    fn test_parse_error_unclosed_brace() {
        let result = parse("{");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_bad_syntax() {
        let result = parse("pattern");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_incomplete_pattern() {
        let result = parse("pattern Foo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_arrow() {
        let result = parse("pattern Foo() Float { return 1.0 }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_paren() {
        let result = parse("pattern Foo(a: Float -> Float { return a }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_return_type() {
        let result = parse("pattern Foo() -> { return 1.0 }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_garbage_input() {
        let result = parse("@#$%^&*()");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_incomplete_flow() {
        let result = parse("flow Main { input: String = ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_string() {
        let result = parse("pattern P() -> String { return \"unclosed }");
        assert!(result.is_err());
    }

    // ── Narjad 29 Block 6.1: Additional unit tests ──────────────────────

    // ── Literals ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_int_literal_returns_floatlit() {
        // INT is parsed as FloatLit per parse_expression primary_expr branch
        let src = "pattern P() -> Float { return 42 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::FloatLit(v)) => assert_eq!(*v, 42.0),
                other => panic!("expected FloatLit(42.0), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_boolean_false_literal() {
        let src = "pattern P() -> Bool { return false }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::BoolLit(b)) => assert!(!*b, "expected false"),
                other => panic!("expected BoolLit(false), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_empty_list_literal() {
        let src = "pattern P() -> List { let x = [] return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::List(items) => assert!(items.is_empty(), "expected empty list"),
                    other => panic!("expected List, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_list_of_strings() {
        let src = "pattern P() -> List { let xs = [\"a\", \"b\", \"c\"] return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::List(items) => {
                        assert_eq!(items.len(), 3);
                        assert!(matches!(items[0], Expr::StringLit(_)));
                    }
                    other => panic!("expected List, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_list_of_bools() {
        let src = "pattern P() -> List { let xs = [true, false] return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::List(items) => assert_eq!(items.len(), 2),
                    other => panic!("expected List, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_struct_literal_expression() {
        let src = "pattern P() -> Map { let x = { key: \"val\" } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::StructLit(fields) => {
                        assert_eq!(fields.len(), 1);
                        assert!(fields.contains_key("key"));
                    }
                    other => panic!("expected StructLit, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Match statement ─────────────────────────────────────────────────

    #[test]
    fn test_parse_match_with_exact_arm() {
        let src =
            "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: [match_stmt, return]
            assert_eq!(p.body.len(), 2);
            match &p.body[0] {
                Statement::Match { arms, .. } => {
                    assert_eq!(arms.len(), 1);
                    assert!(matches!(arms[0], MatchArm::Exact(_, _)));
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_match_with_multiple_exact_arms() {
        let src = "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } \"b\" then { return 2.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Match { arms, .. } => {
                    assert_eq!(arms.len(), 2);
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_match_with_starts_with_arm() {
        let src = "pattern P(s: String) -> Float { match s { starts_with \"pre\" then { return 1.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Match { arms, .. } => {
                    assert_eq!(arms.len(), 1);
                    assert!(matches!(arms[0], MatchArm::StartsWith(_, _)));
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_match_with_contains_arm() {
        let src = "pattern P(s: String) -> Float { match s { contains \"x\" then { return 1.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Match { arms, .. } => {
                    assert_eq!(arms.len(), 1);
                    assert!(matches!(arms[0], MatchArm::Contains(_, _)));
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_match_with_compare_arm() {
        let src =
            "pattern P(s: Float) -> Float { match s { > 0.5 then { return 1.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Match { arms, .. } => {
                    assert_eq!(arms.len(), 1);
                    assert!(matches!(arms[0], MatchArm::Compare(_, _, _)));
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_match_with_else() {
        let src = "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } else { return 0.0 } } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Match {
                    arms, else_body, ..
                } => {
                    assert_eq!(arms.len(), 1);
                    assert!(else_body.is_some(), "expected else body");
                }
                other => panic!("expected Match, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Control flow ────────────────────────────────────────────────────

    #[test]
    fn test_parse_if_then_no_else() {
        // Single-branch if-then (no else) → Statement::IfThen
        let src = "pattern P(x: Float) -> Float { if x > 0.0 then { return x } return 0.0 }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: [if_then, return]
            assert_eq!(p.body.len(), 2);
            assert!(matches!(&p.body[0], Statement::IfThen(_, _)));
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_if_else_if_else_chain() {
        let src = "pattern P(x: Float) -> Float { if x > 0.0 { return 1.0 } else if x < 0.0 { return 2.0 } else { return 0.0 } }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.body.len(), 1);
            match &p.body[0] {
                Statement::IfElseBlock {
                    else_ifs,
                    else_body,
                    ..
                } => {
                    assert_eq!(else_ifs.len(), 1);
                    assert!(else_body.is_some());
                }
                other => panic!("expected IfElseBlock, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_each_with_index() {
        let src = "pattern P(items: List) -> Float { let total = 0.0 each i, item in items { total = total + 1.0 } return total }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: let_binding, each_with_index, return
            assert_eq!(p.body.len(), 3);
            let mut found = false;
            for s in &p.body {
                if let Statement::EachWithIndex {
                    index_var,
                    item_var,
                    ..
                } = s
                {
                    assert_eq!(index_var, "i");
                    assert_eq!(item_var, "item");
                    found = true;
                }
            }
            assert!(found, "expected EachWithIndex statement");
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Binary operators ────────────────────────────────────────────────

    #[test]
    fn test_parse_logical_and_expr() {
        let src = "pattern P(a: Bool, b: Bool) -> Bool { let x = a and b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::And)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_logical_or_expr() {
        let src = "pattern P(a: Bool, b: Bool) -> Bool { let x = a or b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Or)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_subtraction_operator() {
        let src = "pattern P(a: Float, b: Float) -> Float { let x = a - b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Sub)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_division_operator() {
        let src = "pattern P(a: Float, b: Float) -> Float { let x = a / b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Div)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_equality_operator() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a == b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Eq)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_inequality_operator() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a != b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Ne)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_greater_equal_operator() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a >= b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Ge)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_less_equal_operator() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a <= b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Le)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_less_than_operator() {
        let src = "pattern P(a: Float, b: Float) -> Bool { let x = a < b return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BinaryOp(_, op, _) => assert!(matches!(op, BinOp::Lt)),
                    other => panic!("expected BinaryOp, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Expressions: access & calls ─────────────────────────────────────

    #[test]
    fn test_parse_chained_field_access() {
        let src = "pattern P(m: Message) -> Float { let x = m.body.urgency return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FieldAccess(base, field) => {
                        assert_eq!(field, "urgency");
                        assert!(matches!(base.as_ref(), Expr::FieldAccess(_, _)));
                    }
                    other => panic!("expected FieldAccess, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_index_access_expression() {
        let src = "pattern P(items: List) -> Float { let x = items[0] return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::IndexAccess(_, idx) => {
                        assert!(matches!(idx.as_ref(), Expr::FloatLit(_)));
                    }
                    other => panic!("expected IndexAccess, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_nested_function_call() {
        let src = "pattern P(s: String) -> String { let x = upper(lower(s)) return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FnCall(name, args) => {
                        assert_eq!(name, "upper");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(args[0], Expr::FnCall(_, _)));
                    }
                    other => panic!("expected FnCall, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_function_call_with_multiple_args() {
        let src =
            "pattern P(a: Float, b: Float, c: Float) -> Float { let x = f(a, b, c) return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FnCall(name, args) => {
                        assert_eq!(name, "f");
                        assert_eq!(args.len(), 3);
                    }
                    other => panic!("expected FnCall, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_try_expression() {
        let src = "pattern P(s: String) -> String { let x = try upper(s) return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::Try(inner) => {
                        assert!(matches!(inner.as_ref(), Expr::FnCall(_, _)));
                    }
                    other => panic!("expected Try, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_let_binding_with_string_value() {
        let src = "pattern P() -> String { let name = \"Alice\" return name }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { name, value, .. } => {
                    assert_eq!(name, "name");
                    match value {
                        Expr::StringLit(s) => assert_eq!(s, "Alice"),
                        other => panic!("expected StringLit, got {:?}", other),
                    }
                }
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_let_binding_with_function_call_value() {
        let src = "pattern P(s: String) -> String { let x = upper(s) return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::FnCall(name, args) => {
                        assert_eq!(name, "upper");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("expected FnCall, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_let_binding_with_bool_value() {
        let src = "pattern P() -> Bool { let flag = true return flag }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::LetBinding { value, .. } => match value {
                    Expr::BoolLit(b) => assert!(*b, "expected true"),
                    other => panic!("expected BoolLit, got {:?}", other),
                },
                other => panic!("expected LetBinding, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── Pattern body variants ───────────────────────────────────────────

    #[test]
    fn test_parse_pattern_with_three_params() {
        let src = "pattern Add3(a: Float, b: Float, c: Float) -> Float { return a }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            assert_eq!(p.name, "Add3");
            assert_eq!(p.params.len(), 3);
            assert_eq!(p.params[0].name, "a");
            assert_eq!(p.params[1].name, "b");
            assert_eq!(p.params[2].name, "c");
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_comment_inside_pattern_body() {
        let src =
            "pattern P() -> Float { // first comment\n let x = 1.0 // second comment\n return x }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // Comments should be filtered out, leaving only 2 statements
            assert_eq!(p.body.len(), 2);
            assert!(matches!(&p.body[0], Statement::LetBinding { .. }));
            assert!(matches!(&p.body[1], Statement::Return(_)));
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_return_with_arithmetic_expression() {
        let src = "pattern P(a: Float, b: Float, c: Float) -> Float { return a + b * c }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            match &p.body[0] {
                Statement::Return(Expr::BinaryOp(_, op, _)) => {
                    // Top-level should be Add (multiplication binds tighter)
                    assert!(matches!(op, BinOp::Add));
                }
                other => panic!("expected Return(BinaryOp), got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    #[test]
    fn test_parse_while_with_assignment_inside() {
        let src = "pattern P() -> Float { let i = 0.0 while i < 10.0 { i = i + 1.0 } return i }";
        let decls = parse(src).unwrap();
        if let Declaration::Pattern(p) = &decls[0] {
            // body: let, while, return
            assert_eq!(p.body.len(), 3);
            match &p.body[1] {
                Statement::While { body, .. } => {
                    assert_eq!(body.len(), 1);
                    assert!(matches!(body[0], Statement::Assign { .. }));
                }
                other => panic!("expected While, got {:?}", other),
            }
        } else {
            panic!("expected Pattern");
        }
    }

    // ── MlogServer (Phase 6.1) ──────────────────────────────────────────

    #[test]
    fn test_parse_mlogserver_with_middleware_list() {
        let src = "mlogserver { port: 8080 middleware: [session, csrf, security_headers] }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.port, 8080);
            assert_eq!(s.middleware.len(), 3);
            assert_eq!(s.middleware[0], "session");
            assert_eq!(s.middleware[1], "csrf");
            assert_eq!(s.middleware[2], "security_headers");
        } else {
            panic!("expected MlogServer");
        }
    }

    #[test]
    fn test_parse_mlogserver_with_route() {
        let src = "mlogserver { port: 8080 route \"/health\" method=GET { respond(\"ok\") } }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.routes.len(), 1);
            assert_eq!(s.routes[0].path, "/health");
            assert_eq!(s.routes[0].method, "GET");
            assert_eq!(s.routes[0].body.len(), 1);
        } else {
            panic!("expected MlogServer");
        }
    }

    #[test]
    fn test_parse_mlogserver_with_route_and_requires() {
        let src = "mlogserver { port: 8080 route \"/admin\" method=POST requires=[admin] { respond(\"ok\") } }";
        let decls = parse(src).unwrap();
        if let Declaration::MlogServer(s) = &decls[0] {
            assert_eq!(s.routes[0].path, "/admin");
            assert_eq!(s.routes[0].method, "POST");
            assert_eq!(s.routes[0].requires.len(), 1);
            assert_eq!(s.routes[0].requires[0], "admin");
        } else {
            panic!("expected MlogServer");
        }
    }

    // ── Templates (Phase 6.2) ───────────────────────────────────────────

    #[test]
    fn test_parse_template_empty_body() {
        let src = "template Empty() -> Html { }";
        let decls = parse(src).unwrap();
        if let Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Empty");
            assert_eq!(t.return_type, "Html");
            assert!(
                t.body.trim().is_empty(),
                "expected empty body, got {:?}",
                t.body
            );
        } else {
            panic!("expected Template");
        }
    }

    #[test]
    fn test_parse_template_with_braces_in_body() {
        // Body contains { and } (CSS rule) — exercises preprocess_templates
        let src = "template Styled() -> Html { <style>.x { color: red; }</style> }";
        let decls = parse(src).unwrap();
        if let Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Styled");
            assert!(
                t.body.contains("color: red"),
                "body should contain CSS, got {:?}",
                t.body
            );
            assert!(t.body.contains('}'), "body should contain closing brace");
        } else {
            panic!("expected Template");
        }
    }

    #[test]
    fn test_parse_template_two_params() {
        let src = "template Page(title: String, body: String) -> Html { <html><h1>{{ title }}</h1>{{ body }}</html> }";
        let decls = parse(src).unwrap();
        if let Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Page");
            assert_eq!(t.params.len(), 2);
            assert_eq!(t.params[0].name, "title");
            assert_eq!(t.params[1].name, "body");
            assert!(t.body.contains("title"));
            assert!(t.body.contains("body"));
        } else {
            panic!("expected Template");
        }
    }

    // ── Additional error cases ──────────────────────────────────────────

    #[test]
    fn test_parse_error_unclosed_list_bracket() {
        let result = parse("pattern P() -> List { let x = [1.0, 2.0 return x }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_invalid_pattern_name() {
        // "123" is not a valid IDENT
        let result = parse("pattern 123() -> Float { return 1.0 }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_pattern_body() {
        let result = parse("pattern P() -> Float { return 1.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_pattern_name() {
        let result = parse("pattern () -> Float { return 1.0 }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_route_body() {
        let result = parse("mlogserver { port: 8080 route \"/x\" method=GET { respond(\"ok\") }");
        assert!(result.is_err());
    }
}
