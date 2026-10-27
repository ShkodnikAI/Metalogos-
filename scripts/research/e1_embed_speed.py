#!/usr/bin/env python3
"""№823 E1 — the `embed` builtin speed: p50/p95 on 1k/10k lines.

The measurement drives the RELEASE binary (`cargo build --release -p
metalogos-server --bin mlog`) with a generated .mlog program: a list of
N distinct strings is embedded in an `each` loop, the in-program wall
time (now() deltas — f64 epoch SystemTime, non-monotonic, fine for
<10 s windows; recorded as the honest limitation) is printed with a
checksum (the dead-code elimination guard). The startup baseline (the
same loop WITHOUT the embed call, `len` only) is measured the same way;
both raw series go to the CSV, the doc subtracts them for the per-item
figure. Hardware: recorded from this machine (platform + cpuinfo).

Reproduces:
  cargo build --release -p metalogos-server --bin mlog
  python3 scripts/research/e1_embed_speed.py [--reps 7] [--out CSV]
"""
import argparse
import csv
import os
import platform
import re
import subprocess
import sys
import tempfile
import time

BIN = os.environ.get("MLOG_BIN", "target/release/mlog")

PROGRAM_TMPL = """\
// №823 E1 — generated measurement program (do not hand-edit)
pattern BenchEmbed(_x: String) -> String {{
  let texts = [{texts}]
  let mut i = 0.0
  let mut sink = 0.0
  let n = {n}.0
  let t0 = now()
  while i < n {{
    let v = embed(texts[i])
    sink = sink + v[0]
    i = i + 1.0
  }}
  let t1 = now()
  let _ = sink
  return "E1_MS=" + to_string((t1 - t0) * 1000.0) + " SINK=" + to_string(sink)
}}

flow Main {{ input: String = "x" -> BenchEmbed -> output }}
"""

BASELINE_TMPL = """\
// №823 E1 baseline — the same loop WITHOUT the embed call
pattern BenchBase(_x: String) -> String {{
  let texts = [{texts}]
  let mut i = 0.0
  let mut sink = 0.0
  let n = {n}.0
  let t0 = now()
  while i < n {{
    let t = texts[i]
    sink = sink + len(t)
    i = i + 1.0
  }}
  let t1 = now()
  let _ = sink
  return "E1_MS=" + to_string((t1 - t0) * 1000.0) + " SINK=" + to_string(sink)
}}

flow Main {{ input: String = "x" -> BenchBase -> output }}
"""

ALLOC_TMPL = """\
// №823 E1 alloc-matched baseline — the same loop CONSTRUCTING a fresh
// 256-Float list per iteration (the allocation shape of the embed
// result, dim = TFIDF_EMBEDDING_DIM) without the tokenize/hash work.
// The delta embed − alloc-matched isolates the actual feature
// computation from the loop + packaging overhead.
pattern BenchAlloc(_x: String) -> String {{
  let texts = [{texts}]
  let mut i = 0.0
  let mut sink = 0.0
  let n = {n}.0
  let t0 = now()
  while i < n {{
    let _t = texts[i]
    let v = [{fill}]
    sink = sink + v[0]
    i = i + 1.0
  }}
  let t1 = now()
  let _ = sink
  return "E1_MS=" + to_string((t1 - t0) * 1000.0) + " SINK=" + to_string(sink)
}}

flow Main {{ input: String = "x" -> BenchAlloc -> output }}
"""


def gen_texts(n, seed=42, distinct=False):
    """n distinct deterministic strings.

    distinct=False — the STEADY-STATE shape: a fixed small vocabulary
    (20 words x 100 numeric suffixes, ~120 distinct tokens) — the dim
    of the default SSOT backend (TfidfEmbedding, vocab-growing) stays
    at its 256 floor; this is the constant per-line cost.
    distinct=True — the VOCAB-GROWTH shape: every line carries a unique
    token ("run<i>"), the vocabulary (and with it the vector dim) grows
    with the corpus — the honest growth property of the default
    backend, reported separately.
    """
    words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf",
             "hotel", "india", "juliet", "kilo", "lima", "mike", "november",
             "oscar", "papa", "quebec", "romeo", "sierra", "tango"]
    out = []
    x = seed
    for i in range(n):
        x = (x * 1103515245 + 12345) % (2 ** 31)
        w1 = words[x % len(words)]
        w2 = words[(x >> 5) % len(words)]
        if distinct:
            out.append(f'"task{i} {w1}{i % 13} {w2}"')
        else:
            out.append(f'"{w1}{i % 100} {w2}{i % 100} report"')
    return out


def build_program(n, baseline, distinct):
    texts = ", ".join(gen_texts(n, distinct=distinct))
    if baseline == "plain":
        return BASELINE_TMPL.format(texts=texts, n=n)
    if baseline == "alloc":
        fill = ", ".join(f"{j * 0.001:.3f}" for j in range(256))
        return ALLOC_TMPL.format(texts=texts, fill=fill, n=n)
    return PROGRAM_TMPL.format(texts=texts, n=n)


def run_once(program, tmpdir, tag):
    path = os.path.join(tmpdir, f"e1_{tag}.mlog")
    with open(path, "w") as f:
        f.write(program)
    t0 = time.time()
    env = dict(os.environ)
    # The measurement harness raises the №546 contour budget consciously
    # (the default 64/tick is the PRODUCTION posture; a 10k-line embed
    # loop is a benchmark, not a request shape). Recorded in the doc.
    env["METALOGOS_CONTOUR_BUDGET"] = "100000"
    proc = subprocess.run([BIN, "run", path], capture_output=True, text=True,
                          timeout=600, env=env)
    wall = time.time() - t0
    if proc.returncode != 0:
        raise SystemExit(f"mlog run failed rc={proc.returncode}:\n"
                         f"stdout: {proc.stdout[:400]}\nstderr: {proc.stderr[:800]}\n"
                         f"(NOTE: the binary must carry the `vec` feature: "
                         f"cargo build --release -p metalogos-server --bin mlog "
                         f"--features metalogos/vec)")
    m = re.search(r"E1_MS=([0-9.]+) SINK=(-?[0-9.eE+]+)", proc.stdout)
    if not m:
        raise SystemExit(f"no E1_MS marker in output: {proc.stdout[:400]}")
    return float(m.group(1)), m.group(2), wall


def pct(vals, p):
    s = sorted(vals)
    k = (len(s) - 1) * p
    lo, hi = int(k), min(int(k) + 1, len(s) - 1)
    return s[lo] + (s[hi] - s[lo]) * (k - lo)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--reps", type=int, default=7)
    ap.add_argument("--out", default="docs/research/e1_embed_speed.csv")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        raise SystemExit(f"{BIN} not found — build the release binary first")

    rows = []
    summary = {}
    with tempfile.TemporaryDirectory() as td:
        for distinct, shape in ((False, "steady"), (True, "grow")):
            for n, kind in ((1000, "1k"), (10000, "10k")):
                for arm in (False, "plain", "alloc"):
                    ms_list = []
                    for rep in range(args.reps):
                        ms, sink, wall = run_once(
                            build_program(n, arm, distinct), td,
                            f"{shape}_{kind}_{arm}_{rep}")
                        ms_list.append(ms)
                        rows.append({"shape": shape, "size": kind,
                                     "arm": arm, "rep": rep,
                                     "in_program_ms": round(ms, 3),
                                     "checksum": sink, "wall_s": round(wall, 3)})
                        print(f"  {shape:6s} {kind} {str(arm):6s} rep {rep}: "
                              f"{ms:9.2f} ms (wall {wall:.2f}s)", flush=True)
                    summary[(shape, n, arm)] = ms_list

        print("\n=== E1 summary (in-program ms; p50/p95 over %d reps) ===" % args.reps)
        for (shape, n, arm), ms_list in sorted(summary.items(), key=lambda kv: (str(kv[0][0]), kv[0][1], str(kv[0][2]))):
            print(f"{shape:6s} n={n:6d} {str(arm):8s} p50={pct(ms_list, 0.5):9.2f} "
                  f"p95={pct(ms_list, 0.95):9.2f} min={min(ms_list):9.2f}")
        print("\nembed cost per line (p50 delta vs the arms):")
        for shape in ("steady", "grow"):
            for n, kind in ((1000, "1k"), (10000, "10k")):
                emb = pct(summary[(shape, n, False)], 0.5)
                plain = pct(summary[(shape, n, "plain")], 0.5)
                alloc = pct(summary[(shape, n, "alloc")], 0.5)
                print(f"  {shape:6s} {kind}: embed−plain={(emb - plain) / n * 1000:7.1f} "
                      f"us/line; embed−alloc={(emb - alloc) / n * 1000:7.1f} us/line")

    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"CSV written: {args.out}")

    print("\nhardware:")
    print(f"  platform: {platform.platform()}")
    for line in os.popen("grep -m1 'model name' /proc/cpuinfo"):
        print(f"  cpu: {line.split(':', 1)[1].strip()}")
    for line in os.popen("nproc"):
        print(f"  cores: {line.strip()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
