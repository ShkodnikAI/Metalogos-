#!/usr/bin/env python3
"""The honest-boundary protocol check (Naryad №588, audit d63cc1d §8) →
№604 (the audit 25b375e §3 Y-2 + the §7 rule): the SECURITY-RELEVANCE
DICTIONARY + the blocking escalation.

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
  - "honest boundary"          (the tree's canonical form);
  - "pre-existing divergence"  (the audit §8's full phrase — the bare
    "pre-existing" is ordinary English in evidence prose and would
    flood the gate with noise);
  - "known divergence".

THE SECURITY DICTIONARY (№604 — the manual classification failed the
audit's Y-1: the row landed in the same PR by FORM, but the divergence
was never recognized security-relevant — no release-block label, the
fact stayed 0 while the guard bypass was live): a docs/limitations.md
ROW matching any pattern in
`scripts/ci/honest_boundary_security_dict.txt` (one machine-readable,
extensible list — the №586/№606 lesson: no second parser, no parallel
JSON) describes a security-relevant divergence. The classes: a
respond*/deny*/return being ignored; a semantic error or a category-A
check skipped/downgraded; backend-dependent behavior in the №476
domains (SQL, filesystem, exec, labels, secrets).

BLOCKING ON THE DICTIONARY CLASS (the gate-facts-sync образец — the
№604 escalation): an ADDED limitations.md row matching the dictionary
WITHOUT the release-block evidence = exit 1 (the PR is red). The
evidence is one of:
  - the row links an issue (`.../issues/<N>` or `gh#<N>`) whose labels
    include `release-block` (verified via the GitHub API);
  - the PR ITSELF carries the `release-block` label.
Without GH_TOKEN the label verification is a LOUD SKIP (the note is
printed, exit 0 — CI always runs with the token, mirroring
sync_gate_facts.py); a GitHub API error is exit 2 (infra, loud — never
silently OK). The №588 marker/row form gap stays ADVISORY (the
false-positive experience governs it — gh#1002); the SECURITY class is
the one the audit proved must block.

ADDED LINES ONLY in the forward mode (the naryad's boundary). The
`--retrospective` mode scans the WHOLE docs/limitations.md: every
dictionary-matching row must carry its release-block evidence (the
№604 DoD: zero false positives on the existing rows, the synthetic
Y-1 sample caught).

THE CHANGELOG SURFACE (№643, the unified audit 48301708 §3 Q-1 "Что
не так" п.2): the SAME dictionary class extends to CHANGELOG.md — a
security-relevant defect entry that matches the dictionary must lie in
the `### Security` section of its version (the section heading is the
evidence — the section is the release-block carrier for the released
fixes) or carry the release-block evidence like a limitations row
(the same pair of paths: the forward diff mode + the --retrospective
whole-file mode). A dictionary-matching added CHANGELOG entry outside
`### Security` without the evidence = exit 1 (the PR is red). The
`### Security (ADVISORY …)` released form (№620/№629) matches the
section rule too. The live measured boundary (the honest fact the
naryad's claim is corrected by): the LITERAL №629 entry text does NOT
match the current dictionary — its class words ("silent `false`",
"silently-wrong") are not in the machine vocabulary; the mechanism is
built for the dictionary classes as written, and a vocabulary
calibration for that class is a separate evidence-backed PR (the
false-positive budget gh#1002 governs — never extended blindly).

Usage:
    python3 scripts/ci/honest_boundary_check.py --diff <file|-> [--changed <file> ...]
    python3 scripts/ci/honest_boundary_check.py --diff /tmp/pr.diff --pr-number 123
    python3 scripts/ci/honest_boundary_check.py --retrospective

Exit codes: 0 = ok (advisory warnings may print); 1 = the dictionary
class violated (a security-relevant row without the release-block
evidence); 2 = the check itself broke. `--self-test` runs the pinned
cases and exits.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DICT_PATH = os.path.join(HERE, "honest_boundary_security_dict.txt")
REPO = "ShkodnikAI/Metalogos-"
LABEL = "release-block"
ISSUE_REF_RE = re.compile(r"(?i)(?:/issues/|\bissues/|gh#|#)(\d+)")

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


def load_dictionary(path: str = DICT_PATH) -> list[re.Pattern[str]]:
    """The security-relevance patterns (one regex per line, #-comments ok)."""
    patterns: list[re.Pattern[str]] = []
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        patterns.append(re.compile(line, re.IGNORECASE))
    return patterns


def dictionary_match(row: str, patterns: list[re.Pattern[str]]) -> str | None:
    """The matching pattern's source text, or None."""
    for p in patterns:
        if p.search(row):
            return p.pattern
    return None


def issue_refs(text: str) -> list[int]:
    """The issue numbers referenced by a row (issues/<N>, gh#<N>, #<N>)."""
    seen: list[int] = []
    for m in ISSUE_REF_RE.finditer(text):
        n = int(m.group(1))
        if n not in seen:
            seen.append(n)
    return seen


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


# The CHANGELOG heading flow (№643): a `## [` version heading or a
# `### ` section heading; the `### Security` section is the evidence
# carrier (the released-fix section, the №620/№629 ADVISORY form included).
CHANGELOG_HEADING_RE = re.compile(r"^#{2,3}\s+\S", re.UNICODE)
SECURITY_SECTION_RE = re.compile(r"^###\s+Security\b", re.IGNORECASE)
CHANGELOG_VERSION_RE = re.compile(r"^##\s+\[([^\]]+)\]", re.UNICODE)
CHANGELOG_BULLET_RE = re.compile(r"^-\s", re.UNICODE)
UNRELEASED = "Unreleased"


def _changelog_flow(lines, entries):
    """The shared heading/bullet state machine: consumes (kind, text)
    pairs — kind is '+' (added) or ' ' (context) — and fills `entries`
    as (joined-entry-text, under_security, in_unreleased). Added lines
    form entries; context lines advance ONLY the heading state (the
    section an added bullet lands in may be unchanged, hence outside the
    + lines)."""
    state = {"sec": False, "ver": "", "entry": None}

    def flush():
        if state["entry"] is not None:
            entries.append(
                (
                    " ".join(s.strip() for s in state["entry"]),
                    state["sec"],
                    state["ver"] == UNRELEASED,
                )
            )
            state["entry"] = None

    for kind, text in lines:
        ver = CHANGELOG_VERSION_RE.match(text)
        if ver:
            flush()
            state["ver"] = ver.group(1)
            state["sec"] = False
            continue
        heading = CHANGELOG_HEADING_RE.match(text)
        if heading:
            flush()
            state["sec"] = bool(SECURITY_SECTION_RE.match(text))
            continue
        if kind == "+" and CHANGELOG_BULLET_RE.match(text):
            flush()
            state["entry"] = [text]
            continue
        if state["entry"] is not None:
            if not text.strip():
                flush()
            else:
                state["entry"].append(text)
    flush()


def added_changelog_entries(diff: str) -> list[tuple[str, bool, bool]]:
    """(joined-entry-text, under_###_Security, in_[Unreleased]) for the ADDED
    CHANGELOG.md bullet entries. The section attribution follows the
    post-image heading flow: added AND context heading lines both advance
    it, so an entry added UNDER an unchanged `### Security` heading is
    attributed correctly (the №642-class diff). The version scope tracks
    the `## [` heading: only the [Unreleased] entries are the M-3 limbo
    the gate blocks on (a RELEASED section's fix lives IN a release — the
    №562 class does not apply to history; the post-release amendments are
    routed to [Unreleased] by the №620 protocol)."""
    entries: list[tuple[str, bool, bool]] = []
    current_file = "<unknown>"
    in_changelog = False
    flow: list[tuple[str, str]] = []
    for line in diff.splitlines():
        header = FILE_HEADER_RE.match(line)
        if header:
            if in_changelog:
                _changelog_flow(flow, entries)
            flow = []
            current_file = header.group(1)
            in_changelog = current_file.endswith("CHANGELOG.md")
            continue
        if line.startswith(("diff ", "index ", "new file mode", "deleted file mode", "--- ")):
            continue
        if line.startswith("@@"):
            # a hunk boundary inside one file: flush the open entry; the
            # heading state carries over (the best available attribution;
            # the retro mode is the exact one)
            if in_changelog:
                _changelog_flow(flow, entries)
            flow = []
            continue
        if not in_changelog:
            continue
        if line.startswith("+") and not line.startswith("+++"):
            flow.append(("+", line[1:]))
        elif line.startswith(" "):
            flow.append((" ", line[1:]))
    if in_changelog:
        _changelog_flow(flow, entries)
    return entries


def changelog_retro_entries() -> list[tuple[str, bool, bool]]:
    """(joined-entry-text, under_###_Security, in_[Unreleased]) for EVERY
    bullet entry of the whole CHANGELOG.md — the retro mode's exact
    post-image read."""
    root = os.path.dirname(os.path.dirname(HERE))  # scripts/ci → scripts → repo root
    path = os.path.join(root, "CHANGELOG.md")
    entries: list[tuple[str, bool, bool]] = []
    flow: list[tuple[str, str]] = []
    for raw in open(path, encoding="utf-8"):
        line = raw.rstrip("\n")
        if not line.strip():
            flow.append((" ", line))
        else:
            flow.append(("+", line))
    _changelog_flow(flow, entries)
    return entries


def added_limitations_rows(diff: str) -> list[str]:
    """The ADDED rows (joined line-runs) of docs/limitations.md."""
    rows: list[str] = []
    current: list[str] = []
    for path, text in added_lines_with_files(diff):
        if not (path.endswith("docs/limitations.md") or path == "limitations.md"):
            continue
        stripped = text.rstrip("\n")
        if stripped.startswith("| ") or stripped.startswith("|-"):
            current.append(stripped)
        else:
            if current:
                rows.append("\n".join(current))
                current = []
    if current:
        rows.append("\n".join(current))
    return rows


def github_api(path: str, token: str) -> object:
    import urllib.request

    url = f"https://api.github.com/repos/{REPO}/{path}"
    req = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
    })
    return json.load(urllib.request.urlopen(req, timeout=30))


class GithubApiError(RuntimeError):
    """The API broke mid-check — the caller converts it to exit 2
    (infra, loud — never silently OK; the docstring contract)."""


def fetch_issue_labels(issue: int, token: str) -> set[str]:
    try:
        data = github_api(f"issues/{issue}", token)
    except GithubApiError:
        raise
    except Exception as exc:  # noqa: BLE001 — the boundary converts ALL
        # API failures (HTTP, JSON, timeouts) into the loud exit-2 class
        raise GithubApiError(f"issues/{issue}: {exc}") from exc
    return {l["name"] for l in data.get("labels", [])}


def fetch_pr_labels(pr: int, token: str) -> set[str]:
    try:
        data = github_api(f"pulls/{pr}", token)
    except GithubApiError:
        raise
    except Exception as exc:  # noqa: BLE001 — see fetch_issue_labels
        raise GithubApiError(f"pulls/{pr}: {exc}") from exc
    return {l["name"] for l in data.get("labels", [])}


def check(diff: str, changed: list[str] | None) -> tuple[int, list[str], int]:
    """The №588 marker check (advisory, unchanged). Returns
    (exit_code, the warning lines, the flagged marker count)."""
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


def check_security_rows(
    rows: list[str],
    patterns: list[re.Pattern[str]],
    token: str | None,
    pr_number: int | None,
    label_lookup=None,
) -> tuple[int, list[str]]:
    """The №604 dictionary class — the BLOCKING gate.

    Returns (exit_code, the report lines). exit 1 = a dictionary-matching
    row without the release-block evidence; exit 2 = the API broke (infra).
    `label_lookup` overrides the API (the self-test's injected states).
    """
    violations: list[str] = []
    reports: list[str] = []

    def labels_for(issue: int) -> set[str] | None:
        if label_lookup is not None:
            return label_lookup(issue)
        if not token:
            return None
        return fetch_issue_labels(issue, token)

    pr_ok: bool | None = None
    if pr_number and token:
        if label_lookup is not None:
            pr_ok = LABEL in (label_lookup(-pr_number) or set())
        else:
            pr_ok = LABEL in fetch_pr_labels(pr_number, token)
    skipped_api = False
    for row in rows:
        matched = dictionary_match(row, patterns)
        if not matched:
            continue
        refs = issue_refs(row)
        evidence: str | None = None
        if pr_ok:
            evidence = "the PR carries release-block"
        else:
            for n in refs:
                labels = labels_for(n)
                if labels is None:
                    skipped_api = True
                    break
                if LABEL in labels:
                    evidence = f"the linked issue #{n} carries release-block"
                    break
        if evidence:
            reports.append(f"security row OK ({evidence}): {row[:120]}")
            continue
        if not token and not label_lookup:
            print(
                "LOUD SKIP: GH_TOKEN absent — the release-block evidence of a "
                "security-relevant boundary row is NOT verified here (CI always "
                "runs this check with the token; the ADR-0179 §6 step-1 sync "
                "stays a release-time human step)."
            )
            print(f"  security row (unverified): {row[:150]}")
            continue
        violations.append(
            "A security-relevant boundary row (the №604 dictionary: "
            f"'{matched[:60]}...') carries NO release-block evidence — the "
            "linking issue lacks the label and the PR does not carry it "
            "(naryad №604, the audit 25b375e §3 Y-2; the gate-facts-sync "
            "образец):"
        )
        violations.append(f"  row: {row[:200]}")
        if refs:
            violations.append(f"  linked issues checked: {refs}")
        else:
            violations.append(
                "  the row references NO issue — link the release-block carrier "
                "in the row (the machine-verifiable chain)."
            )
    if skipped_api and not violations:
        return 0, reports
    if violations:
        return 1, violations
    return 0, reports


def check_changelog_security(
    diff: str,
    patterns: list[re.Pattern[str]],
    token: str | None,
    pr_number: int | None,
    label_lookup=None,
) -> tuple[int, list[str], int, int]:
    """The №643 CHANGELOG class — the SAME blocking machinery.

    The blocking scope (the fact-backed calibration, the M-3/№562
    definition: the limbo is "a fix lives outside a RELEASE"): an added
    dictionary-matching entry in [Unreleased] OUTSIDE `### Security`
    without the release-block evidence = exit 1. The entries UNDER
    `### Security` (of any version, the №620 ADVISORY form included)
    pass by the section itself; the entries in RELEASED sections pass
    by the release (the history is verbatim, №620; the post-release
    amendments are routed to [Unreleased] by the protocol).

    Returns (exit_code, the report lines, the total added entries, the
    under-Security added entries)."""
    entries = added_changelog_entries(diff)
    pending_outside = [
        text
        for text, sec, unreleased in entries
        if unreleased and not sec
    ]
    under = sum(1 for _, sec, _u in entries if sec)
    released = sum(1 for _, sec, u in entries if not sec and not u)
    reports: list[str] = []
    if under:
        reports.append(
            f"changelog: {under} added entr(y|ies) under `### Security` — "
            "PASS by the section (№643)"
        )
    if released:
        reports.append(
            f"changelog: {released} added entr(y|ies) in released sections — "
            "PASS by the release (the M-3 limbo is closed; the №620 protocol "
            "routes the post-release amendments to [Unreleased])"
        )
    code, sec_reports = check_security_rows(
        pending_outside, patterns, token, pr_number, label_lookup
    )
    reports.extend(sec_reports)
    return code, reports, len(entries), under


def retrospective(token: str | None, label_lookup=None) -> int:
    """The №604 DoD run: the WHOLE docs/limitations.md through the gate."""
    root = os.path.dirname(os.path.dirname(HERE))  # scripts/ci → scripts → repo root
    limits = os.path.join(root, "docs", "limitations.md")
    rows = [l.rstrip("\n") for l in open(limits, encoding="utf-8") if l.startswith("| ")]
    patterns = load_dictionary()
    try:
        code, reports = check_security_rows(rows, patterns, token, None, label_lookup)
    except GithubApiError as exc:
        print(f"::error::the GitHub API broke — exit 2 (infra, loud): {exc}")
        return 2
    for line in reports:
        print(f"  {line}")
    matches = sum(1 for r in rows if dictionary_match(r, patterns))
    print(f"retrospective: {len(rows)} rows scanned, {matches} security-dictionary matches")
    print(f"retrospective: {'IN SYNC' if code == 0 else 'VIOLATIONS — exit 1'}")

    # ── the №643 CHANGELOG retro: every dictionary-matching entry in
    #    [Unreleased] must lie in a `### Security` section (the M-3
    #    scope: the released sections pass by the release; the full
    #    outside-Security inventory is printed for transparency) ──
    cl_entries = changelog_retro_entries()
    cl_matches = sum(1 for t, _s, _u in cl_entries if dictionary_match(t, patterns))
    cl_under = sum(
        1 for t, s, _u in cl_entries if s and dictionary_match(t, patterns)
    )
    cl_pending = [
        t
        for t, s, u in cl_entries
        if u and not s and dictionary_match(t, patterns)
    ]
    cl_outside = [
        t for t, s, u in cl_entries if not s and dictionary_match(t, patterns)
    ]
    print(
        f"changelog retrospective: {len(cl_entries)} entries scanned, "
        f"{cl_matches} security-dictionary matches ({cl_under} under "
        f"`### Security`, {len(cl_outside)} outside — all in released "
        f"sections except {len(cl_pending)} in [Unreleased])"
    )
    for t in cl_pending:
        print(f"  changelog dictionary match OUTSIDE ### Security in [Unreleased]: {t[:160]}")
    print(
        "changelog retrospective: "
        + ("IN SYNC" if not cl_pending else "VIOLATIONS — exit 1")
    )
    if code != 0 or cl_pending:
        return 1
    return 0


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

    # ── the №604 dictionary cases (the injected label states — offline) ──
    patterns = load_dictionary()
    y1_row = (
        "| Since №600 a bare `respond*` NESTED under a top-level block-form if/else "
        "does NOT stop the route on either backend — the blocking RESPOND_SWALLOWED "
        "refusal; the carrier [ShkodnikAI/Metalogos-/issues/1041](https://github.com/ShkodnikAI/Metalogos-/issues/1041) |"
    )
    benign_row = (
        "| The memory builtins run on TWO ENGINES: `memorize`/`recall`/`forget` — "
        "different stores, the same names |"
    )

    def sec_case(name, want_code, rows, lookup):
        nonlocal failed
        code, _reports = check_security_rows(rows, patterns, "fake-token", None, lookup)
        status = "ok" if code == want_code else "FAIL"
        if status == "FAIL":
            failed += 1
        print(f"  self-test [{status}] code={code}/{want_code}: {name}")

    # the synthetic Y-1 sample WITH the release-block carrier — passes
    sec_case(
        "the Y-1 row with the release-block carrier (#1041)",
        0, [y1_row],
        lambda n: {LABEL} if n == 1041 else set(),
    )
    # the synthetic Y-1 sample WITHOUT the label — BLOCKED (the miss test)
    sec_case(
        "the Y-1 row WITHOUT release-block — the blocking miss test",
        1, [y1_row],
        lambda n: set(),
    )
    # the Y-1 row, no evidence at all (no refs) — BLOCKED
    sec_case(
        "a dictionary row referencing no issue — blocked",
        1, ["| a bare respond* nested under a block-if does NOT stop the route |"],
        lambda n: {LABEL},
    )
    # the benign memory row (NOT in the №476 dictionary) — no flag
    sec_case(
        "the memory-pairs row is NOT the security class (zero false positives)",
        0, [benign_row],
        lambda n: set(),
    )
    # the PR-level evidence: the row without refs, but the PR carries the label
    code, _ = check_security_rows(
        ["| a bare respond* nested under a block-if does NOT stop the route |"],
        patterns, "fake-token", 777,
        lambda n: {LABEL} if n == -777 else set(),
    )
    status = "ok" if code == 0 else "FAIL"
    if status == "FAIL":
        failed += 1
    print(f"  self-test [{status}] code={code}/0: the PR-level release-block evidence")

    # ── the №643 CHANGELOG cases (the injected label states — offline) ──
    y1_entry = (
        "- **the Y-1 class (gh#1041):** a bare `respond*` NESTED under a "
        "top-level block-form if/else does NOT stop the route on either "
        "backend — the blocking RESPOND_SWALLOWED refusal"
    )

    def cl_case(name, want_code, diff_text, lookup=None):
        nonlocal failed
        c, _r, _n, _u = check_changelog_security(
            diff_text, patterns, "fake-token", None, lookup
        )
        st = "ok" if c == want_code else "FAIL"
        if st == "FAIL":
            failed += 1
        print(f"  self-test [{st}] code={c}/{want_code}: {name}")

    # the synthetic №629-class line OUTSIDE ### Security — caught (exit 1)
    cl_case(
        "the dictionary-class CHANGELOG entry OUTSIDE ### Security — caught",
        1,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,9 @@\n"
        " ## [Unreleased]\n"
        " \n"
        f"+{y1_entry}\n",
        lambda n: set(),
    )
    # the same entry UNDER ### Security — PASS by the section
    cl_case(
        "the same entry UNDER ### Security — PASS by the section",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,10 @@\n"
        " ## [Unreleased]\n"
        " \n"
        "+### Security\n"
        "+\n"
        f"+{y1_entry}\n",
        lambda n: set(),
    )
    # the released ADVISORY form (№620) — PASS by the section rule
    cl_case(
        "the entry under the released ADVISORY ### Security form — PASS",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,10 @@\n"
        " ## [0.29.0] - 2026-10-06\n"
        " \n"
        "+### Security (ADVISORY — restart your tests)\n"
        "+\n"
        f"+{y1_entry}\n",
        lambda n: set(),
    )
    # an entry added under an UNCHANGED (context) ### Security heading
    cl_case(
        "the entry under the unchanged context ### Security — attributed right",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,9 @@\n"
        " ### Security\n"
        " \n"
        f"+{y1_entry}\n",
        lambda n: set(),
    )
    # the release-block evidence still rescues an outside-Security entry
    cl_case(
        "the outside-Security entry WITH the release-block carrier — PASS",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,9 @@\n"
        " ## [Unreleased]\n"
        " \n"
        f"+{y1_entry}\n",
        lambda n: {LABEL} if n == 1041 else set(),
    )
    # the benign entry (no dictionary words) — zero false positives
    cl_case(
        "the benign CHANGELOG entry — zero false positives",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,9 @@\n"
        " ## [Unreleased]\n"
        " \n"
        "+- **№633 (gh#1101) — the dead_code burn (internal):** 33 → 15\n"
        "+  (18 places removed or justified), the floor re-locked in the same PR.\n",
        lambda n: set(),
    )
    # the №653 fixture: the LITERAL №629 text matches the class-4
    # vocabulary now — OUTSIDE ### Security it is caught (exit 1); the
    # №643 measured boundary is closed, the evidence-backed way
    cl_case(
        "the №629 fixture (class-4) OUTSIDE ### Security — caught (№653)",
        1,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,12 @@\n"
        " ## [Unreleased]\n"
        " \n"
        "+- **№629 (gh#1096) — the VM comparison parity:** the legacy VM\n"
        "+  answered a silent `false` (and `true` for numeric strings) — the\n"
        "+  silently-wrong class; the fix landed on main 2026-10-07.\n",
        lambda n: set(),
    )
    # the same fixture UNDER ### Security — PASS by the section (№643)
    cl_case(
        "the №629 fixture UNDER ### Security — PASS by the section",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,14 @@\n"
        " ## [Unreleased]\n"
        " \n"
        "+### Security\n"
        "+\n"
        "+- **№629 (gh#1096) — the VM comparison parity:** the legacy VM\n"
        "+  answered a silent `false` (and `true` for numeric strings) — the\n"
        "+  silently-wrong class; the fix landed on main 2026-10-07.\n",
        lambda n: set(),
    )
    # the class-4 window is bounded: the silent word and the wrong word
    # FURTHER than 120 chars apart do NOT match (no over-blocking)
    cl_case(
        "the class-4 window: silent and wrong far apart — no match",
        0,
        "diff --git a/CHANGELOG.md b/CHANGELOG.md\n"
        "--- a/CHANGELOG.md\n"
        "+++ b/CHANGELOG.md\n"
        "@@ -10,6 +10,9 @@\n"
        " ## [Unreleased]\n"
        " \n"
        "+- **the changelog notes (internal):** the run stayed silent\n"
        f"+{'the log wire details follow here. ' * 5}"
        "+  and the legacy arithmetic was wrong in the third decimal only.\n",
        lambda n: set(),
    )

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
    parser.add_argument(
        "--pr-number",
        type=int,
        default=None,
        help="the PR number — the PR-level release-block label counts as evidence",
    )
    parser.add_argument("--retrospective", action="store_true",
                        help="scan the WHOLE docs/limitations.md (the №604 DoD)")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.retrospective:
        return retrospective(os.environ.get("GH_TOKEN"))
    if not args.diff:
        parser.error("--diff is required (or --self-test/--retrospective)")
    import pathlib

    if args.diff == "-":
        diff = sys.stdin.read()
    else:
        diff = pathlib.Path(args.diff).read_text(encoding="utf-8", errors="replace")

    code, warnings, _hits = check(diff, args.changed or None)
    for line in warnings:
        print(f"::warning::{line}" if not line.startswith("  ") else line)

    # the №604 dictionary class — blocking on the added limitations rows
    patterns = load_dictionary()
    sec_rows = added_limitations_rows(diff)
    try:
        sec_code, reports = check_security_rows(
            sec_rows, patterns, os.environ.get("GH_TOKEN"), args.pr_number,
        )
    except GithubApiError as exc:
        print(f"::error::the GitHub API broke — exit 2 (infra, loud): {exc}")
        return 2
    for line in reports:
        print(line)
    if sec_code != 0:
        for line in reports:
            print(f"::error::{line}" if not line.startswith("  ") else line)
        return sec_code

    # the №643 CHANGELOG class — the SAME dictionary, the ### Security
    # section as the section-evidence (the CI call site is unchanged: the
    # class runs INSIDE this script, the №643 boundary)
    try:
        cl_code, cl_reports, _n, _under = check_changelog_security(
            diff, patterns, os.environ.get("GH_TOKEN"), args.pr_number,
        )
    except GithubApiError as exc:
        print(f"::error::the GitHub API broke — exit 2 (infra, loud): {exc}")
        return 2
    for line in cl_reports:
        print(line)
    if cl_code != 0:
        for line in cl_reports:
            print(f"::error::{line}" if not line.startswith("  ") else line)
        return cl_code
    return code


if __name__ == "__main__":
    sys.exit(main())
