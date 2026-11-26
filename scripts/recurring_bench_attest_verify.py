#!/usr/bin/env python3
"""The recurring bench of the attest verify surface (№1010, gh#2085 —
волна В107; ADR-0224 §5 W3 recurring-bench follow-up).

Runs the w9 bench scenario (`mlog attest verify --json`, the median over
N spawns — docs/research/w9-line-bench-2026-11-23.md, the 6.7 ms record
of 23.11) as a re-run lane: per run it builds the release binary (the
workflow does), trains the pinned deterministic demo in a fresh temp dir
(`mlog run line_demo.mlog` — the direct train on labeled rows, no LLM),
then spawns `mlog attest verify line_demo.mlog --json` N times, timing
each spawn with `perf_counter`. The observable effect is verified on
EVERY spawn (exit 0 and `"ok": true` in the JSON — otherwise no numbers
are published, loud exit 2, the §3.4 tripwire posture). The median is
compared against the recorded baseline 6.7 ms (never re-baselined by
this lane — a baseline rewrite only via a new run artifact + an
ADR-0224 §3 record) and against the agent-proposed budget 20 ms (the
2-3× rule ≈ 3× the worst measured single record; the delegation
verbatim rides the artifact). Verdict PASS/FAIL; a FAIL exits 2 (the
lane is non-blocking — visibility without a release block).

Numbers only from the recorded run; the platform is labeled honestly (a
CI-runner run when the Actions env is present, otherwise this machine).
No product code, no W3 gate changes (№1008 stays as recorded).

Usage:
  python3 scripts/recurring_bench_attest_verify.py --bin PATH [--runs 30]
          [--json PATH] [--txt PATH]
"""
import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

# The delegation verbatim (№1008, gh#2080, 25.11 — the E-form rule: the
# full quote rides every E-artifact).
OWNER_VERBATIM_W3 = ("я доверяю твоим рекомендациям, при том что все "
                     "решения должны идти только на улучшения")

# The §-form pins (byte-for-byte, the §3.4 posture): the bench scenario
# files. A pin mismatch → no numbers recorded (loud exit 2).
PROGRAM_PIN = ("examples/distill/line_demo.mlog",
               "1d3aa4763b5b3790df8987374480d1060338ca07fd062d0de8e71d0b9e4c663e")
HELD_OUT_PIN = ("examples/distill/held_out.tsv",
                "c8478bd0b4222a7ac9e232edd2c0b848aa093e26e29de85847d19bc8656484dd")

BASELINE_MS = 6.7  # the w9 bench, 23.11 — the single recorded session
BUDGET_MS = 20.0   # agent-proposed, the 2-3× rule ≈ 3× the worst measured


def sha256_of(path: str) -> str:
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def pins_ok() -> bool:
    return (sha256_of(PROGRAM_PIN[0]) == PROGRAM_PIN[1]
            and sha256_of(HELD_OUT_PIN[0]) == HELD_OUT_PIN[1])


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
    runner_env = os.environ.get("RUNNER_ENVIRONMENT") or os.environ.get("ImageOS")
    if run_id:
        return {
            "honest_label": "CI-runner run",
            "kernel": kernel,
            "runner": runner_env or "GitHub Actions",
            "run_url": run_url(),
        }
    return {
        "honest_label": "this machine (not a CI runner)",
        "kernel": kernel,
        "runner": "local",
        "run_url": None,
    }


def run_url() -> str | None:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return None


def p95(values: list[float]) -> float:
    """The w9 form: the nearest-rank p95 over the recorded samples."""
    if not values:
        return 0.0
    ordered = sorted(values)
    import math
    idx = max(0, math.ceil(0.95 * len(ordered)) - 1)
    return ordered[idx]


def verify_spawn_ok(returncode: int, stdout: str) -> bool:
    """The observable effect of one verify spawn: exit 0 and ok:true."""
    if returncode != 0:
        return False
    for line in reversed(stdout.strip().splitlines()):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            return False
        return obj.get("ok") is True
    return False


def verdict_of(median_ms: float, budget_ms: float) -> str:
    return "PASS" if median_ms <= budget_ms else "FAIL"


def run_bench(binary: str, runs: int) -> tuple[dict | None, str]:
    """The scenario. Returns (artifact_or_None, human_summary)."""
    if not pins_ok():
        return None, ("PIN MISMATCH — no numbers recorded "
                      f"(expected {PROGRAM_PIN[0]} {PROGRAM_PIN[1][:12]}…, "
                      f"{HELD_OUT_PIN[0]} {HELD_OUT_PIN[1][:12]}…)")
    import hashlib
    with open(binary, "rb") as f:
        bin_sha = hashlib.sha256(f.read()).hexdigest()

    with tempfile.TemporaryDirectory(prefix="recurring_bench.") as td:
        for src in (PROGRAM_PIN[0], HELD_OUT_PIN[0]):
            with open(src, "rb") as f, open(Path(td) / Path(src).name, "wb") as g:
                g.write(f.read())  # the byte-identical copy (the pins held)
        # 1. the deterministic train (the certificate lands in the store)
        train = subprocess.run([binary, "run", "line_demo.mlog"], cwd=td,
                               capture_output=True, text=True, timeout=60)
        if train.returncode != 0 or "ok=true" not in (train.stdout + train.stderr):
            return None, (f"TRAIN SETUP FAILED (rc={train.returncode}) — "
                          f"no numbers recorded: {(train.stderr or train.stdout)[:200]}")
        # 2. N timed verify spawns, the effect checked on every spawn
        times_ms: list[float] = []
        for _ in range(runs):
            t0 = time.perf_counter()
            p = subprocess.run(
                [binary, "attest", "verify", "line_demo.mlog", "--json"],
                cwd=td, capture_output=True, text=True, timeout=30)
            t1 = time.perf_counter()
            if not verify_spawn_ok(p.returncode, p.stdout):
                return None, ("EFFECT MISMATCH on a verify spawn — no numbers "
                              f"recorded (rc={p.returncode}): {p.stdout[:160]}")
            times_ms.append(round((t1 - t0) * 1000.0, 2))

    median_ms = round(statistics.median(times_ms), 2)
    min_ms = round(min(times_ms), 2)
    max_ms = round(max(times_ms), 2)
    p95_ms = round(p95(times_ms), 2)
    verdict = verdict_of(median_ms, BUDGET_MS)
    art = {
        "artifact": "recurring-bench-attest-verify",
        "form": "E (the numbers only from this recorded run)",
        "naryad": "№1010 (gh#2085, волна В107 — the ADR-0224 §5 W3 "
                  "recurring-bench follow-up)",
        "scenario": ("mlog run line_demo.mlog (the deterministic train, a "
                     "fresh temp dir) → mlog attest verify line_demo.mlog "
                     "--json × N (perf_counter, the effect verified every "
                     "spawn)"),
        "pins": {"program": {"path": PROGRAM_PIN[0], "sha256": PROGRAM_PIN[1]},
                 "held_out": {"path": HELD_OUT_PIN[0], "sha256": HELD_OUT_PIN[1]},
                 "enforced": True},
        "binary_sha256": bin_sha,
        "measurement": {
            "median_ms": median_ms,
            "min_ms": min_ms,
            "p95_ms": p95_ms,
            "max_ms": max_ms,
            "runs": runs,
            "times_ms": times_ms,
        },
        "baseline": {
            "median_ms": BASELINE_MS,
            "source": ("the w9 bench, 23.11 — docs/research/"
                       "w9-line-bench-2026-11-23.md (a single recorded "
                       "session); NOT re-baselined by this lane: a baseline "
                       "rewrite only via a new run artifact + an ADR-0224 "
                       "§3 record"),
        },
        "budget": {
            "median_ms": BUDGET_MS,
            "provenance": ("agent-proposed per the 2-3× rule ≈ 3× the worst "
                           "measured 6.7 ms (a single record); the W3 gates "
                           "of №1008 are not touched"),
        },
        "delegation_verbatim": OWNER_VERBATIM_W3,
        "platform": honest_label(),
        "verdict": verdict,
    }
    summary = (f"median {median_ms} ms over {runs} spawns "
               f"(min {min_ms} / p95 {p95_ms} / max {max_ms}) — "
               f"baseline {BASELINE_MS} ms (w9, 23.11), budget "
               f"{BUDGET_MS} ms → verdict {verdict}")
    return art, summary


def write_txt(path: str, art: dict, summary: str) -> None:
    m = art["measurement"]
    b = art["baseline"]
    bg = art["budget"]
    lines = [
        "recurring-bench-attest-verify — E-form (numbers only from this recorded run)",
        art["naryad"],
        "",
        f"scenario: {art['scenario']}",
        f"pins enforced: {art['pins']['program']['path']} "
        f"{art['pins']['program']['sha256'][:12]}…; "
        f"{art['pins']['held_out']['path']} {art['pins']['held_out']['sha256'][:12]}…",
        f"binary sha256: {art['binary_sha256'][:12]}…",
        "",
        f"runs: {m['runs']}   median: {m['median_ms']} ms   "
        f"min: {m['min_ms']} ms   p95: {m['p95_ms']} ms   max: {m['max_ms']} ms",
        f"times_ms: {m['times_ms']}",
        "",
        f"baseline: {b['median_ms']} ms — {b['source']}",
        f"budget: {bg['median_ms']} ms — {bg['provenance']}",
        "",
        "delegation verbatim (№1008, gh#2080, 25.11): «"
        + art["delegation_verbatim"] + "»",
        f"platform: {art['platform']}",
        "",
        f"VERDICT: {art['verdict']} — {summary}",
        "(the lane is non-blocking; a FAIL exits 2 for visibility)",
    ]
    Path(path).write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True, help="the path to the mlog binary")
    ap.add_argument("--runs", type=int, default=30)
    ap.add_argument("--json", default=None, help="the artifact JSON path")
    ap.add_argument("--txt", default=None, help="the artifact txt path")
    a = ap.parse_args()

    art, summary = run_bench(a.bin, a.runs)
    if art is None:
        print(f"::error::recurring-bench: {summary}", file=sys.stderr)
        return 2  # loud, no numbers invented
    if a.json:
        Path(a.json).parent.mkdir(parents=True, exist_ok=True)
        Path(a.json).write_text(
            json.dumps(art, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8")
    if a.txt:
        Path(a.txt).parent.mkdir(parents=True, exist_ok=True)
        write_txt(a.txt, art, summary)
    print(summary)
    if art["verdict"] == "FAIL":
        print("::error::recurring-bench: the median exceeds the budget — "
              "FAIL (the lane is non-blocking, the visibility is the point)")
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
