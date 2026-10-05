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

THE SQUASH-BODY RULE (Naryad №587, audit d63cc1d X-5): the squash body
= the description of THIS naryad only. The precedent: b678def was
 titled "Наряд №574.1 (issue #975)" but its body OPENED with the full
description of №572.1 (the PR was branched off the №572.1 branch before
its merge and the template description traveled into the squash) — at
tribution time from the commit body the change lands on the wrong
naryad. The check: a naryad claim in a BULLET/HEADING position of the
body ("* Наряд №M …") whose base number differs from the title's (and
that does not name the title's own issue) is a foreign claim — refused.
Prose mentions ("the №493 precedent", "mirrors №500") stay legal — the
claim POSITION is the signal, not the mention.

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

# The naryad claim in a BULLET/HEADING position of a body (№587, X-5):
#   "* Наряд №572.1 (issue #973): ..."  "- **Naryad №500** ..."
#   "## Наряд №500: ..."
# The claim POSITION (the line's first token) is the signal — prose
# mentions are not matched.
BODY_HEADER_RE = re.compile(
    r"(?im)^\s*(?:[-*+]|\d+[.)])\s+(?:\*\*)?\s*(?:naryad|наряд)\s*№?\s*(\d+(?:\.\d+)?)"
    r"|^\s*#{1,6}\s+(?:\*\*)?\s*(?:naryad|наряд)\s*№?\s*(\d+(?:\.\d+)?)",
    re.UNICODE,
)

# The STATUS-LINE exception (the history-audit fact, №587): a bullet that
# EVALUATES a run (": PASS", ": FAIL", "rc=0") is evidence prose, not a
# naryad description claim. The calibration case: "- naryad 465 diff
# fuzzer on this branch: PASS — ..." (the №474 body) is a run report,
# while "* naryad 466 group 5 (audit-ledger): the deny pair ..." in the
# SAME body is a foreign description claim. The verdict marker after a
# colon separates the two.
STATUS_LINE_RE = re.compile(
    r"(?i):\s*(PASS|FAIL|OK|GREEN|RED)\b|\brc\s*=\s*\d", re.UNICODE
)

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


def _base(number_text: str) -> int:
    """The base of a naryad number: '574.1' -> 574, '500' -> 500."""
    return int(number_text.split(".", 1)[0])


def foreign_body_claims(title: str, body: str) -> list[str]:
    """The body bullet/heading claims of naryads OTHER than the title's
    (№587, audit X-5). The same-issue exception of the title grammar
    mirrors here: a foreign NUMBER whose bullet names the title's own
    issue is a same-work enumeration, allowed."""
    claim = title_claim(title)
    if claim is None:
        return []
    own_number, _ = claim
    title_issue = ISSUE_RE.search(title)
    own_issue = int(title_issue.group(1)) if title_issue else None
    foreign: list[str] = []
    for line in body.splitlines():
        match = BODY_HEADER_RE.search(line)
        if not match:
            continue
        if STATUS_LINE_RE.search(line):
            continue  # a run verdict, not a description claim
        text = match.group(1) or match.group(2)
        base = _base(text)
        if base == own_number:
            continue
        line_issue = ISSUE_RE.search(line)
        if own_issue is not None and line_issue and int(line_issue.group(1)) == own_issue:
            continue
        foreign.append(line.strip())
    return foreign


def audit_history(repo_dir: str, limit: int) -> int:
    """The false-positive audit: every merged naryad commit's BODY is
    checked against its own title (№587 DoD: zero false positives on
    the clean wave history)."""
    try:
        out = subprocess.run(
            ["git", "log", f"-{limit}", "--pretty=format:%H%x00%B%x01"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
            timeout=60,
            check=True,
        ).stdout
    except (subprocess.SubprocessError, OSError) as exc:  # pragma: no cover
        print(f"::error::git log unavailable ({exc})")
        return 1
    scanned = flagged = 0
    for record in out.split("\x01"):
        record = record.strip("\n")
        if "\x00" not in record:
            continue
        _sha, _, message = record.partition("\x00")
        lines = message.splitlines()
        if not lines:
            continue
        title, body = lines[0], "\n".join(lines[1:])
        if not NARYAD_RE.search(title):
            continue
        scanned += 1
        for hit in foreign_body_claims(title, body):
            flagged += 1
            print(f"  FLAG: {title[:96]}")
            print(f"        {hit[:120]}")
    print(f"audit-history: {scanned} naryad commits scanned, {flagged} flagged.")
    return 0


def check(title: str, occupied: dict[int, list[str]], body: str = "") -> int:
    if not NARYAD_RE.search(title):
        print("No naryad claim in the title — nothing to check.")
        return 0
    number, kind = title_claim(title)
    assert number is not None

    if body:
        foreign = foreign_body_claims(title, body)
        if foreign:
            print(
                f"::error::The squash body carries the description of a DIFFERENT "
                f"naryad than the title's №{number} (№587, audit d63cc1d X-5):"
            )
            for hit in foreign[:8]:
                print(f"::error::  - {hit[:160]}")
            print(
                "::error::The PR description = THIS naryad's description only; "
                "branching off another naryad's branch does not inherit its "
                "description."
            )
            return 1
        print("The body carries no foreign naryad claims — ok.")

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
    # The №587 body cases: the title is №574.1's, the bodies speak for
    # themselves (the b678def synthetic reconstruction first).
    title_574 = (
        "Наряд №574.1 (issue #975): the VM json_body serve contract"
    )
    body_cases = [
        ("the b678def class — the body OPENS with the foreign naryad's "
         "description", 1,
         title_574,
         "* Наряд №572.1 (issue #973): the release pipeline follows the bin\n"
         "\n- build.yml:19 -> '-p metalogos-server --bin mlog'\n"),
        ("the own bullet-header + prose mentions of foreign naryads", 0,
         title_574,
         "* Наряд №574.1 (issue #975): the VM json_body serve contract\n"
         "- the №493 precedent holds; mirrors №250 branch-tail semantics\n"),
        ("bold bullet header of a foreign naryad", 1,
         title_574,
         "- **Наряд №572 (issue #971): another work**\n  body text\n"),
        ("heading form of a foreign naryad", 1,
         title_574,
         "## Наряд №572: the release pipeline\ntext\n"),
        ("same-issue exception — a foreign NUMBER naming the title's issue", 0,
         title_574,
         "* Наряд №574 (issue #975): the parent work enumeration\n"),
        ("numbered-list form", 1,
         title_574,
         "1. Наряд №573 (issue #977): yet another work\n"),
        ("prose mention is NOT a claim", 0,
         title_574,
         "The route-tail semantics mirrors the №250 and №572.1 contracts.\n"),
        ("the №474 calibration pair: a fuzzer STATUS line is evidence, "
         "not a claim", 0,
         "naryad 474: enum Type stage 1 — the let-type inference",
         "- naryad 465 diff fuzzer on this branch: PASS — the divergence-class\n"
         "  set unchanged (the pass is fuzzer-neutral)\n"),
        ("the №474 calibration pair: the SAME body's group-5 bullet IS a "
         "foreign claim", 1,
         "naryad 474: enum Type stage 1 — the let-type inference",
         "* naryad 466 group 5 (audit-ledger): the deny pair + the event trio\n"),
        ("no body — the check passes through", 0, title_574, ""),
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
    for name, expected, title, body in body_cases:
        hits = foreign_body_claims(title, body)
        code = 1 if hits else 0
        status = "ok" if code == expected else "FAIL"
        if code != expected:
            failed += 1
        print(f"  body-test [{status}] expect={expected} got={code}: {name}")
    print(f"self-test: {'ALL PASS' if failed == 0 else f'{failed} FAILED'}")
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("title", nargs="?", default="", help="the PR title")
    parser.add_argument(
        "--body",
        default="",
        help="the PR body (becomes the squash body — checked for foreign "
        "naryad claims, №587); empty skips the body check",
    )
    parser.add_argument("--repo-dir", default=".")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--audit-history",
        type=int,
        default=0,
        metavar="N",
        help="audit the last N merged commits' bodies against their own "
        "titles (the false-positive audit) and exit",
    )
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.audit_history:
        return audit_history(args.repo_dir, args.audit_history)
    if not args.title:
        parser.error("the PR title is required (or --self-test/--audit-history)")
    occupied = collect_occupied(
        commit_subjects(args.repo_dir), closed_issue_titles()
    )
    print(f"The occupied set: {len(occupied)} naryad numbers from the history + issues.")
    return check(args.title, occupied, body=args.body)


if __name__ == "__main__":
    sys.exit(main())
