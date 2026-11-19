#!/usr/bin/env python3
"""№976 (issue #1985, the consolidated audit 18.11 — finding A): the
RELEASE-COMMIT gate — what the release PROMISES (the live required set)
is what the release commit actually RAN (its check-runs), verified
mechanically on the tag/release path, not on a PR.

The audit fact this closes: v0.40.0 was tagged from a commit whose CI
gave ZERO check-runs — the release shipped with nothing behind it. This
gate runs on the tag path: every context the live required set demands
AS OF the release instant must be PRESENT and GREEN on the release
commit; anything missing, red, pending or skipped is a loud refusal
(fail-closed). A context that became required AFTER the release instant
does not charge the release (the №960 direction, the same
required_at mechanics the merge↔CI detective runs) — a release is
judged by the promises that existed when it was cut.

The act the token cannot do (403 Resource not accessible by
integration): the retro-verification RUN of CI on v0.40.0 — that is the
OWNER's act (docs/reality/release-0.40.0-retro-verification.md carries
the instruction and the result place). This gate is the machine half:
the NEXT release cannot ship behind an empty check-run set.

Usage:
  release_commit_gate.py --sha <sha> [--ref <ref>]   # the gate run
  release_commit_gate.py --self-test                 # the fixture traps

Exit codes: 0 green · 1 the gate reds · 2 fail-closed (the state
unreadable — an unreadable state is not a pass, №525).
"""
import argparse
import json
import os
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from merge_ci_audit import REQUIRED_CHECKS, REQUIRED_SINCE, required_at  # noqa: E402

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"
NON_BLOCKING_HINTS = ("non-blocking",)


def _fetch(url, token):
    req = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.load(r)
    except Exception as e:  # noqa: BLE8 — fail-closed on ANY unreadable state
        raise UnreadableState(f"{url}: {e}")


class UnreadableState(Exception):
    pass


def check_runs_for(sha, token):
    out = []
    page = 1
    while True:
        data = _fetch(f"{API}/commits/{sha}/check-runs?per_page=100&page={page}", token)
        runs = data.get("check_runs", [])
        out.extend(runs)
        if len(runs) < 100:
            return out
        page += 1


def gate(release_sha, runs, required, release_instant=None, since_map=None):
    """The pure decision (the --self-test fixtures feed it synthetic states).

    Returns (exit_code, lines). `runs` is the list of {name, conclusion}
    dicts; `required` the required-context names; `release_instant` the
    release cut instant for the required_at direction (None = the full set);
    `since_map` the per-context since map (None = REQUIRED_SINCE).
    """
    lines: list[str] = []
    names = required_at(release_instant, required=required, since_map=since_map)
    have = {}
    for r in runs:
        have.setdefault(r.get("name"), r.get("conclusion") or r.get("status"))
    blocking = [n for n in names if not any(h in n for h in NON_BLOCKING_HINTS)]
    missing = [n for n in blocking if n not in have]
    bad = [n for n in blocking
           if n in have and have[n] not in ("success",)]
    lines.append(f"release-commit-gate: {release_sha} — required {len(blocking)}, "
                 f"present {len(blocking) - len(missing)}, missing {len(missing)}, "
                 f"not-green {len(bad)}")
    for n in missing:
        lines.append(f"  MISSING: {n} — the release promises it, no run on the release commit")
    for n in bad:
        lines.append(f"  NOT GREEN: {n} — {have[n]}")
    if missing or bad:
        lines.append("release-commit-gate: REFUSED — a release commit is a promise: "
                     "every required context must be green on it (№976).")
        return 1, lines
    lines.append("release-commit-gate: OK — every required context green on the release commit.")
    return 0, lines


def self_test() -> int:
    required = ["a (blocking)", "b (blocking)", "c (non-blocking)"]
    full = [{"name": "a (blocking)", "conclusion": "success"},
            {"name": "b (blocking)", "conclusion": "success"},
            {"name": "c (non-blocking)", "conclusion": "failure"}]
    missing_one = [{"name": "a (blocking)", "conclusion": "success"}]
    red_one = [{"name": "a (blocking)", "conclusion": "success"},
               {"name": "b (blocking)", "conclusion": "failure"}]
    pending = [{"name": "a (blocking)", "conclusion": "success"},
               {"name": "b (blocking)", "conclusion": "in_progress"}]
    cases = [
        ("full green passes (a non-blocking red does not charge)", gate("S", full, required), 0),
        ("a missing required context reds — THE M1 MUTATION", gate("S", missing_one, required), 1),
        ("a red required context reds — M2", gate("S", red_one, required), 1),
        ("a pending required context reds (fail-closed) — M3", gate("S", pending, required), 1),
        ("a context required AFTER the release instant does not charge (№960)",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "late (blocking)"], release_instant="2026-01-01T00:00:00Z",
              since_map={"late (blocking)": "2027-01-01T00:00:00Z"}), 0),
        ("empty check-runs on the release commit reds — THE AUDIT FACT",
         gate("S", [], required), 1),
    ]
    bad = 0
    for name, (code, lines), want in cases:
        if code != want:
            print(f"SELF-TEST TRAP DID NOT SPRING: {name} — exit {code}, want {want}")
            bad += 1
        else:
            print(f"self-test ok: {name} → exit {want}")
    if bad:
        print(f"release_commit_gate --self-test: {bad} trap(s) failed")
        return 1
    print("release_commit_gate --self-test: 6/6 traps green")
    return 0


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sha")
    ap.add_argument("--ref")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    if args.self_test:
        sys.exit(self_test())
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GITHUB_PAT")
    if not args.sha or not token:
        print("release-commit-gate: --sha and GITHUB_TOKEN are required (or --self-test)")
        sys.exit(2)
    try:
        runs = check_runs_for(args.sha, token)
    except UnreadableState as e:
        print(f"release-commit-gate: FAIL-CLOSED — the state is unreadable: {e}")
        sys.exit(2)
    code, lines = gate(args.sha, runs, REQUIRED_CHECKS, release_instant=None)
    for l in lines:
        print(l)
    sys.exit(code)


if __name__ == "__main__":
    main()
