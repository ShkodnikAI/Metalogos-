#!/usr/bin/env python3
"""Naryad №551 (Wave 25 P0; the audit 02.10 M-1 + §6.2): the weekly
merge↔CI audit — every merge commit of `main` is checked against a
GREEN run of the required checks on its PR's head SHA.

THE CLASS IT CLOSES: on 2026-09-30 14:30–19:37 UTC the GitHub Actions
event delivery died; six PRs (№527–№531, №535 — gh#836–gh#840, gh#844)
merged with NO checks, and two defects shipped to main unseen (the
hotfix gh#865: "the two shipped-unseen CI defects the dead event
delivery hid"). Branch protection (the owner's admin checklist —
docs/maintainers.md "When CI is down") makes a check-less merge
impossible; THIS script is the weekly detective control that catches
whatever slips past both (a protection mis-config, a bypass, a fresh
failure class).

RULE: a merge commit of main whose PR head SHA lacks a `success`
check-run for ANY required check is a DIVERGENCE. A `skipped` check is
not green (a required check that can silently skip protects nothing).
The script reports; it never merges, reverts, or edits history.

A 404 on the trailing "(#N)" ref is a DETERMINATE answer (the ref is
not a PR — the squash-merge message convention violated; №813, issue
#1578: №802/№803 carried the ISSUE number in the tail and the audit
died exit 2 without a single verdict). The audit resolves the merge
through the commit-association lane and FILES the break as its own
divergence line — a convention break never heals silently; what stays
fail-closed (exit 2) is the TRANSIENT lane only (5xx, throttling,
network) — an unreadable state is not a pass (№525).

Divergence exit code: 0 = clean, 1 = divergences found (the scheduled
workflow run goes red — visible; a convention break rides here too),
2 = the audit could not run (a TRANSIENT API failure — fail-closed,
a silent auditor is a liar auditor).

Modes:
  python3 scripts/ci/merge_ci_audit.py                  # report only
  python3 scripts/ci/merge_ci_audit.py --days 14        # wider window
  python3 scripts/ci/merge_ci_audit.py --open-issue     # file the issue
  python3 scripts/ci/merge_ci_audit.py --self-test      # embedded fixtures
  python3 scripts/ci/merge_ci_audit.py --tripwire-check # offline: every
        # required context has a PR-producing workflow job (the №809 class)
"""

import json
import os
import re
import sys
import urllib.request

DEFAULT_DAYS = 7

# The reverse direction (№824): the LIVE protection read needs the
# Administration:read token — the BRANCH_PROTECTION_TOKEN repo secret
# (the №586 precedent; GITHUB_TOKEN cannot read the protection and a
# 403 there must never pass for "no protection"). An unreadable
# protection is exit 2 — an unreadable state is not a pass (№525).
BP_TOKEN_ENV = "BP_TOKEN"

# The required set (the branch-protection checklist of №551 — the job
# DISPLAY names as they appear in the check-runs API; fact-checked
# against .github/workflows/ci.yml on the audit revision 6e66d3d;
# `msrv (blocking)` added by №555, issue #916; `release-gap (blocking)` by №562, issue #923).
# №824 (issue #1592, wave 68): the set is now the LIVE branch protection
# verbatim — the 2026-10-28 admin read returned 23 contexts (the owner's
# №1593 toggle dropped the four pillar contexts from the rule; the
# checklist in docs/maintainers.md was synced in the same PR). The
# ten contexts the №551/№586 lane never carried are the audit's former
# blind spot: protection enforced them, the detective verified only the
# original thirteen (the M-1 class on the rest).
REQUIRED_CHECKS = [
    "test-lib (blocking)",
    "test-integration (blocking)",
    "crosscheck (blocking)",
    "clippy (blocking)",
    "fmt (blocking)",
    "cargo-audit (blocking)",
    "cargo-deny (blocking)",
    "gitleaks (blocking)",
    "gate-facts-sync (blocking)",
    "blocking-checks-sync (blocking)",
    "registry-arity-check (blocking)",
    "msrv (blocking)",
    "release-gap (blocking)",
    "ADR numbering (blocking)",
    "branch-freshness (blocking)",
    "doc-tests (blocking)",
    "html-tests (blocking)",
    "ledger-golden (blocking)",
    "minimal-build (blocking)",
    "module-size-guard (blocking)",
    "test-llm-cache-contract (blocking)",
    "vscode-extension (blocking)",
    "xml-tests (blocking)",
]

MERGE_PR_RE = re.compile(r"\(#(\d+)\)\s*$")


class ProtectionUnreadable(Exception):
    """The live-protection read failed — fail-closed (№525), exit 2."""


# ── pure logic (fixture-tested by --self-test and the Rust driver) ──

def protection_drift(protection_contexts, required):
    """The reverse direction (№824): protection − REQUIRED_CHECKS.

    Every context the protection ENFORCES must have a row in the
    audit's required set — otherwise protection holds a check the
    detective never verifies (the M-1 blind class on that name). The
    other direction (a list row not on the live rule) is the
    branch-protection-audit's RED lane (docs/maintainers.md sync), not
    duplicated here.
    """
    required_set = set(required)
    return sorted(c for c in protection_contexts if c not in required_set)


def scan_workflow_producers(workflows_dir):
    """The №809-class tripwire scan: which check-run display names are
    produced by a job of a workflow that runs on pull_request.

    Returns {display_name: [workflow_file, ...]}. The scanner is a
    stdlib subset-parser BY DESIGN (no PyYAML on the audit lane): the
    `on:` block must carry a `pull_request` key (`pull_request_target`
    does NOT count — it runs on the base ref, not the PR head); job
    display names are the 4-space `name:` values under `jobs:` (the
    repo convention, 47 occurrences in ci.yml), falling back to the
    job id when `name:` is absent. LIMITATION (honest): a path- or
    types-filtered `pull_request` trigger counts as producing — the
    tripwire closes the "no PR producer at all" class (the №809
    episode); filter semantics stay a human read.
    """
    from pathlib import Path
    producers = {}
    d = Path(workflows_dir)
    if not d.is_dir():
        return producers
    files = sorted(list(d.glob("*.yml")) + list(d.glob("*.yaml")))
    for path in files:
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        on_m = re.search(r"(?m)^on:\s*$", text)
        if not on_m:
            continue
        rest = text[on_m.end():]
        top = re.search(r"(?m)^[A-Za-z_][A-Za-z0-9_-]*\s*:", rest)
        on_block = rest[:top.start()] if top else rest
        # `pull_request:` — but not `pull_request_target:` (base-ref runs).
        if not (re.search(r"^\s+pull_request\s*:\s*$", on_block, re.M)
                or re.search(r"^\s+pull_request\s*:\s*\S", on_block, re.M)):
            continue
        jobs_m = re.search(r"(?m)^jobs:\s*$", text)
        if not jobs_m:
            continue
        body = text[jobs_m.end():]
        # job blocks: 2-space-indent ids; display name = the 4-space
        # `name:` inside the block, else the id itself.
        blocks = re.split(r"(?m)^(?=  [A-Za-z0-9_-]+:\s*$)", body)
        for block in blocks:
            id_m = re.match(r"  ([A-Za-z0-9_-]+):\s*$", block, re.M)
            if not id_m:
                continue
            name_m = re.search(r"(?m)^    name:\s*(.+?)\s*$", block)
            display = name_m.group(1).strip() if name_m else id_m.group(1)
            if "${{" in display:
                continue  # matrix-generated — not a static required name
            producers.setdefault(display, []).append(path.name)
    return producers


def tripwire_misses(required, producers):
    """Required contexts with NO pull_request-producing job (the №809
    class: a context required by branch protection that no PR workflow
    produces is never green on a PR head — every PR is unmergeable
    forever, silently until the first merge attempt)."""
    return sorted(c for c in required if c not in producers)


def parse_pr_number(commit_first_line):
    """The squash-merge convention: '... (#NNN)' — the trailing PR ref."""
    if not commit_first_line:
        return None
    m = MERGE_PR_RE.search(commit_first_line.strip())
    return int(m.group(1)) if m else None


def required_verdict(check_runs, required):
    """(ok, problems) — every required check needs a `success` run.

    A check absent entirely, or present but skipped/failed/pending, is
    a problem, named explicitly (fail-closed: silence is not green).
    """
    by_name = {}
    for run in check_runs or []:
        by_name.setdefault(run.get("name"), []).append(run.get("conclusion"))
    problems = []
    for name in required:
        conclusions = by_name.get(name)
        if not conclusions:
            problems.append(f"{name}: NO RUN")
        elif "success" not in conclusions:
            problems.append(f"{name}: {sorted(set(conclusions))}")
    return (not problems), problems


def resolve_associated_pr(assoc):
    """The determinate-404 lane (№813): exactly ONE merged PR
    association → its number; zero, several, or none merged → None
    (unresolvable — a divergence, never a silent pass)."""
    merged = [p.get("number") for p in (assoc or []) if p.get("merged_at")]
    return merged[0] if len(merged) == 1 else None


def decide(commit, pr_map):
    """The per-commit verdict — `pr_map` carries the API answers.

    commit: {"first_line": str, "sha": str}
    pr_map: {pr_number: {"head_sha": str, "check_runs": [...]}} — a
    missing key means the PR fetch failed (fail-closed: unknown is a
    divergence, never a pass).
    """
    pr = parse_pr_number(commit.get("first_line", ""))
    if pr is None:
        return (False, ["not a squash-merge of a PR (no trailing '(#N)')"])
    if pr not in pr_map:
        return (False, [f"PR #{pr}: the head data is UNAVAILABLE (fail-closed)"])
    head_sha = pr_map[pr].get("head_sha")
    if not head_sha:
        return (False, [f"PR #{pr}: no head SHA recorded (fail-closed)"])
    ok, problems = required_verdict(pr_map[pr].get("check_runs"), REQUIRED_CHECKS)
    if not ok:
        return (False, [f"PR #{pr} (head {head_sha[:7]}): {p}" for p in problems])
    return (True, [])


# ── the GitHub API lane (network; the scheduled-workflow reality) ──

def api_get(url, token):
    req = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    })
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())


def all_check_runs(repo, sha, token):
    runs, page = [], 1
    while True:
        data = api_get(
            f"https://api.github.com/repos/{repo}/commits/{sha}/check-runs"
            f"?per_page=100&page={page}", token)
        runs.extend(data.get("check_runs", []))
        if len(data.get("check_runs", [])) < 100:
            return runs
        page += 1


def read_protection_contexts(repo, token, branch="main"):
    """The live required-status contexts (№824, the reverse direction).

    Administration:read token (BP_TOKEN — the BRANCH_PROTECTION_TOKEN
    repo secret, the №586 precedent). Any failure raises
    ProtectionUnreadable → exit 2 (fail-closed, №525): a 404 is NOT
    "protection off" here — the checklist state is the enforced rule,
    and an unreadable rule is an unreadable state, not a pass.
    """
    try:
        data = api_get(
            f"https://api.github.com/repos/{repo}/branches/{branch}/protection",
            token)
    except Exception as e:
        raise ProtectionUnreadable(str(e))
    ctx = (data.get("required_status_checks") or {}).get("contexts") or []
    if not ctx and not (data.get("required_status_checks") or {}):
        raise ProtectionUnreadable("the protection carries no required_status_checks")
    return sorted(ctx)


def self_test():
    green_runs = [{"name": n, "conclusion": "success"} for n in REQUIRED_CHECKS]
    skipped_runs = [{"name": n, "conclusion": "success"} for n in REQUIRED_CHECKS]
    skipped_runs[0]["conclusion"] = "skipped"  # test-lib silently skipped
    failed_runs = [{"name": n, "conclusion": "success"} for n in REQUIRED_CHECKS]
    failed_runs[1]["conclusion"] = "failure"  # clippy failed
    cases = [
        # (commit, pr_map, expected_ok, expected_fragment)
        ({"first_line": "fix: something (#892)", "sha": "a" * 7},
         {892: {"head_sha": "b" * 40,
                "check_runs": green_runs + [
                    {"name": "fmt (blocking)", "conclusion": "success"}]}},
         True, None),
        ({"first_line": "hotfix: urgent (#865)", "sha": "c" * 7},
         {865: {"head_sha": "d" * 40, "check_runs": []}},
         False, "NO RUN"),
        ({"first_line": "hotfix: urgent (#865)", "sha": "c" * 7},
         {865: {"head_sha": "d" * 40, "check_runs": skipped_runs}},
         False, "['skipped']"),
        ({"first_line": "hotfix: urgent (#865)", "sha": "c" * 7},
         {865: {"head_sha": "d" * 40, "check_runs": failed_runs}},
         False, "['failure']"),
        ({"first_line": "docs: direct push, no PR", "sha": "e" * 7},
         {}, False, "not a squash-merge"),
        ({"first_line": "fix: ghost (#999999)", "sha": "f" * 7},
         {}, False, "UNAVAILABLE"),
    ]
    failures = 0
    for i, (commit, pr_map, want_ok, fragment) in enumerate(cases):
        ok, problems = decide(commit, pr_map)
        if ok != want_ok or (fragment and not any(
                fragment in p for p in problems)):
            print(f"SELF-TEST FAIL case {i}: ok={ok} problems={problems}")
            failures += 1
    # №824: the association resolver — exactly one merged PR resolves,
    # everything else (none, several, none-merged) stays unresolvable.
    assoc_cases = [
        ([{"number": 1557, "merged_at": "2026-10-24T00:00:00Z"}], 1557),
        ([{"number": 1557, "merged_at": None}], None),
        ([], None),
        ([{"number": 1, "merged_at": "x"}, {"number": 2, "merged_at": "y"}], None),
    ]
    for i, (assoc, want) in enumerate(assoc_cases):
        got = resolve_associated_pr(assoc)
        if got != want:
            print(f"SELF-TEST FAIL assoc case {i}: got={got} want={want}")
            failures += 1
    # №824: the reverse direction — a protection context without a
    # required-set row is the M-1 blind class; a covered context is not.
    drift_cases = [
        (["a (blocking)", "b (blocking)"], ["a (blocking)"], ["b (blocking)"]),
        (["a (blocking)"], ["a (blocking)"], []),
        ([], [], []),
        (["x (blocking)", "a (blocking)"], ["a (blocking)", "x (blocking)", "q"], []),
    ]
    for i, (prot, req, want) in enumerate(drift_cases):
        got = protection_drift(prot, req)
        if got != want:
            print(f"SELF-TEST FAIL drift case {i}: got={got} want={want}")
            failures += 1
    # №824: the tripwire scanner — a PR workflow produces its job names,
    # a push-only workflow does not, pull_request_target does not count.
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        wf = os.path.join(td, "w.yml")
        open(wf, "w").write(
            "name: wf\n"
            "on:\n"
            "  push:\n"
            "    branches: [main]\n"
            "  pull_request:\n"
            "jobs:\n"
            "  one:\n"
            "    name: produced (blocking)\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo hi\n"
            "  bare:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo hi\n"
        )
        wf2 = os.path.join(td, "p.yml")
        open(wf2, "w").write(
            "name: push-only\n"
            "on:\n"
            "  push:\n"
            "    branches: [main]\n"
            "jobs:\n"
            "  two:\n"
            "    name: push-only (blocking)\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo hi\n"
        )
        wf3 = os.path.join(td, "t.yml")
        open(wf3, "w").write(
            "name: target-lane\n"
            "on:\n"
            "  pull_request_target:\n"
            "jobs:\n"
            "  three:\n"
            "    name: target-lane (blocking)\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo hi\n"
        )
        prods = scan_workflow_producers(td)
        if "produced (blocking)" not in prods:
            print(f"SELF-TEST FAIL tripwire: PR-producer not found: {sorted(prods)}")
            failures += 1
        if "bare" not in prods:
            print(f"SELF-TEST FAIL tripwire: the job-id fallback missing")
            failures += 1
        for absent in ("push-only (blocking)", "target-lane (blocking)"):
            if absent in prods:
                print(f"SELF-TEST FAIL tripwire: '{absent}' must not be a PR producer")
                failures += 1
        misses = tripwire_misses(
            ["produced (blocking)", "push-only (blocking)", "target-lane (blocking)",
             "bare"], prods)
        if misses != ["push-only (blocking)", "target-lane (blocking)"]:
            print(f"SELF-TEST FAIL tripwire: misses={misses}")
            failures += 1
    if failures:
        print(f"SELF-TEST FAILED: {failures} case(s)")
        return 1
    print(f"SELF-TEST OK: {len(cases) + len(assoc_cases) + len(drift_cases) + 6} "
          f"assertions "
          f"(green, no-runs, skipped, failed, no-PR, unavailable, "
          f"assoc-resolved, assoc-unmerged, assoc-empty, assoc-ambiguous, "
          f"drift-found, drift-clean, drift-empty, drift-both-covered, "
          f"tripwire-producer, tripwire-id-fallback, tripwire-push-only, "
          f"tripwire-pr-target, tripwire-misses, tripwire-bare-produced)")
    return 0


def tripwire_check_mode():
    """Offline mode (№824, the driver-test lane): scan the repo's
    .github/workflows and require a pull_request producer for EVERY
    REQUIRED_CHECKS context. Exit 0 clean / exit 1 — a context the
    protection would enforce but no PR workflow produces (the №809
    class) — the tripwire runs in CI on every test lane, so the class
    can never reintroduce silently."""
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    wf_dir = os.path.join(repo_root, ".github", "workflows")
    producers = scan_workflow_producers(wf_dir)
    if not producers:
        print(f"merge-ci-audit: no PR workflows found under {wf_dir} "
              "(fail-closed — the scanner found nothing to verify)", file=sys.stderr)
        return 2
    misses = tripwire_misses(REQUIRED_CHECKS, producers)
    if misses:
        print(f"TRIPWIRE: {len(misses)} required context(s) with no "
              f"pull_request producer (the №809 class — a PR can never "
              f"turn them green):")
        for c in misses:
            print(f"  - {c}")
        return 1
    print(f"TRIPWIRE OK: all {len(REQUIRED_CHECKS)} required contexts have a "
          f"pull_request-producing workflow job (scanned {len(producers)} "
          f"distinct display names)")
    return 0


def main():
    argv = sys.argv[1:]
    if "--self-test" in argv:
        sys.exit(self_test())
    if "--tripwire-check" in argv:
        sys.exit(tripwire_check_mode())

    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    repo = os.environ.get("GITHUB_REPOSITORY")
    if not repo and "--repo" in argv:
        repo = argv[argv.index("--repo") + 1]
    if not repo:
        try:
            remote = os.popen("git remote get-url origin").read().strip()
            m = re.search(r"github\.com[:/](.+?)(?:\.git)?$", remote)
            repo = m.group(1) if m else None
        except Exception:
            pass
    days = DEFAULT_DAYS
    if "--days" in argv:
        days = int(argv[argv.index("--days") + 1])
    open_issue = "--open-issue" in argv

    if not token or not repo:
        print("merge-ci-audit: GH_TOKEN / the repository are required "
              "(fail-closed: the audit does not silently skip)", file=sys.stderr)
        sys.exit(2)

    import datetime
    since = (datetime.datetime.now(datetime.timezone.utc)
             - datetime.timedelta(days=days)).isoformat()
    try:
        commits = api_get(
            f"https://api.github.com/repos/{repo}/commits"
            f"?sha=main&since={since}&per_page=100", token)
    except Exception as e:
        print(f"merge-ci-audit: the commit list is UNAVAILABLE: {e} "
              "(fail-closed — a silent auditor is a liar auditor)", file=sys.stderr)
        sys.exit(2)

    pr_numbers = {}
    for c in commits:
        first = (c.get("commit", {}).get("message") or "").splitlines()[0:1]
        first = first[0] if first else ""
        pr = parse_pr_number(first)
        if pr is not None:
            pr_numbers.setdefault(pr, c.get("sha"))

    pr_map = {}
    convention_breaks = []  # (sha7, trailing_ref, resolved_pr) — determinate
    unresolvable = []       # (sha7, trailing_ref) — the association lane found nothing
    try:
        for pr, commit_sha in pr_numbers.items():
            try:
                data = api_get(f"https://api.github.com/repos/{repo}/pulls/{pr}", token)
            except urllib.error.HTTPError as e:
                if e.code != 404:
                    raise  # transient/throttling — the fail-closed lane below
                # 404 is DETERMINATE: the trailing ref is not a PR — the
                # message convention violated (№813). Resolve through the
                # commit-association lane; the break itself stays VISIBLE.
                assoc = api_get(
                    f"https://api.github.com/repos/{repo}/commits/{commit_sha}/pulls",
                    token)
                real = resolve_associated_pr(assoc)
                if real is None:
                    unresolvable.append((commit_sha[:7], pr))
                    pr_map[pr] = {"head_sha": None, "check_runs": []}
                    continue
                convention_breaks.append((commit_sha[:7], pr, real))
                data = api_get(f"https://api.github.com/repos/{repo}/pulls/{real}", token)
            head_sha = data.get("head", {}).get("sha")
            runs = all_check_runs(repo, head_sha, token) if head_sha else []
            pr_map[pr] = {"head_sha": head_sha, "check_runs": runs}
    except Exception as e:
        print(f"merge-ci-audit: the PR/check-run data is UNAVAILABLE: {e} "
              "(fail-closed)", file=sys.stderr)
        sys.exit(2)

    divergences = []
    for sha7, trailing, real in convention_breaks:
        divergences.append(
            f"- `{sha7}`: the trailing ref `#{trailing}` is NOT a PR — the "
            f"message-convention violation (the squash-merge tail must carry "
            f"the PR number); resolved via the association lane to PR #{real}")
    for sha7, trailing in unresolvable:
        divergences.append(
            f"- `{sha7}`: the trailing ref `#{trailing}` is NOT a PR and the "
            f"association lane resolved no merged PR — the merge stays "
            f"unverified (fail-closed)")
    for c in commits:
        first = (c.get("commit", {}).get("message") or "").splitlines()[0:1]
        first = first[0] if first else ""
        ok, problems = decide({"first_line": first, "sha": c.get("sha", "")}, pr_map)
        if not ok:
            divergences.append(
                f"- `{c.get('sha', '')[:7]}` {first}\n  - " + "\n  - ".join(problems))

    # ── №824: the reverse direction — the LIVE protection vs the set ──
    bp_token = os.environ.get(BP_TOKEN_ENV)
    if not bp_token:
        print(f"merge-ci-audit: {BP_TOKEN_ENV} is required for the reverse "
              f"direction (the live-protection read; GITHUB_TOKEN cannot "
              f"read the protection) — fail-closed (№525)", file=sys.stderr)
        sys.exit(2)
    try:
        protection = read_protection_contexts(repo, bp_token)
    except ProtectionUnreadable as e:
        print(f"merge-ci-audit: the live protection is UNREADABLE: {e} "
              "(fail-closed — an unreadable state is not a pass)", file=sys.stderr)
        sys.exit(2)
    for c in protection_drift(protection, REQUIRED_CHECKS):
        divergences.append(
            f"- the LIVE protection enforces `{c}` which has NO row in "
            f"REQUIRED_CHECKS — the M-1 blind class (№824): the detective "
            f"never verifies this check on the merged heads")

    # ── №824: the tripwire — every enforced/audited context must have a
    # pull_request producer (the №809 class: a required context without
    # a PR job is never green on a PR head — unmergeable forever) ──
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    producers = scan_workflow_producers(os.path.join(repo_root, ".github", "workflows"))
    for c in tripwire_misses(sorted(set(REQUIRED_CHECKS) | set(protection)), producers):
        divergences.append(
            f"- the required context `{c}` has NO producing job in any "
            f"pull_request workflow — the №809 class (a PR can never turn "
            f"it green)")

    print(f"merge-ci-audit: {len(commits)} merge(s) on main in the last "
          f"{days} day(s); required checks: {len(REQUIRED_CHECKS)}; "
          f"live protection: {len(protection)} context(s)")
    if divergences:
        print(f"DIVERGENCES: {len(divergences)} (every line needs a human verdict)")
        print("NOTE: a check ABSENT from the head SHA's workflow set (a job "
              "born after the merge) is still flagged — it cannot be proven "
              "retroactively; close the audit issue with the verdict if the "
              "merge is documented (e.g., the 30.09 Actions-outage merges "
              "predating the №525/№535 jobs).")
        for d in divergences:
            print(d)
        if open_issue:
            body = ("The weekly merge↔CI audit (№551 + №824) found divergence(s)\n"
                    "— merges without a green run of the required checks (the\n"
                    "M-1 class), protection contexts the detective never\n"
                    "verifies (the reverse direction), or required contexts\n"
                    "with no pull_request producer (the №809 class):\n\n"
                    + "\n".join(divergences)
                    + "\n\nEvery line is a main merge whose PR head SHA lacked "
                      "a `success` run of the required set (a skipped check "
                      "is not green; a check absent from the head SHA's "
                      "workflow set — a job born after the merge — is still "
                      "flagged and needs a human verdict), or a protection/"
                      "tripwire line (№824) that needs the checklist sync. "
                      "Investigate, then close the issue with the verdict.")
            data = json.dumps({
                "title": f"merge↔CI audit: {len(divergences)} merge(s) without green required checks",
                "body": body,
                "labels": ["process", "audit"],
            }).encode()
            req = urllib.request.Request(
                f"https://api.github.com/repos/{repo}/issues", data=data,
                headers={
                    "Authorization": f"Bearer {token}",
                    "Accept": "application/vnd.github+json",
                    "Content-Type": "application/json",
                }, method="POST")
            with urllib.request.urlopen(req, timeout=30) as resp:
                issue = json.loads(resp.read().decode())
            print(f"issue filed: {issue.get('html_url')}")
        sys.exit(1)
    print("merge-ci-audit: CLEAN — every merge had the green required set")


if __name__ == "__main__":
    main()
