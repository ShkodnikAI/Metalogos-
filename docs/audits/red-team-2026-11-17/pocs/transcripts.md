
## p01_baseline_clean

$ mlog check /tmp/redteam-967/pocs/p01_baseline_clean.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p01_baseline_clean.mlog

```
doc
[exit: 0]
```

## p02_llm_unannotated

$ mlog check /tmp/redteam-967/pocs/p02_llm_unannotated.mlog

```
1 error:
  1: строка 4: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p02_llm_unannotated.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p03_llm_annotated_ok

$ mlog check /tmp/redteam-967/pocs/p03_llm_annotated_ok.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p03_llm_annotated_ok.mlog

```
error: call_llm() failed: LLM is not configured — `call_llm` refuses (№898: no LLM env var and no `llm {}` declaration is set, and the silent mock default is rolled back). Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }` block in the program — or opt in EXPLICITLY to the deterministic offline mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`).
[exit: 1]
```

## p04_llm_annotated_trusted

$ mlog check /tmp/redteam-967/pocs/p04_llm_annotated_trusted.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<public, trusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p04_llm_annotated_trusted.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<public, trusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p05_env_unannotated

$ mlog check /tmp/redteam-967/pocs/p05_env_unannotated.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p05_env_unannotated.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p06_env_annotated_pub_untrust

$ mlog check /tmp/redteam-967/pocs/p06_env_annotated_pub_untrust.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p06_env_annotated_pub_untrust.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p07_env_annotated_pub_trust

$ mlog check /tmp/redteam-967/pocs/p07_env_annotated_pub_trust.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p07_env_annotated_pub_trust.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p08_env_annotated_priv_trust_deadend

$ mlog check /tmp/redteam-967/pocs/p08_env_annotated_priv_trust_deadend.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<private, trusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'private, trusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<private, trusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p08_env_annotated_priv_trust_deadend.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<private, trusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'private, trusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<private, trusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p09_output_private_word

$ mlog check /tmp/redteam-967/pocs/p09_output_private_word.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<private, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, trusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, trusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p09_output_private_word.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<private, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, trusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, trusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p10_input_env_expr

$ mlog check /tmp/redteam-967/pocs/p10_input_env_expr.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p10_input_env_expr.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p11_env_redact_no_launder

$ mlog check /tmp/redteam-967/pocs/p11_env_redact_no_launder.mlog

```
[REDACT][audit-event] redact applied in pattern Scrub — policy 'email', target conf 'unknown'
1 error:
  1: строка 5: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p11_env_redact_no_launder.mlog

```
[REDACT][audit-event] redact applied in pattern Scrub — policy 'email', target conf 'unknown'
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p12_escape_env_conf_kept

$ mlog check /tmp/redteam-967/pocs/p12_escape_env_conf_kept.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p12_escape_env_conf_kept.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p13_http_get_unannotated

$ mlog check /tmp/redteam-967/pocs/p13_http_get_unannotated.mlog

```
1 error:
  1: строка 4: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p13_http_get_unannotated.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p14_read_file_unannotated

$ mlog check /tmp/redteam-967/pocs/p14_read_file_unannotated.mlog

```
1 error:
  1: строка 4: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p14_read_file_unannotated.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p15_mcp_call_unannotated

$ mlog check /tmp/redteam-967/pocs/p15_mcp_call_unannotated.mlog

```
1 error:
  1: строка 4: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p15_mcp_call_unannotated.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p18_each_accumulator

$ mlog check /tmp/redteam-967/pocs/p18_each_accumulator.mlog

```
1 error:
  1: строка 9: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p18_each_accumulator.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p19_pattern_chain3

$ mlog check /tmp/redteam-967/pocs/p19_pattern_chain3.mlog

```
1 error:
  1: строка 4: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p19_pattern_chain3.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p20_if_arm_smuggle

$ mlog check /tmp/redteam-967/pocs/p20_if_arm_smuggle.mlog

```
1 error:
  1: строка 7: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p20_if_arm_smuggle.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p21_untrusted_annot_over_trusted_edge

$ mlog check /tmp/redteam-967/pocs/p21_untrusted_annot_over_trusted_edge.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p21_untrusted_annot_over_trusted_edge.mlog

```
doc
[exit: 0]
```

## p22_consented_word_untrusted_edge

$ mlog check /tmp/redteam-967/pocs/p22_consented_word_untrusted_edge.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<consented, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p22_consented_word_untrusted_edge.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<consented, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p23_poisoned_word_untrusted_edge

$ mlog check /tmp/redteam-967/pocs/p23_poisoned_word_untrusted_edge.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<poisoned, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p23_poisoned_word_untrusted_edge.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<poisoned, untrusted>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p26_bogus_annotation

$ mlog check /tmp/redteam-967/pocs/p26_bogus_annotation.mlog

```
1 error:
  1: строка 5: label annotation '<bogus>' on the output terminator of flow 'Main': unknown label word 'bogus'
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p26_bogus_annotation.mlog

```
error: Compilation error (Naryad #523): semantic findings block execution:
label annotation '<bogus>' on the output terminator of flow 'Main': unknown label word 'bogus' (line 5)
[exit: 1]
```

## p27_single_integrity_word

$ mlog check /tmp/redteam-967/pocs/p27_single_integrity_word.mlog

```
1 error:
  1: строка 5: label annotation '<untrusted>' on the output terminator of flow 'Main': label annotation has no confidentiality word (public|consented|private|poisoned)
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p27_single_integrity_word.mlog

```
error: Compilation error (Naryad #523): semantic findings block execution:
label annotation '<untrusted>' on the output terminator of flow 'Main': label annotation has no confidentiality word (public|consented|private|poisoned) (line 5)
[exit: 1]
```

## p28_single_conf_word

$ mlog check /tmp/redteam-967/pocs/p28_single_conf_word.mlog

```
1 error:
  1: строка 4: [LABEL_ANNOTATION_MISMATCH] label annotation '<public>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p28_single_conf_word.mlog

```
error: Category A security invariant violated:
[LABEL_ANNOTATION_MISMATCH] label annotation '<public>' on the output terminator of flow Main claims a weaker label than the computed edge 'public, untrusted' — the annotation cannot silence what the program computes (§12.2, №967); declare `-> output<public, untrusted>` (the computed one) if the display is intended, or drop the annotation
[exit: 1]
```

## p29_unicode_lookalike

$ mlog check /tmp/redteam-967/pocs/p29_unicode_lookalike.mlog

```
error: parse error:  --> 5:44
  |
5 |     input: String = "doc" -> Ask -> output<рublic, untrusted>
  |                                            ^---
  |
  = expected label_part
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p29_unicode_lookalike.mlog

```
error: parse error:  --> 5:44
  |
5 |     input: String = "doc" -> Ask -> output<рublic, untrusted>
  |                                            ^---
  |
  = expected label_part
[exit: 1]
```

## p30_double_annotation

$ mlog check /tmp/redteam-967/pocs/p30_double_annotation.mlog

```
error: parse error:  --> 5:62
  |
5 |     input: String = "doc" -> Ask -> output<public, untrusted><public, untrusted>
  |                                                              ^---
  |
  = expected IDENT or RBRACE
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p30_double_annotation.mlog

```
error: parse error:  --> 5:62
  |
5 |     input: String = "doc" -> Ask -> output<public, untrusted><public, untrusted>
  |                                                              ^---
  |
  = expected IDENT or RBRACE
[exit: 1]
```

## p31_step_annotation_rejected

$ mlog check /tmp/redteam-967/pocs/p31_step_annotation_rejected.mlog

```
error: parse error:  --> 5:33
  |
5 |     input: String = "doc" -> Ask<public, trusted> -> output
  |                                 ^---
  |
  = expected ARROW
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p31_step_annotation_rejected.mlog

```
error: parse error:  --> 5:33
  |
5 |     input: String = "doc" -> Ask<public, trusted> -> output
  |                                 ^---
  |
  = expected ARROW
[exit: 1]
```

## p32_annotation_whitespace_ok

$ mlog check /tmp/redteam-967/pocs/p32_annotation_whitespace_ok.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p32_annotation_whitespace_ok.mlog

```
error: call_llm() failed: LLM is not configured — `call_llm` refuses (№898: no LLM env var and no `llm {}` declaration is set, and the silent mock default is rolled back). Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }` block in the program — or opt in EXPLICITLY to the deterministic offline mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`).
[exit: 1]
```

## p33_consent_third_part_ok

$ mlog check /tmp/redteam-967/pocs/p33_consent_third_part_ok.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p33_consent_third_part_ok.mlog

```
error: call_llm() failed: LLM is not configured — `call_llm` refuses (№898: no LLM env var and no `llm {}` declaration is set, and the silent mock default is rolled back). Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }` block in the program — or opt in EXPLICITLY to the deterministic offline mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`).
[exit: 1]
```

## p34_annotation_wrong_flow

$ mlog check /tmp/redteam-967/pocs/p34_annotation_wrong_flow.mlog

```
1 error:
  1: строка 3: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Dirty carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p34_annotation_wrong_flow.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Dirty carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p35_duplicate_flow_names

$ mlog check /tmp/redteam-967/pocs/p35_duplicate_flow_names.mlog

```
2 errors:
  1: строка 6: duplicate flow: Main
  2: строка 6: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p35_duplicate_flow_names.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p36_llm_input_binding_unannotated

$ mlog check /tmp/redteam-967/pocs/p36_llm_input_binding_unannotated.mlog

```
1 error:
  1: строка 1: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p36_llm_input_binding_unannotated.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p37_respond_step_env_input

$ mlog check /tmp/redteam-967/pocs/p37_respond_step_env_input.mlog

```
1 error:
  1: строка 1: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p37_respond_step_env_input.mlog

```
error: Category A security invariant violated:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p41_branch_empty_parses

$ mlog check /tmp/redteam-967/pocs/p41_branch_empty_parses.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p41_branch_empty_parses.mlog

```
doc
[exit: 0]
```

## p42_branch_pipeline_body_rejected

$ mlog check /tmp/redteam-967/pocs/p42_branch_pipeline_body_rejected.mlog

```
error: parse error:  --> 4:14
  |
4 |     branch { input: String = "doc" -> S -> output }
  |              ^---
  |
  = expected branch or RBRACE
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p42_branch_pipeline_body_rejected.mlog

```
error: parse error:  --> 4:14
  |
4 |     branch { input: String = "doc" -> S -> output }
  |              ^---
  |
  = expected branch or RBRACE
[exit: 1]
```

## p43_route_decl_hunt

$ mlog check /tmp/redteam-967/pocs/p43_route_decl_hunt.mlog

```
error: parse error:  --> 5:5
  |
5 |     route "/" { }
  |     ^---
  |
  = expected mlogserver_port, mlogserver_host, mlogserver_middleware, mlogserver_rate_limit, mlogserver_redact_mode, route_decl, or RBRACE
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p43_route_decl_hunt.mlog

```
error: parse error:  --> 5:5
  |
5 |     route "/" { }
  |     ^---
  |
  = expected mlogserver_port, mlogserver_host, mlogserver_middleware, mlogserver_rate_limit, mlogserver_redact_mode, route_decl, or RBRACE
[exit: 1]
```

## p46_respond_env

$ mlog check /tmp/redteam-967/pocs/p46_respond_env.mlog

```
3 errors:
  1: строка 1: [SECRET_LEAK] secret may be leaked — env() value passed to respond()
  2: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of respond in pattern P carries label 'private, trusted'; sinks require public — respond is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress
  3: строка 5: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p46_respond_env.mlog

```
error: Category A security invariant violated:
[SECRET_LEAK] secret may be leaked — env() value passed to respond()
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of respond in pattern P carries label 'private, trusted'; sinks require public — respond is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p47_httppost_env

$ mlog check /tmp/redteam-967/pocs/p47_httppost_env.mlog

```
3 errors:
  1: строка 1: [SECRET_LEAK] secret may be leaked — env() value passed as http_post body
  2: [PII_EGRESS_NETWORK] sink clearance violated: argument 1 of http_post in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of http_post must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: private-egress
  3: строка 5: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p47_httppost_env.mlog

```
error: Category A security invariant violated:
[SECRET_LEAK] secret may be leaked — env() value passed as http_post body
[PII_EGRESS_NETWORK] sink clearance violated: argument 1 of http_post in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of http_post must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: private-egress
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p48_exec_env

$ mlog check /tmp/redteam-967/pocs/p48_exec_env.mlog

```
2 errors:
  1: [SECRET_TO_EXEC] sink clearance violated: argument 0 of exec in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: secret-exec
  2: строка 5: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p48_exec_env.mlog

```
error: Category A security invariant violated:
[SECRET_TO_EXEC] sink clearance violated: argument 0 of exec in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: secret-exec
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p49_gitpush_env

$ mlog check /tmp/redteam-967/pocs/p49_gitpush_env.mlog

```
2 errors:
  1: [SECRET_EGRESS_VCS] sink clearance violated: argument 0 of git_push in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of git_push must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: private-egress
  2: строка 5: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p49_gitpush_env.mlog

```
error: Category A security invariant violated:
[SECRET_EGRESS_VCS] sink clearance violated: argument 0 of git_push in pattern P carries label 'private, trusted'; sinks require public — bridge (№391): the decision/data argument of git_push must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: private-egress
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p50_respond_llm

$ mlog check /tmp/redteam-967/pocs/p50_respond_llm.mlog

```
4 errors:
  1: строка 1: [HTML_INJECTION] LLM output passed to respond() — use template/render for XSS safety
  2: строка 1: [TAINT_INTERP] LLM output reaches respond() through merged control-flow state (match arms / loop boundary / memory) — use render()/escape_html() for XSS safety (effects: read, network)
  3: [HTML_INJECTION] sink clearance violated: argument 0 of respond in pattern P carries label 'public, untrusted'; sinks require public — respond is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress
  4: строка 5: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p50_respond_llm.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] LLM output passed to respond() — use template/render for XSS safety
[TAINT_INTERP] LLM output reaches respond() through merged control-flow state (match arms / loop boundary / memory) — use render()/escape_html() for XSS safety (effects: read, network)
[HTML_INJECTION] sink clearance violated: argument 0 of respond in pattern P carries label 'public, untrusted'; sinks require public — respond is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p51_exec_llm

$ mlog check /tmp/redteam-967/pocs/p51_exec_llm.mlog

```
2 errors:
  1: [UNTRUSTED_EXEC_DECISION] sink clearance violated: argument 0 of exec in pattern P carries label 'public, untrusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: untrusted-exec
  2: строка 5: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p51_exec_llm.mlog

```
error: Category A security invariant violated:
[UNTRUSTED_EXEC_DECISION] sink clearance violated: argument 0 of exec in pattern P carries label 'public, untrusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: untrusted-exec
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p52_exec_concat_llm

$ mlog check /tmp/redteam-967/pocs/p52_exec_concat_llm.mlog

```
2 errors:
  1: [UNTRUSTED_EXEC_DECISION] sink clearance violated: argument 0 of exec in pattern P carries label 'public, untrusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: untrusted-exec
  2: строка 5: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p52_exec_concat_llm.mlog

```
error: Category A security invariant violated:
[UNTRUSTED_EXEC_DECISION] sink clearance violated: argument 0 of exec in pattern P carries label 'public, untrusted'; sinks require public — bridge (№391): the decision/data argument of exec must satisfy conf ⊑ public AND integrity ≥ trusted; failed threshold: untrusted-exec
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p53_quarantine_write_legal

$ mlog check /tmp/redteam-967/pocs/p53_quarantine_write_legal.mlog

```
[CONSENT][audit-event] poisoned value reaches the quarantine sink in pattern 'P' — legal egress with audit event (№335)
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p53_quarantine_write_legal.mlog

```
[CONSENT][audit-event] poisoned value reaches the quarantine sink in pattern 'P' — legal egress with audit event (№335)
[CONSENT][audit-event] poisoned value reaches the quarantine sink in pattern 'P' — legal egress with audit event (№335)
[CONSENT][audit-event] poisoned value reaches the quarantine sink in pattern 'P' — legal egress with audit event (№335)
[QUARANTINE_EGRESS] poisoned value reached the quarantine sink (legal egress with audit event, №335; reason: unspecified)
[exit: 0]
```

## p32b_annotation_newline_ok

$ mlog check /tmp/redteam-967/pocs/p32b_annotation_newline_ok.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p32b_annotation_newline_ok.mlog

```
error: call_llm() failed: LLM is not configured — `call_llm` refuses (№898: no LLM env var and no `llm {}` declaration is set, and the silent mock default is rolled back). Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }` block in the program — or opt in EXPLICITLY to the deterministic offline mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`).
[exit: 1]
```

## p24_output_named_pattern_midpipe

$ mlog check /tmp/redteam-967/pocs/p24_output_named_pattern_midpipe.mlog

```
error: parse error:  --> 4:37
  |
4 |     input: String = "doc" -> output -> Ask -> output
  |                                     ^---
  |
  = expected IDENT, LT, or RBRACE
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p24_output_named_pattern_midpipe.mlog

```
error: parse error:  --> 4:37
  |
4 |     input: String = "doc" -> output -> Ask -> output
  |                                     ^---
  |
  = expected IDENT, LT, or RBRACE
[exit: 1]
```

## p25_template_string_corrupts_parse

$ mlog check /tmp/redteam-967/pocs/p25_template_string_corrupts_parse.mlog

```
error: parse error:  --> 4:12
  |
4 | flow Main {__TEMPLATE_BODY_0__}
  |            ^---
  |
  = expected flow_pipeline
[exit: 1]
```

$ mlog run /tmp/redteam-967/pocs/p25_template_string_corrupts_parse.mlog

```
error: parse error:  --> 4:12
  |
4 | flow Main {__TEMPLATE_BODY_0__}
  |            ^---
  |
  = expected flow_pipeline
[exit: 1]
```

## p03_llm_annotated_ok (with METALOGOS_MOCK_LLM=1)

$ METALOGOS_MOCK_LLM=1 mlog run /tmp/redteam-967/pocs/p03_llm_annotated_ok.mlog

```
[MOCK: key | doc]
[exit: 0]
```

## p16_escape_html_llm_sanitized (with METALOGOS_MOCK_LLM=1)

$ METALOGOS_MOCK_LLM=1 mlog run /tmp/redteam-967/pocs/p16_escape_html_llm_sanitized.mlog

```
[MOCK: key | &lt;script&gt;alert(1)&lt;/script&gt;]
[exit: 0]
```

## p38_exec_step_stdout_display

$ mlog check /tmp/redteam-967/pocs/p38_exec_step_stdout_display.mlog

```
OK: no issues found.
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p38_exec_step_stdout_display.mlog

```
error: [EXEC_NOT_PERMITTED] exec() is disabled by default. Set METALOGOS_ALLOW_EXEC=1 to enable — this applies to mlog run, check, and serve top-level alike. For serve route bodies set METALOGOS_SERVE_ALLOW_EXEC=1 instead (Naryad #253).
[exit: 1]
```

$ METALOGOS_ALLOW_EXEC=1 mlog run /tmp/redteam-967/pocs/p38_exec_step_stdout_display.mlog

```
SECRET_MARKER_XYZ

[exit: 0]
```

## p39_import_boundary (pocs/import_probe/main.mlog + leaky.mlog)

$ mlog check /tmp/redteam-967/pocs/import_probe/main.mlog  (cwd = import_probe)

```
1 error:
  1: строка 2: flow 'Main': pipeline step 'Leaky' is not a known pattern, builtin, or branch definition
[exit: 1]
```

$ mlog check --root main.mlog main.mlog  (cwd = import_probe)

```
1 error:
  1: строка 2: [PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run main.mlog  (cwd = import_probe)

```
error: Compilation error (Naryad #523): semantic findings block execution:
[PII_EGRESS_OUTPUT] sink clearance violated: argument 0 of print in flow Main carries label 'private, trusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress — the flow output carries 'private, trusted'; declare `-> output<private, trusted>` if the display is intended (ADR-0154 §13.4 form (а)) (line 2)
[exit: 1]
```

## p40_import_shadowing (pocs/import_probe/shadow_main.mlog)

$ mlog check shadow_main.mlog  (cwd = import_probe)

```
OK: no issues found.
[exit: 0]
```

$ mlog run shadow_main.mlog  (cwd = import_probe)

```
clean doc
[exit: 0]
```

## p44_compile_refusal_source

$ mlog compile /tmp/redteam-967/pocs/p44_compile_refusal_source.mlog

```
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

## p45_clean_ann (bytecode execution)

$ mlog compile /tmp/redteam-967/pocs/p45_clean_ann.mlog

```
Compiled /tmp/redteam-967/pocs/p45_clean_ann.mlog -> /tmp/redteam-967/pocs/p45_clean_ann.mbc (317 bytes)
[exit: 0]
```

$ mlog run /tmp/redteam-967/pocs/p45_clean_ann.mbc

```
error: call_llm() failed: LLM is not configured — `call_llm` refuses (№898: no LLM env var and no `llm {}` declaration is set, and the silent mock default is rolled back). Configure ONE of: `METALOGOS_LLM_PROVIDER` (+ `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`, an `llm { providers: [...] }` block in the program — or opt in EXPLICITLY to the deterministic offline mock with `METALOGOS_MOCK_LLM=1` (or the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`).
[exit: 1]
```

$ METALOGOS_MOCK_LLM=1 mlog run /tmp/redteam-967/pocs/p45_clean_ann.mbc

```
[MOCK: key | doc]
[exit: 0]
```

## p02_llm_unannotated (oracle backend)

$ mlog check --reference /tmp/redteam-967/pocs/p02_llm_unannotated.mlog

```
1 error:
  1: строка 4: [HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```

$ mlog run --reference /tmp/redteam-967/pocs/p02_llm_unannotated.mlog

```
mlog: reference backend (TW) — for comparison only (transitional per ADR-0197; the VM is the executor)
error: Category A security invariant violated:
[HTML_INJECTION] sink clearance violated: argument 0 of print in flow Main carries label 'public, untrusted'; sinks require public — print is classified sink/public/irreversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: untrusted-egress — the flow output carries 'public, untrusted'; declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))
[exit: 1]
```
