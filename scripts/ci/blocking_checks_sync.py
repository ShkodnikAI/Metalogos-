#!/usr/bin/env python3
"""No535 blocking-checks sync (the blocking CI job `blocking-checks-sync`).

The table scripts/ci/blocking_checks.tsv is the machine-readable record of
WHO BLOCKS EXECUTION per command (run/check/compile/serve) × phase × check
with the verdict {blocks, warns, absent}. This script validates:

  1. FORMAT: every row has the 6 columns; the verdict is one of
     blocks|warns|absent; the (command, phase, check) key is unique.
  2. TEST BINDING: every blocks/warns row names a test_id that EXISTS in
     the repo (a word-boundary grep over tests/, src/, benches/ and
     metalogos-server/tests/) — a cell whose test is gone
     (renamed/deleted) is a drift = failure.
  3. COVERAGE: the five commands are present (mcp-serve carries real
     read/parse/audit/semantic cells since №557, gh#918); the JIT
     "absent (зафиксировано)" row exists (the №535 boundary).
  4. --count mode: prints `blocking_check_cells: N` (+ the per-verdict
     counts) for the №525 gate-facts wiring.
  5. --tamper-test: the negative test — a row with an unresolvable
     test_id, a bogus verdict, and a duplicate key must ALL trip the
     check (the traps spring).

Exit 0 = the table is in sync; exit 1 = drift.
"""
from __future__ import annotations

import os
import re
import sys
import tempfile
from pathlib import Path

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = Path(HERE).resolve().parent.parent
TABLE = os.path.join(HERE, 'blocking_checks.tsv')
VERDICTS = ('blocks', 'warns', 'absent')
REQUIRED_COMMANDS = ('run', 'check', 'compile', 'serve', 'mcp-serve')
REQUIRED_ABSENT_ROWS = (('jit', 'jit-compilation'),)


def load_rows(path: str = TABLE) -> list[dict]:
    rows = []
    for lineno, line in enumerate(open(path, encoding='utf-8'), start=1):
        line = line.rstrip('\n')
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        if line.startswith('command|phase|check|'):
            continue  # the header row
        parts = line.split('|')
        if len(parts) != 6:
            raise ValueError('line %d: expected 6 |-separated columns, got %d' % (lineno, len(parts)))
        rows.append({
            'command': parts[0].strip(),
            'phase': parts[1].strip(),
            'check': parts[2].strip(),
            'verdict': parts[3].strip(),
            'test_id': parts[4].strip(),
            'note': parts[5].strip(),
        })
    return rows


def collect_sources() -> str:
    chunks = []
    # №567 (gh#931): the transport tests moved to metalogos-server/tests/
    # — the binding grep follows them (a moved test is not a deleted test).
    for base in ('tests', 'src', 'benches', 'metalogos-server/tests'):
        p = ROOT / base
        if p.exists():
            for f in p.rglob('*.rs'):
                try:
                    chunks.append(f.read_text())
                except OSError:
                    pass
    return '\n'.join(chunks)


def validate(rows: list[dict], sources: str) -> list[str]:
    errors = []
    seen = set()
    for r in rows:
        key = (r['command'], r['phase'], r['check'])
        if key in seen:
            errors.append('duplicate cell: %s' % (key,))
        seen.add(key)
        if r['verdict'] not in VERDICTS:
            errors.append('%s: bogus verdict "%s" (need blocks|warns|absent)' % (key, r['verdict']))
        if r['verdict'] in ('blocks', 'warns'):
            if not r['test_id']:
                errors.append('%s: a blocks/warns cell MUST name its test_id' % (key,))
            elif not re.search(r'\b%s\b' % re.escape(r['test_id']), sources):
                errors.append(
                    '%s: the test `%s` is not found in tests/ or src/ — '
                    'the cell lost its proof (drift)' % (key, r['test_id'])
                )
    commands = {r['command'] for r in rows}
    for cmd in REQUIRED_COMMANDS:
        if cmd not in commands:
            errors.append('the command `%s` has no rows — the coverage is gone' % cmd)
    for cmd, check in REQUIRED_ABSENT_ROWS:
        if not any(r['command'] == cmd and r['check'] == check and r['verdict'] == 'absent' for r in rows):
            errors.append('the "%s × %s" absent (зафиксировано) row is missing' % (cmd, check))
    return errors


def main() -> None:
    args = sys.argv[1:]
    try:
        rows = load_rows()
    except ValueError as e:
        print('::error::%s' % e)
        sys.exit(1)
    if '--count' in args:
        # The №525 gate-facts source: the live cell count.
        print('blocking_check_cells: %d' % len(rows))
        sys.exit(0)
    sources = collect_sources()
    errors = validate(rows, sources)
    if errors:
        print('blocking-checks-sync: FAILED')
        for e in errors:
            print('::error::%s' % e)
        sys.exit(1)
    blocks = sum(1 for r in rows if r['verdict'] == 'blocks')
    warns = sum(1 for r in rows if r['verdict'] == 'warns')
    absent = sum(1 for r in rows if r['verdict'] == 'absent')
    print('blocking-checks-sync: OK — %d cells (blocks=%d, warns=%d, absent=%d)' % (len(rows), blocks, warns, absent))
    if '--tamper-test' in args:
        tamper_tests(rows)


def tamper_tests(rows: list[dict]) -> None:
    """The negative test: the traps must spring."""
    failed = []
    # 1. an unresolvable test_id
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False) as tmp:
        tmp.write(open(TABLE, encoding='utf-8').read())
        path = tmp.name
    lines = open(path, encoding='utf-8').readlines()
    for i, line in enumerate(lines):
        if line.startswith('run|parse|'):
            lines[i] = line.replace('n535_run_parse_error_blocks', 'n535_deleted_test_name')
            break
    open(path, 'w').writelines(lines)
    errors = validate(load_rows(path), collect_sources())
    if any('n535_deleted_test_name' in e for e in errors):
        print('PASS: the unresolvable test_id trap sprang')
    else:
        failed.append('the unresolvable test_id did NOT trip the check')
    # 2. a bogus verdict — HOTFIX (the first real CI run, 30.09): the
    # trap asserted the wrong failure mode. `validate()` RETURNS the
    # error list, it never raises ValueError for a domain violation
    # (only `load_rows` raises, on the column-count drift), so the
    # `except ValueError` branch was unreachable and the trap could
    # never spring — `blocking-checks-sync (blocking)` failed on every
    # run with "the bogus verdict did NOT trip the check". The trap
    # now inspects the returned errors exactly like traps #1/#3 do.
    lines = open(TABLE, encoding='utf-8').readlines()
    for i, line in enumerate(lines):
        if line.startswith('check|semantic|warnings|'):
            lines[i] = line.replace('|warns|', '|blocks-bogus|')
            break
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False) as tmp:
        tmp.writelines(lines)
        path2 = tmp.name
    errors2 = validate(load_rows(path2), collect_sources())
    if any('bogus verdict' in e for e in errors2):
        print('PASS: the bogus-verdict trap sprang (the verdict domain)')
    else:
        failed.append('the bogus verdict did NOT trip the check')
    # 3. a duplicate cell key
    lines = open(TABLE, encoding='utf-8').readlines()
    row_line = next(l for l in lines if l.startswith('run|parse|'))
    lines.append(row_line)
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False) as tmp:
        tmp.writelines(lines)
        path3 = tmp.name
    errors = validate(load_rows(path3), collect_sources())
    if any('duplicate cell' in e for e in errors):
        print('PASS: the duplicate-key trap sprang')
    else:
        failed.append('the duplicate key did NOT trip the check')
    for p in (path, path2, path3):
        os.unlink(p)
    if failed:
        for f in failed:
            print('::error::%s' % f)
        sys.exit(1)


if __name__ == '__main__':
    main()
