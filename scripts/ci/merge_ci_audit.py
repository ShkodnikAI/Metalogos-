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

Divergence exit code: 0 = clean, 1 = divergences found (the scheduled
workflow run goes red — visible), 2 = the audit could not run (API
failure — fail-closed, a silent auditor is a liar auditor).

Modes:
  python3 scripts/ci/merge_ci_audit.py                  # report only
  python3 scripts/ci/merge_ci_audit.py --days 14        # wider window
  python3 scripts/ci/merge_ci_audit.py --open-issue     # file the issue
  python3 scripts/ci/merge_ci_audit.py --self-test      # embedded fixtures
"""

import json
import os
import re
import sys
import urllib.request

DEFAULT_DAYS = 7

# The required set (the branch-protection checklist of №551 — the job
# DISPLAY names as they appear in the check-runs API; fact-checked
# against .github/workflows/ci.yml on the audit revision 6e66d3d;
# `msrv (blocking)` added by №555, issue #916; `release-gap (blocking)` by №562, issue #923).
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
]

MERGE_PR_RE = re.compile(r"\(#(\d+)\)\s*$")


# ── pure logic (fixture-tested by --self-test and the Rust driver) ──

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
    if failures:
        print(f"SELF-TEST FAILED: {failures} case(s)")
        return 1
    print(f"SELF-TEST OK: {len(cases)} cases "
          f"(green, no-runs, skipped, failed, no-PR, unavailable)")
    return 0


def main():
    argv = sys.argv[1:]
    if "--self-test" in argv:
        sys.exit(self_test())

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
    try:
        for pr in pr_numbers:
            data = api_get(f"https://api.github.com/repos/{repo}/pulls/{pr}", token)
            head_sha = data.get("head", {}).get("sha")
            runs = all_check_runs(repo, head_sha, token) if head_sha else []
            pr_map[pr] = {"head_sha": head_sha, "check_runs": runs}
    except Exception as e:
        print(f"merge-ci-audit: the PR/check-run data is UNAVAILABLE: {e} "
              "(fail-closed)", file=sys.stderr)
        sys.exit(2)

    divergences = []
    for c in commits:
        first = (c.get("commit", {}).get("message") or "").splitlines()[0:1]
        first = first[0] if first else ""
        ok, problems = decide({"first_line": first, "sha": c.get("sha", "")}, pr_map)
        if not ok:
            divergences.append(
                f"- `{c.get('sha', '')[:7]}` {first}\n  - " + "\n  - ".join(problems))

    print(f"merge-ci-audit: {len(commits)} merge(s) on main in the last "
          f"{days} day(s); required checks: {len(REQUIRED_CHECKS)}")
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
            body = ("The weekly merge↔CI audit (№551) found merge commit(s) "
                    "without a green run of the required checks — the M-1 "
                    "class the branch-protection checklist guards against:\n\n"
                    + "\n".join(divergences)
                    + "\n\nEvery line is a main merge whose PR head SHA lacked "
                      "a `success` run of the required set (a skipped check "
                      "is not green; a check absent from the head SHA's "
                      "workflow set — a job born after the merge — is still "
                      "flagged and needs a human verdict). Investigate, then "
                      "close the issue with the verdict.")
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
