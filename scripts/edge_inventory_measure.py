#!/usr/bin/env python3
"""The edge-inventory measurement tool (№1001, В105 — the companion of
docs/research/edge-inventory-2026-11-24.md and ADR-0224 §1).

Downloads the named release's mlog-linux-x86_64 asset, verifies the
SHA256 against the release SHA256SUMS, prints the build provenance from
BUILD-INFO.txt and measures the startup (median of N spawns of --help)
and the peak RSS (getrusage children max). Numbers only from the recorded
run; the platform is printed honestly (the measuring machine, not the CI
runner). No product code, no budgets — the observation tool of the
watchlist-class lane.

Usage: python3 scripts/edge_inventory_measure.py [--tag v0.40.1] [--runs 5]
"""
import argparse
import json
import os
import re
import resource
import subprocess
import time
import urllib.request
from pathlib import Path

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="v0.40.1")
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--out", default="/tmp/edge-inventory")
    a = ap.parse_args()
    token = os.environ.get("GH_TOKEN", "")
    hdr = {"Accept": "application/octet-stream"}
    if token:
        hdr["Authorization"] = "Bearer " + token

    def api(url):
        req = urllib.request.Request(url, headers=hdr)
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.read()

    rel = json.loads(api(API + f"/releases/tags/{a.tag}"))
    print("release:", rel["tag_name"], "| created:", rel["created_at"],
          "| commit:", rel["target_commitish"])
    assets = {x["name"]: x for x in rel["assets"]}
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    bin_path = out / "mlog"
    bin_path.write_bytes(api(assets["mlog-linux-x86_64"]["url"]))
    sums = api(assets["SHA256SUMS"]["url"]).decode()
    info = api(assets["BUILD-INFO.txt"]["url"]).decode()
    sha = subprocess.run(["sha256sum", str(bin_path)],
                         capture_output=True, text=True).stdout.split()[0]
    assert sha in sums, f"SHA256 mismatch: {sha}"
    print("sha256 verified:", sha)
    print("--- BUILD-INFO (verbatim) ---")
    print(info.strip())
    print("--- measurement ---")
    bin_path.chmod(0o755)
    times = []
    for _ in range(a.runs):
        t0 = time.perf_counter()
        r = subprocess.run([str(bin_path), "--help"], capture_output=True)
        times.append(time.perf_counter() - t0)
        assert r.returncode == 0, "the binary failed to run"
    times.sort()
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    med = times[len(times) // 2]
    print(f"startup --help: median {med:.3f}s over {a.runs} runs "
          f"({[f'{t:.3f}' for t in times]})")
    print(f"peak RSS (children max): {rss / 1024:.1f} MiB")
    print("measured on:", subprocess.run(["uname", "-r"],
          capture_output=True, text=True).stdout.strip())
    print("honest: these are THIS machine's numbers, not the CI runner's;")
    print("no other platform has a run (see ADR-0224 §3).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
