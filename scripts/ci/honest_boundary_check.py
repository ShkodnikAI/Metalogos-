#!/usr/bin/env python3
"""The honest-boundary protocol check (Naryad №588, audit d63cc1d §8).

The rule: any "honest boundary" marker in the PR diff (a commit body or
a code comment) MUST be accompanied — in the SAME PR — by a
docs/limitations.md row, and if the divergence is security-relevant, by
the `release-block` label on the linking issue and a flag in the PR
description. The marker turns the developer's honesty into a fact the
release gate can see; the precedent of the broken channel is b678def:
it honestly named both divergences (X-1/X-2) in the commit body — and
limitations.md stayed empty, no label, fact_open_high_server: 0.

THE MARKERS (calibrated per the naryad's list — honest boundary /
pre-existing / known divergence):
  - "honest boundary"          (the tree's canonical form, 8 live uses);
  - "pre-existing divergence"  (the audit §8's full phrase — the bare
    "pre-existing" is ordinary English in evidence prose (28 live uses
    unrelated to any boundary) and would flood the gate with noise);
  - "known divergence".

ADDED LINES ONLY, no retrospective scan (the naryad's boundary): the
existing in-tree markers already carry their ADR/limitations records;
the rule acts forward on what a PR adds.

ADVISORY (warn-only) by fact: the naryad's precedent is warn-only and
the blocking escalation is decided by the false-positive experience —
"loudness is mandatory, the block is by fact, not silently" (gh#1002).
The job annotates the PR loudly (the ::warning:: workflow annotation +
the instruction line) and exits 0. A hard failure here means the check
itself broke, not that the protocol was violated.

Usage:
    python3 scripts/ci/honest_boundary_check.py --diff <file|-> [--changed <file> ...]

    --diff      the unified diff of the PR (the added lines are scanned);
                "-" reads stdin
    --changed   the changed-file paths (repeatable); when omitted, the
                changed set is derived from the diff's +++ headers

Exit codes: 0 = ok or advisory warning issued; 2 = the check itself broke.
`--self-test` runs the pinned cases and exits.
"""

from __future__ import annotations

import argparse
import re
import sys

# The marker forms, case-insensitive (the calibration is in the docstring).
MARKERS: tuple[re.Pattern[str], ...] = (
    re.compile(r"(?i)honest\s+boundary"),
    re.compile(r"(?i)pre-?existing\s+divergence"),
    re.compile(r"(?i)known\s+divergence"),
)

# A unified-diff added line (not the +++ file header itself).
ADDED_LINE_RE = re.compile(r"^\+(?!\+\+)", re.UNICODE)
# The diff's file header — the changed-file derivation.
FILE_HEADER_RE = re.compile(r"^\+\+\+\s+(?:b/)?(\S+)", re.UNICODE)


def added_lines_with_files(diff: str) -> list[tuple[str, str]]:
    """(file, added-line text) pairs; the file tracks the current header."""
    current = "<unknown>"
    pairs: list[tuple[str, str]] = []
    for line in diff.splitlines():
        header = FILE_HEADER_RE.match(line)
        if header:
            current = header.group(1)
            continue
        if line.startswith(("--- ", "@@", "diff ", "index ", "new file mode", "deleted file mode")):
            # `--- ` deletions carry no added content; the @@ hunks reset.
            continue
        if ADDED_LINE_RE.match(line):
            pairs.append((current, line[1:]))
    return pairs


def check(diff: str, changed: list[str] | None) -> tuple[int, list[str], int]:
    """Returns (exit_code, the warning lines, the flagged marker count).
    exit 0 = ok/advisory."""
    hits: list[str] = []
    for path, text in added_lines_with_files(diff):
        for marker in MARKERS:
            if marker.search(text):
                hits.append(f"{path}: {text.strip()[:150]}")
                break
    if not hits:
        return 0, [], 0

    changed_set = set(changed) if changed else {p for p, _ in added_lines_with_files(diff)}
    limitations_touched = any(
        p.endswith("docs/limitations.md") or p == "docs/limitations.md" or p == "limitations.md"
        for p in changed_set
    )
    warnings: list[str] = []
    if limitations_touched:
        print("The diff carries honest-boundary markers — limitations.md IS in the changed set.")
        print("The maintainer review verifies the row(s) match the marker(s);")
        print("if the divergence is security-relevant — the release-block label + the PR flag.")
        for hit in hits[:10]:
            print(f"  marker: {hit}")
        return 0, [], len(hits)
    warnings.append(
        "Honest-boundary markers in this diff, but docs/limitations.md is NOT "
        "among the changed files (naryad №588, audit d63cc1d §8):"
    )
    for hit in hits[:10]:
        warnings.append(f"  - {hit}")
    if len(hits) > 10:
        warnings.append(f"  ... and {len(hits) - 10} more")
    warnings.append(
        "The protocol: add a limitations.md row in THIS PR; if the divergence "
        "is security-relevant — the release-block label on the linking issue "
        "+ a flag in the PR description."
    )
    return 0, warnings, len(hits)


def self_test() -> int:
    """The pinned cases (the gate tests itself — no stub work)."""
    cases: list[tuple[str, str, list[str] | None, int, int]] = [
        # (name, diff, changed, expected_warnings, expected_flagged_markers)
        (
            "the b678def class: a marker in an added comment, no limitations.md",
            "diff --git a/src/vm.rs b/src/vm.rs\n"
            "--- a/src/vm.rs\n"
            "+++ b/src/vm.rs\n"
            "@@ -1,3 +1,5 @@\n"
            "+    // honest boundary: pre-existing divergence in the route tail\n",
            ["src/vm.rs"],
            1, 1,
        ),
        (
            "the protocol satisfied: the marker + limitations.md in the same diff",
            "diff --git a/src/vm.rs b/src/vm.rs\n"
            "--- a/src/vm.rs\n"
            "+++ b/src/vm.rs\n"
            "@@ -1,3 +1,5 @@\n"
            "+    // Honest boundary (documented in docs/limitations.md)\n"
            "diff --git a/docs/limitations.md b/docs/limitations.md\n"
            "--- a/docs/limitations.md\n"
            "+++ b/docs/limitations.md\n"
            "@@ -1,3 +1,5 @@\n"
            "+| The route-tail divergence | #123 | open |\n",
            ["src/vm.rs", "docs/limitations.md"],
            0, 1,
        ),
        (
            "plain prose is NOT a marker",
            "diff --git a/src/main.rs b/src/main.rs\n"
            "--- a/src/main.rs\n"
            "+++ b/src/main.rs\n"
            "@@ -1,3 +1,5 @@\n"
            "+    // pre-existing behavior unchanged; see the audit notes\n",
            ["src/main.rs"],
            0, 0,
        ),
        (
            "the 'known divergence' form",
            "diff --git a/src/llm.rs b/src/llm.rs\n"
            "--- a/src/llm.rs\n"
            "+++ b/src/llm.rs\n"
            "@@ -1,3 +1,5 @@\n"
            "+// KNOWN DIVERGENCE: the VM skips the keep-tail here\n",
            ["src/llm.rs"],
            1, 1,
        ),
        (
            "context lines (no +) never flag — the forward-only boundary",
            "diff --git a/src/audit.rs b/src/audit.rs\n"
            "--- a/src/audit.rs\n"
            "+++ b/src/audit.rs\n"
            "@@ -2445,7 +2445,7 @@\n"
            " //   3. Honest boundary (documented in docs/limitations.md): dynamically\n"
            "+    let x = 1;\n",
            ["src/audit.rs"],
            0, 0,
        ),
    ]
    failed = 0
    for name, diff, changed, want_warn, want_hits in cases:
        code, warnings, got_hits = check(diff, changed)
        got_warn = 1 if warnings else 0
        status = "ok" if (code == 0 and got_warn == want_warn and got_hits == want_hits) else "FAIL"
        if status == "FAIL":
            failed += 1
        print(f"  self-test [{status}] warn={got_warn}/{want_warn} hits={got_hits}/{want_hits}: {name}")
    print(f"self-test: {'ALL PASS' if failed == 0 else f'{failed} FAILED'}")
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--diff", default="", help="the unified diff file ('-' = stdin)")
    parser.add_argument(
        "--changed",
        action="append",
        default=[],
        help="a changed-file path (repeatable; derived from the diff when omitted)",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if not args.diff:
        parser.error("--diff is required (or --self-test)")
    import pathlib

    if args.diff == "-":
        diff = sys.stdin.read()
    else:
        diff = pathlib.Path(args.diff).read_text(encoding="utf-8", errors="replace")
    code, warnings, _hits = check(diff, args.changed or None)
    for line in warnings:
        print(f"::warning::{line}" if not line.startswith("  ") else line)
    return code


if __name__ == "__main__":
    sys.exit(main())
