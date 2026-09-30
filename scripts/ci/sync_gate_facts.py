#!/usr/bin/env python3
"""№525: machine-sync of every fact_* record in gate_028_goals.txt.

The class this closes (the audit 30.09, Д-2): the fact lines in the
goals file are hand-edited and nothing verified them against their
generating sources — the checked-in `fact_quorum_*` (9/9) drifted from
the single source of truth (`count_duplicated_names.py` reports ZERO
duplicated names since №483). A gate record that disagrees with its own
source is a liar record — the release gate would read it either way.

The facts and their machine sources:

  fact_quorum_num      count_duplicated_names.py --quorum  (local, deterministic)
  fact_quorum_den      count_duplicated_names.py --quorum  (local, deterministic)
  fact_open_high_server  the GitHub API count of OPEN issues labeled
                       `release-block` (ADR-0179 §4 label discipline).
                       Requires GH_TOKEN — always present in GitHub
                       Actions; without it the check prints a loud SKIP
                       note (the ADR-0179 §6 step-1 sync stays a
                       release-time human step locally).

Fail-closed rules:
  - a fact line whose value disagrees with its source → exit 1;
  - an unknown `fact_*` key in the goals file (a fact without a machine
    source — the class re-opens) → exit 1;
  - a missing expected fact → exit 1;
  - an unparsable fact value → exit 1;
  - the GitHub API erroring → exit 2 (infra, loud — never silently OK).

Usage:
    sync_gate_facts.py               → the CI check (exit 0 = in sync)
    sync_gate_facts.py --tamper-test → the negative test: tampers each
        fact on a temp copy and asserts every trap springs; exit 1 if
        any tampered record passes.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
GOALS = os.path.join(HERE, 'gate_028_goals.txt')
COUNTER = os.path.join(HERE, 'count_duplicated_names.py')
REPO = 'ShkodnikAI/Metalogos-'

EXPECTED_FACTS = ('fact_open_high_server', 'fact_quorum_num', 'fact_quorum_den',
                  'fact_blocking_check_cells')


def read_facts(goals_path: str) -> dict:
    facts = {}
    for line in open(goals_path, encoding='utf-8'):
        m = re.match(r'^(fact_[a-z_]+):\s*(.+?)\s*$', line)
        if m:
            facts[m.group(1)] = m.group(2)
    return facts


def quorum_observed() -> dict:
    out = subprocess.run([sys.executable, COUNTER, '--quorum'],
                         capture_output=True, text=True)
    if out.returncode != 0:
        print(out.stdout + out.stderr)
        print('::error::the quorum source script failed (exit %d)' % out.returncode)
        sys.exit(2)
    observed = {}
    for key in ('quorum_num', 'quorum_den'):
        m = re.search(r'^%s:\s*(\d+)$' % key, out.stdout, re.M)
        if not m:
            print('::error::the quorum source printed no %s record' % key)
            sys.exit(2)
        observed['fact_' + key] = m.group(1)
    return observed


def cells_observed() -> dict:
    """№535: the live cell count of the blocking-checks table."""
    script = os.path.join(HERE, 'blocking_checks_sync.py')
    out = subprocess.run([sys.executable, script, '--count'],
                         capture_output=True, text=True)
    if out.returncode != 0:
        print(out.stdout + out.stderr)
        print('::error::the blocking-checks source script failed (exit %d)' % out.returncode)
        sys.exit(2)
    m = re.search(r'^blocking_check_cells:\s*(\d+)$', out.stdout, re.M)
    if not m:
        print('::error::the blocking-checks source printed no count record')
        sys.exit(2)
    return {'fact_blocking_check_cells': m.group(1)}


def open_high_observed(token: str) -> str:
    count = 0
    page = 1
    while True:
        url = ('https://api.github.com/repos/%s/issues?state=open'
               '&labels=release-block&per_page=100&page=%d' % (REPO, page))
        req = urllib.request.Request(url, headers={
            'Authorization': 'Bearer %s' % token,
            'Accept': 'application/vnd.github+json',
        })
        try:
            data = json.load(urllib.request.urlopen(req))
        except Exception as e:
            print('::error::the GitHub API query failed (%s) — fail-closed' % e)
            sys.exit(2)
        issues = [i for i in data if 'pull_request' not in i]
        count += len(issues)
        if len(data) < 100:
            break
        page += 1
    return str(count)


def verify(goals_path: str, observed: dict) -> list:
    """Compare the goals-file facts with the observed machine facts.

    `observed` maps fact name → observed value string; a fact absent
    from `observed` is skipped loudly (no source available in this
    environment). Returns the list of error strings (empty = in sync).
    """
    errors = []
    facts = read_facts(goals_path)
    for key in EXPECTED_FACTS:
        if key not in facts:
            errors.append('%s is MISSING from the goals file (fail-closed)' % key)
            continue
        if not re.match(r'^\d+$', facts[key]):
            errors.append('%s: value "%s" is not a non-negative integer' % (key, facts[key]))
            continue
    unknown = sorted(set(facts) - set(EXPECTED_FACTS))
    if unknown:
        errors.append('unknown fact record(s) without a machine source: %s (fail-closed)'
                      % ', '.join(unknown))
    for key, value in sorted(observed.items()):
        if key not in facts:
            errors.append('%s: the source reports %s but the goals file has no such fact' % (key, value))
            continue
        if facts[key] != value:
            errors.append('%s: goals file says %s, the machine source says %s — the record is out of sync'
                          % (key, facts[key], value))
    return errors


def report(errors: list, skips: list) -> None:
    for s in skips:
        print('SKIP (loud): %s' % s)
    if errors:
        for e in errors:
            print('::error::%s' % e)
        print('gate-facts-sync: %d OUT-OF-SYNC fact record(s)' % len(errors))
    else:
        print('gate-facts-sync: every fact_* record matches its machine source')


def main() -> None:
    args = sys.argv[1:]
    if '--tamper-test' in args:
        tamper_test()
        return
    token = os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    observed = quorum_observed()
    observed.update(cells_observed())
    skips = []
    if token:
        observed['fact_open_high_server'] = open_high_observed(token)
    else:
        skips.append('GH_TOKEN absent — fact_open_high_server not verified here '
                     '(the ADR-0179 §6 step-1 sync stays a release-time human step; '
                     'CI always runs this check with the token)')
    errors = verify(GOALS, observed)
    report(errors, skips)
    sys.exit(1 if errors else 0)


def tamper_test() -> None:
    """The negative test (№525 task 3): a tampered fact value must fail."""
    token = os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    observed = quorum_observed()
    observed.update(cells_observed())
    if token:
        observed['fact_open_high_server'] = open_high_observed(token)
    else:
        # the deterministic local fixture for the tamper test
        observed['fact_open_high_server'] = '0'
    cases = []
    for key in sorted(observed):
        tampered = str(int(observed[key]) + 1)
        cases.append(('value tamper: %s %s → %s' % (key, observed[key], tampered),
                      key, tampered))
    cases.append(('unknown fact without a machine source', 'fact_bogus_key', '5'))
    failed = []
    for name, key, value in cases:
        with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as tmp:
            tmp.write(open(GOALS, encoding='utf-8').read())
            path = tmp.name
        lines = open(path, encoding='utf-8').readlines()
        if key in read_facts(path):
            replaced = False
            for i, line in enumerate(lines):
                if re.match(r'^%s:' % re.escape(key), line):
                    lines[i] = '%s: %s\n' % (key, value)
                    replaced = True
            if not replaced:
                failed.append('%s — the tamper could not find the line' % name)
        else:
            lines.append('%s: %s\n' % (key, value))
        open(path, 'w', encoding='utf-8').writelines(lines)
        errors = verify(path, observed)
        os.unlink(path)
        if any(key in e for e in errors):
            print('PASS: %s — the trap sprang (%d error(s))' % (name, len(errors)))
        else:
            failed.append('%s — the tampered record PASSED (the trap did not spring)' % name)
    if failed:
        for f in failed:
            print('::error::%s' % f)
        sys.exit(1)
    print('tamper-test: every trap sprang — the sync gate is loaded')


if __name__ == '__main__':
    main()
