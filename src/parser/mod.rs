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
/// №958 (the audit 12.11 M-1): the INPUT-DEPTH pre-scan — the ONE gate
/// every source passes BEFORE pest sees it. Two counters, both bounded by
/// `limits::max_input_depth()` (`METALOGOS_MAX_INPUT_DEPTH`): the
/// bracket nesting depth and the longest consecutive operator run (the
/// audit's probes: 50 000 chained `+` and 3 000 nested `if` overflowed
/// the 8 MB parser thread — SIGABRT; the scan refuses with a clean
/// `[INPUT_TOO_DEEP]` naming the position instead). Strings and comments
/// are modeled: their contents count nothing.
fn scan_input_depth(source: &str) -> Result<(), ParseError> {
    let limit = crate::limits::max_input_depth();
    let bytes: Vec<char> = source.chars().collect();
    let mut depth = 0usize;
    let mut chain = 0usize;
    let mut in_str: Option<char> = None;
    let mut in_block_comment = false;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut byte_off = 0usize;
    let mut i = 0usize;
    let n = bytes.len();
    let mut deep_at: Option<(usize, usize, usize, &str)> = None; // (offset, line, col, kind)
    while i < n {
        let c = bytes[i];
        if in_block_comment {
            if c == '*' && i + 1 < n && bytes[i + 1] == '/' {
                in_block_comment = false;
                i += 1;
                byte_off += c.len_utf8() + bytes[i].len_utf8();
                col += 2;
                i += 1;
                continue;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            byte_off += c.len_utf8();
            i += 1;
            continue;
        }
        if let Some(q) = in_str {
            if c == '\\' {
                byte_off += c.len_utf8() + bytes.get(i + 1).map(|x| x.len_utf8()).unwrap_or(0);
                i += 2;
                col += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            byte_off += c.len_utf8();
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_str = Some('"');
            }
            '/' if i + 1 < n && bytes[i + 1] == '*' => {
                in_block_comment = true;
                byte_off += c.len_utf8();
                i += 1;
                col += 1;
                continue;
            }
            '/' if i + 1 < n && bytes[i + 1] == '/' => {
                while i < n && bytes[i] != '\n' {
                    byte_off += bytes[i].len_utf8();
                    i += 1;
                }
                continue;
            }
            '{' | '(' | '[' => {
                depth += 1;
                if depth > limit && deep_at.is_none() {
                    deep_at = Some((byte_off, line, col, "nesting"));
                }
            }
            '}' | ')' | ']' => {
                depth = depth.saturating_sub(1);
                chain = 0;
            }
            '+' | '-' | '*' | '<' | '>' | '=' | '!' | '&' | '|' | '/' | '%' => {
                // №958: the operator CHAIN — operators bound in one
                // expression line. The reset points are the newline and
                // the bracket boundaries (statements separate at the
                // newline; the audit's 50 000 `+` probe sits on ONE line
                // — the chain counts it; the widest legal line in the
                // corpus carries 1125 operators, the default 2048 keeps
                // it parseable).
                chain += 1;
                if chain > limit && deep_at.is_none() {
                    deep_at = Some((byte_off, line, col, "operator chain"));
                }
            }
            '\n' | ';' => chain = 0,
            _ => {}
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        byte_off += c.len_utf8();
        i += 1;
    }
    if let Some((off, line, col, kind)) = deep_at {
        let msg = format!(
            "[INPUT_TOO_DEEP] the input {} depth exceeds the limit {} (METALOGOS_MAX_INPUT_DEPTH, №958) at line {}:{} — split the expression or raise the limit",
            kind, limit, line, col
        );
        if let Some(pos) = pest::Position::new(source, off) {
            return Err(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError { message: msg },
                pos,
            ));
        }
        return Err(error_at_start(source, msg));
    }
    Ok(())
}

pub fn parse(source: &str) -> Result<Vec<Declaration>, ParseError> {
    // №958 (the audit 12.11 M-1): the depth pre-scan BEFORE pest — the
    // deep-input probes aborted (SIGABRT) inside the parser thread; the
    // scan refuses with a clean [INPUT_TOO_DEEP] instead.
    scan_input_depth(source)?;
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
                Rule::schema_decl => declarations.push(parse_schema_decl(inner_pair)?),
                Rule::skill_index_decl => declarations.push(parse_skill_index_decl(inner_pair)),
                Rule::memory_decl => declarations.push(parse_memory_decl(inner_pair)?),
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
                    effects: None,
                    body: vec![parse_if_block_stmt(inner_pair)?],
                })),
                Rule::fluid_decl => declarations.push(parse_fluid_decl(inner_pair)?),
                Rule::adapt_decl => declarations.push(parse_adapt_decl(inner_pair)?),
                Rule::relate_decl => declarations.push(parse_relate_decl(inner_pair)?),
                Rule::sandbox_decl => declarations.push(parse_sandbox_decl(inner_pair)?),
                Rule::hook_decl => declarations.push(parse_hook_decl(inner_pair)?),
                // Наряд №392: the deny-event handler declaration.
                Rule::on_deny_decl => declarations.push(parse_on_deny_decl(inner_pair)?),
                Rule::mutate_decl => declarations.push(parse_mutate_decl(inner_pair)?),
                Rule::eval_decl => declarations.push(parse_eval_decl(inner_pair)?),
                Rule::test_decl => declarations.push(parse_test_decl(inner_pair)),
                Rule::conversation_decl => declarations.push(parse_conversation_decl(inner_pair)?),
                Rule::context_budget_decl => {
                    declarations.push(parse_context_budget_decl(inner_pair))
                }
                Rule::type_alias_decl => declarations.push(parse_type_alias_decl(inner_pair)),
                Rule::llm_decl => declarations.push(parse_llm_decl(inner_pair)?),
                Rule::tool_decl => declarations.push(parse_tool_decl(inner_pair)?),
                Rule::profile_decl => declarations.push(parse_profile_decl(inner_pair)),
                // №801 (gh#1548): the MCP policy — the upstream tool
                // classes for the floating session label (the gateway G2).
                Rule::mcp_policy_decl => declarations.push(parse_mcp_policy_decl(inner_pair)?),
                // Наряд №332 (ADR-0164): perception origin declaration.
                Rule::origin_decl => declarations.push(decl::parse_origin_decl(inner_pair)),
                Rule::reflex_decl => declarations.push(parse_reflex_decl(inner_pair)?),
                // №947 (ADR-0223, W3 §13.12): the spec declaration.
                Rule::spec_decl => declarations.push(parse_spec_decl(inner_pair)?),
                // Наряд №183: reflex_seq — sequence-model declaration (ADR-0119)
                Rule::reflex_seq_decl => declarations.push(parse_reflex_seq_decl(inner_pair)?),
                Rule::reflex_gen_decl => declarations.push(parse_reflex_gen_decl(inner_pair)?),
                // Наряд №238 (Vision R4.1): vision — image-generation declaration
                Rule::vision_decl => declarations.push(decl::parse_vision_decl(inner_pair)?),
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
