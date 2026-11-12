#!/usr/bin/env bash
# ── Naryad №953 (issue #1920, the audit 12.11 — P0) — the mutation
#    verification for the static `check --root` and the gated `mlog test`
#    / `mlog resume` ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the C-2 execution returns: `check_program_with_root` executes
#        the root again (`interp.run`) "to resolve imports"
#        -> n953_check_root_does_not_execute_the_root MUST FAIL
#        (the side-effect file reappears).
#
#   M2 — the H-1 gate is dropped: `test_program_with_dir` skips
#        `enforce_run_gates`
#        -> n953_test_program_refuses_the_leak_before_execution MUST FAIL
#        (the leak program runs and the "Category A" refusal is gone).
#
#   M3 — the resume gate is dropped: `resume_program_with_dir` skips
#        `enforce_run_gates`
#        -> n953_resume_refuses_the_leak_before_execution MUST FAIL.
#
# Run from the repo root:  bash scripts/mutation_verify_953.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

# The restore is a BACKUP-COPY restore, never `git checkout --`: the
# working tree carries UNCOMMITTED naryad work — a checkout would wipe it
# (the lesson of the first M-run).
BACKUP=/tmp/n953_lib_backup.rs
cp src/lib.rs "$BACKUP" || exit 1
fail() { cp "$BACKUP" src/lib.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BACKUP" src/lib.rs; }

echo "── M1: the C-2 execution returns in check_program_with_root ──"
python3 - <<'EOF'
p = 'src/lib.rs'
s = open(p).read()
needle = """            let (module_decls, _module_groups) =
                semantic::resolve_import_groups(&parsed_root, &root_dir)
                    .map_err(|e| format!("import resolution (№953 static): {}", e))?;
            let mut merged = module_decls;"""
replacement = """            let (module_decls, _module_groups) =
                semantic::resolve_import_groups(&parsed_root, &root_dir)
                    .map_err(|e| format!("import resolution (№953 static): {}", e))?;
            // MUTATION (M1): the C-2 execution returns — the root runs
            let mut interp = interpreter::Interpreter::new();
            interp.set_base_dir(root_dir.clone());
            let _ = interp.run(parsed_root.clone());
            let mut merged = module_decls;"""
assert needle in s, "the static-resolution block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_953_check_root_static n953_check_root_does_not_execute_the_root 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the pin works" || fail "M1: the no-execution pin stayed GREEN under the execution mutation"
restore src/lib.rs

echo "── M2: the mlog test gate is dropped ──"
python3 - <<'EOF'
p = 'src/lib.rs'
s = open(p).read()
needle = """    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    enforce_run_gates(&declarations, &base_dir, source)?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    interp.run(declarations)?;
    Ok(interp.run_test_blocks())
}

/// №953: the explicit `--allow-unchecked` form"""
replacement = """    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    interp.run(declarations)?;
    Ok(interp.run_test_blocks())
}

/// №953: the explicit `--allow-unchecked` form"""
assert needle in s, "the test gate block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_953_check_root_static n953_test_program_refuses_the_leak_before_execution 2>&1 | grep -q "test result: FAILED" \
  && echo "M2 RED — the pin works" || fail "M2: the test-gate pin stayed GREEN under the gate-drop mutation"
restore src/lib.rs

echo "── M3: the resume gate is dropped ──"
python3 - <<'EOF'
p = 'src/lib.rs'
s = open(p).read()
needle = """    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    // №953: the run gates BEFORE the checkpoint machinery — resume
    // executes code, the gate set is the same `mlog run` applies.
    enforce_run_gates(&declarations, &base_dir, source)?;"""
replacement = """    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;"""
assert needle in s, "the resume gate block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_953_check_root_static n953_resume_refuses_the_leak_before_execution 2>&1 | grep -q "test result: FAILED" \
  && echo "M3 RED — the pin works" || fail "M3: the resume-gate pin stayed GREEN under the gate-drop mutation"
restore src/lib.rs

echo "── final: the clean tree is GREEN ──"
CARGO_INCREMENTAL=0 cargo test --test naryad_953_check_root_static 2>&1 | grep -q "test result: ok" \
  && echo "MUTATION VERIFY: OK — 3 mutations red, the clean tree green" || fail "the clean tree is not green after the restores"
