#!/usr/bin/env python3
"""Naryad numbering collision check (Naryad №505, audit 28.09 §3.5).

Under one naryad number different works have already lived: №474 ("enum
Type stage 1" vs "db parity"), №475 ("evidence FO-056" vs "fs_gate"),
№476 ("post-wave sync" vs "the blocked-domains rule"), №493 ("ADR-0177
Lifted" vs "the 8 hygiene findings"). The naryad is the primary unit of
account and nothing checked its uniqueness — until this gate.

The gate: a pull request that claims "Naryad №N" in its title must NOT
reuse an N that already landed as a DIFFERENT work. The occupied set is
collected from the merged git history (commit subjects matching the
naryad grammar) and — when GH_TOKEN is available — from the closed
issue titles (they carry the canonical "Наряд №N (priority, area): …"
form).

THE TITLE GRAMMAR (what this gate accepts):

  [Naryad|Наряд] №?N            — the canonical claim (checked for
                                  collisions);
  [Naryad|Наряд] №?N group M    — the explicit work-group exception
                                  (№466 group 1..7): the group M shares
                                  the parent number on purpose;
  [Naryad|Наряд] №?N.B          — the dotted re-issue suffix (№475.1):
                                  a deliberate re-edition, B >= 1;
  ... <reland/retry/revert/rerun/re-run/take-N/redo suffix>
                                — the re-land suffix: a deliberate
                                  re-edition of the SAME naryad;
  ... (issue #M)                — the same-issue exception: if the
                                  occupied record for N points at the
                                  same issue M, the reuse is a
                                  follow-up of the same work, allowed.

Anything else colliding with an occupied number fails with the list of
the occupied works (commit subjects + issue titles).

Usage:
    python3 scripts/ci/naryad_number_check.py "PR TITLE" [--repo-dir DIR]

Exit codes: 0 = ok or skipped (no naryad claim), 1 = collision.
`--self-test` runs the grammar cases and exits.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import urllib.request

# The naryad claim: "naryad 500", "Naryad №500", "Наряд №500".
NARYAD_RE = re.compile(
    r"(?iu)\b(?:naryad|наряд)\s*№?\s*(\d+)", re.UNICODE
)
# The explicit work-group suffix: "naryad 466 group 7".
GROUP_RE = re.compile(r"(?iu)\bgroup\s*#?\s*(\d+)", re.UNICODE)
# The dotted re-issue suffix: "naryad 475.1".
DOT_RE = re.compile(r"(?iu)\bnaryad\s*№?\s*(\d+)\.(\d+)", re.UNICODE)
# The re-land suffix anywhere in the title.
RELAND_RE = re.compile(r"(?iu)\b(re-?land|retry|revert|re-?run|redo|take[- ]?\d+)\b")
# The issue pointer in a title: "(issue #783)".
ISSUE_RE = re.compile(r"(?i)\bissue\s*#?(\d+)", re.UNICODE)

API = "https://api.github.com"


def commit_subjects(repo_dir: str) -> list[str]:
    """Every commit subject reachable from HEAD (the CI job checks out
    with fetch-depth: 0; a shallow clone simply covers a shorter span)."""
    try:
        out = subprocess.run(
            ["git", "log", "--pretty=format:%s"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
            timeout=60,
            check=True,
        ).stdout
    except (subprocess.SubprocessError, OSError) as exc:  # pragma: no cover
        print(f"::warning::git log unavailable ({exc}); history coverage reduced")
        return []
    return [line for line in out.splitlines() if line.strip()]


def closed_issue_titles() -> list[str]:
    """The closed issue titles (the canonical 'Наряд №N …' form). Best
    effort: with no GH_TOKEN or on any API trouble the history-only set
    still holds (the merged works always leave a commit)."""
    token = os.environ.get("GH_TOKEN", "").strip()
    if not token:
        print("::notice::GH_TOKEN not set — the issue-title source skipped")
        return []
    repo = os.environ.get("GITHUB_REPOSITORY", "").strip()
    if not repo:
        return []
    titles: list[str] = []
    for page in range(1, 4):  # 3 pages x 100 — the closed tail suffices
        url = f"{API}/repos/{repo}/issues?state=closed&per_page=100&page={page}"
        req = urllib.request.Request(url, headers={"Authorization": f"token {token}"})
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                batch = json.load(resp)
        except (OSError, ValueError) as exc:  # pragma: no cover
            print(f"::warning::issues page {page} unavailable ({exc}); continuing")
            break
        if not batch:
            break
        titles += [item.get("title", "") for item in batch if "pull_request" not in item]
    return titles


def collect_occupied(subjects: list[str], titles: list[str]) -> dict[int, list[str]]:
    """N -> the human-readable list of the works carrying it."""
    occupied: dict[int, list[str]] = {}
    for text in subjects + titles:
        for match in NARYAD_RE.finditer(text):
            number = int(match.group(1))
            occupied.setdefault(number, []).append(text)
    return occupied


def title_claim(title: str) -> tuple[int, str] | None:
    """The (number, reason) the PR title claims, or None when no claim.

    The dotted re-issue (№475.1) is its own number by grammar — the
    claim is (4751, dotted) is NOT how it works: the dotted form names
    base 475 DELIBERATELY, so the gate maps it to (475, "dotted
    re-issue") and lets the suffix rule allow it below.
    """
    dot = DOT_RE.search(title)
    if dot:
        return int(dot.group(1)), "dotted re-issue"
    match = NARYAD_RE.search(title)
    if match:
        return int(match.group(1)), "naryad claim"
    return None


def check(title: str, occupied: dict[int, list[str]]) -> int:
    if not NARYAD_RE.search(title):
        print("No naryad claim in the title — nothing to check.")
        return 0
    number, kind = title_claim(title)
    assert number is not None

    if number not in occupied:
        print(f"Naryad №{number} is not occupied — ok.")
        return 0

    # The exception grammar.
    group = GROUP_RE.search(title)
    dotted = kind == "dotted re-issue"
    relanded = bool(RELAND_RE.search(title))
    pr_issue = ISSUE_RE.search(title)
    if pr_issue:
        pr_issue = int(pr_issue.group(1))

    if group or dotted or relanded:
        which = (
            f"the explicit work-group ({group.group(0)})" if group
            else "the dotted re-issue suffix" if dotted
            else "the re-land suffix"
        )
        print(
            f"Naryad №{number} is occupied, but the title carries {which} — "
            "the re-edition grammar allows it."
        )
        return 0

    if pr_issue:
        for work in occupied[number]:
            work_issue = ISSUE_RE.search(work)
            if work_issue and int(work_issue.group(1)) == pr_issue:
                print(
                    f"Naryad №{number} is occupied by a work on the SAME issue "
                    f"#{pr_issue} — a follow-up of the same work, allowed."
                )
                return 0

    print(f"::error::Naryad number collision: №{number} is already occupied by:")
    for work in occupied[number][:12]:
        print(f"::error::  - {work[:160]}")
    print(
        "::error::Re-use the dotted suffix (№N.1), the re-land suffix, the group "
        "grammar, or pick a free number. (Naryad №505, audit 28.09 §3.5)"
    )
    return 1


def self_test() -> int:
    """The grammar cases pinned (the gate tests itself — no stub work)."""
    occupied = {
        474: ["naryad 474 (issue #100): db parity", "naryad 474: enum Type stage 1"],
        466: ["naryad 466 group 1 (x) (#700)"],
        500: ["naryad 500 (issue #783): pdf path-APIs through fs_gate (#807)"],
        495: ["naryad 495 (issue #780): DistillHub (#806)"],
    }
    ok = [
        ("Naryad №505 (issue #788): the numbering gate", 0, None),
        ("fix: typos in README", 0, None),
        ("Naryad №501 (issue #784): the deny-list registers", 0, None),
        # The exceptions.
        ("Naryad №466 group 8 (the next group)", 0, None),
        ("Naryad №474.1 (the dotted re-issue)", 0, None),
        ("Naryad №500 (reland): the gate follow-up", 0, None),
        ("Naryad №500 (issue #783): the follow-up commit batch", 0, None),
    ]
    bad = [
        ("Naryad №474 (issue #999): a DIFFERENT work on a busy number", 1, None),
        ("naryad 495 (issue #900): another DistillHub-style work", 1, None),
    ]
    failed = 0
    for title, expected, _ in ok + bad:
        import io
        import contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = check(title, occupied)
        status = "ok" if code == expected else "FAIL"
        if code != expected:
            failed += 1
        print(f"  self-test [{status}] expect={expected} got={code}: {title[:60]}")
    print(f"self-test: {'ALL PASS' if failed == 0 else f'{failed} FAILED'}")
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("title", nargs="?", default="", help="the PR title")
    parser.add_argument("--repo-dir", default=".")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if not args.title:
        parser.error("the PR title is required (or --self-test)")
    occupied = collect_occupied(
        commit_subjects(args.repo_dir), closed_issue_titles()
    )
    print(f"The occupied set: {len(occupied)} naryad numbers from the history + issues.")
    return check(args.title, occupied)


if __name__ == "__main__":
    sys.exit(main())
