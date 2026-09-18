## Naryad

<!-- Link to the naryad issue: Closes #NNN -->
<!-- If the PR is not tied to a specific naryad (e.g. dependabot) — leave empty. -->

## What was done

<!-- Briefly: what changed and why. Fact base — per the code, not per the documentation. -->

## Fact base (verified)

<!-- file:line, reproducible scenario — the same standard as in the naryad issue (AGENTS.md §1). -->

## Contracts

<!-- Tests/contracts proving the naryad is done — concrete, not "should work". -->

## Pre-review checklist

- [ ] The branch is not more than 20 commits behind `main` (the `branch-freshness` job checks
      this automatically — if it is behind, first run `git fetch origin main && git rebase origin/main`)
- [ ] All 14 blocking jobs are actually green on the merge commit of this PR, not only on the
      branch head before the merge (`test-lib`, `crosscheck`, `candle-tests`, `vision-tests`,
      `registry-arity-check`, `test-llm-cache-contract`, `minimal-build`, `test-integration`,
      `fmt`, `clippy`, `ADR numbering`, `module-size-guard`, `vscode-extension`, `cargo-audit`)
- [ ] If the PR touches security-sensitive code — `mlog audit` has been run on the changed
      examples
- [ ] If the naryad contains an owner decision point (AGENTS.md §3) — the decision is explicitly
      documented in the naryad issue, not assumed

## Security Considerations
<!-- Only if applicable -->
- [ ] The change does not introduce new vulnerabilities
- [ ] Opaque types and security invariants are preserved
