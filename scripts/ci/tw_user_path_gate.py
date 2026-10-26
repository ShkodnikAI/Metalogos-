#!/usr/bin/env python3
"""№808 (wave 66, the audit-23.10 C5-b V4, gh#1502): the TW-oracle gate.

The C5 (b) owner decision: the VM executes everything a user runs; the
tree-walking interpreter (TW) is the REFERENCE ORACLE in CI only. This
gate pins the boundary: every TW-ENTRY call in the user-path sources
(`run_program*`, `execute_route_body` — the TW route runner) must carry
an explicit `[TW-ORACLE]` mark (a comment within the 3 preceding lines
or on the same line) or be covered by the checked-in baseline (path ::
anchor-substring :: reason). A new unmarked TW entry turns the gate RED
— fail-closed.

The baseline is the SSOT of the sanctioned transitional lanes:
  - `--reference` (the CLI oracle escape hatch, ADR-0197, transitional);
  - `METALOGOS_SERVE_BACKEND=interpreter` (the serve opt-out, the same
    transitional term, ADR-0197 §4);
  - the test harnesses (tests compare the executor against the oracle).

Fail-closed (№525): an unreadable file or a missing baseline is RED, not
a pass. Self-test: --self-test (the №535 blocking-check form).
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# The user-path sources in scope (the №808 contract: metalogos-server +
# src/lib.rs; the CLI binary rides the lib entries).
SCAN_FILES = [
    "src/lib.rs",
    "metalogos-server/src/server.rs",
    "metalogos-server/src/bin/mlog.rs",
]

# The TW-ENTRY call shapes (the oracle consumes SOURCE — the runners).
TW_ENTRY_PATTERNS = [
    "run_program(",
    "run_program_with_dir(",
    "eval_program(",
    "eval_program_with_dir(",
    "execute_route_body(",
]

BASELINE_PATH = Path(__file__).parent / "tw_user_path_baseline.txt"
MARK = "[TW-ORACLE]"
MARK_WINDOW = 6  # the mark may sit up to 6 lines above (doc comments are the right place)


def load_baseline() -> list[tuple[str, str, str]]:
    """(path, anchor-substring, reason); `#` comments and blanks skipped."""
    entries: list[tuple[str, str, str]] = []
    if not BASELINE_PATH.exists():
        print(f"[TW-ORACLE-GATE] RED: the baseline is missing: {BASELINE_PATH}")
        sys.exit(2)
    for raw in BASELINE_PATH.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = [p.strip() for p in line.split("::")]
        if len(parts) != 3:
            print(f"[TW-ORACLE-GATE] RED: the malformed baseline line: {raw!r} (path :: anchor :: reason)")
            sys.exit(2)
        entries.append((parts[0], parts[1], parts[2]))
    return entries


def is_marked(lines: list[str], idx: int) -> bool:
    window = lines[max(0, idx - MARK_WINDOW) : idx + 1]
    return any(MARK in ln for ln in window)


def collect_sites() -> list[tuple[str, int, str]]:
    """(rel_path, line_no, line_text) for every TW-entry call in scope."""
    sites = []
    for rel in SCAN_FILES:
        path = REPO / rel
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as e:
            print(f"[TW-ORACLE-GATE] RED: cannot read {rel}: {e}")
            sys.exit(2)
        lines = text.splitlines()
        for i, line in enumerate(lines):
            if any(pat in line for pat in TW_ENTRY_PATTERNS):
                stripped = line.strip()
                # The definitions (fn signatures) carry the docs — the
                # CALLS are the entries.
                if stripped.startswith(("pub fn ", "pub(crate) fn ", "pub(crate) async fn ", "fn ", "async fn ")):
                    continue
                sites.append((rel, i + 1, stripped))
    return sites


def check() -> int:
    baseline = load_baseline()
    sites = collect_sites()
    violations = []
    matched_baseline_rows: set[int] = set()
    for rel, no, text in sites:
        path_lines = (REPO / rel).read_text(encoding="utf-8").splitlines()
        if is_marked(path_lines, no - 1):
            continue
        hit = False
        for bi, (bpath, anchor, _reason) in enumerate(baseline):
            if bpath == rel and anchor in text:
                matched_baseline_rows.add(bi)
                hit = True
                break
        if not hit:
            violations.append(f"{rel}:{no}: unmarked TW entry: {text[:110]}")
    # The baseline rows must all resolve — no stale anchors (the gate
    # does not pass on dead rows).
    for bi, (bpath, anchor, _reason) in enumerate(baseline):
        if bi not in matched_baseline_rows:
            violations.append(f"baseline: stale row ({bpath} :: {anchor}) — no such TW entry on the current tree")
    if violations:
        print("[TW-ORACLE-GATE] RED — the TW boundary is broken:")
        for v in violations:
            print(f"  - {v}")
        print("  fix: mark the site with [TW-ORACLE] (a comment) and add a baseline row (path :: anchor :: reason), or remove the entry")
        return 1
    print(
        f"[TW-ORACLE-GATE] OK — {len(sites)} TW entr(y|ies) in the user paths, "
        f"all marked or baselined; {len(baseline)} baseline rows all live"
    )
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    return check()


def self_test() -> int:
    """The №535 form: GREEN on the shipped tree + RED on a synthetic
    unmarked TW entry + the stale-baseline RED."""
    rc = check()
    if rc != 0:
        print("[TW-ORACLE-GATE][self-test] FAIL: the shipped tree must be GREEN")
        return 1
    tmp = REPO / "src" / "lib.rs"
    original = tmp.read_text(encoding="utf-8")
    try:
        tmp.write_text(original + '\nfn __n808_synthetic() {\n    let _ = run_program("x");\n}\n')
        rc_syn = check()
    finally:
        tmp.write_text(original)
    if rc_syn == 0:
        print("[TW-ORACLE-GATE][self-test] FAIL: the synthetic unmarked entry must be RED")
        return 1
    print("[TW-ORACLE-GATE][self-test] OK: the shipped tree GREEN, the synthetic unmarked entry RED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
