#!/usr/bin/env python3
"""No stale grant-application references (naryad №786, gh#1484).

The owner's directive (23.10, repeated 24.10): the grant-application
(funding-programme) mentions are swept out of Metalogos — the milestone
M1 was closed by the owner (#1135). This gate keeps them from returning
(the dispatch-template recurrence root). Closed milestones are not
silently deleted: the sweep is documented in the current REALITY section.

OUT OF SCOPE (untouched by design): the language capability grants
(`grant_issue`, `src/grants.rs`, `IRREVERSIBLE_NO_GRANT`, ADR-0167) and
the Open Collective sponsorship (`.github/FUNDING.yml`, the README badge).
"""
import re
import subprocess
import sys

BANNED = re.compile(r'nlnet|restack', re.IGNORECASE)
# The tracked text surfaces (Cargo.lock is dependency noise, excluded).
PATHS = ['docs', 'README.md', 'llms.txt', 'AGENTS.md', 'CLAUDE.md', 'CHANGELOG.md', '.github', 'examples', 'self-host']

def main() -> int:
    res = subprocess.run(
        ['git', 'grep', '-I', '-i', '-n', '-E', 'nlnet|restack', '--'] + PATHS,
        capture_output=True, text=True,
    )
    hits = [line for line in res.stdout.splitlines() if BANNED.search(line)]
    if hits:
        print('STALE GRANT-APPLICATION REFERENCES (the funding-programme mentions are removed by the owner directive 23.10, gh#1484):')
        for h in hits:
            print('  ' + h)
        print('Rewrite the line without the programme names (the closed-milestone records stay, the names go) or drop the mention.')
        return 1
    print('no-stale-grant-refs: clean (the funding-programme mentions absent; grant_issue/Open Collective untouched)')
    return 0

if __name__ == '__main__':
    sys.exit(main())
