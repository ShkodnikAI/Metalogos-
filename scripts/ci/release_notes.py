#!/usr/bin/env python3
"""Naryad №497: the GitHub Release body is GENERATED from the CHANGELOG.

The release description is never hand-written: this script extracts the
`## [VERSION]` section of CHANGELOG.md verbatim and prints it, so the
release notes and the changelog can never disagree.

Usage:
    python3 scripts/ci/release_notes.py 0.27.1
"""
import re
import sys

SECTIONS = [
    r'\d+\.\d+\.\d+',
    'Unreleased',
]


def main() -> int:
    if len(sys.argv) != 2:
        print('usage: release_notes.py <version>', file=sys.stderr)
        return 2
    version = sys.argv[1].lstrip('v')

    with open('CHANGELOG.md', encoding='utf-8') as f:
        text = f.read()

    head_re = re.compile(r'^## \[(' + '|'.join(SECTIONS) + r')\]', re.M)
    heads = list(head_re.finditer(text))
    if not heads:
        print('FAIL: no version sections found in CHANGELOG.md', file=sys.stderr)
        return 1

    target = None
    for i, m in enumerate(heads):
        if m.group(1) == version:
            line_end = text.find('\n', m.start()) + 1
            end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
            target = (line_end, end)
            break
    if target is None:
        print(f'FAIL: section [{version}] not found in CHANGELOG.md', file=sys.stderr)
        return 1

    section = text[target[0]:target[1]].strip('\n')
    print(section)
    return 0


if __name__ == '__main__':
    sys.exit(main())
