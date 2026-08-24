// ── Pest → AST conversion for METALOGOS M1+M2 ──────────────────────

use pest::iterators::Pair;
use pest::Parser as _;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct MlogParser;

pub type ParseError = pest::error::Error<Rule>;

use crate::ast::*;

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
    #[allow(clippy::expect_used)]
    let pos = pest::Position::new(source, 0)
        .or_else(|| pest::Position::new("", 0))
        .expect("position 0 is always valid in any string");
    pest::error::Error::new_from_pos(pest::error::ErrorVariant::CustomError { message: msg }, pos)
}

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
        Err(_) => Err(error_at_start(source, "parser thread panicked".to_string())),
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
                Rule::entity_record_decl => {
                    declarations.push(parse_entity_record_decl(inner_pair)?)
                }
                Rule::entity_simple_decl => {
                    declarations.push(parse_entity_simple_decl(inner_pair)?)
                }
                Rule::rule_decl => declarations.push(parse_rule_decl(inner_pair)?),
                Rule::memorize_decl => declarations.push(parse_memorize_decl(inner_pair)?),
                Rule::forget_decl => declarations.push(parse_forget_decl(inner_pair)?),
                Rule::if_block_stmt => declarations.push(Declaration::Pattern(PatternDecl {
                    span: Span::from_pest(inner_pair.as_span()),
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
                Rule::test_decl => declarations.push(parse_test_decl(inner_pair)),
                Rule::conversation_decl => declarations.push(parse_conversation_decl(inner_pair)),
                Rule::context_budget_decl => {
                    declarations.push(parse_context_budget_decl(inner_pair))
                }
                Rule::type_alias_decl => declarations.push(parse_type_alias_decl(inner_pair)),
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

pub(crate) mod decl;
pub(crate) mod expr;
pub(crate) mod helpers;
pub(crate) mod stmt;
#[cfg(test)]
mod tests;

use decl::*;
use expr::*;
use helpers::*;
use stmt::*;
