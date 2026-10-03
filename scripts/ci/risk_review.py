#!/usr/bin/env python3
"""№469 (gh#690): the risk-based review — the mechanical diff surface.

Decision 7-A.2 (gate gh#680): the review goes BY RISK, not by order. PRs
touching the high-risk files (`io.rs`, `server.rs`, `audit.rs`,
`semantic.rs`, `llm*`) or ADDING a `spawn` must go through the checklist
review; the result attaches to the PR MECHANICALLY (this script's report
— the CI job publishes it), never as the writing agent's self-check.

The job is ADVISORY (non-blocking) until a separate owner decision; the
checklist itself lives in `docs/risk-review-checklist.md` (checked in).

№480 (gh#728): the report also computes the SAME-EFFECT PATHS section —
the three mechanical lists (raw `std::fs::` calls, stateful-name backend
parity, ALL-CAPS env literals without the METALOGOS_ prefix) the
checklist item 8 requires every completion report to close or justify.

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
    # №567 (gh#931): server.rs moved with the transport crate; the
    # perimeter follows the file.
    "metalogos-server/src/server.rs",
    "src/audit.rs",
    "src/semantic.rs",
)

PERIMETER_PREFIXES = (
    "src/llm",  # llm.rs, llm_stream.rs — the llm* family
)

# The checklist items (the mechanical observation patterns per item).
# 8 items — the naryad names 6 areas; the error surface is the 7th
# (the fail-loud discipline of №454-№460 line); item 8 is the
# same-effect method rule of №480 (the section below computes its
# three lists).
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


# ── №480: the SAME-EFFECT PATHS lists (the checklist item 8) ────────────

VM_PATH = "src/vm.rs"
TW_PATHS = ("src/interpreter/",)


def same_effect_lists(files):
    """The three mechanical lists of the №480 rule, computed over the
    ADDED lines of the diff (.rs files):

    1. raw filesystem calls — every `std::fs::` in the added lines;
    2. stateful-name parity — builtin-name-like string literals, each
       mechanically grepped against BOTH backends (src/vm.rs vs
       src/interpreter/): which side knows the name;
    3. ALL-CAPS env literals — every env-name-like literal in the added
       lines, flagged when it lacks the METALOGOS_ prefix (the №758
       class: `env("DATABASE_URL")` is invisible to a prefixed grep).
    """
    fs_hits = []
    name_hits = {}
    env_hits = []
    for path, lines in sorted(files.items()):
        if not path.endswith(".rs"):
            continue
        for n, text in lines:
            if "std::fs::" in text:
                fs_hits.append(f"{path}:{n}")
            for m in re.finditer(r'"([a-z][a-z0-9_]{3,})"', text):
                name_hits.setdefault(m.group(1), []).append(f"{path}:{n}")
            for m in re.finditer(r'"([A-Z][A-Z0-9_]{2,})"', text):
                env_hits.append((m.group(1), f"{path}:{n}"))
    parity = {}
    for name, locs in name_hits.items():
        def count(args):
            out = run(["git", "grep", "-c", name, "--"] + list(args)).strip()
            if not out:
                return 0
            total = 0
            for line in out.splitlines():
                try:
                    total += int(line.rsplit(":", 1)[-1])
                except ValueError:
                    pass
            return total
        parity[name] = (count([VM_PATH]) > 0, count([TW_PATHS[0]]) > 0, locs)
    return fs_hits, parity, env_hits


def same_effect_section(files):
    fs_hits, parity, env_hits = same_effect_lists(files)
    out = []
    out.append("## Same-effect paths (№480 — checklist item 8)")
    out.append("")
    out.append(
        "Close each entry at a COMMON POINT or justify it in the completion "
        "report (the three-grep protocol of the checklist)."
    )
    out.append("")
    # 1. filesystem
    if fs_hits:
        out.append("**Raw `std::fs::` in the added lines** (the №475 facade is the common point):")
        out.append("")
        for h in fs_hits:
            out.append(f"- `{h}`")
    else:
        out.append("**Raw `std::fs::` in the added lines:** none.")
    out.append("")
    # 2. backend parity
    single_sided = {n: v for n, v in parity.items() if v[0] != v[1]}
    if parity:
        if single_sided:
            out.append(
                "**Stateful-name literals known to exactly ONE backend** "
                "(close or justify — the №462 counter holds the fact):"
            )
            out.append("")
            for name, (vm, tw, locs) in sorted(single_sided.items()):
                side = "vm only" if vm else "tw only"
                out.append(f"- `{name}` ({side}) at {', '.join(locs[:3])}")
        else:
            out.append(
                "**Stateful-name literals:** every name in the diff is known "
                "to both backends mechanically."
            )
    else:
        out.append("**Stateful-name literals:** none in the added lines.")
    out.append("")
    # 3. env literals
    if env_hits:
        out.append(
            "**Env-like ALL-CAPS literals in the added lines** — grep the "
            "TAIL without the `METALOGOS_` prefix across the zone (the №758 "
            "class); unprefixed names are flagged:"
        )
        out.append("")
        for n, loc in env_hits:
            flag = " ← no METALOGOS_ prefix" if not n.startswith("METALOGOS_") else ""
            out.append(f"- `{n}` at `{loc}`{flag}")
    else:
        out.append("**Env-like ALL-CAPS literals:** none in the added lines.")
    out.append("")
    return "\n".join(out)


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
    print()
    # №480: the same-effect lists close the report — item 8's protocol.
    print(same_effect_section(files))


if __name__ == "__main__":
    main()
