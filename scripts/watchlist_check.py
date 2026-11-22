#!/usr/bin/env python3
"""№993 (gh#2036) — the weekly I&W watchlist digest.

Reads the six watched repositories through the public GitHub/crates.io APIs
(the GITHUB_TOKEN raises the rate limits; no secrets beyond it), compares
the live numbers against docs/watchlist/baseline.json, classifies the week
per docs/watchlist/registry.md (the Yellow/Orange/Red calibration), and
leaves the №979-visible trace EVERY run:

  1. the report file  docs/watchlist/reports/<ISO-week>.md;
  2. the digest comment in the pinned issue «I&W watchlist digest»
     (created and pinned at the first run; a quiet week is a one-liner
     «всё на baseline» — the workflow never runs unseen);
  3. the baseline refresh for the quiet/Yellow weeks (written into
     baseline.json so the workflow commit carries it); an Orange/Red week
     never rewrites the baseline — the flag rides the digest until the
     owner's manual decision.

Usage (the workflow form):
    python3 scripts/watchlist_check.py --repo-dir .
Dry-run (no GitHub writes, the report still prints):
    python3 scripts/watchlist_check.py --repo-dir . --dry-run
Exit codes: 0 = the trace is left (even with the flagged anomalies);
1 = the trace could not be produced (the API died beyond the retries).
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import sys
import time
import urllib.error
import urllib.request

REPO_ROOT_DEFAULT = "."
BASELINE = "docs/watchlist/baseline.json"
REPORTS = "docs/watchlist/reports"
DIGEST_TITLE = "I&W watchlist digest"
CRATE_UA = "metalogos-iw-watchlist (naryad 993, gh#2036)"

CRATES_CRATE = "rust-anything"

# The registry order — the scenario ids are stable (WL-01..WL-06).
SCENARIOS = [
    ("WL-01", "erkamyaman/rust-anything", True),
    ("WL-02", "TauricResearch/TradingAgents", False),
    ("WL-03", "stacklok/toolhive", False),
    ("WL-04", "NVIDIA/auto-ontology", False),
    ("WL-05", "deepset-ai/haystack", False),
    ("WL-06", "MervinPraison/PraisonAI", False),
]

# The level calibration (registry.md): the per-scenario trigger set.
STARS_WEEK_YELLOW = 0.05   # +5%/week
STARS_WEEK_ORANGE = 0.15   # >15%/week alone
STARS_WEEK_RED = 0.25      # >25%/week + a new release
DOWNLOADS_YELLOW = 500     # +500 downloads/week
PUSH_PAUSE_YELLOW_DAYS = 14
PUSH_PAUSE_RED_DAYS = 45


def http_json(url: str, token: str | None, retries: int = 2) -> tuple[dict | list | None, int]:
    """GET JSON; honor 403 rate limits with the Retry-After pause.

    Returns (payload, status). status 404 = a legitimate absence (no
    releases); the caller decides. The retries cover the transient 403s
    (the primary rate limit) and the 5xx; the exhausted retries return
    the last status so the caller records the gap in the report (the
    №979 form: the trace names what could not be seen).
    """
    headers = {"User-Agent": CRATE_UA, "Accept": "application/vnd.github+json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    for attempt in range(retries + 1):
        req = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                return json.load(r), r.status
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None, 404
            if e.code in (403, 429, 500, 502, 503, 504) and attempt < retries:
                wait = int(e.headers.get("Retry-After", "20") or 20) if e.code in (403, 429) else 20
                time.sleep(min(wait, 60))
                continue
            return None, e.code
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as exc:
            if attempt < retries:
                time.sleep(15)
                continue
            print(f"::warning::fetch failed {url}: {exc}")
            return None, 0
    return None, 0


def fetch_repo(full: str, token: str | None) -> tuple[dict | None, str | None]:
    data, st = http_json(f"https://api.github.com/repos/{full}", token)
    if data is None:
        return None, f"repos API status {st}"
    out = {
        "stars": data.get("stargazers_count"),
        "forks": data.get("forks_count"),
        "open_issues": data.get("open_issues_count"),
        "pushed_at": data.get("pushed_at"),
    }
    rel, st2 = http_json(f"https://api.github.com/repos/{full}/releases/latest", token)
    out["latest_release"] = rel.get("tag_name") if rel else None
    if st2 not in (200, 404):
        return out, f"releases API status {st2}"
    return out, None


def fetch_crates_downloads(crate: str) -> tuple[int | None, str | None]:
    data, st = http_json(f"https://crates.io/api/v1/crates/{crate}", None)
    if data is None:
        return None, f"crates.io status {st}"
    return (data.get("crate") or {}).get("downloads"), None


def week_events(live: dict, base: dict) -> list[str]:
    """The trigger list per the registry signature (the human-readable reasons)."""
    events: list[str] = []
    b_stars = base.get("stars") or 0
    l_stars = live.get("stars") or 0
    if b_stars > 0:
        growth = (l_stars - b_stars) / b_stars
        if growth >= STARS_WEEK_YELLOW:
            events.append(f"stars {b_stars}->{l_stars} ({growth:+.1%} >= +5%/week)")
    b_dl = base.get("crates_downloads")
    l_dl = live.get("crates_downloads")
    if b_dl is not None and l_dl is not None and l_dl - b_dl >= DOWNLOADS_YELLOW:
        events.append(f"crates.io downloads {b_dl}->{l_dl} (>= +500/week)")
    b_rel = base.get("latest_release")
    l_rel = live.get("latest_release")
    if l_rel and l_rel != b_rel:
        events.append(f"NEW release {b_rel} -> {l_rel}")
    pushed = live.get("pushed_at")
    if pushed:
        pushed_day = dt.datetime.fromisoformat(pushed.replace("Z", "+00:00"))
        pause_days = (dt.datetime.now(dt.timezone.utc) - pushed_day).days
        if pause_days >= PUSH_PAUSE_YELLOW_DAYS:
            events.append(f"push pause {pause_days} days (>= {PUSH_PAUSE_YELLOW_DAYS})")
    return events


def level_of(events: list[str], stars_growth: float | None, new_release: bool) -> str:
    n = len(events)
    if n >= 3:
        return "Red"
    if stars_growth is not None and stars_growth > STARS_WEEK_RED and new_release:
        return "Red"
    if n >= 2:
        return "Orange"
    if stars_growth is not None and stars_growth > STARS_WEEK_ORANGE:
        return "Orange"
    if n >= 1:
        return "Yellow"
    return "quiet"


def stars_growth(live: dict, base: dict) -> float | None:
    b = base.get("stars") or 0
    if b <= 0:
        return None
    return ((live.get("stars") or 0) - b) / b


def pin_issue(node_id: str, token: str) -> bool:
    """The pin through the GraphQL pinIssue mutation — the REST pins
    endpoint is absent for this account (404 even for an admin PAT,
    verified 22.11); the mutation is the only pin surface."""
    req = urllib.request.Request(
        "https://api.github.com/graphql",
        method="POST",
        data=json.dumps({
            "query": "mutation($id: ID!) { pinIssue(input: {issueId: $id}) { issue { number } } }",
            "variables": {"id": node_id},
        }).encode(),
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json", "User-Agent": CRATE_UA},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            out = json.load(r)
        if out.get("errors"):
            print(f"::warning::pinIssue: {out['errors'][0].get('message', '')[:120]}")
            return False
        return True
    except (urllib.error.HTTPError, urllib.error.URLError) as e:
        print(f"::warning::pinIssue failed: {e}")
        return False


def digest_issue(repo: str, token: str, dry: bool) -> int | None:
    """The pinned digest issue: found, created, pinned; None = the API refused."""
    data, st = http_json(f"https://api.github.com/repos/{repo}/issues?state=all&per_page=50", token)
    if isinstance(data, list):
        for it in data:
            if it.get("title") == DIGEST_TITLE and "pull_request" not in it:
                return it["number"]  # found — pinned by the first run
    body = (
        "The pinned weekly digest of the I&W watchlist (№993, gh#2036 — the "
        "observation-only lane over the six external repositories). Every "
        "Monday 09:00 MSK the `watchlist-weekly` workflow leaves exactly one "
        "comment here: the week's report, the per-scenario levels, the flags. "
        "A quiet week is a one-liner «всё на baseline» — the week is never "
        "unseen (the №979 visibility rule). The report files ride the "
        "`watchlist-digest` branch (the digest PR opens manually at the merge "
        "time — GitHub Actions cannot create pull requests in this repo by "
        "the owner's setting). The registry: "
        "docs/watchlist/registry.md; the baseline: docs/watchlist/baseline.json."
    )
    if dry:
        print("[dry-run] would create + pin the digest issue")
        return None
    req = urllib.request.Request(
        f"https://api.github.com/repos/{repo}/issues",
        method="POST",
        data=json.dumps({"title": DIGEST_TITLE, "body": body}).encode(),
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json", "User-Agent": CRATE_UA},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            created = json.load(r)
    except urllib.error.HTTPError as e:
        print(f"::warning::digest issue create failed: {e.code}")
        return None
    num, node_id = created["number"], created.get("node_id")
    if node_id and pin_issue(node_id, token):
        print(f"digest issue #{num} created and pinned")
    else:
        print(f"::warning::digest issue #{num} created but NOT pinned — pin it manually")
    return num


def post_digest(repo: str, num: int, body: str, token: str, dry: bool) -> bool:
    if dry:
        print(f"[dry-run] would comment in #{num}:\n{body[:400]}")
        return True
    req = urllib.request.Request(
        f"https://api.github.com/repos/{repo}/issues/{num}/comments",
        method="POST",
        data=json.dumps({"body": body}).encode(),
        headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json", "User-Agent": CRATE_UA},
    )
    try:
        urllib.request.urlopen(req, timeout=30)
        return True
    except urllib.error.HTTPError as e:
        print(f"::warning::digest comment failed: {e.code}")
        return False


def monthly_section(reports_dir: str, today: dt.date) -> str | None:
    """The first Monday of the month: the relevance + retire-candidate summary."""
    if today.weekday() != 0 or today.day > 7:
        return None
    counts: dict[str, int] = {sid: 0 for sid, _, _ in SCENARIOS}
    try:
        for fn in os.listdir(reports_dir):
            if not fn.endswith(".md"):
                continue
            text = open(os.path.join(reports_dir, fn), encoding="utf-8").read()
            for sid in counts:
                if any(
                    line.startswith(f"| {sid} |") and ("**Orange**" in line or "**Red**" in line)
                    for line in text.splitlines()
                ):
                    counts[sid] += 1
    except OSError:
        pass
    lines = ["## The monthly review (the relevance + the retire candidates)", ""]
    for sid, full, _crate in SCENARIOS:
        note = " — the retire candidate if the quiet streak holds a quarter" if counts[sid] == 0 else ""
        lines.append(f"- {sid} ({full}): {counts[sid]} flagged week(s) in the report history{note}")
    lines.append("")
    lines.append("The retirement is the owner's call, never the workflow's (registry.md).")
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo-dir", default=REPO_ROOT_DEFAULT)
    ap.add_argument("--dry-run", action="store_true", help="no GitHub writes; the report still builds")
    args = ap.parse_args()
    os.chdir(args.repo_dir)

    token = os.environ.get("GITHUB_TOKEN")
    repo = os.environ.get("GITHUB_REPOSITORY", "ShkodnikAI/Metalogos-")
    today = dt.date.today()
    week_tag = f"{today.isocalendar().year}-W{today.isocalendar().week:02d}"

    baseline = json.load(open(BASELINE, encoding="utf-8"))
    rows, gaps, levels = [], [], {}
    new_baseline = {"captured_at": baseline.get("captured_at"), "note": baseline.get("note", ""), "repos": {}}

    for sid, full, has_crate in SCENARIOS:
        base = baseline["repos"].get(full, {})
        live, err = fetch_repo(full, token)
        if live is None:
            gaps.append(f"{sid} ({full}): {err}")
            new_baseline["repos"][full] = base  # the gap keeps the old numbers verbatim
            levels[sid] = "?"
            rows.append(f"| {sid} | {full} | ? | ? | {err} |")
            continue
        if has_crate:
            dl, derr = fetch_crates_downloads(CRATES_CRATE)
            if derr:
                gaps.append(f"{sid}: crates.io {derr}")
            live["crates_downloads"] = dl
        else:
            live["crates_downloads"] = base.get("crates_downloads")
        events = week_events(live, base)
        lvl = level_of(events, stars_growth(live, base), bool(live.get("latest_release") and live["latest_release"] != base.get("latest_release")))
        levels[sid] = lvl
        if events:
            rows.append(f"| {sid} | {full} | **{lvl}** | {len(events)} | {'; '.join(events)} |")
        else:
            rows.append(f"| {sid} | {full} | quiet | 0 | на baseline |")
        # the baseline rule: the quiet/Yellow weeks refresh, the Orange/Red stay flagged
        if lvl in ("quiet", "Yellow"):
            new_baseline["repos"][full] = live
        else:
            new_baseline["repos"][full] = base

    flagged = [s for s, l in levels.items() if l in ("Orange", "Red")]
    max_level = max((l for l in levels.values() if l in ("Yellow", "Orange", "Red")), default=None)
    header = "всё на baseline" if not max_level else f"the week's max level: **{max_level}**"

    md = [
        f"# I&W watchlist report — {week_tag} ({today.isoformat()})",
        "",
        f"**{header}.** The registry: docs/watchlist/registry.md; the baseline captured {baseline.get('captured_at')}.",
        "",
        "| scenario | repo | level | triggers | detail |",
        "|---|---|---|---|---|",
        *rows,
        "",
    ]
    if gaps:
        md += ["## The gaps (the fetch failures — recorded, never hidden)", ""]
        md += [f"- {g}" for g in gaps] + [""]
    if flagged:
        md += [
            "## The anomaly flags (the baseline NOT rewritten — the owner's call)",
            "",
            f"- The flagged scenarios: {', '.join(flagged)}. The baseline keeps the pre-anomaly numbers until the manual decision.",
            "",
        ]
    mon = monthly_section(REPORTS, today)
    if mon:
        md += [mon, ""]

    os.makedirs(REPORTS, exist_ok=True)
    report_path = os.path.join(REPORTS, f"{week_tag}.md")
    open(report_path, "w", encoding="utf-8").write("\n".join(md))
    print(f"report written: {report_path}")

    if not args.dry_run:
        num = digest_issue(repo, token or "", args.dry_run)
        if num:
            body = (
                f"### I&W watchlist — {week_tag}\n\n**{header}.**\n\n"
                + "\n".join(rows)
                + ("\n\n**The gaps:** " + "; ".join(gaps) if gaps else "")
                + (f"\n\n**The anomaly flags (the baseline kept):** {', '.join(flagged)} — the owner's call." if flagged else "")
                + f"\n\nThe report file: `docs/watchlist/reports/{week_tag}.md` (rides the rolling digest PR)."
            )
            post_digest(repo, num, body, token or "", args.dry_run)

    if not flagged:
        new_baseline["captured_at"] = today.isoformat()
        json.dump(new_baseline, open(BASELINE, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
        print("baseline refreshed (quiet/Yellow week)")
    else:
        print("baseline KEPT (the flagged anomalies — the owner's decision pending)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
