#!/usr/bin/env python3
"""№525: machine-sync of every fact_* record in the goal records —
gate_028_goals.txt, gate_029_goals.txt (№605: the 0.29 record is
verified too — it carries the X-3 precise-share fact; the 0.28 record
predates that parameter and carries no precise record) AND
gate_030_goals.txt (№630: the §5 fixation record — it carries the
parameterized-share fact, the machine twin of goal_parameterized_share_bp,
beside the precise-share twin it inherited from the draft era; the
branch-protection VERDICT fact joined in the gh#1000-closure PR
(2026-10-08) — the checker landed WITH the key, exactly as the §5 note
recorded, №525).

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
  fact_precise_share_bp  type_signature_share.py --gate
                       type_signature_precise_baseline.txt --precise
                       (№560/№605; the 0.29 record only — the machine
                       twin of goal_precise_share_bp, ADR-0181 §3.1).
  fact_parameterized_share_bp
                       type_signature_share.py --parameterized --gate
                       type_signature_parameterized_baseline.txt
                       (№623/№630; the 0.30 record only — the machine
                       twin of goal_parameterized_share_bp, ADR-0186 §3:
                       the third metric, the Z-2 successor of the scalar
                       typed/precise parameters).
  fact_branch_protection_audit
                       the conclusion of the LAST COMPLETED run of the
                       branch-protection-audit (weekly) workflow via
                       the GitHub API (success → GREEN, any other
                       completed conclusion → RED; an API failure or
                       no completed runs → exit 2 — an unreadable
                       state is never a pass). The gh#1000-closure PR
                       (2026-10-08) — the 0.30 record only (the В34
                       addendum Z-3, ADR-0186 §5); a VERDICT fact
                       (GREEN|RED), not a counter.

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
COUNTER = os.path.join(HERE, 'count_duplicated_names.py')
SHARE_PRECISE = os.path.join(HERE, 'type_signature_share.py')
PRECISE_BASELINE = os.path.join(HERE, 'type_signature_precise_baseline.txt')
PARAM_BASELINE = os.path.join(HERE, 'type_signature_parameterized_baseline.txt')
REPO = 'ShkodnikAI/Metalogos-'

# The goal records the sync verifies, each with its own expected fact set
# (№605: the 0.29 record carries the X-3 precise-share fact; №630: the
# 0.30 record carries the parameterized-share fact — the §5 fixation
# introduced the key WITH this checker in the same PR).
RECORDS = (
    (os.path.join(HERE, 'gate_028_goals.txt'),
     ('fact_open_high_server', 'fact_quorum_num', 'fact_quorum_den',
      'fact_blocking_check_cells')),
    (os.path.join(HERE, 'gate_029_goals.txt'),
     ('fact_open_high_server', 'fact_quorum_num', 'fact_quorum_den',
      'fact_blocking_check_cells', 'fact_precise_share_bp')),
    (os.path.join(HERE, 'gate_030_goals.txt'),
     ('fact_open_high_server', 'fact_quorum_num', 'fact_quorum_den',
      'fact_blocking_check_cells', 'fact_precise_share_bp',
      'fact_parameterized_share_bp', 'fact_branch_protection_audit')),
)

# The VERDICT-type facts (a state, not a counter): the value domain is
# GREEN|RED, the tamper flips the verdict, and verify() validates the
# domain instead of the integer form.
VERDICT_FACTS = frozenset({'fact_branch_protection_audit'})


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


def precise_share_observed() -> dict:
    """№605 (gh#1046; the audit 25b375e §3 X-3): the live PRECISE
    typed-share fact — the machine twin of goal_precise_share_bp in the
    0.29 record (ADR-0181 §3.1). The same №467 gate script, the precise
    mode (№560); the percentage → basis-point rounding mirrors the v2
    gate reader exactly."""
    out = subprocess.run([sys.executable, SHARE_PRECISE, '--gate',
                          PRECISE_BASELINE, '--precise'],
                         capture_output=True, text=True)
    if out.returncode != 0:
        print(out.stdout + out.stderr)
        print('::error::the precise-share source script failed (exit %d)'
              % out.returncode)
        sys.exit(2)
    m = re.search(r'precise signatures:\s*\d+/\d+\s*\(([\d.]+)%\)', out.stdout)
    if not m:
        print('::error::the precise-share source printed no parsable record')
        sys.exit(2)
    bp = int(round(float(m.group(1)) * 100))
    return {'fact_precise_share_bp': str(bp)}


def parameterized_share_observed() -> dict:
    """№630 (gh#1098; ADR-0186 §3/§5): the live PARAMETERIZED-share fact —
    the machine twin of goal_parameterized_share_bp in the 0.30 record.
    The №623 third metric (type_signature_share.py --parameterized); the
    percentage → basis-point rounding mirrors the v2 gate reader exactly
    (the same shape as precise_share_observed)."""
    out = subprocess.run([sys.executable, SHARE_PRECISE, '--parameterized',
                          '--gate', PARAM_BASELINE],
                         capture_output=True, text=True)
    if out.returncode != 0:
        print(out.stdout + out.stderr)
        print('::error::the parameterized-share source script failed (exit %d)'
              % out.returncode)
        sys.exit(2)
    m = re.search(r'parameterized signatures:\s*\d+/\d+\s*\(([\d.]+)%\)',
                  out.stdout)
    if not m:
        print('::error::the parameterized-share source printed no parsable record')
        sys.exit(2)
    bp = int(round(float(m.group(1)) * 100))
    return {'fact_parameterized_share_bp': str(bp)}


def branch_protection_audit_observed(token: str) -> dict:
    """The gh#1000-closure PR (2026-10-08; the В34 addendum Z-3,
    ADR-0186 §5): the VERDICT of the LAST COMPLETED run of the
    branch-protection-audit (weekly) workflow. success → GREEN, any
    other completed conclusion (failure/cancelled/timed out) → RED; an
    API failure or no completed runs → exit 2 (loud INFRA — the №525
    rule: an unreadable state is not a pass). Any ref counts: the audit
    script pins DEFAULT_BRANCH='main' — every run of the workflow reads
    MAIN's live protection regardless of the ref it was dispatched on,
    so the last completed run is always a verdict about main."""
    url = ('https://api.github.com/repos/%s/actions/workflows/'
           'branch-protection-audit.yml/runs?per_page=1&status=completed'
           % REPO)
    req = urllib.request.Request(url, headers={
        'Authorization': 'Bearer %s' % token,
        'Accept': 'application/vnd.github+json',
    })
    try:
        data = json.load(urllib.request.urlopen(req))
    except Exception as e:
        print('::error::the GitHub API query failed (%s) — fail-closed' % e)
        sys.exit(2)
    runs = data.get('workflow_runs') or []
    if not runs:
        print('::error::no completed branch-protection-audit runs — the '
              'verdict is unreadable (fail-closed INFRA)')
        sys.exit(2)
    verdict = 'GREEN' if runs[0].get('conclusion') == 'success' else 'RED'
    return {'fact_branch_protection_audit': verdict}


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


def verify(goals_path: str, observed: dict, expected) -> list:
    """Compare the goals-file facts with the observed machine facts.

    `observed` maps fact name → observed value string; a fact absent
    from `observed` is skipped loudly (no source available in this
    environment). `expected` is the record's own fact set (№605: the
    records differ — the 0.29 record carries the precise-share fact).
    Returns the list of error strings (empty = in sync).
    """
    errors = []
    facts = read_facts(goals_path)
    for key in expected:
        if key not in facts:
            errors.append('%s is MISSING from the goals file (fail-closed)' % key)
            continue
        if key in VERDICT_FACTS:
            if facts[key] not in ('GREEN', 'RED'):
                errors.append('%s: value "%s" is not a verdict (GREEN|RED)'
                              % (key, facts[key]))
            continue
        if not re.match(r'^\d+$', facts[key]):
            errors.append('%s: value "%s" is not a non-negative integer' % (key, facts[key]))
            continue
    unknown = sorted(set(facts) - set(expected))
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
    observed_common = quorum_observed()
    observed_common.update(cells_observed())
    skips = []
    if token:
        observed_common['fact_open_high_server'] = open_high_observed(token)
    else:
        skips.append('GH_TOKEN absent — fact_open_high_server not verified here '
                     '(the ADR-0179 §6 step-1 sync stays a release-time human step; '
                     'CI always runs this check with the token)')
    errors = []
    # №605: the 0.28 record verifies against the common observed set, the
    # 0.29 record with the precise-share machine twin; №630: the 0.30
    # record with the parameterized-share machine twin. A fact key is
    # added to a record's observed set exactly when the record expects it
    # (a source reporting a key the record does not carry is itself a
    # verify() error — the per-record sets stay exact).
    for goals_path, expected in RECORDS:
        observed = dict(observed_common)
        if 'fact_precise_share_bp' in expected:
            observed.update(precise_share_observed())
        if 'fact_parameterized_share_bp' in expected:
            observed.update(parameterized_share_observed())
        if 'fact_branch_protection_audit' in expected:
            if token:
                observed.update(branch_protection_audit_observed(token))
            else:
                skips.append('GH_TOKEN absent — fact_branch_protection_audit '
                             'not verified here (the audit-conclusion read '
                             'needs the API; CI always runs this check with '
                             'the token)')
        errors += verify(goals_path, observed, expected)
    report(errors, skips)
    sys.exit(1 if errors else 0)


def tamper_test() -> None:
    """The negative test (№525 task 3): a tampered fact value must fail —
    for ALL goal records (№605: the 0.29 record; №630: the 0.30 record
    too)."""
    token = os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    observed_common = quorum_observed()
    observed_common.update(cells_observed())
    if token:
        observed_common['fact_open_high_server'] = open_high_observed(token)
    else:
        # the deterministic local fixture for the tamper test
        observed_common['fact_open_high_server'] = '0'
    failed = []
    for goals_path, expected in RECORDS:
        observed = dict(observed_common)
        if 'fact_precise_share_bp' in expected:
            observed.update(precise_share_observed())
        if 'fact_parameterized_share_bp' in expected:
            observed.update(parameterized_share_observed())
        if 'fact_branch_protection_audit' in expected:
            if token:
                observed.update(branch_protection_audit_observed(token))
            else:
                # the deterministic local fixture for the tamper test
                observed['fact_branch_protection_audit'] = 'GREEN'
        cases = []
        for key in sorted(observed):
            if key in VERDICT_FACTS:
                # the verdict tamper: flip the state, not the counter
                tampered = 'RED' if observed[key] == 'GREEN' else 'GREEN'
            else:
                tampered = str(int(observed[key]) + 1)
            cases.append(('value tamper: %s %s → %s' % (key, observed[key], tampered),
                          key, tampered))
        cases.append(('unknown fact without a machine source', 'fact_bogus_key', '5'))
        for name, key, value in cases:
            with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as tmp:
                tmp.write(open(goals_path, encoding='utf-8').read())
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
            errors = verify(path, observed, expected)
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
