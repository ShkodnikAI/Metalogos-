#!/usr/bin/env python3
"""№562 (issue #923; the audit 02.10 §6.1 — the proposal on 0.29): the
tag↔main gap gate — the release-hygiene criterion that turns the M-3
class ("a High fix lives outside a release") into a MEASURED state.

The M-3 fact: the High fix №523 landed on main and waited for a release
inside the CHANGELOG's [Unreleased] section while v0.27.1 stayed the
latest tag — an unverifiable limbo. This gate measures it:

  - if the CHANGELOG's [Unreleased] section carries a `### Security`
    part, every Security entry must be DATE-STAMPED (a YYYY-MM-DD date
    in the entry text — the fix/audit date); an unstamped Security
    entry is a FAIL (an undated pending fix cannot be measured —
    fail-closed, not fail-silent);
  - if the OLDEST stamp in the pending Security part is older than
    --max-age-days (default 14; the OWNER fixes the parameter — №562
    does not choose it), the gate FAILS naming the oldest date and the
    age;
  - no Security part in [Unreleased] → PASS (nothing is stuck).

Exit 0 = the release gap is healthy (or nothing pending); exit 1 = the
gap exceeded the floor (release the fixes — the publication itself
stays the OWNER's gate, №562 only measures).

--self-test runs the embedded fixtures (no pending Security, a fresh
Security part, an over-age Security part, an unstamped Security part)
and must print SELF-TEST OK.
"""

from __future__ import annotations

import os
import re
import sys
from datetime import date, datetime
from pathlib import Path

DEFAULT_MAX_AGE_DAYS = 14
DATE_RE = re.compile(r"(\d{4}-\d{2}-\d{2})")


def extract_unreleased_security(text: str) -> str | None:
    """The [Unreleased] section's `### Security` part, or None."""
    # the [Unreleased] section runs to the next `## [` heading
    m = re.search(r"^##\s*\[Unreleased\][^\n]*\n(.*?)(?=^##\s*\[)", text,
                  re.M | re.S)
    if not m:
        return None
    unreleased = m.group(1)
    m2 = re.search(r"^###\s+Security\s*\n(.*?)(?=^###\s|\Z)", unreleased,
                   re.M | re.S)
    if not m2:
        return None
    return m2.group(1)


def check_changelog(text: str, today: date, max_age_days: int):
    """Returns (status, detail) — status in {ok, fail}."""
    security = extract_unreleased_security(text)
    if security is None:
        return ("ok",
                "no pending Security entries in [Unreleased] — nothing is "
                "stuck outside a release")
    stamps = [datetime.strptime(d, "%Y-%m-%d").date()
              for d in DATE_RE.findall(security)]
    if not stamps:
        return ("fail",
                "a pending Security entry in [Unreleased] carries NO "
                "YYYY-MM-DD date stamp — an undated pending fix cannot be "
                "measured (stamp the fix/audit date; fail-closed, №562)")
    oldest = min(stamps)
    age = (today - oldest).days
    if age > max_age_days:
        return ("fail",
                f"the oldest pending Security entry is dated {oldest.isoformat()} "
                f"({age} days old > the {max_age_days}-day floor) — the fix "
                f"lives outside a release (the M-3 class, №562); release the "
                f"pending Security fixes")
    return ("ok",
            f"the oldest pending Security entry is {age} day(s) old "
            f"(the floor is {max_age_days}) — within the release-gap floor")


def self_test() -> int:
    ok = True
    today = date(2026, 10, 3)

    def run_case(name, text, expect, detail_substr=None):
        nonlocal ok
        status, detail = check_changelog(text, today, DEFAULT_MAX_AGE_DAYS)
        if status == expect and (detail_substr is None or detail_substr in detail):
            print(f"[{name}] {status}: {detail[:80]}")
        else:
            print(f"[{name}] expected {expect}/{detail_substr}, "
                  f"got {status}: {detail}")
            ok = False

    run_case(
        "no-security-part",
        "# Changelog\n\n## [Unreleased]\n\n- a change entry\n\n"
        "## [0.28.0] - 2026-10-03\n\n### Security\n\n- the released fix\n",
        "ok",
    )
    run_case(
        "fresh-security-pass",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n"
        "- **the fix (gh#1):** landed 2026-10-01, the audit 2026-10-01\n\n"
        "## [0.28.0] - 2026-09-01\n",
        "ok",
    )
    run_case(
        "over-age-security-fails",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n"
        "- **the fix (gh#1):** landed 2026-09-01, the audit 2026-09-15\n\n"
        "## [0.27.1] - 2026-08-01\n",
        "fail",
        "the oldest pending Security entry is dated 2026-09-01",
    )
    run_case(
        "unstamped-security-fails",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n"
        "- **the fix (gh#1):** no date anywhere in this entry\n\n"
        "## [0.28.0] - 2026-10-01\n",
        "fail",
        "NO YYYY-MM-DD date stamp",
    )
    run_case(
        "no-unreleased-at-all",
        "# Changelog\n\n## [0.28.0] - 2026-10-03\n\n### Security\n\n- fix\n",
        "ok",
    )

    if ok:
        print("SELF-TEST OK: no-security-part, fresh-security-pass, "
              "over-age-security-fails, unstamped-security-fails, "
              "no-unreleased-at-all")
        return 0
    print("SELF-TEST FAILED")
    return 1


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    changelog = os.environ.get(
        "N562_CHANGELOG",
        str(Path(__file__).resolve().parent.parent.parent / "CHANGELOG.md"),
    )
    max_age = DEFAULT_MAX_AGE_DAYS
    if "--max-age-days" in argv:
        max_age = int(argv[argv.index("--max-age-days") + 1])
    text = Path(changelog).read_text(encoding="utf-8")
    status, detail = check_changelog(text, date.today(), max_age)
    print(f"release-gap: {status.upper()} — {detail}")
    print(f"  (the floor N={max_age} days is the OWNER's parameter to fix — "
          f"№562 proposes 14, it does not choose)")
    return 1 if status == "fail" else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
