#!/usr/bin/env python3
"""Naryad №497: the README version badge is RENDERED from Cargo.toml.

The v0.27.0 release shipped a stale badge (v0.26.1 while the Cargo
version was 0.27.0) — a double desync, caught by the audit 28.09 §3.4.
The badge line is no longer hand-edited: this script renders it from
`[workspace.package] version` and verifies it in CI (the blocking
"Badge sync" job fails on any desync).

Naryad №676 (D5, the audit 7c3456b §4): the CI badge is RENDERED from
.github/workflows/ci.yml the same way — the count of job-level blocking
jobs (names containing "(blocking" at the job level). The static
«CI — 28 blocking jobs» badge had drifted (the real count is 44); a
hand-edited count can never stay true, so the badge is no longer
hand-edited.

Naryad №690 (gh#1257, the audit f92c61c D5): the badge itself is now
the REAL GitHub workflow badge
(actions/workflows/ci.yml/badge.svg?branch=main — it turns RED when
CI fails; the static brightgreen shields picture never did). The
№676 honesty survives intact: the blocking-job COUNT still renders
from ci.yml into the text beside the badge — the machine renders,
the human never hand-edits. The old shields CI form is retired; its
desync proved the point (the static line claimed a stale count
while the job count had moved).

Usage:
    python3 scripts/ci/badge_sync.py            # render (fix) the badges
    python3 scripts/ci/badge_sync.py --check    # verify only, exit 1 on desync
"""
import re
import subprocess
import sys

ROOT = subprocess.run(
    ['git', 'rev-parse', '--show-toplevel'], capture_output=True, text=True, check=True
).stdout.strip()

CARGO_TOML = f'{ROOT}/Cargo.toml'
README = f'{ROOT}/README.md'
CI_YML = f'{ROOT}/.github/workflows/ci.yml'

# The Version badge line rendered by the shields.io pattern used in README.
BADGE_RE = re.compile(
    r'(\[!\[Version\]\(https://img\.shields\.io/badge/)(v[\w.]+)(-blue\.svg\)\])'
)

# The CI badge line (№676 D5; №690 moved the badge to the REAL GitHub
# workflow badge — the count stays RENDERED into the text beside it):
#   [![CI](https://github.com/.../actions/workflows/ci.yml/badge.svg?branch=main)](...)
#   — the real workflow status (N blocking jobs on every push and PR; light/dark adaptive)
CI_BADGE_RE = re.compile(
    r'(\[!\[CI\]\(https://github\.com/ShkodnikAI/Metalogos-/actions/workflows/'
    r'ci\.yml/badge\.svg\?branch=main\)\]\(https://github\.com/ShkodnikAI/'
    r'Metalogos-/actions/workflows/ci\.yml\) — the real workflow status \()(\d+)'
    r'( blocking jobs on every push and PR; light/dark adaptive\))'
)

# A job-LEVEL name line: exactly 4 spaces + `name:` (step names carry
# 6+ spaces and a `- ` bullet, so they never match).
JOB_NAME_RE = re.compile(r'^    name: .+\(blocking[ )]', re.M)


def blocking_job_count() -> int:
    """The SSOT count: job-level names containing "(blocking" in ci.yml."""
    with open(CI_YML, encoding='utf-8') as f:
        return len(JOB_NAME_RE.findall(f.read()))


def cargo_version() -> str:
    with open(CARGO_TOML, encoding='utf-8') as f:
        for line in f:
            m = re.match(r'^version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    raise SystemExit('FAIL: no version in [workspace.package] of Cargo.toml')


def main() -> int:
    check = '--check' in sys.argv
    version = cargo_version()
    want = f'v{version}'
    ci_count = blocking_job_count()

    with open(README, encoding='utf-8') as f:
        text = f.read()

    rc = 0

    # ── the Version badge (№497) ──
    matches = BADGE_RE.findall(text)
    if len(matches) != 1:
        print(
            f'FAIL: expected exactly one Version badge line in README.md, found {len(matches)}',
            file=sys.stderr,
        )
        return 1

    current = matches[0][1]
    if current == want:
        print(f'OK: the badge ({current}) matches the Cargo version ({version}).')
    elif check:
        print(
            f'FAIL: the README badge says {current} but Cargo.toml says {version} '
            f'(expected {want}) — run scripts/ci/badge_sync.py to render it.',
            file=sys.stderr,
        )
        return 1
    else:
        text = BADGE_RE.sub(lambda m: f'{m.group(1)}{want}{m.group(3)}', text)
        print(f'OK: the badge rendered {current} -> {want} (from Cargo.toml {version}).')
        rc = 2  # rendered

    # ── the CI badge (№676 D5) ──
    ci_matches = CI_BADGE_RE.findall(text)
    if len(ci_matches) != 1:
        print(
            f'FAIL: expected exactly one CI badge line in README.md, found {len(ci_matches)}',
            file=sys.stderr,
        )
        return 1

    ci_current = int(ci_matches[0][1])
    if ci_current == ci_count:
        print(f'OK: the CI badge ({ci_current}) matches the ci.yml blocking-job count ({ci_count}).')
    elif check:
        print(
            f'FAIL: the README CI badge says {ci_current} blocking jobs but ci.yml has {ci_count} '
            f'— run scripts/ci/badge_sync.py to render it.',
            file=sys.stderr,
        )
        return 1
    else:
        text = CI_BADGE_RE.sub(lambda m: f'{m.group(1)}{ci_count}{m.group(3)}', text)
        print(f'OK: the CI badge rendered {ci_current} -> {ci_count} (from ci.yml).')
        rc = 2  # rendered

    if rc == 2:
        with open(README, 'w', encoding='utf-8') as f:
            f.write(text)
    return 0


if __name__ == '__main__':
    sys.exit(main())
