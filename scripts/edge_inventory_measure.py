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
import resource
import subprocess
import time
import urllib.request
from pathlib import Path

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
    kernel = subprocess.run(["uname", "-r"], capture_output=True,
                            text=True).stdout.strip()
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


def build_artifact(rel: dict, sha: str, size_bytes: int, build_info: str,
                   times: list[float], rss_kib: int, runs: int,
                   workload: dict | None = None) -> dict:
    """The E-form artifact: the numbers only from this recorded run."""
    return {
        "artifact": "edge-profile",
        "form": "E (the numbers only from this recorded run)",
        "naryad": "№1004 + №1005 (gh#2067, gh#2071, волна В106 — W2/W2+)",
        "release": {"tag": rel["tag_name"], "created_at": rel["created_at"],
                    "commit": rel["target_commitish"]},
        "asset": {"name": "mlog-linux-x86_64", "size_bytes": size_bytes},
        "sha256_verified": True,
        "sha256": sha,
        "build_info_verbatim": build_info.strip(),
        "measurement": {
            "startup_help_median_s": round(median(times), 3),
            "startup_help_runs_s": [round(t, 3) for t in times],
            "runs": runs,
            "peak_rss_children_max_mib": round(rss_kib / 1024, 1),
            "workload": workload,
        },
        "platform": honest_label(),
        "budgets": ("none — the budgets are the owner's verbatim after the "
                    "measured W2 numbers (ADR-0224 §5 W3; numbers measured only)"),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="v0.40.1")
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

    rel = json.loads(api(API + f"/releases/tags/{a.tag}"))
    print("release:", rel["tag_name"], "| created:", rel["created_at"],
          "| commit:", rel["target_commitish"])
    assets = {x["name"]: x for x in rel["assets"]}
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    bin_path = out / "mlog"
    data = api(assets["mlog-linux-x86_64"]["url"], asset=True)
    bin_path.write_bytes(data)
    size_bytes = len(data)
    sums = api(assets["SHA256SUMS"]["url"], asset=True).decode()
    info = api(assets["BUILD-INFO.txt"]["url"], asset=True).decode()
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
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    med = median(times)
    print(f"startup --help: median {med:.3f}s over {a.runs} runs "
          f"({[f'{t:.3f}' for t in times]})")
    print(f"peak RSS (children max): {rss / 1024:.1f} MiB")

    workload = None
    if workload_pins_ok():
        wtimes = []
        for _ in range(a.runs):
            t0 = time.perf_counter()
            r = subprocess.run([str(bin_path), "run", WORKLOAD_PROGRAM[0]],
                               capture_output=True, text=True)
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

    art = build_artifact(rel, sha, size_bytes, info, times, rss, a.runs, workload)
    if a.json:
        Path(a.json).parent.mkdir(parents=True, exist_ok=True)
        Path(a.json).write_text(json.dumps(art, ensure_ascii=False, indent=2) + "\n")
        print("artifact json:", a.json)
    if a.txt:
        Path(a.txt).parent.mkdir(parents=True, exist_ok=True)
        Path(a.txt).write_text(
            "edge-profile (the E-form; numbers only from this recorded run)\n"
            f"release {rel['tag_name']} ({rel['created_at']}, commit {rel['target_commitish']})\n"
            f"asset mlog-linux-x86_64: {size_bytes} bytes; sha256 verified: {sha}\n"
            f"startup --help median {med:.3f}s over {a.runs} runs; "
            f"peak RSS {rss / 1024:.1f} MiB\n"
            + (f"workload (§3.4, pins verified): median {workload['median_s']:.3f}s "
               f"over {workload['runs']} runs, the effect verified each run\n"
               if workload["status"] == "verified"
               else "workload: PIN MISMATCH — no workload numbers recorded\n")
            + f"platform: {label['honest_label']} | kernel {label['kernel']}\n"
            "budgets: none (ADR-0224 §5 W3 — the owner's verbatim after the W2 numbers)\n")
        print("artifact txt:", a.txt)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
