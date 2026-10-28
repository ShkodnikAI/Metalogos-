#!/usr/bin/env python3
"""№763 (gh#1416, Wave 58): the agent-index consistency gate (blocking).

The finding (the office research contour, candidate R-01, 16.10): llms.txt
is covered by NO generator gate and drifts. The r29 kant-facts: llms.txt
carried "409 builtins" (the live registry — 527), "214 .mlog" (the
generator — 253), "132 ADR" (the generator — 187), "наряды №1-№289+" (the
live sequence — №762+); CLAUDE.md (the declared "bridge copy of
AGENTS.md") was NOT byte-identical to AGENTS.md (8958 vs 10703 bytes).
Stale agent indexes are false facts served to IDE agents and RAG
pipelines on their first screen — the same honesty family as the "stale
draft is a lie" rule (№713).

The gate (this script):
  1. llms.txt builtins count == gen_metrics.count_total_builtins()
     (the SSOT registry scan — the SAME machine the README block uses);
  2. llms.txt examples count == gen_metrics.count_examples();
  3. llms.txt ADR count == gen_metrics.count_adrs();
  4. llms.txt naryad range end == the highest naryad number in the merged
     git history (the №505 grammar scan — the machine of the naryad
     sequence; the checkout needs fetch-depth: 0);
  5. CLAUDE.md bytes == AGENTS.md bytes (the bridge-copy contract — the
     content source is AGENTS.md, the №676 A5 form);
  6. every llms.txt markdown link target exists in the tree (the file
     rows stay honest — the missing canonical file is a drift).

Exit 0 = in sync; exit 1 = the drift named. --self-test runs the tamper
traps (a stale counter, a broken bridge copy, a dangling link) against a
temp copy and verifies each trips the check.
"""
from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "scripts"))
import gen_metrics  # noqa: E402 — the machine counters (the №566 form)

NARYAD_RE = re.compile(r"(?iu)\b(?:naryad|наряд)\s*№?\s*(\d+)", re.UNICODE)
LINK_RE = re.compile(r"\]\(([^)]+)\)")


def git_history_naryads(repo: Path) -> list[int]:
    out = subprocess.run(
        ["git", "log", "--format=%s"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return [int(m.group(1)) for m in NARYAD_RE.finditer(out)]


def extract(llms: str, pattern: str) -> int | None:
    m = re.search(pattern, llms)
    return int(m.group(1)) if m else None


def is_shallow(repo: Path) -> bool:
    """The PR checkouts (actions/checkout default) are depth-1 AND point at
    the MERGE commit (whose subject is not a naryad) — the git-history
    machine cannot run there. The FULL check lives in the agent-index CI
    job (fetch-depth: 0); a shallow checkout SKIPS the range check loudly."""
    r = subprocess.run(
        ["git", "rev-parse", "--is-shallow-repository"],
        cwd=repo, capture_output=True, text=True,
    )
    return r.returncode == 0 and r.stdout.strip() == "true"


def check_repo(repo: Path) -> list[str]:
    """Every drift, named. Empty list = in sync."""
    problems: list[str] = []
    llms_path = repo / "llms.txt"
    if not llms_path.exists():
        return ["llms.txt is missing"]

    llms = llms_path.read_text(encoding="utf-8")

    checks = [
        (
            r"\((\d+) builtins",
            gen_metrics.count_total_builtins(),
            "builtins count",
        ),
        (
            r":\s(\d+) ADR",
            gen_metrics.count_adrs(),
            "ADR count",
        ),
        (
            r":\s(\d+) working `.mlog`",
            gen_metrics.count_examples(),
            "examples count",
        ),
    ]
    for pattern, machine, label in checks:
        stated = extract(llms, pattern)
        if stated is None:
            problems.append(f"llms.txt: the {label} statement is gone (the gate cannot find /{pattern}/)")
        elif stated != machine:
            problems.append(
                f"llms.txt: the {label} drifted — stated {stated}, the generator says {machine}"
            )

    # The naryad range: №1-№N where N is the highest merged naryad.
    # №798: llms.txt is ENGLISH (the B6 owner directive 2026-09-17 now
    # machine-enforced for this file too) — the stated form is
    # "naryads №1-№N"; the Russian-spelled statement is gone and its
    # absence trips the gate loudly (a regression back to Russian is
    # caught twice: here and by the docs-language lint).
    stated_range = re.search(r"naryads №1-№(\d+)\+?", llms)
    if stated_range is None:
        problems.append("llms.txt: the naryad range statement (naryads №1-№N) is gone")
    elif is_shallow(repo):
        print(
            "agent-index-gate: SKIP (loud) — the naryad-range check needs the full "
            "history; this checkout is shallow (the PR merge ref) — the full check "
            "runs in the agent-index (blocking) CI job (fetch-depth: 0)",
        )
    else:
        hist = git_history_naryads(repo)
        if not hist:
            problems.append(
                "naryad sequence: the git history has no naryad-numbered subjects "
                "(shallow clone? the agent-index CI job needs fetch-depth: 0)"
            )
        else:
            highest = max(hist)
            stated_end = int(stated_range.group(1))
            if stated_end != highest:
                problems.append(
                    f"llms.txt: the naryad range drifted — stated №1-№{stated_end}, "
                    f"the machine says №1-№{highest} (the merged-history scan)"
                )

    # The bridge copy: byte-for-byte.
    agents = repo / "AGENTS.md"
    claude = repo / "CLAUDE.md"
    if not agents.exists():
        problems.append("AGENTS.md is missing (the content source of the bridge)")
    elif not claude.exists():
        problems.append("CLAUDE.md is missing (the declared bridge copy)")
    elif agents.read_bytes() != claude.read_bytes():
        problems.append(
            f"CLAUDE.md is not byte-identical to AGENTS.md "
            f"({claude.stat().st_size} vs {agents.stat().st_size} bytes) — "
            f"copy AGENTS.md over CLAUDE.md (the №676 A5 form)"
        )

    # The link targets exist in the tree.
    for target in sorted(set(LINK_RE.findall(llms))):
        if target.endswith("/"):
            if not (repo / target.rstrip("/")).is_dir():
                problems.append(f"llms.txt: the link target directory is gone: {target}")
        elif not (repo / target).exists():
            problems.append(f"llms.txt: the link target is gone: {target}")

    return problems


def self_test() -> int:
    """The tamper traps — each must trip the check on a temp copy."""
    traps: list[tuple[str, callable]] = [
        (
            "a stale builtins counter",
            lambda text: text.replace("(527 builtins", "(999 builtins"),
        ),
        (
            "a broken bridge copy",
            lambda text: text,  # the bridge tamper is a FILE tamper, below
        ),
        (
            "a dangling link target",
            lambda text: text.replace("docs/privacy.md", "docs/gone-privacy.md"),
        ),
    ]
    failed = False
    with tempfile.TemporaryDirectory() as td:
        repo_copy = Path(td) / "repo"
        shutil.copytree(REPO, repo_copy,
                        ignore=shutil.ignore_patterns("target", ".git"),
                        dirs_exist_ok=False)
        # The git history: the copy has no .git — plant a minimal one whose
        # highest naryad MATCHES llms.txt's stated range end (the self-test
        # proves the CHECKS, not a number coincidence).
        # №798: the stated form is English ("naryads №1-№N") — the same
        # pattern the check above reads; the self-test extracts the range
        # end with the SAME regex (a divergence here would let the
        # self-test's planted git history disagree with the check).
        stated_end = re.search(r"naryads №1-№(\d+)\+?",
                               (repo_copy / "llms.txt").read_text(encoding="utf-8")).group(1)
        subprocess.run(["git", "init", "-q"], cwd=repo_copy, check=True)
        subprocess.run(["git", "add", "-A"], cwd=repo_copy, check=True)
        subprocess.run(
            ["git", "-c", "user.email=g@l", "-c", "user.name=g",
             "commit", "-qm", f"Naryad №{stated_end} (issue #0, wave 0): the self-test subject"],
            cwd=repo_copy, check=True,
        )

        base = check_repo(repo_copy)
        if base:
            print(f"self-test: the CLEAN copy must pass; got: {base}")
            return 1

        # Trap 1: the stale counter.
        llms = (repo_copy / "llms.txt").read_text(encoding="utf-8")
        (repo_copy / "llms.txt").write_text(
            llms.replace("(527 builtins", "(999 builtins"),
            encoding="utf-8",
        )
        if not check_repo(repo_copy):
            print("self-test: the stale-counter trap did NOT trip")
            failed = True
        (repo_copy / "llms.txt").write_text(llms, encoding="utf-8")

        # Trap 2: the broken bridge copy.
        (repo_copy / "CLAUDE.md").write_text("drifted\n", encoding="utf-8")
        if not check_repo(repo_copy):
            print("self-test: the bridge-drift trap did NOT trip")
            failed = True
        shutil.copy(repo_copy / "AGENTS.md", repo_copy / "CLAUDE.md")

        # Trap 3: the dangling link.
        (repo_copy / "llms.txt").write_text(
            llms.replace("docs/privacy.md", "docs/gone-privacy.md"), encoding="utf-8"
        )
        if not check_repo(repo_copy):
            print("self-test: the dangling-link trap did NOT trip")
            failed = True

    if failed:
        return 1
    print("agent-index-gate self-test: all tamper traps spring")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    problems = check_repo(REPO)
    if problems:
        print("agent-index-gate: STALE — the agent indexes lie:")
        for p in problems:
            print(f"  - {p}")
        print("  fix: refresh llms.txt from the generators (gen_metrics counters, "
              "the git-history naryad scan) and copy AGENTS.md over CLAUDE.md")
        return 1
    print("agent-index-gate: OK — llms.txt counters == the generators, "
          "CLAUDE.md == AGENTS.md (bytes), the link targets exist")
    return 0


if __name__ == "__main__":
    sys.exit(main())
