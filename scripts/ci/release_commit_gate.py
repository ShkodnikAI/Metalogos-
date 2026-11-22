#!/usr/bin/env python3
"""№976 (issue #1985, the consolidated audit 18.11 — finding A): the
RELEASE-COMMIT gate — what the release PROMISES (the live required set)
is what the release commit actually RAN (its check-runs), verified
mechanically on the tag/release path, not on a PR.

The audit fact this closes: v0.40.0 was tagged from a commit whose CI
gave ZERO check-runs — the release shipped with nothing behind it. This
gate runs on the tag path: every context the live required set demands
AS OF the release instant must be PRESENT and GREEN on the release
commit; anything missing, red, pending or skipped is a loud refusal
(fail-closed). A context that became required AFTER the release instant
does not charge the release (the №960 direction) — a release is judged
by the promises that existed when it was cut.

№976 follow-up 2 (the owner decision 21.11, «принимаю») — the gate
learns the PATCH SLICE from an old tag. Two heads of the same
structural problem:

1. **The context waiver (the conjunction, both conditions at once).**
   A required context is waived on a patch release (vX.Y.Z, Z>0) ONLY
   when BOTH hold: (a) the context is NOT DEFINED in the release
   commit's workflow files — the tree cannot promise a gate it cannot
   run; (b) the context's REQUIRED_SINCE is LATER than the line moment —
   the moment of the vX.Y.0 commit the patch patches; the line never
   promised it. Either condition alone is a bypass: (a) alone lets a
   pre-tag job deletion buy waivers; (b) alone waives TODAY's gates
   from a tag cut off main. A context with no REQUIRED_SINCE entry is
   never waived. A minor/rc release (Z=0) has no waiver path at all —
   the full set. An unresolvable vX.Y.0 means the line is undefined —
   no waiver, the full set, the log says so. Every waiver PRINTS its
   name, both conditions and the line moment; the summary line carries
   both numbers — how many were required and how many were waived.

2. **The ref resolution is fail-closed.** The workflow checkout gets
   the full history and the tags; the silent `|| echo github.sha`
   fallback is GONE — an unresolvable ref is a loud refusal naming the
   ref. A gate that verifies a DIFFERENT commit and says OK is worse
   than no gate (false refusals annoy; false permissions ship).

The act the token cannot do (403 Resource not accessible by
integration): the retro-verification RUN of CI on v0.40.0 — that is the
OWNER's act (docs/reality/release-0.40.0-retro-verification.md carries
the instruction and the result place). This gate is the machine half:
the NEXT release cannot ship behind an empty check-run set.

A patch tag does NOT self-verify: the workflow files do not exist on
the patch branch, so the tag push runs no gate. The verdict arrives via
`workflow_dispatch` from main with `ref=<the patch ref>` — the
dispatch-run verdict is recorded in docs/reality/.

Usage:
  release_commit_gate.py --sha <sha> --ref <ref>    # the gate run
  release_commit_gate.py --ref <ref>                # resolves the ref itself (fail-closed)
  release_commit_gate.py --self-test                # the 11 fixture traps
  release_commit_gate.py --mutation <1|2>           # drops one conjunction condition; the trap must spring

Exit codes: 0 green · 1 the gate reds · 2 fail-closed (the state
unreadable — an unreadable state is not a pass, №525).
"""
import argparse
import json
import os
import re
import subprocess
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from merge_ci_audit import REQUIRED_CHECKS, REQUIRED_SINCE, required_at, _parse_iso  # noqa: E402

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"
NON_BLOCKING_HINTS = ("non-blocking",)
# The ref forms the line moment understands: the tag vX.Y.Z and the
# release branch release/X.Y.Z (the patch slice is cut as the branch
# BEFORE the tag exists — the dispatch verdict runs against the branch
# ref first, the tag ref after the owner tags).
VERSION_RE = re.compile(r"(?:^|/)(?:v)?(\d+)\.(\d+)\.(\d+)$")


def _fetch(url, token):
    req = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.load(r)
    except Exception as e:  # noqa: BLE8 — fail-closed on ANY unreadable state
        raise UnreadableState(f"{url}: {e}")


class UnreadableState(Exception):
    pass


def check_runs_for(sha, token):
    out = []
    page = 1
    while True:
        data = _fetch(f"{API}/commits/{sha}/check-runs?per_page=100&page={page}", token)
        runs = data.get("check_runs", [])
        out.extend(runs)
        if len(runs) < 100:
            return out
        page += 1


def _git(*args):
    """A local git read; None on failure (the caller decides the
    fail-closed direction — a git miss never buys a waiver)."""
    try:
        out = subprocess.run(["git", *args], capture_output=True, text=True, timeout=60)
        if out.returncode != 0:
            return None
        return out.stdout.strip()
    except Exception:
        return None


def resolve_ref(ref):
    """Fail-closed ref resolution: the DWIM forms in order — the bare
    ref, the tag namespace, the origin remote-tracking namespace (a
    patch slice is cut as a BRANCH before the owner tags, so the
    pre-tag dispatch verdict resolves `release/X.Y.Z` through
    `origin/release/X.Y.Z`). None when nothing resolves — the caller
    refuses loudly (no silent fallback to another commit, ever)."""
    for cand in (f"{ref}^{{commit}}",
                 f"refs/tags/{ref}^{{commit}}",
                 f"refs/remotes/origin/{ref}^{{commit}}"):
        got = _git("rev-parse", "--verify", cand)
        if got:
            return got
    return None


def line_moment_for(ref):
    """The patch line moment: for vX.Y.Z with Z>0 (or release/X.Y.Z) —
    the COMMIT date of vX.Y.0 (the commit date, not the tag object's:
    a lightweight tag carries no date of its own). Returns
    (moment|None, reason). No version, a minor (Z=0) or an
    unresolvable base — no waiver, the full set, and the reason says
    so in the log."""
    m = VERSION_RE.search(ref or "")
    if not m:
        return None, f"no patch version in the ref '{ref}' — the full set (no waiver path)"
    major, minor, patch = (int(g) for g in m.groups())
    if patch == 0:
        return None, (f"a minor release v{major}.{minor}.{patch} — no waiver path, "
                      "the full set is charged")
    base = f"v{major}.{minor}.0"
    base_sha = resolve_ref(base)
    if not base_sha:
        return None, (f"{base} does not resolve — the line the patch patches is "
                      "undefined, the full set is charged (no waiver)")
    date = _git("show", "-s", "--format=%cI", base_sha)
    if not date:
        return None, f"{base} resolved but its commit date is unreadable — the full set"
    return date, f"the line moment: {base} @ {base_sha[:12]} committed {date}"


def defined_contexts(sha):
    """The required contexts DEFINED in the release commit's workflow
    files (the exact job `name:` strings). An unreadable tree returns
    None — the caller treats everything as defined (an unreadable state
    never buys a waiver; fail-closed)."""
    files = _git("ls-tree", "--name-only", f"{sha}:.github/workflows")
    if files is None:
        return None
    blobs = []
    for f in [x for x in files.splitlines() if x.endswith((".yml", ".yaml"))]:
        # ls-tree names are relative to the listed directory — the
        # show path needs the prefix back.
        blob = _git("show", f"{sha}:.github/workflows/{f}")
        if blob is None:
            return None
        blobs.append(blob)
    text = "\n".join(blobs)
    return {n for n in REQUIRED_CHECKS if n in text}


def waiver_applies(name, since_map, line_moment, defined_in_tree, mutation=None):
    """The conjunction — BOTH conditions, never one. mutation=1 drops
    condition (1) (the defined-in-tree check); mutation=2 drops
    condition (2) (the since check). The mutations exist for the
    self-test contract: each must make its trap spring."""
    since = _parse_iso((since_map or {}).get(name)) if since_map else None
    if since is None:
        return False  # a context without REQUIRED_SINCE is never waived
    cond1 = True if mutation == 1 else (name not in (defined_in_tree or set()))
    cond2 = True if mutation == 2 else (since > line_moment)
    return cond1 and cond2


def gate(release_sha, runs, required, release_instant=None, since_map=None,
         line_moment=None, defined_in_tree=None, mutation=None):
    """The pure decision (the --self-test fixtures feed it synthetic states).

    Returns (exit_code, lines). `runs` is the list of {name, conclusion}
    dicts; `required` the required-context names; `release_instant` the
    release cut instant for the required_at direction (None = the full
    set); `since_map` the per-context since map (None = REQUIRED_SINCE);
    `line_moment` + `defined_in_tree` arm the patch-slice waiver (both
    or neither — the waiver is the conjunction).
    """
    lines: list[str] = []
    smap = REQUIRED_SINCE if since_map is None else since_map
    names = required_at(release_instant, required=required, since_map=smap)
    # The patch-slice waiver — the conjunction, every waiver printed.
    waived = []
    if line_moment is not None and defined_in_tree is not None:
        lm = _parse_iso(line_moment)
        if lm is not None:
            for n in list(names):
                if waiver_applies(n, smap, lm, defined_in_tree, mutation):
                    since = _parse_iso(smap.get(n))
                    waived.append((n, since.isoformat(), line_moment))
                    names.remove(n)
        else:
            lines.append(f"  NO WAIVER: the line moment '{line_moment}' does not parse — "
                         "the full set is charged (an unknown date never buys a pass)")
    have = {}
    for r in runs:
        have.setdefault(r.get("name"), r.get("conclusion") or r.get("status"))
    blocking = [n for n in names if not any(h in n for h in NON_BLOCKING_HINTS)]
    missing = [n for n in blocking if n not in have]
    bad = [n for n in blocking
           if n in have and have[n] not in ("success",)]
    summary = (f"release-commit-gate: {release_sha} — required {len(blocking) + len(waived)}, "
               f"waived {len(waived)}, "
               f"present {len(blocking) - len(missing)}, "
               f"missing {len(missing)}, "
               f"not-green {len(bad)}")
    lines.append(summary)
    for n, since, moment in waived:
        lines.append(f"  WAIVED: {n} — not defined in the release tree's workflows AND "
                     f"required since {since} is later than the line moment {moment} "
                     "(the patch line never promised it; both conditions hold)")
    for n in missing:
        lines.append(f"  MISSING: {n} — the release promises it, no run on the release commit")
    for n in bad:
        lines.append(f"  NOT GREEN: {n} — {have[n]}")
    if missing or bad:
        lines.append("release-commit-gate: REFUSED — a release commit is a promise: "
                     "every required context must be green on it (№976).")
        return 1, lines
    lines.append("release-commit-gate: OK — every required context green on the release commit.")
    return 0, lines


def self_test() -> int:
    required = ["a (blocking)", "b (blocking)", "c (non-blocking)"]
    full = [{"name": "a (blocking)", "conclusion": "success"},
            {"name": "b (blocking)", "conclusion": "success"},
            {"name": "c (non-blocking)", "conclusion": "failure"}]
    missing_one = [{"name": "a (blocking)", "conclusion": "success"}]
    red_one = [{"name": "a (blocking)", "conclusion": "success"},
               {"name": "b (blocking)", "conclusion": "failure"}]
    pending = [{"name": "a (blocking)", "conclusion": "success"},
               {"name": "b (blocking)", "conclusion": "in_progress"}]
    cases = [
        ("full green passes (a non-blocking red does not charge)", gate("S", full, required), 0),
        ("a missing required context reds — THE M1 MUTATION", gate("S", missing_one, required), 1),
        ("a red required context reds — M2", gate("S", red_one, required), 1),
        ("a pending required context reds (fail-closed) — M3", gate("S", pending, required), 1),
        ("a context required AFTER the release instant does not charge (№960)",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "late (blocking)"], release_instant="2026-01-01T00:00:00Z",
              since_map={"late (blocking)": "2027-01-01T00:00:00Z"}), 0),
        ("empty check-runs on the release commit reds — THE AUDIT FACT",
         gate("S", [], required), 1),
        # №976 follow-up 2 — the patch-slice waiver (the conjunction):
        ("trap 7: the patch slice — not defined in the tree AND since later than "
         "the line → WAIVED, exit 0 (the live 0.40.1 shape)",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "new-gate (blocking)"],
              line_moment="2026-01-01T00:00:00Z",
              defined_in_tree={"a (blocking)"},
              since_map={"a (blocking)": "2025-06-01T00:00:00Z",
                         "new-gate (blocking)": "2027-01-01T00:00:00Z"}), 0),
        ("trap 8: the patch slice — DEFINED in the tree but no run → NOT waived, exit 1 "
         "(the job exists and did not report)",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "new-gate (blocking)"],
              line_moment="2026-01-01T00:00:00Z",
              defined_in_tree={"a (blocking)", "new-gate (blocking)"},
              since_map={"new-gate (blocking)": "2027-01-01T00:00:00Z"}), 1),
        ("trap 9: the patch slice — not defined but since EARLIER than the line → NOT "
         "waived, exit 1 (the pre-tag job deletion is not a bypass)",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "old-gate (blocking)"],
              line_moment="2026-06-01T00:00:00Z",
              defined_in_tree={"a (blocking)"},
              since_map={"old-gate (blocking)": "2025-01-01T00:00:00Z"}), 1),
        ("trap 10: a minor release (Z=0) — no waiver path, the full set",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "late2 (blocking)"],
              line_moment=None,
              defined_in_tree={"a (blocking)"},
              since_map={"late2 (blocking)": "2027-01-01T00:00:00Z"}), 1),
        ("trap 11: the line does not resolve — no waiver, the full set, the log says so",
         gate("S", [{"name": "a (blocking)", "conclusion": "success"}],
              ["a (blocking)", "late3 (blocking)"],
              line_moment="not-a-date",
              defined_in_tree={"a (blocking)"},
              since_map={"late3 (blocking)": "2027-01-01T00:00:00Z"}), 1),
    ]
    bad = 0
    for name, (code, lines), want in cases:
        if code != want:
            print(f"SELF-TEST TRAP DID NOT SPRING: {name} — exit {code}, want {want}")
            bad += 1
        else:
            print(f"self-test ok: {name} → exit {want}")
    if bad:
        print(f"release_commit_gate --self-test: {bad} trap(s) failed")
        return 1
    print("release_commit_gate --self-test: 11/11 traps green")
    return 0


def run_mutation(which: int) -> int:
    """Drops one conjunction condition and runs the trap that must
    spring: mutation 1 (condition (1) dropped — the defined-in-tree
    check) must make trap 8 red; mutation 2 (condition (2) dropped —
    the since check) must make trap 9 red. A green trap under the
    mutation means the suite is leaky."""
    target = 8 if which == 1 else 9
    name = (f"trap {target} under mutation {which} "
            f"(condition ({which}) of the conjunction dropped)")
    if which == 1:
        code, _ = gate(
            "S", [{"name": "a (blocking)", "conclusion": "success"}],
            ["a (blocking)", "new-gate (blocking)"],
            line_moment="2026-01-01T00:00:00Z",
            defined_in_tree={"a (blocking)", "new-gate (blocking)"},
            since_map={"new-gate (blocking)": "2027-01-01T00:00:00Z"},
            mutation=1)
    else:
        code, _ = gate(
            "S", [{"name": "a (blocking)", "conclusion": "success"}],
            ["a (blocking)", "old-gate (blocking)"],
            line_moment="2026-06-01T00:00:00Z",
            defined_in_tree={"a (blocking)"},
            since_map={"old-gate (blocking)": "2025-01-01T00:00:00Z"},
            mutation=2)
    if code == 0:
        # The mutation caused a FALSE PASS (the gate said OK about the
        # trap's missing context) — the trap, which demands exit 1,
        # catches the difference. That is the trap springing.
        print(f"MUTATION {which} SPRUNG — {name} caught it (the mutated gate false-passes)")
        return 0
    print(f"MUTATION {which} LEAKED — {name} still reds under the dropped condition")
    return 1


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sha")
    ap.add_argument("--ref")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--mutation", type=int, choices=(1, 2))
    args = ap.parse_args()
    if args.self_test:
        sys.exit(self_test())
    if args.mutation:
        sys.exit(run_mutation(args.mutation))
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GITHUB_PAT")
    if not token:
        print("release-commit-gate: GITHUB_TOKEN is required (or --self-test / --mutation)")
        sys.exit(2)

    # The ref resolution — fail-closed (№976 follow-up 2, part 2): a
    # silent fallback to another commit would make the gate say OK
    # about something it never verified.
    sha = args.sha
    ref = args.ref or args.sha
    if not sha and not ref:
        print("release-commit-gate: --sha and/or --ref are required (or --self-test)")
        sys.exit(2)
    if not sha:
        sha = resolve_ref(ref)
        if not sha:
            print(f"release-commit-gate: REFUSED — the ref '{ref}' does not resolve "
                  "(fail-closed; the silent github.sha fallback would verify a "
                  "DIFFERENT commit)")
            sys.exit(1)
    print(f"release-commit-gate: REF={ref}")
    print(f"release-commit-gate: SHA={sha} — verifying exactly this commit")

    # The patch-slice inputs: the line moment and the release tree's
    # defined contexts. Each miss LOGS and lands in the full set — a
    # miss never buys a waiver.
    moment, reason = line_moment_for(ref)
    print(f"release-commit-gate: {reason}")
    tree = defined_contexts(sha) if moment is not None else None
    if moment is not None and tree is None:
        print("release-commit-gate: the release tree's workflow files are unreadable — "
              "no waivers granted (an unreadable state never buys a pass)")
    try:
        runs = check_runs_for(sha, token)
    except UnreadableState as e:
        print(f"release-commit-gate: FAIL-CLOSED — the state is unreadable: {e}")
        sys.exit(2)
    code, lines = gate(sha, runs, REQUIRED_CHECKS, release_instant=None,
                       line_moment=moment, defined_in_tree=tree)
    for l in lines:
        print(l)
    sys.exit(code)


if __name__ == "__main__":
    main()
