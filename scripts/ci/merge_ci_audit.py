#!/usr/bin/env python3
"""Naryad №551 (issue #912; the audit 02.10 M-1 + §6.2): the weekly
merge ↔ CI reconciliation.

THE RULE (docs/maintainers.md, «When CI is down»): a merge without a
green required-check run is a process violation, not a judgement call.
This script is the machine half of the rule — the weekly audit that
closes the M-1 class (the 30.09 Actions event-delivery outage let six
PRs merge checkless; two defects reached main unseen — the hotfix
0c9a013, gh#865).

HOW IT READS MAIN: the repository merges are SQUASH merges, so every
merged PR leaves one main commit whose subject ends with the `(#NNN)`
ref. Each such commit is the push trigger of its own full CI run —
the audit fetches the commit's check-runs (the Actions API) and
demands a GREEN run of every required job. A commit whose required
set is not green is a divergence.

DIVERGENCES: one tracking issue per audit run (deduped against the
already-open audit issue), listing the offending commits, their PRs
and the missing/failed required jobs. An OWNER-ACKNOWLEDGED merge —
the PR body carries the exact marker below — is reported as
acknowledged, never silent, and is not a violation.

Exit codes: 0 — no violations (acknowledged merges may exist);
1 — at least one violation (the weekly run stays visibly red until
the divergence is resolved: re-run of the CI on the commit is not
possible post-merge, so the resolution is the follow-up issue the
audit itself opens).

Usage:
    GH_TOKEN=... python3 scripts/ci/merge_ci_audit.py [--deep 30] [--repo OWNER/NAME]

The fixture test (no network — the canned responses, the same posture
as `sync_gate_facts.py --tamper-test`, №525):
    python3 scripts/ci/merge_ci_audit.py --fixture-test
"""
import json
import os
import re
import sys
import urllib.request

# The required checks — synced with docs/maintainers.md
# («When CI is down», the branch-protection checklist) and verified
# against the .github/workflows/ci.yml job ids (2026-10-03, №551).
REQUIRED_JOBS = [
    'test-lib',
    'test-integration',
    'crosscheck',
    'clippy',
    'fmt',
    'cargo-audit',
    'cargo-deny',
    'gitleaks',
    'gate-facts-sync',
    'blocking-checks-sync',
    'registry-arity-check',
]

# The exact marker an owner puts in a PR body to acknowledge a
# checkless merge during an Actions outage (the security-hotfix
# exception; anything else is a violation).
OWNER_MARKER = 'CI-down merge acknowledged by the owner'

ISSUE_TITLE_PREFIX = 'merge-ci-audit: merges without green required checks'

PR_REF_RE = re.compile(r'\(#(\d+)\)\s*$')


class Api:
    """The GitHub REST surface the audit needs — injectable for the
    fixture test (the same seam style as the №525 tamper test)."""

    def __init__(self, token, repo):
        self.token = token
        self.repo = repo

    def _get(self, path):
        url = 'https://api.github.com/repos/%s/%s' % (self.repo, path)
        req = urllib.request.Request(url, headers={
            'Authorization': 'token %s' % self.token,
            'Accept': 'application/vnd.github+json',
        })
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.load(resp)

    def main_commits(self, deep):
        return self._get('commits?sha=main&per_page=%d' % deep)

    def check_runs(self, sha):
        return self._get('commits/%s/check-runs?per_page=100' % sha)

    def pr_body(self, number):
        return self._get('pulls/%d' % number).get('body') or ''

    def find_open_audit_issue(self):
        issues = self._get(
            'issues?state=open&per_page=100&labels=process')
        for it in issues:
            if (it.get('title') or '').startswith(ISSUE_TITLE_PREFIX):
                return it.get('number')
        return None

    def open_audit_issue(self, title, body):
        data = json.dumps({
            'title': title,
            'body': body,
            'labels': ['process', 'ci'],
        }).encode()
        req = urllib.request.Request(
            'https://api.github.com/repos/%s/issues' % self.repo,
            data=data, method='POST', headers={
                'Authorization': 'token %s' % self.token,
                'Accept': 'application/vnd.github+json',
            })
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.load(resp).get('number')


def audit(api, deep):
    """The core reconciliation. Returns (ok, report_lines, new_issue_no)."""
    commits = api.main_commits(deep)
    merged = []
    for c in commits:
        msg = (c.get('commit') or {}).get('message') or ''
        first = msg.split('\n', 1)[0]
        m = PR_REF_RE.search(first)
        if m:
            merged.append((c['sha'], first, int(m.group(1))))

    violations, acknowledged, greens = [], [], []
    for sha, subject, pr in merged:
        runs = (api.check_runs(sha) or {}).get('check_runs') or []
        by_name = {}
        for r in runs:
            name = r.get('name') or ''
            conclusion = r.get('conclusion')
            # The BEST conclusion wins — a re-run repairs an earlier
            # failure in place (the same commit, the same name).
            if name not in by_name or conclusion == 'success':
                by_name[name] = conclusion
        missing = [j for j in REQUIRED_JOBS if by_name.get(j) != 'success']
        if not missing:
            greens.append((sha, subject, pr))
            continue
        body = api.pr_body(pr)
        if OWNER_MARKER in body:
            acknowledged.append((sha, subject, pr))
        else:
            violations.append((sha, subject, pr, missing))

    lines = []
    lines.append('merge-ci-audit (№551) — the last %d main commits scanned, %d PR merges found'
                 % (deep, len(merged)))
    lines.append('green required-set: %d | acknowledged: %d | VIOLATIONS: %d'
                 % (len(greens), len(acknowledged), len(violations)))
    for sha, subject, pr in acknowledged:
        lines.append('')
        lines.append('ACKNOWLEDGED (the owner marker in the PR body): %s (#%d)'
                     % (subject, pr))
    new_issue = None
    if violations:
        lines.append('')
        lines.append('VIOLATIONS — the main commits whose required checks did not run green:')
        for sha, subject, pr, missing in violations:
            lines.append('- %s' % subject)
            lines.append('  commit %s, PR #%d' % (sha[:12], pr))
            lines.append('  not green: %s' % ', '.join(missing))
        title = '%s (%d)' % (ISSUE_TITLE_PREFIX, len(violations))
        existing = api.find_open_audit_issue()
        if existing:
            lines.append('')
            lines.append('the tracking issue already open: #%s' % existing)
        else:
            new_issue = api.open_audit_issue(
                title,
                'The weekly merge ↔ CI audit (№551) found %d main '
                'commits whose required checks did not run green.\n\n'
                'The rule: docs/maintainers.md — «When CI is down». '
                'A merge without a green required-check run is a process '
                'violation, not a judgement call. Each entry below needs '
                'the post-hoc verification (the CI re-run on the merge '
                'content or the equivalent test evidence) and the honest '
                'record in the PR.\n\n%s' % (len(violations), '\n'.join(lines)))
            lines.append('')
            lines.append('tracking issue opened: #%s' % new_issue)
    ok = not violations
    return ok, lines, new_issue


def fixture_test():
    """The canned-network proof: the green path, the violation path, the
    owner-acknowledged path, and the issue-open dedup — no HTTP."""
    print('── fixture test: the canned merge ↔ CI reconciliation ──')

    class FixtureApi:
        def __init__(self, runs_by_sha, bodies, existing):
            self.runs_by_sha = runs_by_sha
            self.bodies = bodies
            self.existing = existing
            self.opened = None

        def main_commits(self, deep):
            return [
                {'sha': sha, 'commit': {'message': '%s (#%d)\n\ndetails' % (subj, pr)}}
                for sha, subj, pr in self.commits
            ]

        def check_runs(self, sha):
            return {'check_runs': self.runs_by_sha.get(sha, [])}

        def pr_body(self, number):
            return self.bodies.get(number, '')

        def find_open_audit_issue(self):
            return self.existing

        def open_audit_issue(self, title, body):
            self.opened = (title, body)
            return 9999

    def runs(*jobs):
        return [{'name': j, 'conclusion': 'success'} for j in jobs]

    full_green = runs(*REQUIRED_JOBS)
    no_runs = []
    partial = runs('test-lib', 'clippy')

    fx = FixtureApi(
        runs_by_sha={'g' * 40: full_green, 'b' * 40: no_runs, 'p' * 40: partial},
        bodies={777: 'the hotfix note. %s' % OWNER_MARKER},
        existing=None,
    )
    fx.commits = [
        ('g' * 40, 'Naryad 540 (issue #850): the green merge', 850),
        ('b' * 40, 'Naryad 541 (issue #851): merged during the outage', 851),
        ('p' * 40, 'Naryad 542 (issue #852): the partial run', 852),
        ('a' * 40, 'the security hotfix, owner-acknowledged', 777),
    ]
    ok, lines, new_issue = audit(fx, deep=10)
    assert not ok, 'violations must fail the audit'
    assert new_issue == 9999, 'the first run opens the tracking issue'
    text = '\n'.join(lines)
    assert '#851' in text and '#852' in text, 'both violations listed'
    assert 'b' * 12 in text and 'test-integration' in text, 'the missing jobs named'
    assert '#777' in text and 'ACKNOWLEDGED' in text, 'the owner marker honoured'
    assert '#850' not in text.split('green required-set')[1].split('VIOLATIONS')[0], 'the green merge is silent'
    print('  green merge stays silent, the missing jobs are named, the marker honoured — OK')

    fx2 = FixtureApi(
        runs_by_sha={'b' * 40: no_runs},
        bodies={},
        existing=4242,
    )
    fx2.commits = [('b' * 40, 'Naryad 541 (issue #851): merged during the outage', 851)]
    ok2, lines2, new_issue2 = audit(fx2, deep=10)
    assert not ok2 and new_issue2 is None, 'the dedup: no second issue'
    assert '#4242' in '\n'.join(lines2), 'the existing tracking issue is referenced'
    print('  the dedup: one open tracking issue, no duplicates — OK')

    fx3 = FixtureApi(
        runs_by_sha={'g' * 40: full_green},
        bodies={},
        existing=None,
    )
    fx3.commits = [('g' * 40, 'Naryad 540 (issue #850): the green merge', 850)]
    ok3, _, _ = audit(fx3, deep=10)
    assert ok3, 'the all-green week passes with exit 0'
    print('  the all-green week exits 0 — OK')
    print('fixture test: 4/4 assertions green')
    return 0


def main():
    args = sys.argv[1:]
    if '--fixture-test' in args:
        return fixture_test()
    token = os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    if not token:
        print('GH_TOKEN is required (fail-closed: no token — no audit)', file=sys.stderr)
        return 2
    repo = os.environ.get('GITHUB_REPOSITORY')
    if '--repo' in args:
        repo = args[args.index('--repo') + 1]
    if not repo:
        print('GITHUB_REPOSITORY or --repo OWNER/NAME is required', file=sys.stderr)
        return 2
    deep = int(args[args.index('--deep') + 1]) if '--deep' in args else 30
    ok, lines, _ = audit(Api(token, repo), deep)
    print('\n'.join(lines))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
