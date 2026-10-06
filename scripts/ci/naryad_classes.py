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

Usage:
  naryad_classes.py DISPATCH_ISSUE [--freeze]   # the dispatch body's gh# refs
  naryad_classes.py --issues 682 683 ... [--freeze]
  naryad_classes.py DISPATCH_ISSUE --line       # the one-line wave-report form
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


def main():
    args = sys.argv[1:]
    freeze = "--freeze" in args
    line_mode = "--line" in args
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


if __name__ == "__main__":
    main()
