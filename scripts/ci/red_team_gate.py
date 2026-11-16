#!/usr/bin/env python3
"""№961 (issue #1928): the red-team gate — a SECURITY-CLASS PR does not
merge without the `red-team: <report link>` field in its description.

The class: the PR title carries `[security]` (the naryad class marker
convention) or the PR carries the `security` label. The field: a line in
the PR body matching `^\\s*red-team:\\s*\\S+` — a NON-EMPTY link to the
pass report (docs/red-team-protocol.md, §1).

CI wiring (the `red-team-gate` job, blocking on the security class):
  python3 scripts/ci/red_team_gate.py --pr-number "$PR_NUMBER"
The script reads the PR through the GitHub API with GH_TOKEN. A push or
main event is not a PR — the job exits 0 with the loud skip (the
release-candidate line lives in the release checklist, not in CI).

Exit codes: 0 = ok or not the class; 1 = a security-class PR without a
valid field (the BLOCKING refusal); 2 = the API broke (infra, loud).
--self-test runs the pinned trap circuit (the guard's own check is a
test — the merge_ci_audit --self-test form) and exits 0 only when every
trap springs.
"""
import json
import os
import re
import sys
import urllib.request

CLASS_TITLE_RE = re.compile(r"\[security\]", re.IGNORECASE)
RED_TEAM_RE = re.compile(r"^\s*red-team:\s*\S+", re.MULTILINE)
API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"


def fetch_pr(number: int, token: str) -> dict:
    req = urllib.request.Request(
        f"{API}/pulls/{number}",
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"},
    )
    with urllib.request.urlopen(req) as r:
        return json.load(r)


def fetch_labels(number: int, token: str) -> list[str]:
    req = urllib.request.Request(
        f"{API}/issues/{number}/labels",
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json"},
    )
    with urllib.request.urlopen(req) as r:
        return [x["name"] for x in json.load(r)]


def gate(title: str, body: str, labels: list[str]) -> tuple[int, list[str]]:
    """The pure decision (the self-test feeds it synthetic states).
    Returns (exit_code, the report lines)."""
    out: list[str] = []
    is_security = bool(CLASS_TITLE_RE.search(title or "")) or "security" in (labels or [])
    if not is_security:
        out.append("red-team-gate: not a security-class PR — the field is not required (№961).")
        return 0, out
    out.append("red-team-gate: SECURITY-CLASS PR — the independent pass is required (№961, docs/red-team-protocol.md §1).")
    if RED_TEAM_RE.search(body or ""):
        out.append("red-team-gate: OK — the `red-team:` field is present and non-empty.")
        return 0, out
    out.append(
        "red-team-gate: REFUSED — the description carries NO `red-team: <report link>` "
        "field. Run the pass per docs/red-team-protocol.md §2 (a separate adversary "
        "session, black-box, the candidate binary), attach the report under "
        "docs/audits/red-team-<date>/, and add the field line to this PR's description."
    )
    return 1, out


def self_test() -> int:
    cases = [
        ("security title, no field", gate("[security] fix the gate", "body without the field", []), 1),
        ("security title, with field", gate("[security] fix the gate", "red-team: docs/audits/red-team-x/report.md", []), 0),
        ("security label, no field", gate("some title", "body", ["security"]), 1),
        ("non-security, no field", gate("docs row", "body", []), 0),
        ("empty link is not a field", gate("[security] x", "red-team:   ", []), 1),
        ("field needs its own line", gate("[security] x", "the red-team: link inline prose", []), 1),
    ]
    bad = 0
    for name, (code, lines), want in cases:
        if code != want:
            print(f"SELF-TEST TRAP DID NOT SPRING: {name} — exit {code}, want {want}")
            bad += 1
        else:
            print(f"self-test ok: {name} → exit {want}")
    if bad:
        print(f"red_team_gate --self-test: {bad} trap(s) failed")
        return 1
    print("red_team_gate --self-test: 6/6 traps green")
    return 0


def main() -> None:
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    args = sys.argv[1:]
    if "--pr-number" not in args:
        print(__doc__)
        sys.exit(2)
    number = int(args[args.index("--pr-number") + 1])
    token = os.environ.get("GH_TOKEN", "")
    if not token:
        print("LOUD SKIP: GH_TOKEN absent — the gate cannot read the PR (CI always sets it).")
        sys.exit(0)
    try:
        pr = fetch_pr(number, token)
        labels = fetch_labels(number, token)
    except Exception as e:  # noqa: BLE001 — the infra break is its own loud class
        print(f"red-team-gate: INFRA — the PR read failed: {e}")
        sys.exit(2)
    code, lines = gate(pr.get("title") or "", pr.get("body") or "", labels)
    for line in lines:
        print(line)
    sys.exit(code)


if __name__ == "__main__":
    main()
