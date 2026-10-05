#!/usr/bin/env python3
"""Naryad №586 (audit d63cc1d X-4): the branch-protection audit job.

The class this closes (X-4): whether branch protection is ENABLED is
invisible from the repository itself, and the public API without a
token rate-limits — the audit could not read the state. The office
measurement 05.10 (out-of-repo, via API) recorded the protection
alive — 22 required checks, no required review — but the fact lives
nowhere: in a month nobody knows it again. This job fixates the state
on a date and compares it to the checklist the document already
records (docs/maintainers.md, "The branch-protection checklist") —
the state is FIXATED, not assumed (the №551 / gate-facts-sync
discipline: a machine check with a loud fail).

The checklist items and their machine mapping (the doc's prose is the
SSOT; this script is its machine twin):

  checklist item (maintainers.md)                     API field
  -------------------------------------------------   -------------------------
  Require a pull request before merging               required_pull_request_reviews != null
  (no direct pushes)                                  + allow_force_pushes.enabled == false
                                                      + allow_deletions.enabled == false
  Require status checks to pass — the required set    every doc name present in
                                                      required_status_checks (checks[].context
                                                      or contexts[])
  Require branches to be up to date                   required_status_checks.strict == true
  Do not allow bypassing (incl. administrators)       enforce_admins.enabled == true
  Require review from Code Owners (the №471 lane,     require_code_owner_reviews — the
  active with the second maintainer)                  lane state is RECORDED, not failed:
                                                      either state is checklist-legal
                                                      until the maintainer joins.

Verdict rules (fail-closed):
  - a documented required check MISSING from the live set → RED
    (under-protection is a hole);
  - a live check MISSING from the document → GREEN with a DOC-STALE
    warning: over-protection is not a hole; the extra names are
    recorded in the report and re-enter the checklist by its own
    fact-check procedure ("as the check-runs API reports them,
    fact-checked against ci.yml @ <sha>") with the API provenance —
    the doc refresh is a docs step, not a silent act;
  - a boolean item diverging (strict / enforce_admins / no PR reviews
    / force-push or deletion allowed) → RED;
  - protection OFF (404 on a VALID token) → RED — the item-1 divergence;
  - the API erroring, the token dead or lacking administration:read
    → exit 2 INFRA — loud, never silently OK (the №525 rule: an
    unreadable state is not a pass).

Read-only boundary: the job NEVER modifies the protection (no write
endpoint, the token is Administration: read-only); fork PRs receive no
secrets, and the workflow does not run on pull_request at all.

Usage:
    branch_protection_audit.py [--report-out FILE] [--repo R] [--branch B]
                               [--protection-json FILE] [--self-test]
    --protection-json  read the live-state JSON from a file instead of
                       the API (the synthetic-divergence harness, the
                       local runs); the same parser, the same verdicts
    --self-test        the negative test: synthetic fixtures for every
                       trap (missing check, boolean divergence,
                       protection off, doc-stale extras, the №471 lane
                       states) — each must produce its recorded
                       verdict; exit 1 if any trap fails to spring

Exit codes: 0 GREEN · 1 RED divergence · 2 INFRA (loud).
"""
import argparse
import datetime
import json
import os
import re
import sys
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_REPO = 'ShkodnikAI/Metalogos-'
DEFAULT_BRANCH = 'main'
CHECKLIST = os.path.join(HERE, '..', '..', 'docs', 'maintainers.md')
CHECKLIST_HEADING = '### The branch-protection checklist'
API = 'https://api.github.com'

GREEN, RED, INFRA = 'GREEN', 'RED', 'INFRA'


# ---------------------------------------------------------------- checklist

def parse_checklist(path: str) -> dict:
    """Parse the maintainers.md checklist section: the required-set
    names (backticked, `name (blocking)` form) — the document is the
    SSOT; the boolean items' semantics are the prose above, mapped in
    the module docstring."""
    text = open(path, encoding='utf-8').read()
    m = re.search(r'^%s.*?$' % re.escape(CHECKLIST_HEADING), text, re.M)
    if not m:
        raise SystemExit('FATAL: the checklist heading not found in %s' % path)
    rest = text[m.start():]
    first_nl = rest.find('\n')
    nxt = re.search(r'^##+ ', rest[first_nl + 1:], re.M)
    section = rest[:first_nl + 1 + nxt.start()] if nxt else rest
    # the FULL display names (the checklist's own rule: "the job display
    # names as the check-runs API reports them") — the backticked
    # `name (blocking)` tokens verbatim
    names = sorted(set(re.findall(r'`([^`]+?\(blocking\))`', section)))
    if not names:
        raise SystemExit('FATAL: no required-check names parsed from %s' % path)
    return {'names': names, 'path': path}


# -------------------------------------------------------------- live state

def _api_get(url: str, token: str) -> 'tuple[int, str]':
    req = urllib.request.Request(url, headers={
        'Authorization': 'Bearer %s' % token,
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        'User-Agent': 'metalogos-branch-protection-audit',
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, r.read().decode('utf-8')
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode('utf-8', 'replace')


def fetch_protection(repo: str, branch: str, token: str) -> 'tuple[int, dict | None]':
    """(verdict_class, payload) — 200 → (GREEN-candidate, the JSON);
    404 on a VALID token → the protection-off state (RED);
    401/403/other → INFRA (loud; a dead or under-permissioned token
    must never masquerade as a protection state)."""
    code, body = _api_get('%s/repos/%s/branches/%s/protection' % (API, repo, branch), token)
    if code == 200:
        return GREEN, json.loads(body)
    if code == 404:
        # the token-validity proof before trusting "off": the repo GET
        # must return 200 for the same token, else the 404 is unreadable
        rcode, _ = _api_get('%s/repos/%s' % (API, repo), token)
        if rcode == 200:
            return RED, {'_protection_off': True}
        return INFRA, {'_error': 'protection 404 with a non-working token (repo GET %s)' % rcode}
    if code in (401, 403):
        return INFRA, {'_error': 'API %s — the token is dead or lacks administration:read' % code}
    return INFRA, {'_error': 'API %s: %s' % (code, body[:200])}


# ------------------------------------------------------------------ verdict

def _contexts(protection: dict) -> 'tuple[list, bool | None]':
    rsc = protection.get('required_status_checks') or {}
    names = [c.get('context') for c in (rsc.get('checks') or []) if c.get('context')]
    if not names:
        names = list(rsc.get('contexts') or [])
    return sorted(names), rsc.get('strict')


def audit(expected: dict, protection: dict) -> 'tuple[str, list, dict]':
    """(verdict, findings, report) — the fail-closed comparison."""
    findings, verdict = [], GREEN
    if protection.get('_protection_off'):
        findings.append(('RED', 'branch protection is OFF (404 on a valid token) — checklist item 1 diverges'))
        return RED, findings, _report(None, {}, expected, findings)
    ctx, strict = _contexts(protection)
    rpr = protection.get('required_pull_request_reviews')
    enforce = (protection.get('enforce_admins') or {}).get('enabled')
    force = (protection.get('allow_force_pushes') or {}).get('enabled')
    delete = (protection.get('allow_deletions') or {}).get('enabled')
    code_owner = (rpr or {}).get('require_code_owner_reviews') if rpr else None

    live = set(ctx)
    want = set(expected['names'])
    missing = sorted(want - live)
    extra = sorted(live - want)

    for name in missing:
        findings.append(('RED', 'required check MISSING from the live protection: %s' % name))
    if not ctx and want:
        findings.append(('RED', 'required_status_checks absent while the checklist requires %d checks' % len(want)))
    if rpr is None:
        findings.append(('RED', 'Require a pull request before merging is OFF (required_pull_request_reviews is null)'))
    if strict is not True:
        findings.append(('RED', 'Require branches to be up to date is OFF (strict=%r)' % strict))
    if enforce is not True:
        findings.append(('RED', 'Do not allow bypassing the settings is OFF (enforce_admins=%r)' % enforce))
    if force is True:
        findings.append(('RED', 'force pushes allowed (allow_force_pushes=true) — the "no direct pushes" intent diverges'))
    if delete is True:
        findings.append(('RED', 'branch deletion allowed (allow_deletions=true)'))
    if extra:
        findings.append(('DOC-STALE', 'live-required but not in the checklist (%d): %s — recorded; the checklist refresh follows its own fact-check procedure with this API provenance' % (len(extra), ', '.join(extra))))
    if code_owner is True:
        findings.append(('NOTE', 'the №471 lane is ACTIVE (require_code_owner_reviews=true) — the second-maintainer toggle recorded'))

    if any(f[0] == 'RED' for f in findings):
        verdict = RED
    rep = _report(ctx, {
        'strict': strict, 'enforce_admins': enforce,
        'pr_reviews': rpr is not None, 'code_owner_reviews': code_owner,
        'allow_force_pushes': force, 'allow_deletions': delete,
    }, expected, findings, extra=extra)
    return verdict, findings, rep


def _report(ctx, booleans, expected, findings, extra=None) -> dict:
    date = datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%d')
    red = any(f[0] == 'RED' for f in findings)
    verdict = RED if red else (INFRA if any(f[0] == 'INFRA' for f in findings) else GREEN)
    if ctx is None and verdict == INFRA:
        line = 'branch_protection_audit: %s INFRA (the state unreadable — loud, never silently OK)' % date
    elif ctx is None:
        line = 'branch_protection_audit: %s %s (protection off)' % (date, verdict)
    else:
        line = ('branch_protection_audit: %s %s contexts=%d missing=%d doc_stale_extra=%d '
                'strict=%s enforce_admins=%s pr_reviews=%s code_owner_reviews=%s '
                'allow_force_pushes=%s allow_deletions=%s' % (
                    date, verdict, len(ctx), len(set(expected['names']) - set(ctx)),
                    len(extra or []), booleans.get('strict'), booleans.get('enforce_admins'),
                    booleans.get('pr_reviews'), booleans.get('code_owner_reviews'),
                    booleans.get('allow_force_pushes'), booleans.get('allow_deletions')))
    lines = [line, '']
    if ctx is not None:
        lines.append('live required contexts (%d):' % len(ctx))
        lines += ['  - %s' % n for n in ctx]
    lines.append('checklist required names (%d): %s' % (len(expected['names']), ', '.join(expected['names'])))
    lines.append('source: %s' % expected['path'])
    lines.append('')
    for kind, text in findings:
        lines.append('[%s] %s' % (kind, text))
    if not findings:
        lines.append('no divergences — the live protection matches the checklist')
    return {'verdict': verdict, 'line': line, 'text': '\n'.join(lines) + '\n'}


# --------------------------------------------------------------- self-test

def _fixture(names, **kw) -> dict:
    # the fixture's required set derives from the PARSED CHECKLIST (the
    # SSOT), never from a hardcoded copy of it
    p = {
        'required_status_checks': {'strict': True, 'checks': [
            {'context': n, 'app_id': None} for n in kw.get('contexts', names)]},
        'enforce_admins': {'enabled': True},
        'required_pull_request_reviews': {
            'require_code_owner_reviews': kw.get('code_owner', False),
            'required_approving_review_count': 1},
        'allow_force_pushes': {'enabled': kw.get('force', False)},
        'allow_deletions': {'enabled': kw.get('delete', False)},
    }
    if kw.get('off'):
        return {'_protection_off': True}
    return p


def self_test(expected: dict) -> int:
    base = list(expected['names'])
    good = _fixture(base)
    cases = [
        ('all-good (the checklist set, booleans on)', good, GREEN),
        ('protection OFF', _fixture(base, off=True), RED),
        ('a required check missing from live', _fixture(base, contexts=base[1:]), RED),
        ('strict off (not up to date)', {**good, 'required_status_checks': {'strict': False, 'checks': good['required_status_checks']['checks']}}, RED),
        ('enforce_admins off (the bypass hole)', {**good, 'enforce_admins': {'enabled': False}}, RED),
        ('no PR reviews', {**good, 'required_pull_request_reviews': None}, RED),
        ('force pushes allowed', _fixture(base, force=True), RED),
        ('deletion allowed', _fixture(base, delete=True), RED),
        ('extra live checks (doc-stale, recorded not failed)', _fixture(base, contexts=base + ['new-job (blocking)']), GREEN),
        ('the №471 lane ACTIVE (recorded)', _fixture(base, code_owner=True), GREEN),
    ]
    bad = 0
    for name, prot, want in cases:
        verdict, findings, rep = audit(expected, prot)
        ok = verdict == want
        bad += 0 if ok else 1
        print('  %-52s want=%-5s got=%-5s %s' % (name[:52], want, verdict, 'ok' if ok else 'TRAP DID NOT SPRING'))
    print('self-test: %d/%d traps sprung correctly' % (len(cases) - bad, len(cases)))
    return 1 if bad else 0


# -------------------------------------------------------------------- main

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--repo', default=DEFAULT_REPO)
    ap.add_argument('--branch', default=DEFAULT_BRANCH)
    ap.add_argument('--checklist', default=CHECKLIST)
    ap.add_argument('--token', default=os.environ.get('BP_TOKEN') or os.environ.get('GH_TOKEN', ''))
    ap.add_argument('--protection-json', default=None,
                    help='read the live-state JSON from a file (the synthetic harness)')
    ap.add_argument('--report-out', default=None, help='write the report file (the artifact)')
    ap.add_argument('--self-test', action='store_true')
    args = ap.parse_args()

    expected = parse_checklist(args.checklist)

    if args.self_test:
        rc = self_test(expected)
        return rc

    if args.protection_json:
        protection = json.load(open(args.protection_json, encoding='utf-8'))
        src = 'file: %s' % args.protection_json
    else:
        if not args.token:
            print('FATAL: no token (BP_TOKEN/GH_TOKEN) — the state is unreadable, fail-closed INFRA', file=sys.stderr)
            return 2
        verdict, protection = fetch_protection(args.repo, args.branch, args.token)
        src = 'API: %s/branches/%s/protection' % (args.repo, args.branch)
        if verdict == INFRA:
            rep = _report(None, {}, expected, [('INFRA', protection.get('_error', 'unreadable'))])
            if args.report_out:
                os.makedirs(os.path.dirname(args.report_out) or '.', exist_ok=True)
                open(args.report_out, 'w', encoding='utf-8').write(rep['text'])
            print(rep['text'])
            print('VERDICT: INFRA — the audit cannot read the state; loud, never silently OK')
            return 2

    verdict, findings, rep = audit(expected, protection)
    rep['text'] = rep['text'].replace('source: %s' % expected['path'], 'source: %s (%s)' % (expected['path'], src))
    if args.report_out:
        os.makedirs(os.path.dirname(args.report_out) or '.', exist_ok=True)
        open(args.report_out, 'w', encoding='utf-8').write(rep['text'])
    print(rep['text'])
    print('VERDICT: %s' % rep['verdict'])
    return {'GREEN': 0, 'RED': 1, 'INFRA': 2}[rep['verdict']]


if __name__ == '__main__':
    sys.exit(main())
