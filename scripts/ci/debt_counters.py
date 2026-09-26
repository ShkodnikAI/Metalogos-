#!/usr/bin/env python3
"""№468 (gh#689): the CI debt gate — the ignore/dead_code counters vs the checked-in fact.

Heuristic (documented, checked-in, reproducible):
  1. `#[ignore]` — every `#[ignore` attribute found on the pre-comment part
     of a line in src/, tests/, benches/ (`//`-suffix stripped first, so a
     commented-out attribute never counts). The TODO subset — the attribute
     line or either of the two following lines (RAW text, comments kept)
     containing "TODO": the audit's phase23/flaky/webhook markers carry the
     TODO in the adjacent comment; the 3-line window is the documented
     approximation, identical to the one that produced the checked-in fact.
  2. `allow(dead_code)` — every `allow(dead_code)` attribute found on the
     pre-comment part of a line in the same trees.
  3. The TW/VM duplicate-name count is NOT re-counted here — the №462
     script and its baseline remain the single source of truth; the gate
     re-uses the №462 script verbatim (subprocess, same interpreter) with
     the baseline path recorded in the fixture (`dup_baseline:` key).

The thresholds are the repository's CURRENT FACT at the naryad's landing
(2026-09-26: after Wave 17's №466 groups 1-2 and the №465 RuntimeContext
removal): ignore 85, ignore-with-TODO 49, dead_code 37. The audit 25.09
reported ignore = 112 — that number included 27 comment/doc PROSE mentions
of `#[ignore]`; the counter counts the attribute occurrences only (the
reconciliation lives in the baseline header, the naryad's "the executor
verifies the fact"). They move ONLY
DOWN (the owner's hygiene strengthening; the audit 25.09 §8.2.1 rule: no
new feature naryads while the debt is above the threshold — bugfix /
security / docs naryads are EXEMPT from the gate). Every downward revision
is a separate line in the naryad report that earned it (the test-hygiene
line rides in the Wave 18 queue).

Usage:
  debt_counters.py                          → prints the counts
  debt_counters.py --list [ignore|dead_code]→ prints the file:line inventory
  debt_counters.py --gate BASELINE          → exit 1 if any counter > threshold
"""
import glob
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TREES = ['src', 'tests', 'benches']

IGNORE_RE = re.compile(r'#\[ignore')
DEAD_CODE_RE = re.compile(r'allow\(dead_code\)')


def rs_files():
    for tree in TREES:
        for path in sorted(glob.glob(os.path.join(ROOT, tree, '**', '*.rs'), recursive=True)):
            yield path


def strip_line_suffix(line):
    """Strip the `//...` suffix so a commented-out attribute never counts."""
    return line.split('//', 1)[0]


def collect():
    """Return (ignores, ignores_todo, dead_codes) as file:line inventories."""
    ignores, ignores_todo, dead_codes = [], [], []
    for path in rs_files():
        rel = os.path.relpath(path, ROOT)
        lines = open(path, encoding='utf-8', errors='replace').read().splitlines()
        for i, line in enumerate(lines):
            code = strip_line_suffix(line)
            if IGNORE_RE.search(code):
                ignores.append(f'{rel}:{i + 1}')
                window = '\n'.join(lines[i:i + 3])
                if 'TODO' in window:
                    ignores_todo.append(f'{rel}:{i + 1}')
            if DEAD_CODE_RE.search(code):
                dead_codes.append(f'{rel}:{i + 1}')
    return ignores, ignores_todo, dead_codes


def parse_baseline(path):
    counters, dup_baseline = {}, None
    for line in open(path, encoding='utf-8'):
        line = line.strip()
        m = re.match(r'^(ignore|ignore_todo|dead_code):\s*(\d+)$', line)
        if m:
            counters[m.group(1)] = int(m.group(2))
            continue
        m = re.match(r'^dup_baseline:\s*(.+)$', line)
        if m:
            dup_baseline = m.group(1).strip()
    if set(counters) != {'ignore', 'ignore_todo', 'dead_code'} or not dup_baseline:
        sys.exit(f'debt baseline {path}: expected ignore/ignore_todo/dead_code keys and a dup_baseline path')
    return counters, dup_baseline


def main():
    ignores, ignores_todo, dead_codes = collect()
    counts = {'ignore': len(ignores), 'ignore_todo': len(ignores_todo), 'dead_code': len(dead_codes)}
    argv = sys.argv[1:]

    if not argv:
        for key in ('ignore', 'ignore_todo', 'dead_code'):
            print(f'{key}: {counts[key]}')
        return 0

    if argv[0] == '--list':
        which = argv[1] if len(argv) > 1 else 'ignore'
        inventory = {'ignore': ignores, 'ignore_todo': ignores_todo, 'dead_code': dead_codes}.get(which)
        if inventory is None:
            sys.exit(f'--list: unknown counter {which!r} (ignore|ignore_todo|dead_code)')
        print('\n'.join(inventory))
        return 0

    if argv[0] == '--gate':
        if len(argv) < 2:
            sys.exit('--gate: a baseline path is required')
        thresholds, dup_baseline = parse_baseline(argv[1])

        failures = []
        for key in ('ignore', 'ignore_todo', 'dead_code'):
            threshold = thresholds[key]
            if counts[key] > threshold:
                failures.append((key, counts[key], threshold))
            else:
                print(f'{key}: {counts[key]} (threshold {threshold}) OK')

        # The №462 duplicate-name gate, re-used verbatim (single source of
        # truth for the dup counter and its threshold).
        dup_script = os.path.join(ROOT, 'scripts', 'ci', 'count_duplicated_names.py')
        dup_run = subprocess.run(
            [sys.executable, dup_script, '--gate', os.path.join(ROOT, dup_baseline)],
            capture_output=True, text=True)
        dup_line = (dup_run.stdout.strip().splitlines() or [''])[0]
        if dup_run.returncode == 0:
            print(f'dup (via №462 gate): {dup_line} OK')
        else:
            failures.append(('dup (via №462 gate)', dup_line, 'its baseline'))

        if failures:
            print('DEBT GATE FAILED — the fact moved UP; the owner rule (audit 25.09 §8.2.1):')
            print('no new FEATURE naryads while the debt is above the threshold')
            print('(bugfix / security / docs naryads are exempt).')
            for key, value, threshold in failures:
                print(f'  {key}: fact {value} > threshold {threshold}')
                for site in {'ignore': ignores, 'ignore_todo': ignores_todo, 'dead_code': dead_codes}.get(key, []):
                    print(f'    {site}')
            return 1
        print('DEBT GATE OK — every counter at or below its threshold.')
        return 0

    sys.exit(f'unknown mode: {argv[0]!r}')


if __name__ == '__main__':
    sys.exit(main())
