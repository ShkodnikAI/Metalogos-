# №532 — the wildcard tail ledger (the grep-protocol artifact)

The `wildcard_enum_match_arm` lint is `deny` in `src/interpreter/execution.rs`,
`src/vm.rs` and `src/semantic.rs` (the N-1 class leaked through a wildcard Value
arm — the liar-string truthiness, closed explicitly in both backends). The tail
below survives under FUNCTION-LEVEL `#[allow]` with the shared reason: these are
report/parse/classification/dispatch catch-alls where NO security decision reads
the wildcard arm. The security surface — Value truthiness — is EXPLICIT in both
backends (TW twin + VM twin), pinned by the №503 outcome-parity suite.
A NEW wildcard in these files fails CI until it is enumerated or added to this
ledger with a reason.

Total tail: 98 wildcard arms.

- `fn collect_import_decls` — wildcard arms at lines 205
- `fn decl_import_ident` — wildcard arms at lines 277
- `fn label_source` — wildcard arms at lines 479, 506
- `fn block_expr_label` — wildcard arms at lines 698
- `fn collect_assigned_vars` — wildcard arms at lines 783
- `fn validate_decl_labels` — wildcard arms at lines 1078
- `fn is_media_binding_expr` — wildcard arms at lines 1153
- `fn media_typed_object` — wildcard arms at lines 1167
- `fn check_media_expr` — wildcard arms at lines 1270
- `fn media_opacity_violations` — wildcard arms at lines 1440
- `fn media_origin_violations` — wildcard arms at lines 1554, 1582
- `fn classify_binding` — wildcard arms at lines 1620
- `fn check_origin_stmts` — wildcard arms at lines 1730
- `fn check_origin_expr` — wildcard arms at lines 1900
- `fn check_origin_expr_stmt` — wildcard arms at lines 1919
- `fn backend_select_ladder_violations` — wildcard arms at lines 2012
- `fn verify_backend_ladder` — wildcard arms at lines 2056
- `fn check_backend_stmts` — wildcard arms at lines 2205
- `fn recall_surface_violations` — wildcard arms at lines 2372
- `fn verify_recall_call` — wildcard arms at lines 2400
- `fn check_recall_stmts` — wildcard arms at lines 2553
- `fn check_recall_expr` — wildcard arms at lines 2631
- `fn forecast_surface_violations` — wildcard arms at lines 2655
- `fn check_forecast_stmts` — wildcard arms at lines 2766
- `fn check_effect_trails` — wildcard arms at lines 3167
- `fn register` — wildcard arms at lines 3222
- `fn sink_arg_label` — wildcard arms at lines 3441
- `fn media_arg_origin_kind` — wildcard arms at lines 3482
- `fn sink_clearance_violations` — wildcard arms at lines 3620, 3642
- `fn walk_expr` — wildcard arms at lines 4081
- `fn walk_stmts` — wildcard arms at lines 4147, 4541
- `fn check_deny_events` — wildcard arms at lines 4628
- `fn check_deny_match_exhaustiveness` — wildcard arms at lines 4725
- `fn expr_operands` — wildcard arms at lines 4930
- `fn integrity_decision_violations` — wildcard arms at lines 4959
- `fn prov_of` — wildcard arms at lines 5019
- `fn args_of` — wildcard arms at lines 5027, 5266
- `fn check_program` — wildcard arms at lines 5450, 5774
- `fn svg_security_lint` — wildcard arms at lines 7658
