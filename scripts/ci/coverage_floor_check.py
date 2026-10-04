#!/usr/bin/env python3
"""The coverage floor check (the owner decision 2А, 2026-10-04).

№569 (gh#933) measured the workspace line coverage 76.60% on main
@ e662ffd and proposed the floor; the OWNER fixed it: line ≥ 76%,
mode ADVISORY now, BLOCKING after two consecutive stable waves (the
wave count lives in scripts/ci/coverage_baseline.txt — a wave is
stable when its wave-closing completion-audit lands with the floor
met; the flip is a one-word edit of that record, cited to the two
wave audits).

The parse is pinned to the cargo-llvm-cov --summary-only text table:
the TOTAL row carries the percentages in the fixed order (region
cover, function executed, LINE cover, [branch cover or '-']) — the
LINE cover is the THIRD percentage token (verified against the
cargo-llvm-cov 0.6.16 output; the №569 reading 78.46/70.65/76.60 maps
to the same order).

advisory: a below-floor prints a LOUD ::warning line, exit 0 — the
   drift is visible, the job stays green (the coverage job is
   non-blocking, №445 shape).
blocking: a below-floor exits 1 — the floor is the release-line gate.

Exit codes: 0 = met / advisory below-floor; 1 = blocking below-floor;
2 = unparsable input (fail-closed: an unreadable summary or record is
never a silent pass).

Usage:
    python3 scripts/ci/coverage_floor_check.py coverage-summary.txt
    python3 scripts/ci/coverage_floor_check.py --self-test
"""

from __future__ import annotations

import os
import re
import sys

BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        'coverage_baseline.txt')

TOTAL_RE = re.compile(r'^TOTAL\b')
PCT_RE = re.compile(r'(\d+(?:\.\d+)?)%')


def parse_line_cover(text: str) -> float | None:
    """The LINE cover = the third percentage token of the LAST TOTAL row."""
    total_row = None
    for line in text.splitlines():
        if TOTAL_RE.match(line):
            total_row = line
    if total_row is None:
        return None
    pcts = PCT_RE.findall(total_row)
    if len(pcts) < 3:
        return None
    return float(pcts[2])


def parse_baseline(path: str = BASELINE) -> tuple[float, str] | None:
    """(floor, mode) or None — a missing/unparsable record is fail-closed."""
    if not os.path.isfile(path):
        return None
    floor = mode = None
    for line in open(path, encoding='utf-8'):
        m = re.match(r'^floor_line_pct:\s*(\d+(?:\.\d+)?)\s*$', line)
        if m:
            floor = float(m.group(1))
        m = re.match(r'^mode:\s*(\w+)\s*$', line)
        if m:
            mode = m.group(1)
    if floor is None or mode not in ('advisory', 'blocking'):
        return None
    return floor, mode


def check(text: str, path: str = BASELINE) -> int:
    base = parse_baseline(path)
    line_cover = parse_line_cover(text)
    if base is None:
        print('::error::the coverage floor record is missing/unparsable '
              '(%s) — fail-closed, never a silent pass' % path)
        return 2
    if line_cover is None:
        print('::error::the coverage summary has no parsable TOTAL row '
              '(fail-closed)')
        return 2
    floor, mode = base
    if line_cover + 1e-9 >= floor:
        print('coverage floor: line %.2f%% vs floor %.2f%% — MET (%s)'
              % (line_cover, floor, mode))
        return 0
    msg = ('coverage floor: line %.2f%% vs floor %.2f%% — BELOW FLOOR (%s)'
           % (line_cover, floor, mode))
    if mode == 'blocking':
        print('::error::' + msg)
        return 1
    print('::warning::' + msg + ' — the job stays green (advisory); '
          'the mode flips to blocking after two consecutive stable '
          'waves (the owner criterion, the record counts the waves)')
    return 0


def self_test() -> int:
    """The grammar cases pinned (the real cargo-llvm-cov 0.6.16 row)."""
    row = ('TOTAL                               8                 2    75.00%'
           '           3                 1    66.67%           3'
           '                 1    66.67%           0                 0         -')
    n569 = ('TOTAL        12345  2000  78.46%  900  260  70.65%  5000  1170'
            '  76.60%  800  300  62.50%')
    cases = [
        ('met advisory', 'x\n' + row + '\n',
         'floor_line_pct: 60\nmode: advisory\n', 0),
        ('below advisory (loud, stays green)', 'x\n' + row + '\n',
         'floor_line_pct: 80\nmode: advisory\n', 0),
        ('below blocking (red)', 'x\n' + row + '\n',
         'floor_line_pct: 80\nmode: blocking\n', 1),
        ('the №569 row vs floor 76', 'x\n' + n569 + '\n',
         'floor_line_pct: 76\nmode: advisory\n', 0),
        ('no TOTAL row (fail-closed)', 'nothing here\n',
         'floor_line_pct: 76\nmode: advisory\n', 2),
        ('bad mode (fail-closed)', 'x\n' + row + '\n',
         'floor_line_pct: 76\nmode: warn\n', 2),
        ('missing floor key (fail-closed)', 'x\n' + row + '\n',
         'mode: advisory\n', 2),
    ]
    import io
    import contextlib
    import tempfile
    failed = 0
    for name, summary, baseline, expected in cases:
        with tempfile.NamedTemporaryFile('w', suffix='.txt',
                                         delete=False) as f:
            f.write(baseline)
            bpath = f.name
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = check(summary, bpath)
        os.unlink(bpath)
        status = 'ok' if code == expected else 'FAIL'
        if code != expected:
            failed += 1
        print('  self-test [%s] expect=%d got=%d: %s'
              % (status, expected, code, name))
    print('self-test: %s' % ('ALL PASS' if not failed else '%d FAILED'
                             % failed))
    return 1 if failed else 0


def main() -> int:
    args = sys.argv[1:]
    if '--self-test' in args:
        return self_test()
    if not args:
        print('usage: coverage_floor_check.py <coverage-summary.txt>'
              ' | --self-test')
        return 2
    path = args[0]
    if not os.path.isfile(path):
        print('::error::the coverage summary %s is missing (fail-closed)'
              % path)
        return 2
    return check(open(path, encoding='utf-8').read())


if __name__ == '__main__':
    sys.exit(main())
