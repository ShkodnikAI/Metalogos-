#!/usr/bin/env python3
"""Naryad №497: the README version badge is RENDERED from Cargo.toml.

The v0.27.0 release shipped a stale badge (v0.26.1 while the Cargo
version was 0.27.0) — a double desync, caught by the audit 28.09 §3.4.
The badge line is no longer hand-edited: this script renders it from
`[workspace.package] version` and verifies it in CI (the blocking
"Badge sync" job fails on any desync).

Usage:
    python3 scripts/ci/badge_sync.py            # render (fix) the badge
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

# The Version badge line rendered by the shields.io pattern used in README.
BADGE_RE = re.compile(
    r'(\[!\[Version\]\(https://img\.shields\.io/badge/)(v[\w.]+)(-blue\.svg\)\])'
)


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

    with open(README, encoding='utf-8') as f:
        text = f.read()

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
        return 0

    if check:
        print(
            f'FAIL: the README badge says {current} but Cargo.toml says {version} '
            f'(expected {want}) — run scripts/ci/badge_sync.py to render it.',
            file=sys.stderr,
        )
        return 1

    new_text = BADGE_RE.sub(lambda m: f'{m.group(1)}{want}{m.group(3)}', text)
    with open(README, 'w', encoding='utf-8') as f:
        f.write(new_text)
    print(f'OK: the badge rendered {current} -> {want} (from Cargo.toml {version}).')
    return 0


if __name__ == '__main__':
    sys.exit(main())
