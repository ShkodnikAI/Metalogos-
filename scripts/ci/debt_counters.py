#!/usr/bin/env python3
"""№468 (gh#689): the CI debt gate — the ignore/dead_code counters vs the checked-in fact.

Heuristic (documented, checked-in, reproducible):
  1. `#[ignore]` — every `#[ignore` ATTRIBUTE occurrence in src/, tests/,
     benches/. Since №569 (gh#933) the attribute recognition is strict: the
     pre-comment part of the line must START with `#[ignore` (trimmed) — a
     real attribute always opens its line in this codebase. This closes the
     string-literal false positives (the №569 audit found 12: the
     ignore_reasons_lint.rs fixtures and the naryad_240 print_skip message
     contain the `#[ignore` TEXT inside strings — the old substring match
     counted them as debt). The TODO subset — the attribute line or either
     of the two following lines (RAW text, comments kept) containing "TODO":
     the 3-line window is the documented approximation, identical to the one
     that produced the checked-in fact.
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
            # №569 (gh#933): strict attribute recognition — the trimmed line
            # must START with `#[ignore`. The old substring match counted the
            # `#[ignore` TEXT inside string literals (12 false positives: the
            # ignore_reasons_lint.rs fixtures, the naryad_240 print_skip
            # message) as debt.
            if code.lstrip().startswith('#[ignore'):
                ignores.append(f'{rel}:{i + 1}')
                window = '\n'.join(lines[i:i + 3])
                if 'TODO' in window:
                    ignores_todo.append(f'{rel}:{i + 1}')
            if DEAD_CODE_RE.search(code):
                dead_codes.append(f'{rel}:{i + 1}')
    return ignores, ignores_todo, dead_codes


# №531 (issue #840; audit 30.09 N-9): the read_file soft-missing TRANSITION
# counter — the occurrences of the stable `READ_FILE_MISSING` marker (the
# stderr warning in io.rs + the tests that pin the transition contract).
# The marker exists ONLY during the transition release: the flip to the
# loud [IO_ERROR] refusal deletes the warning site and the pins, the
# counter reaches 0, the threshold follows it down and the class closes.
READ_FILE_MISSING_RE = re.compile(r'READ_FILE_MISSING')


def read_file_missing_inventory():
    sites = []
    for path in rs_files():
        rel = os.path.relpath(path, ROOT)
        lines = open(path, encoding='utf-8', errors='replace').read().splitlines()
        for i, line in enumerate(lines):
            if READ_FILE_MISSING_RE.search(line):
                sites.append(f'{rel}:{i + 1}')
    return sites


# №513 (gh#797): an example WITHOUT a check is the audit 28.09 C-09 debt:
# no golden sidecar, no named check in the repo's check-bearing trees,
# no COMPAT-N header tag (the honest removal the issue sanctions).
# The counter moves ONLY DOWN (the №468 hygiene rule).
TEXT_SUFFIXES = ('.rs', '.yml', '.yaml', '.py', '.txt', '.toml', '.md')


def example_uncovered_inventory():
    """Return examples/*.mlog files with NO sidecar, NO mention, NO COMPAT tag."""
    haystack_parts = []
    for tree in ('tests', 'benches', 'scripts', '.github'):
        for path in sorted(glob.glob(os.path.join(ROOT, tree, '**', '*'), recursive=True)):
            if os.path.isfile(path) and path.endswith(TEXT_SUFFIXES):
                try:
                    haystack_parts.append(open(path, encoding='utf-8', errors='replace').read())
                except OSError:
                    pass
    haystack = '\n'.join(haystack_parts)
    uncovered = []
    for path in sorted(glob.glob(os.path.join(ROOT, 'examples', '*.mlog'))):
        base = os.path.splitext(path)[0]
        if os.path.exists(base + '.expected') or os.path.exists(base + '.error'):
            continue
        stem = os.path.splitext(os.path.basename(path))[0]
        if stem in haystack:
            continue
        # №533: the COMPAT-tag skip is GONE — the stale-syntax examples
        # live in examples/compat/ (excluded by PATH: this glob is
        # top-level only); a tagged file in the LIVE catalog is debt.
        uncovered.append(os.path.relpath(path, ROOT))
    return uncovered


def parse_baseline(path):
    counters, dup_baseline = {}, None
    for line in open(path, encoding='utf-8'):
        line = line.strip()
        m = re.match(r'^(ignore|ignore_todo|dead_code|example_uncovered):\s*(\d+)$', line)
        if m:
            counters[m.group(1)] = int(m.group(2))
            continue
        m = re.match(r'^dup_baseline:\s*(.+)$', line)
        if m:
            dup_baseline = m.group(1).strip()
    if not {'ignore', 'ignore_todo', 'dead_code'} <= set(counters) or not dup_baseline:
        sys.exit(f'debt baseline {path}: expected ignore/ignore_todo/dead_code keys (and optional example_uncovered) plus a dup_baseline path')
    return counters, dup_baseline


def main():
    ignores, ignores_todo, dead_codes = collect()
    uncovered_examples = example_uncovered_inventory()
    read_file_missing = read_file_missing_inventory()
    counts = {
        'ignore': len(ignores),
        'ignore_todo': len(ignores_todo),
        'dead_code': len(dead_codes),
        'example_uncovered': len(uncovered_examples),
        'read_file_soft_missing': len(read_file_missing),
    }
    argv = sys.argv[1:]

    if not argv:
        for key in ('ignore', 'ignore_todo', 'dead_code', 'example_uncovered', 'read_file_soft_missing'):
            print(f'{key}: {counts[key]}')
        return 0

    if argv[0] == '--list':
        which = argv[1] if len(argv) > 1 else 'ignore'
        inventory = {'ignore': ignores, 'ignore_todo': ignores_todo, 'dead_code': dead_codes, 'example_uncovered': uncovered_examples, 'read_file_soft_missing': read_file_missing}.get(which)
        if inventory is None:
            sys.exit(f'--list: unknown counter {which!r} (ignore|ignore_todo|dead_code|example_uncovered|read_file_soft_missing)')
        print('\n'.join(inventory))
        return 0

    if argv[0] == '--gate':
        if len(argv) < 2:
            sys.exit('--gate: a baseline path is required')
        thresholds, dup_baseline = parse_baseline(argv[1])

        failures = []
        for key in ('ignore', 'ignore_todo', 'dead_code', 'example_uncovered', 'read_file_soft_missing'):
            # example_uncovered is a №513 counter: a baseline without the key
            # (pre-№513 baselines) cannot gate it — treat as absent.
            # read_file_soft_missing is a №531 counter: same optional posture.
            if key not in thresholds:
                continue
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
                for site in {'ignore': ignores, 'ignore_todo': ignores_todo, 'dead_code': dead_codes, 'example_uncovered': uncovered_examples, 'read_file_soft_missing': read_file_missing}.get(key, []):
                    print(f'    {site}')
            return 1
        print('DEBT GATE OK — every counter at or below its threshold.')
        return 0

    sys.exit(f'unknown mode: {argv[0]!r}')


if __name__ == '__main__':
    sys.exit(main())
