#!/usr/bin/env python3
"""№865 (issue #1707, P0 — the migration honesty 0.35–0.38): the
repeat-gate — a new deny-path on a carrier cannot land without its
BREAKING row.

The 0.36.0/0.38.0 lesson (the №865 fact list): the №845 sharp-set rule
and the №861 `cron_run` mandate both shipped documented as `### Added`
only — a reader upgrading from 0.34 saw "existing programs
byte-unchanged" (the №844 grant-carrier clause) and broke at the first
http_post / cron_run. This gate closes the repeat class:

  - it diffs the deny-path rosters between the previous release tag and
    the current tree: `GRANT_ALWAYS` and `PROFILE_PERMIT_SHARP`
    (`src/capability.rs`), `HUMAN_OVERSIGHT` (`src/approval.rs`) —
    a name ADDED to any of them is a NEW deny path on a carrier;
  - if the tag range adds such a name, the pending CHANGELOG version
    section ([Unreleased] when it carries content, else the first
    stamped `## [X.Y.Z]` section) must contain a `### Changed` part
    whose text contains `BREAKING` and names EVERY added carrier;
  - no roster addition → green (nothing to migrate, nothing to
    document).

Exit 0 = green; exit 1 = red (the release read does not pass — the
breaking change ships with its migration row or it does not ship).
The publication itself stays the OWNER's act (ADR-0186 §4.5); this
gate only refuses to certify a dishonest cut.

Fail-closed: an unreadable roster site at HEAD, an unreadable
CHANGELOG, or a tag range that cannot be computed is a RED with the
reason named, never a silent pass. (A repository with NO release tags
at all has no baseline to diff against — the gate reports the SKIP
loudly and exits 0, because there is no release machinery to protect
yet; the first tag switches it to the fail-closed read.)

--self-test runs the embedded mutation fixtures (the green tree, the
roster mutation without the row → RED, the same mutation with the row
→ GREEN, a partial row → RED, a BREAKING row without the name → RED)
and must print SELF-TEST OK.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The deny-path rosters: (file, const name). A name added to one of
# these is a NEW refusal on an existing (or new) carrier. The list is
# extensible — the next deny roster lands here in the same PR as the
# roster itself (the №865 discipline).
ROSTERS: list[tuple[str, str]] = [
    ("src/capability.rs", "GRANT_ALWAYS"),
    ("src/capability.rs", "PROFILE_PERMIT_SHARP"),
    ("src/approval.rs", "HUMAN_OVERSIGHT"),
]

ROSTER_RE = r"pub\s+const\s+{name}\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\];"
NAME_RE = re.compile(r'"([^"]+)"')


def extract_roster(source: str, const_name: str) -> list[str] | None:
    """The quoted names of one roster constant, or None when the const
    is absent from the source."""
    m = re.search(ROSTER_RE.format(name=const_name), source, re.S)
    if not m:
        return None
    return NAME_RE.findall(m.group(1))


def git(*args: str) -> str:
    p = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip() or "git failed")
    return p.stdout


def latest_tag() -> str | None:
    """The newest release tag reachable from HEAD (semver tags only —
    the release machinery's own naming)."""
    out = subprocess.run(
        ["git", "tag", "--list", "v*", "--sort=-v:refname"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if out.returncode != 0:
        return None
    tags = [t.strip() for t in out.stdout.splitlines() if t.strip()]
    return tags[0] if tags else None


def rosters_at(rev: str | None) -> dict[tuple[str, str], list[str] | None]:
    """The roster name sets at a revision (None = the current worktree).
    A missing FILE at the revision reads as an empty roster set (the
    file did not exist yet); a missing CONST reads as None."""
    result: dict[tuple[str, str], list[str] | None] = {}
    for path, name in ROSTERS:
        if rev is None:
            f = ROOT / path
            if not f.is_file():
                raise RuntimeError(f"the roster site is unreadable at HEAD: {path}")
            source = f.read_text(encoding="utf-8")
        else:
            probe = subprocess.run(
                ["git", "cat-file", "-e", f"{rev}:{path}"],
                cwd=ROOT, capture_output=True, text=True,
            )
            if probe.returncode != 0:
                result[(path, name)] = []
                continue
            source = git("show", f"{rev}:{path}")
        result[(path, name)] = extract_roster(source, name)
    return result


def added_deny_names(base_rev: str) -> dict[str, list[str]]:
    """The names ADDED per roster between the base revision and the
    worktree. A roster that exists at HEAD but not at the base
    contributes ALL its names (a new roster is a deny-path growth)."""
    base = rosters_at(base_rev)
    head = rosters_at(None)
    added: dict[str, list[str]] = {}
    for key, head_names in head.items():
        label = f"{key[0]}::{key[1]}"
        base_names = base.get(key)
        if head_names is None:
            # the const vanished — a roster deletion is not a deny-path
            # addition (the loosening direction has its own gates);
            # nothing to flag here
            continue
        old = set(base_names or [])
        fresh = [n for n in head_names if n not in old]
        if fresh:
            added[label] = fresh
    return added


def pending_section(changelog: str) -> tuple[str, str]:
    """The pending version section: [Unreleased] when it carries
    content, else the first stamped `## [X.Y.Z]` section. Returns
    (label, section_text)."""
    heads = list(re.finditer(r"^##\s+\[([^\]]+)\][^\n]*\n", changelog, re.M))
    for i, m in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(changelog)
        body = changelog[m.end():end]
        label = m.group(1)
        if label == "Unreleased":
            if body.strip():
                return (label, body)
            # an empty [Unreleased] — fall through to the first stamped
            continue
        return (label, body)
    raise RuntimeError("no version sections found in CHANGELOG.md")


def breaking_covers(section_body: str, names: list[str]) -> tuple[bool, str]:
    """True when the section's `### Changed` part contains `BREAKING`
    and names every added carrier."""
    changed_parts = re.findall(
        r"^###\s+Changed\s*\n(.*?)(?=^###\s|\Z)", section_body, re.M | re.S
    )
    if not changed_parts:
        return (False, "the section carries no `### Changed` part")
    changed_text = "\n".join(changed_parts)
    if "BREAKING" not in changed_text:
        return (False, "the `### Changed` part carries no BREAKING mark")
    missing = [n for n in names if n not in changed_text]
    if missing:
        return (False, "the BREAKING row does not name: " + ", ".join(missing))
    return (True, "the BREAKING row covers every added carrier")


def check(base_rev: str | None, changelog_text: str) -> tuple[str, str]:
    """Returns (status, detail) — status in {ok, fail, skip}."""
    if base_rev is None:
        return ("skip",
                "no release tag found — no deny-roster baseline to diff "
                "against (the first tag switches this gate to the "
                "fail-closed read)")
    try:
        added = added_deny_names(base_rev)
    except RuntimeError as e:
        return ("fail", f"the roster diff is unreadable (fail-closed): {e}")
    if not added:
        return ("ok",
                f"no deny-path name added to the rosters since {base_rev} "
                "— nothing to document")
    all_names = sorted(n for names in added.values() for n in names)
    roster_note = "; ".join(
        f"{roster}: {', '.join(names)}" for roster, names in added.items()
    )
    try:
        label, body = pending_section(changelog_text)
    except RuntimeError as e:
        return ("fail", f"the CHANGELOG is unparsable (fail-closed): {e}")
    ok, why = breaking_covers(body, all_names)
    detail = (
        f"the range {base_rev}..HEAD adds deny-path names ({roster_note}); "
        f"the pending section [{label}]: {why}"
    )
    return ("ok" if ok else "fail", detail)


def self_test() -> int:
    ok = True

    def run_case(name, base, head_rosters, changelog, expect, substr=None):
        nonlocal ok
        real_rosters_at = rosters_at
        real_latest = latest_tag
        try:
            # pin the fixture world: rosters_at(rev) — the base state,
            # rosters_at(None) — the head state (the rev argument is
            # ignored: the fixtures carry one base revision).
            def fake_rosters_at(rev):
                src = base if rev is not None else head_rosters
                return {
                    ("src/capability.rs", "GRANT_ALWAYS"):
                        extract_roster(src, "GRANT_ALWAYS"),
                    ("src/capability.rs", "PROFILE_PERMIT_SHARP"):
                        extract_roster(src, "PROFILE_PERMIT_SHARP"),
                    ("src/approval.rs", "HUMAN_OVERSIGHT"):
                        extract_roster(src, "HUMAN_OVERSIGHT"),
                }
            # rebind the module-level helpers the check() reads
            globals()["rosters_at"] = fake_rosters_at
            globals()["latest_tag"] = lambda: "v9.9.9"
            status, detail = check("v9.8.0", changelog)
        finally:
            globals()["rosters_at"] = real_rosters_at
            globals()["latest_tag"] = real_latest
        if status == expect and (substr is None or substr in detail):
            print(f"[{name}] {status}: {detail[:90]}")
        else:
            print(f"[{name}] expected {expect}/{substr}, got {status}: {detail}")
            ok = False

    base_rs = 'pub const GRANT_ALWAYS: &[&str] = &["forget"];\n'
    base_rs += (
        'pub const PROFILE_PERMIT_SHARP: &[&str] = &["exec", "http_post"];\n'
    )
    base_rs += 'pub const HUMAN_OVERSIGHT: &[&str] = &["cron_run"];\n'

    run_case(
        "clean-tree",
        base_rs, base_rs,
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-01-01\n",
        "ok", "no deny-path name added",
    )
    run_case(
        "mutation-without-row",
        base_rs, base_rs.replace('"exec", "http_post"', '"exec", "http_post", "ssh_send"'),
        "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- a new carrier ssh_send\n"
        "\n## [0.1.0] - 2026-01-01\n",
        "fail", "no `### Changed` part",
    )
    run_case(
        "mutation-with-row",
        base_rs, base_rs.replace('"exec", "http_post"', '"exec", "http_post", "ssh_send"'),
        "# Changelog\n\n## [Unreleased]\n\n### Changed\n\n- **BREAKING:**"
        " `ssh_send` now refuses without a grant\n"
        "\n## [0.1.0] - 2026-01-01\n",
        "ok", "covers every added carrier",
    )
    run_case(
        "mutation-partial-row",
        base_rs,
        base_rs.replace('"exec", "http_post"', '"exec", "http_post", "ssh_send"')
               .replace('"cron_run"', '"cron_run", "ssh_exec"'),
        "# Changelog\n\n## [Unreleased]\n\n### Changed\n\n- **BREAKING:**"
        " `ssh_send` now refuses without a grant\n"
        "\n## [0.1.0] - 2026-01-01\n",
        "fail", "does not name: ssh_exec",
    )
    run_case(
        "breaking-without-name",
        base_rs, base_rs.replace('"cron_run"', '"cron_run", "ssh_exec"'),
        "# Changelog\n\n## [Unreleased]\n\n### Changed\n\n- **BREAKING:**"
        " some carriers changed\n"
        "\n## [0.1.0] - 2026-01-01\n",
        "fail", "does not name: ssh_exec",
    )
    run_case(
        "fallthrough-to-stamped-section",
        base_rs, base_rs.replace('"cron_run"', '"cron_run", "ssh_exec"'),
        "# Changelog\n\n## [Unreleased]\n\n## [0.2.0] - 2026-02-01\n\n"
        "### Changed\n\n- **BREAKING:** `ssh_exec` refuses\n",
        "ok", "pending section [0.2.0]",
    )

    print("SELF-TEST OK" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    tag = latest_tag()
    changelog_path = ROOT / "CHANGELOG.md"
    if not changelog_path.is_file():
        print("FAIL: CHANGELOG.md is unreadable (fail-closed)")
        return 1
    changelog_text = changelog_path.read_text(encoding="utf-8")
    status, detail = check(tag, changelog_text)
    mark = {"ok": "GREEN", "fail": "RED", "skip": "SKIP"}[status]
    print(f"breaking-change-gate: {mark} — {detail}")
    if status == "fail":
        print(
            "::error::the deny-path growth has no BREAKING migration row "
            "(№865): add a `### Changed` BREAKING row naming the carriers "
            "to the pending CHANGELOG section — see docs/upgrade.md"
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
