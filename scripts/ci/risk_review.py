#!/usr/bin/env python3
"""№469 (gh#690): the risk-based review — the mechanical diff surface.

Decision 7-A.2 (gate gh#680): the review goes BY RISK, not by order. PRs
touching the high-risk files (`io.rs`, `server.rs`, `audit.rs`,
`semantic.rs`, `llm*`) or ADDING a `spawn` must go through the checklist
review; the result attaches to the PR MECHANICALLY (this script's report
— the CI job publishes it), never as the writing agent's self-check.

The job is ADVISORY (non-blocking) until a separate owner decision; the
checklist itself lives in `docs/risk-review-checklist.md` (checked in).

This script does the MECHANICAL half only: it maps the changed lines to
the checklist items (item → file:line → observation). The JUDGMENT half
belongs to the reviewer working through the checklist against this
report.

Usage:
  risk_review.py --changed BASE HEAD   → exit 0 triggered / 1 not
  risk_review.py --report BASE HEAD    → the markdown report (stdout)
"""
import re
import subprocess
import sys

ROOT = __file__.rsplit('/scripts/ci/', 1)[0]

# The high-risk perimeter (the audit 25.09 §8.2 p.3).
PERIMETER = (
    "src/io.rs",
    "src/server.rs",
    "src/audit.rs",
    "src/semantic.rs",
)

PERIMETER_PREFIXES = (
    "src/llm",  # llm.rs, llm_stream.rs — the llm* family
)

# The checklist items (the mechanical observation patterns per item).
# 7 items — the naryad names 6 areas; the error surface is the 7th
# (the fail-loud discipline of №454-№460 line).
ITEMS = [
    (
        "1. Execution context (ServeRoute / cron / exec gates)",
        [
            (r"\.route\(", "an HTTP route is touched/added"),
            (r"ServeRoute|serve_route", "a serve-route gate is touched"),
            (r"cron|webhook", "a cron/webhook tick path is touched"),
            (r"exec_context|ExecutionContext", "an execution-context gate is touched"),
        ],
    ),
    (
        "2. Result confidentiality labels",
        [
            (r"Secret|SECRET", "a Secret value/label path is touched"),
            (r"labels?\b|Label::", "a label assignment is touched"),
            (r"canary", "canary marking/checking is touched"),
        ],
    ),
    (
        "3. Default behavior (fail-open / fail-closed)",
        [
            (r"unwrap_or\(|unwrap_or_default\(|unwrap_or_else\(",
             "a default on failure — check it fails CLOSED where gated"),
            (r"\bdefault\b.*=>|Default for", "a Default impl or default arm is touched"),
        ],
    ),
    (
        "4. NaN / empty / unbounded values",
        [
            (r"NaN|is_nan\(", "a NaN path is touched"),
            (r"unwrap\(\)", "an unwrap — can it panic on empty/None input?"),
            (r"\[\.\.\d+\]|\.take\(|\.truncate\(", "a bound/slice is touched"),
        ],
    ),
    (
        "5. TW/VM parity (stateful builtin names)",
        [
            (r"\"[a-z_]{4,}\"", "a builtin-name-like literal — does it appear on BOTH backends? (the №462 counter holds the fact)"),
        ],
    ),
    (
        "6. Core joints (new routes / bypasses / effects)",
        [
            (r"\.spawn\(", "a spawn is ADDED — the async/joint surface"),
            (r"std::process|Command::new", "a process is spawned/execed"),
            (r"std::fs::|File::open|read_to_string", "a filesystem read/write is touched"),
            (r"reqwest|TcpStream|UdpSocket", "a network surface is touched"),
            (r"ledger|audit_event|record\(", "an audit/ledger record is touched"),
        ],
    ),
    (
        "7. The error surface (fail-loud discipline)",
        [
            (r'Ok\(Value::Unit\)|Ok\(""[\),]', "an empty/Unit success — is the failure silent?"),
            (r"eprintln!|warn!", "a stderr/warn path — loud enough for the refusal?"),
        ],
    ),
]


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True).stdout


def changed_lines(base, head):
    """{path: [(lineno, '+'-side text)]} for the changed files."""
    diff = run(["git", "diff", "--unified=0", f"{base}...{head}"])
    files = {}
    path = None
    lineno = 0
    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            path = line[6:]
        elif line.startswith("@@"):
            m = re.search(r"\+(\d+)", line)
            if m:
                lineno = int(m.group(1))
        elif line.startswith("+") and not line.startswith("+++"):
            if path:
                files.setdefault(path, []).append((lineno, line[1:]))
                lineno += 1
    return files


def in_perimeter(path):
    return path in PERIMETER or path.startswith(PERIMETER_PREFIXES)


def main():
    args = sys.argv[1:]
    base = args[args.index("--base")] if "--base" in args else None
    i = args.index("--changed") if "--changed" in args else (
        args.index("--report") if "--report" in args else None
    )
    if i is None:
        print(__doc__)
        sys.exit(2)
    base, head = args[i + 1], args[i + 2]
    mode = "--changed" if "--changed" in args else "--report"

    files = changed_lines(base, head)
    perimeter_hits = sorted(p for p in files if in_perimeter(p))
    spawn_hits = sorted(
        (p, n, t)
        for p, lines in files.items()
        if p.endswith(".rs")
        for n, t in lines
        if ".spawn(" in t
    )
    triggered = bool(perimeter_hits or spawn_hits)

    if mode == "--changed":
        for p in perimeter_hits:
            print(f"perimeter: {p}")
        for p, n, _ in spawn_hits:
            print(f"spawn-added: {p}:{n}")
        print(f"triggered: {triggered}")
        sys.exit(0 if triggered else 1)

    # --report
    if not triggered:
        print(
            "risk-review: NOT TRIGGERED — the PR touches no high-risk file "
            "(io/server/audit/semantic/llm*) and adds no spawn."
        )
        return
    rows = []
    # The boundary rule of №469: the review does not scan files OUTSIDE
    # the perimeter — the scan set is the perimeter hits plus the .rs
    # files that ADD a spawn (the trigger surface itself).
    scan_set = sorted(set(perimeter_hits) | {p for p, _, _ in spawn_hits})
    for title, patterns in ITEMS:
        for path in scan_set:
            for lineno, text in files[path]:
                for pat, note in patterns:
                    if re.search(pat, text):
                        rows.append((title, f"{path}:{lineno}", f"`{text.strip()[:90]}` — {note}"))
    print("## Risk-review surface (mechanical, №469)")
    print()
    print(
        f"Triggered by: {', '.join(perimeter_hits) or 'spawn additions'}"
        + (f" + spawn x{len(spawn_hits)}" if spawn_hits and perimeter_hits else "")
    )
    print()
    print("The REVIEWER works through `docs/risk-review-checklist.md` against")
    print("this surface — the table below is the mechanical map, not the verdict.")
    print()
    if rows:
        print("| Checklist item | Location | Observation |")
        print("|---|---|---|")
        for title, loc, obs in rows:
            print(f"| {title} | {loc} | {obs} |")
    else:
        print("No checklist-relevant pattern found in the added lines mechanically; the reviewer still walks the checklist for the touched files.")


if __name__ == "__main__":
    main()
