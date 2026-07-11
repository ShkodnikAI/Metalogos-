// Problem A integration test: tiered skill index
// Tests skill_index parsing, resolve_skill_index(), trigger matching via matches_any().
// No .expected file — these are interpreter-only tests (same as problem_c_schema.rs).

#[cfg(test)]
mod tests {
    use metalogos::run_program;

    #[test]
    fn skill_index_basic_loading() {
        let source = r#"
skill_index test_dept {
  tier 1 always [
    "core_skill_a", "core_skill_b"
  ]
  tier 2 when_matches [
    { skill: "market_skill", triggers: ["рынок", "актив", "товар"] }
  ]
  budget: 25000 tokens
  truncation: whole_skill_only
}

pattern GetTier1(dept: String) -> String {
  let idx = resolve_skill_index(dept)
  let t1 = idx.tier1
  return get(t1, 0)
}

flow Main {
  -> GetTier1("test_dept") -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        assert_eq!(result.unwrap_or_default().trim(), "core_skill_a");
    }

    #[test]
    fn skill_index_trigger_matching() {
        let source = r#"
skill_index test_dept {
  tier 1 always [
    "deconstruct"
  ]
  tier 2 when_matches [
    { skill: "market_analysis", triggers: ["рынок", "актив", "валют"] },
    { skill: "leader_analysis", triggers: ["персона", "лидер"] }
  ]
}

pattern MatchSkills(dept: String, query: String) -> String {
  let idx = resolve_skill_index(dept)
  let result = "deconstruct"
  each rule in idx.tier2 {
    if matches_any(query, rule.triggers) {
      let result = result + "+" + rule.skill
    }
  }
  return result
}

flow Main {
  input: String = "проанализируй рынок и валюту" -> MatchSkills("test_dept") -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        assert_eq!(result.unwrap_or_default().trim(), "deconstruct+market_analysis");
    }

    #[test]
    fn skill_index_budget_and_truncation() {
        let source = r#"
skill_index osp {
  tier 1 always [
    "deconstruct", "awareness-frame"
  ]
  budget: 15000 tokens
  truncation: whole_skill_only
}

pattern GetBudget(dept: String) -> String {
  let idx = resolve_skill_index(dept)
  return to_string(idx.budget) + ":" + idx.truncation
}

flow Main {
  -> GetBudget("osp") -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        assert_eq!(result.unwrap_or_default().trim(), "15000:whole_skill_only");
    }

    #[test]
    fn skill_index_unknown_dept_errors() {
        let source = r#"
pattern TryResolve(dept: String) -> String {
  let idx = resolve_skill_index(dept)
  return "ok"
}

flow Main {
  -> TryResolve("nonexistent") -> output
}
"#;
        let result = run_program(source.to_string());
        assert!(result.is_err(), "expected error for unknown skill_index department");
        let err = result.unwrap_err();
        assert!(err.contains("no skill_index declared"), "error should mention missing declaration: {}", err);
    }

    #[test]
    fn skill_index_three_tiers() {
        let source = r#"
skill_index full_dept {
  tier 1 always [
    "core"
  ]
  tier 2 when_matches [
    { skill: "t2_skill", triggers: ["рынок"] }
  ]
  tier 3 when_matches [
    { skill: "t3_skill", triggers: ["контр-анализ", "redteam"] }
  ]
}

pattern CheckTier3(dept: String) -> String {
  let idx = resolve_skill_index(dept)
  let t3 = idx.tier3
  let rule = get(t3, 0)
  return rule.skill
}

flow Main {
  -> CheckTier3("full_dept") -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        assert_eq!(result.unwrap_or_default().trim(), "t3_skill");
    }
}