#!/usr/bin/env python3
"""№899 (issue #1793; the ADR-0189 amendment §5 — the calendar train):
the release-train numbering gate — the machine of the §5.2 rules.

The R38-era train produced FOUR minor releases in three days
(30.10–01.11) and the 0.37 number skip; the amendment (2026-11-05)
replaces the Monday-only window with the calendar cycle (Sat — the rc
candidate, Sun — the office stand, Mon — the final tag on the SAME
commit) and adds the numbering honesty:

  1. MINOR NUMBERING — a final train tag `vX.Y.0` created ON or AFTER
     the amendment date must continue the minor sequence: its minor
     equals the previous train's minor + 1 (the 0.37-class skip is
     RED). Trains created BEFORE the amendment date are grandfathered
     (the historical 0.36→0.38 gap stays a recorded fact, not a red).
  2. RC ↔ FINAL SHA — a final train tag `vX.Y.0` created post-amendment
     must carry at least one `vX.Y.0-rc.N` candidate AND point at the
     SAME commit as the HIGHEST rc of that train (the commit the
     office stand verified — a final tag on a different SHA is RED).
  3. RC NUMBERING — the rc candidates of one train are numbered 1..N
     without gaps (`-rc.1`, `-rc.2`, …); a gap is RED.
  4. TAG SHAPE — every `v`-tag matches `vMAJOR.MINOR.PATCH` with an
     optional `-rc.N` suffix; anything else is RED.

Patch tags (`vX.Y.Z`, Z > 0) are outside the rc contract (the §2.6
unscheduled security cut stays legal on any day) and outside the
minor-numbering rule; they only obey the tag-shape rule.

--final-tag vX.Y.0 runs the same rules against a NOT-YET-CUT final tag
(the release.yml pre-build step: the final build refuses before any
asset is produced if the train is dishonest).

--self-test runs the embedded fixtures (the post-amendment gap, the
grandfathered historical gap, the missing rc, the SHA mismatch, the rc
gap, the happy path) against the fixture tag source and must print
SELF-TEST OK.

Exit 0 = the train is honest; exit 1 = the drift named.
"""
from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone

# The ADR-0189 amendment acceptance (№899, 2026-11-05): the rules bind
# the tags created from this date forward (the §5.2 grandfathering).
AMENDMENT_DATE = datetime(2026, 11, 5, tzinfo=timezone.utc)

TAG_RE = re.compile(
    r"^v(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)"
    r"(?:-rc\.(?P<rc>\d+))?$"
)


@dataclass
class Tag:
    name: str
    major: int
    minor: int
    patch: int
    rc: int | None
    created: datetime
    sha: str


def git_tag_source() -> list[Tag]:
    """The real tag source: `git for-each-ref` over refs/tags."""
    out = subprocess.run(
        [
            "git",
            "for-each-ref",
            "refs/tags",
            "--format=%(refname:short)\t%(creatordate:iso-strict)\t%(*objectname)",
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    tags: list[Tag] = []
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) != 3:
            continue
        name, created_s, sha = parts
        m = TAG_RE.match(name)
        if not m:
            # a non-conforming v-tag is reported by the shape rule
            tags.append(Tag(name, -1, -1, -1, None,
                            datetime.now(timezone.utc), sha or "unknown"))
            continue
        created = datetime.fromisoformat(created_s)
        if created.tzinfo is None:
            created = created.replace(tzinfo=timezone.utc)
        tags.append(
            Tag(
                name,
                int(m.group("major")),
                int(m.group("minor")),
                int(m.group("patch")),
                int(m.group("rc")) if m.group("rc") else None,
                created,
                sha or "unknown",
            )
        )
    return tags


def check_tags(tags: list[Tag], final_tag: str | None = None) -> list[str]:
    """The §5.2 rules over a tag list. Returns the FAIL lines (empty = green)."""
    fails: list[str] = []

    v_tags = [t for t in tags if t.major >= 0]
    bad_shape = [t.name for t in tags if t.major < 0]
    for name in sorted(bad_shape):
        fails.append(
            f"tag shape: '{name}' does not match vMAJOR.MINOR.PATCH[-rc.N]"
        )

    # ── the rc bookkeeping per (major, minor, patch) train ──
    trains: dict[tuple[int, int, int], list[Tag]] = {}
    for t in v_tags:
        if t.rc is not None:
            trains.setdefault((t.major, t.minor, t.patch), []).append(t)
    for key, rcs in trains.items():
        rcs.sort(key=lambda t: t.rc)
        nums = [t.rc for t in rcs]
        if nums != list(range(1, len(nums) + 1)):
            fails.append(
                f"rc numbering: train v{key[0]}.{key[1]}.{key[2]} carries "
                f"{[t.name for t in rcs]} — the rc numbers must run 1..N "
                f"without gaps"
            )

    # ── the final trains (patch == 0), sorted by version ──
    finals = sorted(
        (t for t in v_tags if t.rc is None and t.patch == 0),
        key=lambda t: (t.major, t.minor),
    )

    def post_amendment(t: Tag) -> bool:
        return t.created >= AMENDMENT_DATE

    # 1. the minor numbering (post-amendment trains only)
    for prev, cur in zip(finals, finals[1:]):
        if post_amendment(cur):
            if (cur.major, cur.minor) != (prev.major, prev.minor + 1):
                fails.append(
                    f"minor numbering: {cur.name} follows {prev.name} — "
                    f"a post-amendment train must be the previous minor + 1 "
                    f"(expected v{prev.major}.{prev.minor + 1}.0, the "
                    f"0.37-class skip is refused)"
                )

    # 2. the rc↔final SHA contract (post-amendment trains only)
    checked = (
        [t for t in finals if t.name == final_tag]
        if final_tag
        else [t for t in finals if post_amendment(t)]
    )
    for final in checked:
        if not final_tag and not post_amendment(final):
            continue
        rcs = trains.get((final.major, final.minor, final.patch), [])
        if not rcs:
            fails.append(
                f"rc contract: {final.name} has NO -rc.N candidate — a "
                f"post-amendment train departs only from a stand-verified "
                f"candidate (the patch path stays the §2.6 exception)"
            )
            continue
        last_rc = max(rcs, key=lambda t: t.rc)
        if final.sha != last_rc.sha:
            fails.append(
                f"rc↔final SHA: {final.name} points at {final.sha[:12]} but "
                f"the last candidate {last_rc.name} points at "
                f"{last_rc.sha[:12]} — the final tag must sit on the SAME "
                f"commit the stand verified"
            )
    return fails


def main() -> int:
    final_tag = None
    if "--self-test" in sys.argv:
        return self_test()
    if "--final-tag" in sys.argv:
        final_tag = sys.argv[sys.argv.index("--final-tag") + 1]
        if not TAG_RE.match(final_tag) or "-rc." in final_tag:
            print(f"FAIL: --final-tag {final_tag} is not a final vMAJOR.MINOR.PATCH")
            return 1
    fails = check_tags(git_tag_source(), final_tag)
    if fails:
        print("release-train gate: RED — the ADR-0189 §5.2 drift:")
        for f in fails:
            print(f"  - {f}")
        return 1
    print(
        "release-train gate: OK — the numbering runs consecutively, the "
        "post-amendment trains sit on their stand-verified candidates"
    )
    return 0


def self_test() -> int:
    """The embedded fixtures: each trap springs, the happy path passes."""
    cases = [
        (
            "post-amendment gap 0.40 after 0.38 → RED",
            [
                Tag("v0.38.0", 0, 38, 0, None, _d("2026-11-01T00:00:00Z"), "sha038"),
                Tag("v0.38.0-rc.1", 0, 38, 0, 1, _d("2026-10-31T00:00:00Z"), "sha038"),
                Tag("v0.40.0", 0, 40, 0, None, _d("2026-11-09T00:00:00Z"), "sha040"),
                Tag("v0.40.0-rc.1", 0, 40, 0, 1, _d("2026-11-08T00:00:00Z"), "sha040"),
            ],
            ["minor numbering"],
        ),
        (
            "grandfathered historical gap 0.36→0.38 → green",
            [
                Tag("v0.36.0", 0, 36, 0, None, _d("2026-10-31T06:00:00Z"), "sha036"),
                Tag("v0.38.0", 0, 38, 0, None, _d("2026-11-01T04:00:00Z"), "sha038"),
                Tag("v0.38.0-rc.1", 0, 38, 0, 1, _d("2026-10-31T00:00:00Z"), "sha038"),
            ],
            [],
        ),
        (
            "final without rc → RED",
            [
                Tag("v0.39.0", 0, 39, 0, None, _d("2026-11-09T12:00:00Z"), "sha039"),
            ],
            ["rc contract"],
        ),
        (
            "final on a different SHA than the last rc → RED",
            [
                Tag("v0.39.0-rc.1", 0, 39, 0, 1, _d("2026-11-07T12:00:00Z"), "sharc1"),
                Tag("v0.39.0-rc.2", 0, 39, 0, 2, _d("2026-11-08T12:00:00Z"), "sharc2"),
                Tag("v0.39.0", 0, 39, 0, None, _d("2026-11-09T12:00:00Z"), "shafinal"),
            ],
            ["rc↔final SHA"],
        ),
        (
            "rc gap -rc.1 → -rc.3 → RED",
            [
                Tag("v0.39.0-rc.1", 0, 39, 0, 1, _d("2026-11-07T12:00:00Z"), "sha1"),
                Tag("v0.39.0-rc.3", 0, 39, 0, 3, _d("2026-11-08T12:00:00Z"), "sha3"),
                Tag("v0.39.0", 0, 39, 0, None, _d("2026-11-09T12:00:00Z"), "sha3"),
            ],
            ["rc numbering"],
        ),
        (
            "happy path: rc.1@sha == final@sha, consecutive minors → green",
            [
                Tag("v0.38.0", 0, 38, 0, None, _d("2026-11-01T04:00:00Z"), "sha038"),
                Tag("v0.39.0-rc.1", 0, 39, 0, 1, _d("2026-11-07T12:00:00Z"), "sharc"),
                Tag("v0.39.0", 0, 39, 0, None, _d("2026-11-09T12:00:00Z"), "sharc"),
            ],
            [],
        ),
        (
            "patch cuts carry no rc contract → green",
            [
                Tag("v0.39.0-rc.1", 0, 39, 0, 1, _d("2026-11-07T12:00:00Z"), "sharc"),
                Tag("v0.39.0", 0, 39, 0, None, _d("2026-11-09T12:00:00Z"), "sharc"),
                Tag("v0.39.1", 0, 39, 1, None, _d("2026-11-10T12:00:00Z"), "shapatch"),
            ],
            [],
        ),
    ]
    ok = True
    for name, tags, expect_substrings in cases:
        fails = check_tags(tags)
        hit = all(any(sub in f for f in fails) for sub in expect_substrings)
        clean = fails == [] and not expect_substrings
        if not (hit or clean):
            ok = False
            print(f"  TRAP DID NOT SPRING: {name} — got {fails}")
        else:
            print(f"  ok: {name}")
    print("SELF-TEST OK" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def _d(s: str) -> datetime:
    return datetime.fromisoformat(s.replace("Z", "+00:00"))


if __name__ == "__main__":
    sys.exit(main())
