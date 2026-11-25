#!/usr/bin/env python3
"""The edge-inventory measurement tool (№1001, В105 — the companion of
docs/research/edge-inventory-2026-11-24.md and ADR-0224 §1; the W2
runner machine — №1004, gh#2067, волна В106).

Downloads the named release's mlog-linux-x86_64 asset, verifies the
SHA256 against the release SHA256SUMS, prints the build provenance from
BUILD-INFO.txt and measures the startup (median of N spawns of --help)
and the peak RSS (getrusage children max). Numbers only from the recorded
run; the platform is labeled honestly: a CI-runner run when the GitHub
Actions environment is present, otherwise this machine (not a CI runner).
No product code, no budgets — the observation tool of the
watchlist-class lane.

Usage:
  python3 scripts/edge_inventory_measure.py [--tag v0.40.1] [--runs 5]
          [--json PATH] [--txt PATH]
"""
import argparse
import json
import os
import platform
import subprocess
import time
import urllib.request
from pathlib import Path

try:
    import resource
except ImportError:  # Windows: no getrusage — the RSS is honestly not measured
    resource = None

API = "https://api.github.com/repos/ShkodnikAI/Metalogos-"

# The §3.4 workload pin (№1005, В106 — W2+): a pinned non-trivial
# evaluation that exercises the VM (not the CLI surface). The pins are
# enforced: the files must match byte-for-byte or the workload numbers
# are not recorded (status «pin-mismatch» — loud, not silent).
WORKLOAD_PROGRAM = ("examples/dag_demo.mlog",
                    "ae89af54b04bd456c105629ee7d1deba85340bf84a706dff10a4dded1b3da599")
WORKLOAD_EXPECTED = ("examples/dag_demo.expected",
                     "9337ffb9f49f51e23eaf48f4c5fe641463322cac204128c0d63f6e55d9cc2624")


def sha256_of(path: str) -> str:
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def workload_pins_ok() -> bool:
    return (sha256_of(WORKLOAD_PROGRAM[0]) == WORKLOAD_PROGRAM[1]
            and sha256_of(WORKLOAD_EXPECTED[0]) == WORKLOAD_EXPECTED[1])


def workload_matches(stdout: str) -> bool:
    """The observable effect: the program output equals the expected,
    normalized for the trailing newline only."""
    return stdout.strip() == open(WORKLOAD_EXPECTED[0]).read().strip()


def honest_label() -> dict:
    """The platform label: a CI-runner run only when the Actions env says so."""
    run_id = os.environ.get("GITHUB_RUN_ID")
    try:
        kernel = subprocess.run(["uname", "-r"], capture_output=True,
                                text=True).stdout.strip()
    except (FileNotFoundError, OSError):  # Windows: no uname
        kernel = f"{platform.system()} {platform.release()} {platform.machine()}".strip()
    if not kernel:
        kernel = f"{platform.system()} {platform.release()} {platform.machine()}".strip()
    if run_id:
        return {
            "honest_label": "CI-runner run",
            "kernel": kernel,
            "runner_name": os.environ.get("RUNNER_NAME"),
            "image_os": os.environ.get("ImageOS"),
            "run_url": os.environ.get("GITHUB_SERVER_URL", "https://github.com")
                       + "/" + os.environ.get("GITHUB_REPOSITORY", "")
                       + "/actions/runs/" + run_id,
        }
    return {
        "honest_label": "not a CI runner (this machine)",
        "kernel": kernel,
        "runner_name": None,
        "image_os": None,
        "run_url": None,
    }


def median(values: list[float]) -> float:
    """The median of the recorded run times (the №998 bench convention)."""
    s = sorted(values)
    return s[len(s) // 2]


# №1008 (gh#2080, ADR-0224 §5 W3 RESOLVED 25.11): the budgets are live —
# the owner's verbatim delegated the numbers (the full quote is carried in
# every E-artifact via OWNER_VERBATIM_W3 below) and the gate values are
# agent-proposed per ADR-0224 §5 W3, each 2-3x the worst measured W2/W4
# value (workload 0.004-0.016 s — re-baselined №1009 after the first
# windows workload run: 16 ms; CLI-floor 0.002-0.011 s; RSS 19.5-75.7
# MiB; size 31 899 648-37 318 352 B) — they catch ~2x regressions, not
# runner noise. The size gate is deliberately tight (~1.2x): the binary
# size is the earliest bloat signal. A gate FAIL exits 2 (the lanes are
# non-blocking — visibility, no release block).
OWNER_VERBATIM_W3 = ("я доверяю твоим рекомендациям, при том что все "
                     "решения должны идти только на улучшения")
BUDGETS = {
    "workload_median_ms": 30.0,  # №1009: was 10 (linux-only data); the windows run measured 16 ms
    "cli_floor_median_ms": 25.0,
    "peak_rss_mib": 150.0,
    "size_bytes": 45_000_000,
}


def budget_verdicts(size_bytes, med_s, rss_mib, workload):
    """The W3 gate verdicts: a list of (gate, ok); ok: True/False/None
    (None = no number measured — RSS without getrusage, a workload
    pin-mismatch; N/A is the honest form of numbers-measured-only)."""
    v = []
    ok = size_bytes <= BUDGETS["size_bytes"]
    v.append((f"size_bytes <= {BUDGETS['size_bytes']}: "
              f"{'PASS' if ok else 'FAIL'} ({size_bytes} measured)", ok))
    if med_s is not None:
        ms = med_s * 1000.0
        ok = ms <= BUDGETS["cli_floor_median_ms"]
        v.append((f"cli_floor_ms <= {BUDGETS['cli_floor_median_ms']:g}: "
                  f"{'PASS' if ok else 'FAIL'} ({ms:.1f} measured)", ok))
    else:
        v.append(("cli_floor_ms: N/A (no measurement)", None))
    if rss_mib is not None:
        ok = rss_mib <= BUDGETS["peak_rss_mib"]
        v.append((f"peak_rss_mib <= {BUDGETS['peak_rss_mib']:g}: "
                  f"{'PASS' if ok else 'FAIL'} ({rss_mib:.1f} measured)", ok))
    else:
        v.append(("peak_rss_mib: N/A (no getrusage on this OS)", None))
    if workload and workload.get("status") == "verified":
        wms = workload["median_s"] * 1000.0
        ok = wms <= BUDGETS["workload_median_ms"]
        v.append((f"workload_ms <= {BUDGETS['workload_median_ms']:g}: "
                  f"{'PASS' if ok else 'FAIL'} ({wms:.1f} measured)", ok))
    else:
        v.append(("workload_ms: N/A (no workload number recorded)", None))
    return v


def build_artifact(source: dict, asset_name: str, sha: str, sha256_source: str,
                   size_bytes: int, build_info: str, times: list[float],
                   rss_kib: int | None, runs: int,
                   workload: dict | None = None) -> dict:
    """The E-form artifact: the numbers only from this recorded run.
    source: {"kind": "release", "tag", "created_at", "commit"} or
    {"kind": "local-build"} (the W4 lane — the sha is computed from the
    built file, no SHA256SUMS check — named, not silent)."""
    return {
        "artifact": "edge-profile",
        "form": "E (the numbers only from this recorded run)",
        "naryad": "№1004-№1006 (gh#2067, gh#2071, gh#2074, волна В106 — W2/W2+/W4)",
        "source": source,
        "asset": {"name": asset_name, "size_bytes": size_bytes},
        "sha256": sha,
        "sha256_source": sha256_source,
        "build_info_verbatim": build_info.strip(),
        "measurement": {
            "startup_help_median_s": round(median(times), 3),
            "startup_help_runs_s": [round(t, 3) for t in times],
            "runs": runs,
            "peak_rss_children_max_mib": (round(rss_kib / 1024, 1)
                                          if rss_kib is not None else None),
            "workload": workload,
        },
        "platform": honest_label(),
        "budgets": ("ADR-0224 §5 W3 RESOLVED 25.11 (№1008, gh#2080) — the "
                    "owner's verbatim: «" + OWNER_VERBATIM_W3 + "» (the "
                    "delegation; the gate values agent-proposed, 2-3x the "
                    "worst measured W2/W4 value; numbers measured only): "
                    + "; ".join(g for g, _ in budget_verdicts(
                        size_bytes, median(times),
                        None if rss_kib is None else rss_kib / 1024.0,
                        workload))),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="v0.40.1")
    ap.add_argument("--binary", default=None,
                    help="measure a locally built binary instead of a release "
                         "asset (the W4 lane; the sha256 is computed, no "
                         "SHA256SUMS check — named in the artifact)")
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--out", default="/tmp/edge-inventory")
    ap.add_argument("--json", default=None, help="the E-form artifact path")
    ap.add_argument("--txt", default=None, help="the human-readable artifact path")
    a = ap.parse_args()
    token = os.environ.get("GH_TOKEN", "")
    ua = {"User-Agent": "edge-inventory-measure"}
    hdr_json = {"Accept": "application/vnd.github+json", **ua}
    hdr_asset = {"Accept": "application/octet-stream", **ua}
    if token:
        hdr_json["Authorization"] = "Bearer " + token
        hdr_asset["Authorization"] = "Bearer " + token

    def api(url, asset=False):
        req = urllib.request.Request(url, headers=hdr_asset if asset else hdr_json)
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.read()

    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    if a.binary:
        # The W4 lane (№1006): a locally built binary — no download, the
        # sha256 computed from the built file (no SHA256SUMS check — named).
        bin_path = Path(a.binary).resolve()
        assert bin_path.exists(), f"the binary is missing: {bin_path}"
        size_bytes = bin_path.stat().st_size
        sha = sha256_of(str(bin_path))
        info = "(a local build — no BUILD-INFO)"
        source = {"kind": "local-build"}
        asset_name = bin_path.name
        sha256_source = "computed-from-the-built-file (a local build; no SHA256SUMS check)"
        print("local build:", asset_name, "| size:", size_bytes)
        print("sha256 computed:", sha, "(no SHA256SUMS check — a local build)")
    else:
        rel = json.loads(api(API + f"/releases/tags/{a.tag}"))
        print("release:", rel["tag_name"], "| created:", rel["created_at"],
              "| commit:", rel["target_commitish"])
        assets = {x["name"]: x for x in rel["assets"]}
        bin_path = out / "mlog"
        data = api(assets["mlog-linux-x86_64"]["url"], asset=True)
        bin_path.write_bytes(data)
        size_bytes = len(data)
        sums = api(assets["SHA256SUMS"]["url"], asset=True).decode()
        info = api(assets["BUILD-INFO.txt"]["url"], asset=True).decode()
        sha = subprocess.run(["sha256sum", str(bin_path)],
                             capture_output=True, text=True).stdout.split()[0]
        assert sha in sums, f"SHA256 mismatch: {sha}"
        source = {"kind": "release", "tag": rel["tag_name"],
                  "created_at": rel["created_at"],
                  "commit": rel["target_commitish"]}
        asset_name = "mlog-linux-x86_64"
        sha256_source = "verified-against-the-release-SHA256SUMS"
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
    rss = (resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
           if resource is not None else None)
    med = median(times)
    print(f"startup --help: median {med:.3f}s over {a.runs} runs "
          f"({[f'{t:.3f}' for t in times]})")
    print(f"peak RSS (children max): "
          f"{f'{rss / 1024:.1f} MiB' if rss is not None else 'not measured on this OS (no getrusage)'}")

    workload = None
    if workload_pins_ok():
        wtimes = []
        for _ in range(a.runs):
            t0 = time.perf_counter()
            r = subprocess.run([str(bin_path), "run", WORKLOAD_PROGRAM[0]],
                               capture_output=True, text=True,
                               encoding="utf-8", errors="replace")
            wtimes.append(time.perf_counter() - t0)
            assert r.returncode == 0, "the workload run failed"
            assert workload_matches(r.stdout), "the workload effect mismatch"
        wmed = median(wtimes)
        print(f"workload (§3.4, {WORKLOAD_PROGRAM[0]}, pins verified): "
              f"median {wmed:.3f}s over {a.runs} runs "
              f"({[f'{t:.3f}' for t in wtimes]}), the effect verified each run")
        workload = {"program": WORKLOAD_PROGRAM[0],
                    "program_sha256": WORKLOAD_PROGRAM[1],
                    "expected_sha256": WORKLOAD_EXPECTED[1],
                    "status": "verified",
                    "median_s": round(wmed, 3),
                    "runs_s": [round(t, 3) for t in wtimes],
                    "runs": a.runs}
    else:
        print("workload: PIN MISMATCH — the pinned program/expected files "
              "differ from the §3.4 pins; NO workload numbers recorded "
              "(loud, not silent)")
        workload = {"program": WORKLOAD_PROGRAM[0],
                    "program_sha256": WORKLOAD_PROGRAM[1],
                    "expected_sha256": WORKLOAD_EXPECTED[1],
                    "status": "pin-mismatch"}

    label = honest_label()
    print("measured on:", label["kernel"], "|", label["honest_label"])
    if label["honest_label"] != "CI-runner run":
        print("honest: these are THIS machine's numbers, not the CI runner's;")
    print("no other platform has a run (see ADR-0224 §3).")

    art = build_artifact(source, asset_name, sha, sha256_source, size_bytes,
                         info, times, rss, a.runs, workload)
    verdicts = budget_verdicts(size_bytes, med,
                               None if rss is None else rss / 1024.0,
                               workload)
    if a.json:
        Path(a.json).parent.mkdir(parents=True, exist_ok=True)
        Path(a.json).write_text(json.dumps(art, ensure_ascii=False, indent=2) + "\n",
                         encoding="utf-8")  # windows: the default cp1252 cannot encode «№»
        print("artifact json:", a.json)
    if a.txt:
        Path(a.txt).parent.mkdir(parents=True, exist_ok=True)
        Path(a.txt).write_text(
            "edge-profile (the E-form; numbers only from this recorded run)\n"
            + ("release " + rel["tag_name"] + " (" + rel["created_at"]
               + ", commit " + rel["target_commitish"] + ")\n"
               if source["kind"] == "release"
               else "source: local build (no release, no BUILD-INFO)\n")
            + f"asset {asset_name}: {size_bytes} bytes; "
              f"sha256 {('verified: ' if source['kind'] == 'release' else 'computed: ')}{sha}\n"
            f"startup --help median {med:.3f}s over {a.runs} runs; "
            f"peak RSS {f'{rss / 1024:.1f} MiB' if rss is not None else 'not measured on this OS (no getrusage)'}\n"
            + (f"workload (§3.4, pins verified): median {workload['median_s']:.3f}s "
               f"over {workload['runs']} runs, the effect verified each run\n"
               if workload["status"] == "verified"
               else "workload: PIN MISMATCH — no workload numbers recorded\n")
            + f"platform: {label['honest_label']} | kernel {label['kernel']}\n"
            + "budgets (ADR-0224 §5 W3 RESOLVED 25.11, №1008 — the owner's "
              "verbatim delegation; the values 2-3x the worst measured; "
              "numbers measured only):\n"
            + "".join("  " + g + "\n" for g, _ in verdicts),
            encoding="utf-8")  # windows: the default cp1252 cannot encode «№»
        print("artifact txt:", a.txt)
    failed = [g for g, ok in verdicts if ok is False]
    if failed:
        print("BUDGET FAIL (ADR-0224 §5 W3, №1008):", "; ".join(failed))
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
