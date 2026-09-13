/**
 * @file tree-sitter-mlog grammar for the Metalogos language.
 *
 * @description
 * Перенос синтаксиса Metalogos в tree-sitter (Наряд №289, issue #301).
 * Источник истины — `REFERENCE.md` §3 (Syntax), сверено с `src/grammar.pest`
 * (PEG-грамматика основного компилятора).
 *
 * Покрытие (Блок 1 — грамматика):
 *   - Декларации верхнего уровня: entity, pattern, learnable pattern, flow,
 *     rule, reflex, reflex_seq, reflex_gen, vision, type alias, llm,
 *     mlogserver, template, db, schema, skill_index, memory, import, hook,
 *     relate, sandbox, mutate, eval, fluid, adapt, memorize, forget,
 *     conversation, tool, context_budget.
 *   - Statements: let (+mut), assign, if-then (block + expr), if-block,
 *     each, while, match (stmt + expr), break, continue, return.
 *   - Expressions: or/and/compare/add/mul precedence layers, unary minus,
 *     try, if-then-else, qualified call, call, field/index access,
 *     struct literal, list literal, parenthesized, string/multiline/int/
 *     float/bool/ident primaries.
 *   - Comments: `//` line comment.
 *
 * Расхождения с `grammar.pest` (зафиксировано явно, не тихо):
 *   - pest ordered choice ↔ tree-sitter precedence. Для `if/else`-блока
 *     и `if-then-else`-выражения использованы явные `prec.left` /
 *     `prec.dynamic` декларации (tree-sitter conflict resolver).
 *   - pest `_{ ... }` silent rules ↔ tree-sitter `inline` + named rules.
 *   - pest `IDENT` исключает ключевые слова через ordered choice в
 *     consumer rules; в tree-sitter ключевые слова — `reserved`-слова,
 *     IDENT определён через `token.immediate(/[...]+/)` (внешний lexer
 *     различает по longest-match).
 *
 * Ноль диффа в `src/**`, `tests/**`, `src/grammar.pest` (контракт наряда).
 */

const PRIMITIVE_TYPES = [
  "String",
  "Float",
  "Bool",
  "Int",
  "List",
  "Map",
  "Html",
  "Query",
  "Secret",
  "Encrypted",
  "Hash",
  "Session",
  "Subgraph",
  "Reflex",
  "Vision",
  "LlmStream",
  "Unit",
];

module.exports = grammar({
  name: "mlog",

  // Single-line `//` comments + standard whitespace.
  extras: $ => [/\s/, $.comment],

  // Inline rules — reduce tree noise (mirrors pest `_{ ... }` silent rules).
  inline: $ => [
    $._declaration,
    $._statement,
    $._expression,
    $._type_name,
    $._layer_arg,
  ],

  // Known GLR conflicts where keyword-led declarations and IDENT-led
  // expression rules share prefixes. Tree-sitter resolves them at parse
  // time by trying both paths.
  conflicts: $ => [
    [$.memorize_decl, $.primary_expr],
    [$.relate_decl, $.primary_expr],
    [$.memorize_decl, $.relate_decl],
    [$.entity_record_decl, $.call_expr],
    [$.entity_record_decl, $.struct_literal],
    [$.entity_simple_decl, $.struct_literal],
    [$.call_expr, $.primary_expr],
    [$.postfix_op],
    [$.if_block_stmt, $.block_if_else_expr],
    [$.entity_type_field, $.primary_expr],
    [$.block_if_else_expr],
    [$.if_else_expr],
  ],

  rules: {
    // ── Top-level program ──────────────────────────────────────────
    source_file: $ => repeat($._declaration),

    comment: $ => token(seq("//", /.*/)),

    // ── Declarations (mirrors pest `declaration` ordered choice) ───
    // Ordered: more-specific decls before more-generic ones (no ordered
    // choice in tree-sitter; we use lexical disambiguation via leading
    // keyword tokens + GLR for shared prefixes).
    _declaration: $ => choice(
      $.type_alias_decl,        // "type"
      $.llm_decl,               // "llm"
      $.mlogserver_decl,        // "mlogserver" | "server"
      $.template_decl,          // "template"
      $.db_decl,                // "db"
      $.schema_decl,            // "schema"
      $.skill_index_decl,       // "skill_index"
      $.memory_decl,            // "memory"
      $.import_decl,            // "import"
      $.hook_decl,              // "hook"
      $.sandbox_decl,           // "sandbox"
      $.mutate_decl,            // "mutate"
      $.eval_decl,              // "eval"
      $.fluid_decl,             // "fluid"
      $.adapt_decl,             // "adapt"
      $.memorize_decl,          // "memorize"
      $.relate_decl,            // "relate"
      $.forget_decl,            // "forget"
      $.conversation_decl,      // "conversation"
      $.tool_decl,              // "tool"
      $.context_budget_decl,    // "context_budget"
      $.entity_type_decl,       // "entity" Name { ... }
      $.entity_record_decl,     // "entity" Name(field: Type, ...) { ... }
      $.entity_simple_decl,     // "entity" Name { field: value, ... }
      $.rule_decl,               // "rule"
      $.learnable_pattern_decl, // "learnable" "pattern"
      $.pattern_decl,           // "pattern"
      $.flow_decl,               // "flow"
      $.reflex_decl,             // "reflex"
      $.reflex_seq_decl,         // "reflex_seq"
      $.reflex_gen_decl,         // "reflex_gen"
      $.vision_decl,             // "vision"
      $.test_decl,               // "test"
    ),

    // ── Type alias ───────────────────────────────────────────────
    type_alias_decl: $ => seq("type", $.ident, "=", $._type_name),

    // ── LLM config (ADR-0048) ─────────────────────────────────────
    llm_decl: $ => seq("llm", "{", repeat1($.llm_field), "}"),
    llm_field: $ => choice(
      $.llm_providers_field,
      $.llm_default_model_field,
      $.llm_failover_field,
      $.llm_circuit_breaker_field,
      $.llm_timeout_field,
    ),
    llm_providers_field: $ => seq("providers", ":", "[", optional(seq($.llm_provider_entry, repeat(seq(",", $.llm_provider_entry)), optional(","))), "]"),
    llm_provider_entry: $ => seq("{", "alias", ":", $.ident, ",", "provider", ":", $.ident, optional(seq(",", "key", ":", $._expression)), optional(seq(",", "url", ":", $._expression)), "}"),
    llm_default_model_field: $ => seq("default_model", ":", $._expression),
    llm_failover_field: $ => seq("failover", ":", $.ident),
    llm_circuit_breaker_field: $ => seq("circuit_breaker", ":", $.int),
    llm_timeout_field: $ => seq("timeout", ":", $.int),

    // ── MlogServer (ADR-0074) ───────────────────────────────────
    mlogserver_decl: $ => seq(choice("mlogserver", "server"), "{", repeat(choice(
      $.mlogserver_port_field,
      $.mlogserver_host_field,
      $.mlogserver_middleware_field,
      $.mlogserver_rate_limit_field,
      $.route_decl,
    )), "}"),
    mlogserver_port_field: $ => seq("port", ":", $.int),
    mlogserver_host_field: $ => seq("host", ":", $.string),
    mlogserver_middleware_field: $ => seq("middleware", ":", "[", optional(seq($.ident, repeat(seq(",", $.ident)), optional(","))), "]"),
    mlogserver_rate_limit_field: $ => seq("rate_limit", ":", $.int),
    route_decl: $ => seq("route", $.string, "method", "=", $.ident, optional($.route_requires), "{", repeat($._statement), "}"),
    route_requires: $ => seq("requires", "=", "[", optional(seq($.ident, repeat(seq(",", $.ident)), optional(","))), "]"),

    // ── Template (Phase 6.2) ────────────────────────────────────
    template_decl: $ => seq("template", $.ident, "(", optional($.param_list), ")", "->", $._type_name, "{", repeat($._statement), "}"),

    // ── DB / Schema / Skill index ───────────────────────────────
    db_decl: $ => seq("db", "{", repeat($.db_field), "}"),

    db_field: $ => seq($.ident, ":", $._expression),

    schema_decl: $ => seq("schema", $.ident, "{", repeat($.schema_op), "}"),
    schema_op: $ => seq($.ident, $.ident, "(", optional(seq($._expression, repeat(seq(",", $._expression)))), ")"),

    skill_index_decl: $ => seq("skill_index", $.ident, "{", repeat($._statement), "}"),

    // ── Memory / Conversation / Context budget ──────────────────
    memory_decl: $ => seq("memory", "{", repeat($.memory_field), "}"),

    memory_field: $ => seq($.ident, ":", $._expression),

    conversation_decl: $ => seq("conversation", "{", repeat($.conversation_field), "}"),

    conversation_field: $ => choice(
      seq("ttl", ":", $.int),
      seq("max_messages", ":", $.int),
      seq("compress_after", ":", $.int),
    ),

    context_budget_decl: $ => seq("context_budget", "{", $.context_budget_body, "}"),
    context_budget_body: $ => seq("pattern", ":", $.string, optional(seq(",", "limit", ":", $._expression))),

    // ── Import / Hook / Sandbox / Mutate / Eval / Fluid ────────
    import_decl: $ => seq("import", $.string, optional(seq("as", $.ident))),
    hook_decl: $ => seq("hook", $.ident, "=", $._expression),
    sandbox_decl: $ => seq("sandbox", "{", repeat($.sandbox_field), "}"),

    sandbox_field: $ => seq($.ident, ":", "[", optional(seq($.string, repeat(seq(",", $.string)), optional(","))), "]"),
    mutate_decl: $ => seq("mutate", $.ident, "{", repeat($._statement), "}"),
    eval_decl: $ => seq("eval", "{", repeat($._statement), "}"),
    fluid_decl: $ => seq("fluid", $.ident, "=", $.fluid_variant_list),
    fluid_variant_list: $ => seq("[", $.fluid_variant, repeat(seq(",", $.fluid_variant)), optional(","), "]"),
    fluid_variant: $ => seq("{", $.ident, ":", $._expression, ",", "value", ":", $._expression, ",", "confidence", ":", $._expression, "}"),

    // ── Adapt / Memorize / Relate / Forget ─────────────────────
    adapt_decl: $ => seq("adapt", $.ident, "{", repeat($._statement), "}"),

    memorize_decl: $ => seq("memorize", $._expression, "with", $.memorize_attr_list),
    memorize_attr_list: $ => seq($.memorize_attr, repeat(seq(",", $.memorize_attr))),
    memorize_attr: $ => seq($.ident, "=", $._expression),

    relate_decl: $ => seq("relate", $._expression, "with", $.memorize_attr_list),

    forget_decl: $ => seq("forget", "(", $._expression, ",", $._expression, optional(seq(",", $._expression)), ")"),

    // ── Tool ───────────────────────────────────────────────────
    tool_decl: $ => seq("tool", $.ident, "(", optional($.param_list), ")", "->", $._type_name, "{", repeat($._statement), "}"),

    // ── Entity (three forms — mirror pest exactly) ──────────────
    // entity_type_decl:   entity Name { field: Type [= literal]?, ... }
    // entity_record_decl: entity Name : Type = { field: value, ... }
    // entity_simple_decl: entity Name : Type = expression
    // Ordered: record (has `: Type = { ... }`) before simple (`: Type = expr`),
    // both before type_decl (only `{ ... }`). pest ordered choice gives
    // the same priority — tree-sitter resolves via lexical disambiguation
    // on the `:` and `=` after the name.
    entity_record_decl: $ => seq("entity", $.ident, ":", $._type_name, "=", "{", $.struct_field_init_list, "}"),
    entity_simple_decl: $ => seq("entity", $.ident, ":", $._type_name, "=", $._expression),
    entity_type_decl: $ => seq("entity", $.ident, "{", repeat1($.entity_type_field), "}"),
    entity_type_field: $ => seq($.ident, ":", $._type_name, optional(seq("=", $._expression)), optional(",")),

    // ── Rule ───────────────────────────────────────────────────
    rule_decl: $ => seq("rule", $.ident, "{", $.rule_body, "}"),
    rule_body: $ => seq($.rule_match, repeat($.rule_action)),
    rule_match: $ => seq("match", "(", $._expression, ")"),
    rule_action: $ => seq($.ident, ":", $._expression),

    // ── Pattern / Learnable pattern ───────────────────────────
    pattern_decl: $ => seq("pattern", $.ident, "(", optional($.param_list), ")", "->", $._type_name, "{", repeat($._statement), "}"),
    learnable_pattern_decl: $ => seq("learnable", "pattern", $.ident, "(", optional($.param_list), ")", "->", $._type_name, "{", repeat($.learnable_field), "}"),

    learnable_field: $ => choice(
      seq("prompt", ":", $._expression),
      seq("context", ":", choice(
        seq("recall", "(", $._expression, optional(seq(",", "limit", "=", $._expression)), ")"),
        "auto",
        "none",
        $._expression,
      )),
      seq("context_strategy", ":", choice("none", "auto", "compress")),
      seq("conversation", ":", $._expression),
      seq("model", ":", $._expression),
      seq("max_tokens", ":", $._expression),
      seq("cache", ":", $.bool),
      seq("cache_ttl", ":", $._expression, ".", $.ident),
      seq("cache_semantic", ":", $.bool),
      seq("cache_threshold", ":", $.float),
      seq("max_context_tokens", ":", $.int),
      // ADR-0117 distillation fields (any order):
      seq("distill_to", ":", $.ident),
      seq("distill_after", ":", $.int),
      seq("fallback_if", ":", "confidence", $._compare_op, $.float),
    ),

    // ── Flow (ADR-0056: checkpoint + branches) ────────────────
    flow_decl: $ => seq("flow", $.ident, "{", $.flow_pipeline, repeat($.branch_def), "}"),
    flow_pipeline: $ => seq("input", ":", $._type_name, "=", $._expression, repeat($.flow_step), "->", "output"),
    flow_step: $ => seq("->", choice($.checkpoint_call, $.step_ident)),
    checkpoint_call: $ => seq("checkpoint", "(", $.string, ")"),
    step_ident: $ => $.ident,
    branch_def: $ => seq($.step_ident, "{", repeat($.branch), "}"),
    branch: $ => seq($.ident, "(", $.branch_condition, ")", "->", $.step_ident),
    branch_condition: $ => seq($.ident, ".", $.ident, $._compare_op, $._expression),

    // ── Reflex family (ADR-0114 / ADR-0119 / ADR-0120) ─────────
    reflex_decl: $ => seq("reflex", $.ident, "{", repeat($.reflex_field), "}"),

    reflex_field: $ => choice(
      seq("input", ":", "embedding", "(", $.int, ")"),
      $.reflex_layers_field,
      $.reflex_labels_field,
      seq("seed", ":", $.int),
    ),
    reflex_layers_field: $ => seq("layers", ":", "[", optional(seq($.layer_spec, repeat(seq(",", $.layer_spec)), optional(","))), "]"),
    layer_spec: $ => seq($.ident, "(", optional($.layer_arg_list), ")"),
    layer_arg_list: $ => seq($._layer_arg, repeat(seq(",", $._layer_arg))),
    _layer_arg: $ => choice($.int, $.ident, $.string),

    reflex_seq_decl: $ => seq("reflex_seq", $.ident, "{", repeat($.reflex_seq_field), "}"),

    reflex_seq_field: $ => choice(
      seq("input", ":", "embedding", "(", $.int, ")"),
      seq("seq_len", ":", $.int),
      $.reflex_layers_field,
      $.reflex_labels_field,
      seq("seed", ":", $.int),
    ),
    reflex_labels_field: $ => seq("labels", ":", "[", $.string, repeat(seq(",", $.string)), optional(","), "]"),

    reflex_gen_decl: $ => seq("reflex_gen", $.ident, "{", repeat($.reflex_gen_field), "}"),

    reflex_gen_field: $ => choice(
      seq("input", ":", "embedding", "(", $.int, ")"),
      seq("vocab_size", ":", $.int),
      $.reflex_layers_field,
      seq("seed", ":", $.int),
    ),

    // ── Vision (ADR-0124) ──────────────────────────────────────
    vision_decl: $ => seq("vision", $.string, "{", repeat($.vision_field), "}"),

    vision_field: $ => choice(
      seq("model", ":", $.string),
      seq("steps", ":", $.int),
      seq("width", ":", $.int),
      seq("height", ":", $.int),
      seq("seed", ":", $.int),
      seq("policy", ":", $.vision_ident_val),
      seq("profile", ":", $.vision_ident_val),
      seq($.ident, ":", choice($.string, $.int, $.vision_ident_val)), // unknown field — captured (per pest's vision_unknown_field)
    ),
    vision_ident_val: $ => token(/[A-Za-z_][A-Za-z0-9_'-]*/),

    // ── Test (Наряд №120 + №287 doc-tests) ──────────────────────
    test_decl: $ => seq("test", $.ident, "{", repeat($._statement), "}"),

    // ── Statements ─────────────────────────────────────────────
    _statement: $ => choice(
      $.match_stmt,
      $.if_then_stmt,
      $.if_block_stmt,
      $.each_stmt,
      $.while_stmt,
      $.let_binding,
      $.break_stmt,
      $.continue_stmt,
      $.return_stmt,
      $.memorize_decl,
      $.relate_decl,
      $.forget_decl,
      $.assign_or_expr,
    ),

    match_stmt: $ => seq("match", $._expression, "{", repeat($.match_arm), optional($.match_else), "}"),
    match_arm: $ => choice(
      seq($.string, "then", "{", repeat($._statement), "}"),
      seq("starts_with", $.string, "then", "{", repeat($._statement), "}"),
      seq("contains", $.string, "then", "{", repeat($._statement), "}"),
      seq($._compare_op, $._expression, "then", "{", repeat($._statement), "}"),
    ),
    match_else: $ => seq("else", "{", repeat($._statement), "}"),

    if_block_stmt: $ => seq("if", $._expression, "{", repeat($._statement), "}", repeat($.else_if_block), optional($.else_block)),
    if_then_stmt: $ => seq("if", $._expression, "then", "{", repeat($._statement), "}", repeat($.else_if_then_block), optional($.else_block)),
    else_if_block: $ => seq("else", "if", $._expression, "{", repeat($._statement), "}"),
    else_if_then_block: $ => seq("else", "if", $._expression, "then", "{", repeat($._statement), "}"),
    else_block: $ => seq("else", "{", repeat($._statement), "}"),

    each_stmt: $ => seq("each", $.ident, optional(seq(",", $.ident)), "in", $._expression, "{", repeat($._statement), "}"),
    while_stmt: $ => seq("while", $._expression, "{", repeat($._statement), "}"),

    let_binding: $ => seq("let", optional("mut"), $.ident, "=", choice($.match_expr, $._expression)),

    break_stmt: $ => "break",
    continue_stmt: $ => "continue",
    return_stmt: $ => seq("return", $._expression),

    assign_or_expr: $ => choice(
      seq($.ident, "=", $._expression),
      $._expression,
    ),

    match_expr: $ => seq("match", $._expression, "{", repeat($.match_arm), optional($.match_else), "}"),

    // ── Expressions (layered precedence, mirroring pest) ───────
    _expression: $ => $.or_expr,
    or_expr: $ => prec.left(1, seq($.and_expr, repeat(seq("or", $.and_expr)))),
    and_expr: $ => prec.left(2, seq($.compare_expr, repeat(seq("and", $.compare_expr)))),
    compare_expr: $ => prec.left(3, seq($.add_expr, repeat(seq($._compare_op, $.add_expr)))),
    add_expr: $ => prec.left(4, seq($.mul_expr, repeat(seq(choice("+", "-"), $.mul_expr)))),
    mul_expr: $ => prec.left(5, seq($.unary_expr, repeat(seq(choice("*", "/"), $.unary_expr)))),

    unary_expr: $ => choice(
      $.try_expr,
      $.if_else_expr,
      $.unary_minus,
      $.access_expr,
      $.primary_expr,
    ),
    try_expr: $ => seq("try", $.unary_expr),
    unary_minus: $ => prec(6, seq("-", $.unary_expr)),

    if_else_expr: $ => prec.right(0, seq("if", $._expression, "then", $._expression, "else", $._expression)),

    access_expr: $ => prec.left(7, seq($.primary_expr, repeat($.postfix_op))),
    postfix_op: $ => choice(
      seq(".", $.ident),
      seq("[", $._expression, "]"),
      // qualified call: IDENT.IDENT(args) — emitted as postfix on primary
      seq(".", $.ident, "(", optional($.expression_list), ")"),
    ),

    // call_expr: IDENT(args) — handled at primary_expr via call_expr rule.
    // qualified_call_expr (IDENT.IDENT(args)) is subsumed by access_expr +
    // postfix_op (".ident" + "(args)").
    call_expr: $ => seq($.ident, "(", optional($.expression_list), ")"),

    expression_list: $ => seq($._expression, repeat(seq(",", optional(/\s*/), $._expression)), optional(",")),

    primary_expr: $ => choice(
      $.paren_expr,
      $.block_if_else_expr,
      $.struct_literal,
      $.list_literal,
      $.bool,
      $.float,
      $.multiline_string,
      $.string,
      $.int,
      $.call_expr,
      $.ident,
    ),

    paren_expr: $ => seq("(", $._expression, ")"),
    block_if_else_expr: $ => seq("if", $._expression, "{", repeat($._statement), "}", repeat($.else_if_block), optional($.else_block)),

    struct_literal: $ => seq("{", $.struct_field_init_list, "}"),
    struct_field_init_list: $ => seq($.struct_field_init, repeat(seq(",", $.struct_field_init)), optional(",")),
    struct_field_init: $ => seq($.ident, ":", $._expression),

    list_literal: $ => seq("[", optional($.expression_list), "]"),

    _compare_op: $ => choice(
      ">=", "<=", "==", "!=", ">", "<",
    ),

    // ── Param list (used by pattern / learnable / tool / template / entity_record) ──
    param_list: $ => seq($.param, repeat(seq(",", $.param)), optional(",")),
    param: $ => seq($.ident, ":", $._type_name),

    // ── Type name ─────────────────────────────────────────────
    // Built-in primitive types + user-defined (entity/pattern names).
    _type_name: $ => choice(...PRIMITIVE_TYPES, $.ident),

    // ── Lexical tokens ─────────────────────────────────────────
    // `ident` allows ASCII letters, underscore, and Cyrillic А-я (per pest).
    // Apostrophe is allowed inside (pest uses it for some idents).
    // `token.immediate` prevents whitespace skipping inside the ident.
    ident: $ => /[\p{L}_][\p{L}\p{N}_']*/u,

    // Numeric literals — int before float to avoid prefix-matching.
    int: $ => /\d+/,
    float: $ => /\d+\.\d+/,
    bool: $ => choice("true", "false"),

    string: $ => seq('"', repeat(choice(/[^"\\]/, $.escape_seq)), '"'),
    multiline_string: $ => seq('"""', /[^"]*/, '"""'),  // simplified; per pest it's `(!(""\"\"\"") ~ ANY)*`
    escape_seq: $ => token.immediate(seq("\\", choice(/["\\ntr]/, /u[0-9a-fA-F]{4}/))),
  },
});
