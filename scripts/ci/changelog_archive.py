#!/usr/bin/env python3
"""Naryad №676 (gh#1203, wave 41), item A7: split the CHANGELOG archive.

The audit 7c3456b §4 (A7): CHANGELOG.md grew to ~727 KB (the target is
<100 KB). The fix: the recent releases ([Unreleased], 0.30.1, 0.30.0,
0.29.0 — the Security-carrying live surface of the №588 protocol) stay
in the root CHANGELOG.md; every older release moves BYTE-EXACT into
docs/changelog/CHANGELOG-2025-06_2026-10.md. No record is lost, no
anchor breaks (no file outside CHANGELOG.md links to its section
anchors — verified by git grep), the archive head links back.

Run from the repository root: python3 scripts/ci/changelog_archive.py
Idempotence: refuses to run if the marker section already exists.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
MAIN = ROOT / "CHANGELOG.md"
ARCHIVE = ROOT / "docs" / "changelog" / "CHANGELOG-2025-06_2026-10.md"

KEEP_UNTIL = re.compile(r"^## \[0\.28\.2\] - 2026-10-06$", re.M)

ARCHIVE_HEAD = """# Changelog — archive (2025-06 → 2026-10, releases 0.5.0 → 0.28.2)

The archived release notes of the Metalogos project, moved BYTE-EXACT
from the root `CHANGELOG.md` by Naryad №676 (gh#1203, wave 41, item A7
of the audit 7c3456b §4): the root file keeps the live releases
([Unreleased], 0.29.0, 0.30.0, 0.30.1) under the 100 KB hygiene target;
every record survives, unmodified, here. The current changelog lives in
[CHANGELOG.md](../../CHANGELOG.md).

"""

POINTER = """## [Archived releases — 0.5.0 → 0.28.2]

Every release older than 0.29.0 lives in
[docs/changelog/CHANGELOG-2025-06_2026-10.md](docs/changelog/CHANGELOG-2025-06_2026-10.md)
— moved byte-exact by Naryad №676 (A7), no record lost, the №588
Security records of the live releases stay in this file.
"""


def main() -> int:
    if "CHANGELOG-2025-06_2026-10.md" in MAIN.read_text(encoding="utf-8"):
        print("FAIL: the archive pointer already exists — refusing to double-split", file=sys.stderr)
        return 1
    text = MAIN.read_text(encoding="utf-8")
    m = KEEP_UNTIL.search(text)
    if not m:
        print("FAIL: the split anchor '## [0.28.2] - 2026-10-06' not found", file=sys.stderr)
        return 1
    keep, archive = text[: m.start()], text[m.start():]

    ARCHIVE.parent.mkdir(parents=True, exist_ok=True)
    ARCHIVE.write_text(ARCHIVE_HEAD + archive, encoding="utf-8")
    MAIN.write_text(keep.rstrip("\n") + "\n\n" + POINTER, encoding="utf-8")

    kb = lambda p: p.stat().st_size / 1024
    print(f"OK: main CHANGELOG.md now {kb(MAIN):.0f} KB; archive {kb(ARCHIVE):.0f} KB")
    print(f"OK: archived release sections: {len(re.findall(r'^## \[', archive, re.M))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
