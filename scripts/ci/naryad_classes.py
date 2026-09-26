#!/usr/bin/env python3
"""№470 (gh#691): the naryad-class counter — the automatic domain quota.

Decision 7-A.3 (gate gh#680): the naryads are divided into classes
(`[domain]` / `[core]` / `[std]` / `[process]` / `[docs]` / `[bugfix]` /
`[security]` — the tag lives in the meta line or the class comment) and
the domain quota is enforced AUTOMATICALLY — "who watches the quota?" is
answered by this counter, run by the dispatcher at wave composition; the
output goes into the dispatch text.

The quota rule (fixed in the issue body and in ADR-0177):
  - while the ADR-0177 domain freeze is active (--freeze): the domain
    quota is ZERO — a wave composition with ANY domain-classed naryad
    fails (the red verdict);
  - after the freeze is lifted: the domain share must be <= 1/3 (the
    standing limiter, the "peaceful continuation" of decision 1).

The class source per naryad (in priority order):
  1. the issue BODY: `Класс: [X]` or `[class: X]` (the №470 template
     dropdown fills the meta line);
  2. the issue COMMENTS: the latest `[class: X]` marker (the
     retrospective form of the №470 rollout).

Usage:
  naryad_classes.py DISPATCH_ISSUE [--freeze]   # the dispatch body's gh# refs
  naryad_classes.py --issues 682 683 ... [--freeze]
"""
import json
import os
import re
import sys
import urllib.request

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"
CLASSES = ("domain", "core", "std", "process", "docs", "bugfix", "security")
CLASS_RE = re.compile(r"\[class:\s*(" + "|".join(CLASSES) + r")\]")
META_RE = re.compile(r"Класс:\s*\[(" + "|".join(CLASSES) + r")\]")
GH_REF_RE = re.compile(r"gh#(\d+)")


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
    for c in api(f"/issues/{num}/comments?per_page=100"):
        m = CLASS_RE.search(c.get("body") or "")
        if m:
            return m.group(1), "comment"
    return None, "unclassified"


def main():
    args = sys.argv[1:]
    freeze = "--freeze" in args
    if "--issues" in args:
        i = args.index("--issues")
        j = args.index("--freeze") if freeze else len(args)
        nums = [int(x) for x in args[i + 1:j]]
    else:
        disp = [a for a in args if a.isdigit()]
        if not disp:
            print(__doc__)
            sys.exit(2)
        body = api(f"/issues/{disp[0]}").get("body") or ""
        nums = sorted({int(x) for x in GH_REF_RE.findall(body)})
    counts = {}
    rows = []
    for n in nums:
        cls, src = class_of(n)
        rows.append((n, cls, src))
        counts[cls] = counts.get(cls, 0) + 1
    total = len(rows)
    dom = counts.get("domain", 0)
    share = dom / total if total else 0.0
    print(f"naryad-class composition over {total} naryads (№470 counter):")
    for n, cls, src in rows:
        print(f"  #{n}: [{cls or 'unclassified'}] ({src})")
    print()
    print("classes: " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items(), key=lambda kv: -kv[1]) if k))
    print(f"domain share: {dom}/{total} = {share:.1%}")
    if freeze:
        print("freeze: ACTIVE (ADR-0177) — the domain quota is ZERO (new domain naryads are forbidden)")
        verdict = dom == 0
        print(f"verdict: {'PASS (0 domain naryads)' if verdict else 'FAIL (the freeze forbids domain naryads)'}")
    else:
        verdict = share <= (1 / 3)
        print("freeze: not active — the standing limiter applies (domain share <= 1/3)")
        print(f"verdict: {'PASS' if verdict else 'FAIL (share > 1/3)'}")
    sys.exit(0 if verdict else 1)


if __name__ == "__main__":
    main()
