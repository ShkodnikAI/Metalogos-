#!/usr/bin/env python3
"""№470 (gh#691): the naryad-class counter — the automatic domain quota.

№608 (gh#1049, the audit 25b375e Y-4): the counter's output becomes a
MANDATORY line of the wave summary report (beside the pool-size counter,
docs/maintainers.md) — the quota must be externally checkable; the
`--line` mode emits that machine-ready report line. The classifier also
learns the REAL class formats: the №608 recon found the №470 matchers
blind to the modern waves (they saw ZERO classified naryads across В29/
В30/В31) — the bodies say «класс [core]» (no colon, lowercase), the
composites are «core/media», and the class also lives in the issue
title's "(P0, core/math)" field.

Decision 7-A.3 (gate gh#680): the naryads are divided into classes and
the domain quota is enforced AUTOMATICALLY — "who watches the quota?" is
answered by this counter, run by the dispatcher at wave composition; the
output goes into the dispatch text AND into the wave summary report.

The quota rule (fixed in the issue body and in ADR-0177):
  - while the ADR-0177 domain freeze is active (--freeze): the domain
    quota is ZERO — a wave composition with ANY domain-classed naryad
    fails (the red verdict);
  - after the freeze is lifted: the domain share must be <= 1/3 (the
    standing limiter, the "peaceful continuation" of decision 1).

The domain set (№608): a naryad counts toward the quota when ANY component
of its class string (the composites split on "/") is in DOMAIN_CLASSES —
the language-surface classes. The interpretation follows the В30 audit
slice verbatim: №590/№591/№595 (core) and №599 (core/media) were counted
as the four предметных of В30. The non-surface classes (process, docs,
testing, ci, release, adr, compiler, security...) never count.

The class source per naryad (in priority order):
  1. the issue BODY: «класс [X]» / «Класс: [X]» (the №470 template
     meta line — case-insensitive, the composites allowed);
  2. the issue BODY: `[class: X]`;
  3. the issue TITLE: «Наряд №N (P0, X): …» (the title field);
  4. the issue COMMENTS: the latest `[class: X]` marker (the
     retrospective form of the №470 rollout).

№687 (gh#1254, the audit f92c61c S-2): the counter also counts the
commits OUTSIDE the naryads — the PE-1/PE-2 lesson (14 commits, more
than half of the window, new surfaces: HTML/XML parsing, Telegram,
outbound HTTP, headers, raw insert — ALL outside the machine's quota,
checklist, and honest boundaries; S-1 was the direct consequence). A
commit without a recognized naryad number that grows the SURFACE —
`src/builtins/registry.rs` in its diff, OR a NEW feature/dependency key
in `Cargo.toml` — is a red-line finding the counter cannot "not notice".
Docs-only and test-only diffs outside naryads do not grow surfaces and
stay legal (the boundary). The report is THREE lines: the naryad
quota PASS/FAIL, the off-naryad findings (sha + files), the quota
verdict. The dispatch `--line` format is unchanged (the backward
compatibility of reading).

Usage:
  naryad_classes.py DISPATCH_ISSUE [--freeze]   # the dispatch body's gh# refs
  naryad_classes.py --issues 682 683 ... [--freeze]
  naryad_classes.py DISPATCH_ISSUE --line       # the one-line wave-report form
  naryad_classes.py --commits BASE HEAD         # the off-naryad surface scan (№687)
  naryad_classes.py --self-test                 # the fixture self-test (green)
"""
import json
import os
import re
import sys
import urllib.request

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"
CLASSES = ("domain", "core", "std", "process", "docs", "bugfix", "security")
DOMAIN_CLASSES = ("domain", "core", "std", "media")
CLASS_RE = re.compile(r"\[class:\s*([a-z/]+)\]", re.I)
META_RE = re.compile(r"класс\s*\[([a-z/]+)\]", re.I)
TITLE_RE = re.compile(r"[Нн]аряд №\d+\s*\(\s*P\d[^,)]*,\s*([^):]+?)\s*\)", re.I)
GH_REF_RE = re.compile(r"gh#(\d+)")

# №687: the subject pattern of a NARYAD commit (the squash convention:
# "Naryad №685 (issue #1252, wave 44): …" — LATIN "Naryad", the docs-
# language spelling, AND the Cyrillic "Наряд" the older windows used).
# A commit whose subject does not match is an OFF-NARYAD candidate (the
# pe:/pe1:/pe2: prefixes and the bare "Naryad N…" without a number are
# the live S-2 examples).
NARYAD_SUBJECT_RE = re.compile(r"(?:[Nn]aryad|[Нн]аряд)\s*№\s*(\d+)")

# №687: the SURFACE markers — a commit that touches these grows the
# product surface (the registry rows / the feature & dependency graph).
REGISTRY_PATH = "src/builtins/registry.rs"
CARGO_PATH = "Cargo.toml"


def api(path):
    token = os.environ.get("GH_TOKEN", "")
    if not token:
        # The office fallback (the dispatcher's local run); never a
        # checked-in secret — the file is machine-local.
        p = os.path.expanduser("~/.gh_token")
        if os.path.exists(p):
            token = open(p).read().strip()
    req = urllib.request.Request(API + path, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
    })
    with urllib.request.urlopen(req) as r:
        return json.load(r)


def class_of(num):
    issue = api(f"/issues/{num}")
    body = issue.get("body") or ""
    m = META_RE.search(body) or CLASS_RE.search(body)
    if m:
        return m.group(1), "meta"
    m = TITLE_RE.search(issue.get("title") or "")
    if m:
        return m.group(1), "title"
    for c in api(f"/issues/{num}/comments?per_page=100"):
        m = CLASS_RE.search(c.get("body") or "")
        if m:
            return m.group(1), "comment"
    return None, "unclassified"


def is_domain(cls):
    if not cls:
        return False
    return any(c in DOMAIN_CLASSES for c in cls.split("/"))


# ── №687: the off-naryad surface scan ────────────────────────────────

def is_docs_or_test_only(files):
    """True when EVERY changed file is a docs/test artifact — such diffs
    do not grow the product surface (the №687 boundary: they stay legal
    outside naryads)."""
    if not files:
        return True
    for f in files:
        if f == CARGO_PATH or f.endswith(".lock"):
            return False
        if f.endswith(".md") or "/".join(f.split("/")[:1]) in ("docs", "tests"):
            continue
        if f.startswith("examples/") or f.endswith(".expected"):
            continue  # examples/goldens are content, not surface
        return False
    return True


def off_naryad_finding(subject, files, new_cargo_keys):
    """The №687 finding for one commit, or None.

    A commit with a recognized naryad number in the subject is never a
    finding. An off-naryad commit is a finding when it (a) touches the
    builtin registry, or (b) ADDS a Cargo.toml feature/dependency key
    (a key present in the added lines and absent from the removed ones
    — a version bump modifies, it does not add). Docs-only/test-only
    diffs stay legal (the boundary).
    """
    if NARYAD_SUBJECT_RE.search(subject):
        return None
    if is_docs_or_test_only(files):
        return None
    reasons = []
    if REGISTRY_PATH in files:
        reasons.append(f"{REGISTRY_PATH} (the registry growth)")
    if new_cargo_keys:
        reasons.append(f"{CARGO_PATH} (new keys: {', '.join(sorted(new_cargo_keys))})")
    if not reasons:
        return None
    return reasons


def self_test():
    """The №687 fixture self-test — the pure classification core over
    synthetic commits (no git, no network): the counter must RED on an
    off-naryad registry diff, RED on a new Cargo key, GREEN on naryad
    commits, docs-only diffs, and dep version bumps; the quota verdict
    combines both halves.
    """
    # 1. the S-2 live shape: "pe2: n7" touching the registry → RED.
    f = off_naryad_finding("pe2: n7", ["src/builtins/registry.rs"], set())
    assert f and REGISTRY_PATH in f[0], "the off-naryad registry diff must be a finding"
    # 2. a new Cargo dependency outside a naryad → RED.
    f = off_naryad_finding("pe2: n7", ["Cargo.toml", "Cargo.lock"], {"some_new_dep"})
    assert f and any("some_new_dep" in r for r in f), "a new Cargo key must be a finding"
    # 3. a dependency version BUMP (the key in added AND removed) → GREEN.
    assert off_naryad_finding("pe2: n7", ["Cargo.toml"], set()) is None, \
        "a version bump must not color"
    # 4. a naryad commit touching the registry → GREEN (recognized).
    assert off_naryad_finding("Naryad №685 (issue #1252, wave 44): the S-1 fix",
                              ["src/builtins/registry.rs"], set()) is None
    # 5. docs-only / test-only off-naryad diffs → GREEN (the boundary).
    assert off_naryad_finding("fix docs typo", ["README.md", "docs/adr/0193-x.md"], set()) is None
    assert off_naryad_finding("fix docs typo", ["tests/naryad_687_off_naryad_quota.rs"], set()) is None
    # 6. the green window: the naryad-only commit list → no findings,
    #    verdict PASS (the dispatch reading survives).
    window = [("a1", "Naryad №686 (issue #1253, wave 44): x", ["scripts/ci/risk_review.py"], set()),
              ("a2", "pe2: n7", ["README.md"], set())]
    findings = [(sha, subj, off_naryad_finding(subj, files, keys))
                for sha, subj, files, keys in window]
    findings = [(s, j, f) for s, j, f in findings if f]
    assert not findings, "the green window must carry zero findings"
    print("naryad_classes --self-test: 6/6 fixture groups green (registry red, cargo red, bump green, naryad green, docs/test green, window green)")


def main():
    args = sys.argv[1:]
    freeze = "--freeze" in args
    line_mode = "--line" in args
    if "--self-test" in args:
        self_test()
        sys.exit(0)
    if "--commits" in args:
        # №687: the off-naryad surface scan over a commit window.
        i = args.index("--commits")
        base, head = args[i + 1], args[i + 2]
        scan_commits_window(base, head)
        return
    if "--issues" in args:
        i = args.index("--issues")
        j = next((k for k in range(i + 1, len(args)) if args[k].startswith("--")), len(args))
        nums = [int(x) for x in args[i + 1:j]]
    else:
        disp = [a for a in args if a.isdigit()]
        if not disp:
            print(__doc__)
            sys.exit(2)
        body = api(f"/issues/{disp[0]}").get("body") or ""
        nums = sorted({int(x) for x in GH_REF_RE.findall(body)})
    rows = []
    for n in nums:
        cls, src = class_of(n)
        rows.append((n, cls, src))
    total = len(rows)
    dom = sum(1 for _, cls, _ in rows if is_domain(cls))
    share = dom / total if total else 0.0
    verdict = (dom == 0) if freeze else (share <= (1 / 3))
    if line_mode:
        kind = "freeze ACTIVE (the quota is ZERO)" if freeze else "the standing limiter <= 1/3"
        print(f"naryad-quota (№470/№608): {total} naryads — domain {dom}/{total} "
              f"= {share:.1%} — {'PASS' if verdict else 'FAIL'} ({kind})")
        sys.exit(0 if verdict else 1)
    counts = {}
    for _, cls, _ in rows:
        key = cls or "unclassified"
        counts[key] = counts.get(key, 0) + 1
    print(f"naryad-class composition over {total} naryads (№470 counter):")
    for n, cls, src in rows:
        mark = " *domain" if is_domain(cls) else ""
        print(f"  #{n}: [{cls or 'unclassified'}] ({src}){mark}")
    print()
    print("classes: " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items(), key=lambda kv: -kv[1])))
    print(f"domain share: {dom}/{total} = {share:.1%} "
          f"(the domain set: {'/'.join(DOMAIN_CLASSES)}; the composites count via any component — №608)")
    if freeze:
        print("freeze: ACTIVE (ADR-0177) — the domain quota is ZERO (new domain naryads are forbidden)")
        print(f"verdict: {'PASS (0 domain naryads)' if verdict else 'FAIL (the freeze forbids domain naryads)'}")
    else:
        print("freeze: not active — the standing limiter applies (domain share <= 1/3)")
        print(f"verdict: {'PASS' if verdict else 'FAIL (share > 1/3)'}")
    sys.exit(0 if verdict else 1)


# ── №687: the git side of the commit scan (separated from the pure
# core so the self-test stays offline) ────────────────────────────────

def _run_git(cmd):
    import subprocess
    return subprocess.run(["git"] + cmd, capture_output=True, text=True).stdout


def _cargo_key_delta(sha):
    """(new_keys, gone_keys) of a commit's Cargo.toml — a key present in
    the added lines and absent from the removed ones is NEW (a version
    bump modifies an existing key and lands in both sets)."""
    diff = _run_git(["diff-tree", "--no-commit-id", "-U0", sha, "--", CARGO_PATH])
    added, removed = set(), set()
    for line in diff.splitlines():
        m = re.match(r"^\+([A-Za-z0-9_.-]+)\s*=", line)
        if m:
            added.add(m.group(1))
        m = re.match(r"^-([A-Za-z0-9_.-]+)\s*=", line)
        if m:
            removed.add(m.group(1))
    return added - removed, removed - added


def _collect_commits(base, head):
    """[(sha, subject, files, new_cargo_keys)] over BASE..HEAD — merge
    commits skipped (the squash-only discipline keeps the window
    linear; a merge is never a surface commit here)."""
    out = _run_git(["log", "--format=%H%x1f%s", f"{base}..{head}"])
    commits = []
    for line in out.splitlines():
        if "\x1f" not in line:
            continue
        sha, subject = line.split("\x1f", 1)
        parents = _run_git(["rev-list", "--parents", "-n", "1", sha]).split()
        if len(parents) > 2:
            continue
        files = [f for f in _run_git(
            ["diff-tree", "--no-commit-id", "--name-only", "-r", sha]
        ).splitlines() if f]
        new_keys, _gone = _cargo_key_delta(sha)
        commits.append((sha, subject, files, new_keys))
    return commits


def scan_commits_window(base, head):
    """The №687 THREE-LINE report over a commit window: the naryad quota
    line (the dispatch format, backward compatible), the off-naryad
    findings (sha + files), the combined quota verdict. Exit 1 on FAIL.
    """
    commits = _collect_commits(base, head)
    findings = []
    for sha, subject, files, new_keys in commits:
        f = off_naryad_finding(subject, files, new_keys)
        if f:
            findings.append((sha, subject, f))
    # Line 1 — the naryad-side quota (the same reading the dispatches use).
    naryad_count = sum(1 for _, s, _, _ in commits if NARYAD_SUBJECT_RE.search(s))
    print(f"naryad-quota (№470/№608): {naryad_count} naryad commits of "
          f"{len(commits)} in the window — the naryad half "
          f"{'PASS' if naryad_count else 'WARN (no recognized naryad commits)'}")
    # Line 2 — the off-naryad findings (sha + files).
    if findings:
        print(f"off-naryad surface commits (№687): {len(findings)}")
        for sha, subject, reasons in findings:
            print(f"  {sha[:8]} {subject[:60]} → {'; '.join(reasons)}")
    else:
        print("off-naryad surface commits (№687): none")
    # Line 3 — the combined verdict: the quota fails when ANY surface
    # grew outside the naryads (the counter cannot 'not notice').
    verdict = not findings
    print(f"quota verdict: {'PASS' if verdict else 'FAIL (the surface grew outside the naryads — №687)'}")
    sys.exit(0 if verdict else 1)


if __name__ == "__main__":
    main()
