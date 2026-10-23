#!/usr/bin/env python3
"""gen_wave_record.py — the wave-record generator (naryad №789, gh#1496).

The C4-a T1 owner decision (23.10): the wave records in REALITY /
PLAN-SUMMARY are GENERATED from the GitHub API + git — the per-wave
docs naryad ("strictly last") is retired; a WEEKLY run produces ONE PR
covering all the waves of the week (manually for now — a scheduled run
is the owner's separate decision).

What it collects for `--wave N` (the dispatch is found by the
`wave-N` label — the machine-readable wave identity since #1490):
  - the dispatch issue (the composition table → the naryad №s + issues);
  - per naryad: the issue title/state, the SOURCE issue (gh#NNNN in the
    naryad preamble), the merge commit (the git-history scan — the
    house commit-title form `Naryad №NNN ...`), the PR number (the
    `(#NNNN)` suffix), the CI verdict on the merge commit (check-runs);
  - the machine counters — VERBATIM output lines of the scripts/ci
    machines (the honest form: the generator paraphrases nothing);
  - the prose section «Честные находки» — ONLY from the marked lines
    (`HONEST-FINDING:`) in the naryad completion-audit comments; no
    markers = an empty section (the generator invents no prose).

Usage:
  GH_TOKEN=... python3 scripts/gen_wave_record.py --wave 62 --out /tmp/wr
  GH_TOKEN=... python3 scripts/gen_wave_record.py --wave 62 --wave 63

Outputs (stdout + --out DIR files): a REALITY §-record block and a
PLAN-SUMMARY one-line block per wave. The weekly PR pastes them into
docs/REALITY.md (the new sections) and docs/PLAN-SUMMARY.md (the new
row) — the docs gates (gen_metrics --check, docs_language_lint,
readme_consistency) run on the RESULT unchanged.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(HERE)
REPO = "ShkodnikAI/Metalogos-"

NARYAD_TABLE_ROW_RE = re.compile(r"^\|\s*№(\d+)\s*\|\s*gh#(\d+)\s*\|", re.MULTILINE)
SOURCE_REF_RE = re.compile(r"Источник:\s*gh#(\d+)")
MERGE_SUBJECT_RE = re.compile(r"^(Naryad №(\d+)\b.*)$", re.MULTILINE)
PR_SUFFIX_RE = re.compile(r"\(#(\d+)\)\s*$")
HONEST_FINDING_RE = re.compile(r"^\s*(?:[-*]\s*)?HONEST-FINDING:\s*(.+)$", re.MULTILINE)


class ApiError(RuntimeError):
    pass


def api(url: str, token: str | None):
    if not token:
        raise ApiError("GH_TOKEN absent — the wave identity (the dispatch) is unresolvable; the generator refuses to guess (loud, no silent empty record)")
    last: Exception | None = None
    for attempt in range(4):
        try:
            req = urllib.request.Request(
                url,
                headers={
                    "Authorization": f"Bearer {token}",
                    "Accept": "application/vnd.github+json",
                    "User-Agent": "metalogos-gen-wave-record",
                },
            )
            with urllib.request.urlopen(req, data=None) as r:
                return json.loads(r.read().decode())
        except Exception as exc:  # noqa: BLE001 — the retry log is the loud trail
            last = exc
            print(f"  [retry {attempt + 1}] {url} -> {exc}", file=sys.stderr)
            time.sleep(3 * (attempt + 1))
    raise ApiError(f"the GitHub API broke: {last}")


def find_dispatch(wave: int, token: str | None) -> dict:
    """The dispatch = the issue labeled `wave-N` whose title names the dispatch."""
    label = f"wave-{wave}"
    found: list[dict] = []
    page = 1
    while True:
        batch = api(
            f"https://api.github.com/repos/{REPO}/issues?labels={label}&state=all&per_page=50&page={page}",
            token,
        )
        if not batch:
            break
        found.extend(batch)
        page += 1
    dispatches = [i for i in found if "диспатч" in (i["title"] or "").lower()]
    if len(dispatches) != 1:
        raise ApiError(
            f"wave {wave}: expected exactly one dispatch issue labeled `{label}`, got {len(dispatches)} — "
            "the wave identity is ambiguous, the generator refuses to guess"
        )
    return dispatches[0]


def parse_naryads(dispatch_body: str) -> list[tuple[int, int]]:
    rows = NARYAD_TABLE_ROW_RE.findall(dispatch_body or "")
    if not rows:
        raise ApiError("the dispatch body carries no naryad table rows — the composition is unreadable")
    return [(int(no), int(issue)) for no, issue in rows]


def git_log_naryad(naryad_no: int) -> tuple[str, str] | None:
    """(sha, subject) of the merge commit whose title opens with `Naryad №N`."""
    out = subprocess.run(
        ["git", "log", "--format=%H%x00%s%x00", "--grep", f"Naryad №{naryad_no} ", "origin/main"],
        capture_output=True, text=True, cwd=REPO_ROOT,
    )
    if out.returncode != 0:
        raise ApiError(f"git log broke: {out.stderr.strip()[:200]}")
    for block in out.stdout.split("\x00\n"):
        if not block.strip():
            continue
        sha, subject = block.split("\x00", 1)
        # the house form opens the subject with `Naryad №N ` — the grep
        # already narrows it; the exact-prefix check skips №78 matching №7.
        if subject.startswith(f"Naryad №{naryad_no} ") or subject.startswith(f"Naryad №{naryad_no} ("):
            return sha, subject
    return None


def ci_verdict(sha: str, token: str | None) -> str:
    if not token:
        return "CI UNVERIFIED (GH_TOKEN absent — the machine refuses to invent the verdict)"
    runs = api(f"https://api.github.com/repos/{REPO}/commits/{sha}/check-runs?per_page=100", token)
    runs = runs.get("check_runs", [])
    from collections import Counter

    cnt = Counter((r["status"], r["conclusion"]) for r in runs)
    done = sum(n for (st, _), n in cnt.items() if st == "completed")
    ok = sum(n for (st, c), n in cnt.items() if st == "completed" and c == "success")
    skip = sum(n for (st, c), n in cnt.items() if st == "completed" and c == "skipped")
    fail = sum(n for (st, c), n in cnt.items() if st == "completed" and c not in ("success", "skipped"))
    pending = sum(n for (st, _), n in cnt.items() if st != "completed")
    if pending:
        return f"CI INCOMPLETE ({pending} pending of {len(runs)} runs on {sha[:12]})"
    if fail:
        return f"CI RED ({fail} failed / {ok} success / {skip} skipped of {len(runs)} runs on {sha[:12]})"
    return f"CI GREEN ({ok} success / {skip} skipped of {len(runs)} runs on {sha[:12]})"


def run_machine(cmd: list[str]) -> list[str]:
    """The verbatim summary lines of one scripts/ci machine — a failure is
    printed LOUDLY into the record (never silently dropped, never invented)."""
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT)
    lines = [l for l in proc.stdout.splitlines() if l.strip()]
    if proc.returncode != 0:
        return [f"MACHINE UNAVAILABLE: {' '.join(cmd)} exit {proc.returncode} — {proc.stderr.strip()[:120]}"]
    return lines


def nesting_pairs_floor() -> str:
    path = os.path.join(REPO_ROOT, "scripts", "ci", "fuzz_nesting_pairs_baseline.txt")
    try:
        with open(path, encoding="utf-8") as f:
            content = f.read().strip().splitlines()
        tail = content[-1].strip() if content else "(empty baseline)"
        return f"fuzz nesting-pairs floor (the baseline file, only-up): {tail}"
    except OSError as exc:
        return f"MACHINE UNAVAILABLE: the nesting-pairs baseline is unreadable ({exc})"


def collect_machines() -> list[str]:
    lines: list[str] = []
    lines += run_machine(["python3", "scripts/ci/type_signature_share.py"])
    lines += run_machine(["python3", "scripts/ci/sensitive_surface_share.py"])
    lines += run_machine(["python3", "scripts/ci/debt_counters.py"])
    mirrors = run_machine(["python3", "scripts/ci/mirror_counter.py"])
    n = sum(1 for l in mirrors if "mirror of the" in l)
    lines.append(f"mirror records: {n} (the mirror_counter scan; the gate verdict rides the CI job)")
    dups = run_machine(["python3", "scripts/ci/count_duplicated_names.py"])
    lines += dups[-1:]
    ref = run_machine(["python3", "scripts/gen_reference.py", "--check"])
    lines += [l for l in ref if "OK" in l or "check" in l.lower()][-1:]
    lines += run_machine([
        "python3", "scripts/ci/sensitive_surface_share.py", "--goal032",
        "scripts/ci/gate_032_untyped_sources.txt",
    ])[-2:]
    lines.append(nesting_pairs_floor())
    return lines


def honest_findings(naryad_issue: int, token: str | None) -> list[str]:
    comments = api(
        f"https://api.github.com/repos/{REPO}/issues/{naryad_issue}/comments?per_page=50",
        token,
    )
    out: list[str] = []
    for c in comments:
        for m in HONEST_FINDING_RE.finditer(c.get("body") or ""):
            out.append(f"№{naryad_issue}: {m.group(1).strip()}")
    return out


def wave_record(wave: int, token: str | None) -> tuple[str, str]:
    """(the REALITY block, the PLAN-SUMMARY line)."""
    dispatch = find_dispatch(wave, token)
    naryads = parse_naryads(dispatch.get("body") or "")
    rec: list[str] = []
    rec.append(f"### 6.{wave}. Wave {wave} record (GENERATED by scripts/gen_wave_record.py — the C4-a T1 form; the dispatch gh#{dispatch['number']}, published {dispatch['created_at'][:10]}; the record run {time.strftime('%Y-%m-%d')})")
    rec.append("")
    rec.append(f"**Dispatch:** {dispatch['title']}")
    rec.append("")
    merged = 0
    for no, issue_no in naryads:
        it = api(f"https://api.github.com/repos/{REPO}/issues/{issue_no}", token)
        title = (it["title"] or "").strip()
        state = it["state"]
        body = it.get("body") or ""
        src = SOURCE_REF_RE.search(body)
        src_note = f"; source gh#{src.group(1)}" if src else ""
        entry = api(f"https://api.github.com/repos/{REPO}/issues/{issue_no}/timeline?per_page=100", token)
        closed_at = it.get("closed_at")
        merge = git_log_naryad(no)
        if merge:
            sha, subject = merge
            pr_m = PR_SUFFIX_RE.search(subject)
            pr_note = f", PR #{pr_m.group(1)}" if pr_m else ""
            verdict = ci_verdict(sha, token)
            rec.append(
                f"- **№{no}** (gh#{issue_no}{src_note}) — merged: `{sha[:12]}`{pr_note}; {verdict}"
            )
            merged += 1
        else:
            rec.append(
                f"- **№{no}** (gh#{issue_no}{src_note}) — NO MERGE COMMIT on origin/main (state: {state}, closed_at: {closed_at})"
            )
    rec.append("")
    rec.append(f"**Merge coverage:** {merged}/{len(naryads)} naryads have the merge commit on origin/main; the wave closure rule (C4): merged PRs + the green CI on the merge commits.")
    rec.append("")
    rec.append("**Machine counters (the verbatim generator outputs at the record run):**")
    rec.append("")
    rec.append("```")
    rec.extend(collect_machines())
    rec.append("```")
    findings: list[str] = []
    for _no, issue_no in naryads:
        findings += honest_findings(issue_no, token)
    rec.append("")
    rec.append("**Честные находки (from the `HONEST-FINDING:` markers only — the generator writes no prose):**")
    if findings:
        rec.extend(f"- {f}" for f in findings)
    else:
        rec.append("- (no markers — the section stays empty by the C4-a T1 form)")
    reality = "\n".join(rec)

    subj = []
    for no, issue_no in naryads:
        it = api(f"https://api.github.com/repos/{REPO}/issues/{issue_no}", token)
        m = re.search(r"\((?:wave \d+,\s*)?([^)]+)\)", it["title"] or "")
        label = m.group(1) if m else (it["title"] or "")[:60]
        subj.append(f"№{no}")
    plansummary = (
        f"Wave {wave} EXECUTED (the dispatch gh#{dispatch['number']}): "
        + " → ".join(f"№{no}" for no, _ in naryads)
        + f". The machine record: the generated §6.{wave} (the verbatim counters, the merge-CI verdicts)."
    )
    return reality, plansummary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--wave", type=int, action="append", required=True,
                        help="the wave number (repeatable — one weekly PR covers all the waves)")
    parser.add_argument("--out", default="",
                        help="write the blocks to DIR/reality-wave-N.md and DIR/plansummary-wave-N.md")
    args = parser.parse_args()
    token = os.environ.get("GH_TOKEN")

    ok = True
    for wave in args.wave:
        try:
            reality, plansummary = wave_record(wave, token)
        except ApiError as exc:
            print(f"::error::wave {wave}: {exc}")
            ok = False
            continue
        print(f"\n{'=' * 30} REALITY block — wave {wave} {'=' * 30}\n{reality}")
        print(f"\n{'=' * 30} PLAN-SUMMARY line — wave {wave} {'=' * 30}\n{plansummary}")
        if args.out:
            os.makedirs(args.out, exist_ok=True)
            with open(os.path.join(args.out, f"reality-wave-{wave}.md"), "w", encoding="utf-8") as f:
                f.write(reality + "\n")
            with open(os.path.join(args.out, f"plansummary-wave-{wave}.md"), "w", encoding="utf-8") as f:
                f.write(plansummary + "\n")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
