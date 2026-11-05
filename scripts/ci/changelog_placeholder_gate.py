#!/usr/bin/env python3
"""№902 (issue #1796; the R50 lesson repeated): the placeholder gate —
the release documents never ship an unfilled template token.

The fact that founded the naryad: the released 0.38.2 CHANGELOG carried
the row «№897 (issue #ISSUE_NUM …)» — a template placeholder that
survived composition, review and CI into a published release note. A
placeholder in a shipped document is a lie the index serves on the
first read.

The gate scans the release-facing documents:

  - CHANGELOG.md
  - docs/release-notes-*.md (the drafts and the migration addenda)

for the placeholder family:

  #ISSUE_NUM     the naryad-issue placeholder (the №897/0.38.2 case)
  ISSUE_NUM      the bare token (the same template, half-filled)
  gh#{           the unrendered GitHub-expression form («gh#{…}»)
  {DISPATCH}     the unrendered dispatch slot
  {N}            the unrendered naryad-number slot

Any hit = exit 1, naming the file, the line and the token. The scan is
LINE-oriented and literal — the honest scope boundary: PR bodies live
on GitHub (outside the tree) and are the composition discipline; the
machine guards what the repo ships.

--self-test runs the embedded fixtures (a clean block passes; every
pattern trips) and must print SELF-TEST OK.

Usage:
    python3 scripts/ci/changelog_placeholder_gate.py [--self-test]
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent

# The targets: the changelog + every release-notes document in docs/.
TARGETS = ["CHANGELOG.md"]
NOTES_GLOB = "docs/release-notes-*.md"

# The placeholder family (literal tokens; {…} is NOT a format string here).
PATTERNS = [
    ("#ISSUE_NUM", re.compile(r"#ISSUE_NUM")),
    ("ISSUE_NUM", re.compile(r"\bISSUE_NUM\b")),
    ("gh#{", re.compile(r"gh#\{")),
    ("{DISPATCH}", re.compile(r"\{DISPATCH\}")),
    ("{N}", re.compile(r"\{N\}")),
]


def scan_text(text: str) -> list[tuple[int, str, str]]:
    """(line_no, line, token) hits in one text."""
    hits: list[tuple[int, str, str]] = []
    for i, line in enumerate(text.splitlines(), start=1):
        for token, rx in PATTERNS:
            if rx.search(line):
                hits.append((i, line.strip(), token))
    return hits


def scan_repo() -> list[str]:
    fails: list[str] = []
    paths = [REPO / t for t in TARGETS] + sorted(REPO.glob(NOTES_GLOB))
    for path in paths:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        for line_no, line, token in scan_text(text):
            fails.append(
                f"{path.relative_to(REPO)}:{line_no}: the placeholder "
                f"'{token}' — {line[:110]}"
            )
    return fails


def self_test() -> int:
    clean = (
        "- **№897 (issue #1789 — [process][bugfix]):** the fix shipped\n"
        "  with the issue number pinned; no template token survived.\n"
    )
    dirties = [
        "- **№897 (issue #ISSUE_NUM — [process][bugfix]):** the fix.",
        "the source gh#{DISPATCH} rides the wave.",
        "naryad {N} closes the lane; gh#{WAVE} pending.",
        "the bare ISSUE_NUM token also trips.",
    ]
    ok = True
    if scan_text(clean):
        ok = False
        print(f"  TRAP DID NOT SPRING (clean red): {scan_text(clean)}")
    else:
        print("  ok: the clean block passes")
    for i, dirty in enumerate(dirties, 1):
        hits = scan_text(dirty)
        if not hits:
            ok = False
            print(f"  TRAP DID NOT SPRING (dirty #{i} passed): {dirty}")
        else:
            print(f"  ok: dirty #{i} trips on '{hits[0][2]}'")
    print("SELF-TEST OK" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    fails = scan_repo()
    if fails:
        print("placeholder gate: RED — the release documents carry unfilled template tokens:")
        for f in fails:
            print(f"  - {f}")
        print(
            "fix: fill the token (the real issue/PR/naryad number) or remove "
            "the row — a published placeholder is the №897/0.38.2 lesson (gh#1796)"
        )
        return 1
    print("placeholder gate: OK — no template tokens in the release documents")
    return 0


if __name__ == "__main__":
    sys.exit(main())
