#!/usr/bin/env python3
"""№823 E2 — the reflex_train time/memory curve vs examples and classes.

Drives the RELEASE binary (the `vec` feature is NOT required for
reflex_train; the default release build suffices) with generated .mlog
programs: a Dense reflex model (input 16, one dense(32, relu) hidden
layer, C softmax outputs, seed 42) trains on N synthetic rows for 200
epochs (lr 0.1 — the dispatch constant). The in-program wall time
(now() deltas) wraps ONLY the reflex_train call; the process max RSS
(/usr/bin/time -v) is recorded per run alongside a no-train baseline
program's RSS — both raw. The dataset is linearly separable
(class-dependent offset + seeded jitter) so the training converges and
the reported loss/accuracy are meaningful.

Reproduces:
  cargo build --release -p metalogos-server --bin mlog
  python3 scripts/research/e2_train_curve.py [--out CSV]
"""
import argparse
import csv
import os
import platform
import re
import subprocess
import sys
import tempfile

BIN = os.environ.get("MLOG_BIN", "target/release/mlog")
EPOCHS = 200.0
FEATURES = 16

PROGRAM_TMPL = """\
// №823 E2 — generated measurement program (do not hand-edit)
reflex BenchModel {{
  input: embedding({features})
  layers: [dense(32, relu), dense({classes}, softmax)]
  labels: [{labels}]
  seed: 42
}}

pattern Train(_x: String) -> String {{
  let data = [{data}]
  let t0 = now()
  let r = reflex_train(BenchModel, data, {epochs}, "accuracy", 0.99)
  let t1 = now()
  return "E2_MS=" + to_string((t1 - t0) * 1000.0)
       + " LOSS=" + to_string(r.loss)
       + " ACC=" + to_string(r.accuracy)
}}

flow Main {{ input: String = "x" -> Train -> output }}
"""

BASELINE_TMPL = """\
// №823 E2 baseline — the same data built, NO reflex_train
reflex BenchModel {{
  input: embedding({features})
  layers: [dense(32, relu), dense({classes}, softmax)]
  labels: [{labels}]
  seed: 42
}}

pattern Train(_x: String) -> String {{
  let data = [{data}]
  return "E2_MS=0.0 LOSS=0.0 ACC=0.0"
}}

flow Main {{ input: String = "x" -> Train -> output }}
"""


def gen_data(n, classes, seed=42):
    """n rows [f1..f16, class_idx] — linearly separable: class c lives
    around offset c*2.0 on axis 0, seeded jitter ±0.8 elsewhere (the
    compact scale: the softmax head at lr 0.1 / 200 epochs saturates on
    wide offsets — C=2 converges to acc 1.0, C=5 to ~0.8, C=10 to
    ~0.35 under this budget; the convergence quality per C is part of
    the measured result, the timing is E2's object)."""
    rows = []
    x = seed
    for i in range(n):
        x = (x * 1103515245 + 12345) % (2 ** 31)
        c = i % classes
        feats = [f"{(c * 2.0 + (x % 100) / 62.5 - 0.8):.3f}"]
        for _ in range(FEATURES - 1):
            x = (x * 1103515245 + 12345) % (2 ** 31)
            feats.append(f"{(x % 200) / 100.0:.3f}")
        rows.append("[" + ", ".join(feats) + f", {c}.0]")
    return rows


def build_program(n, classes, baseline):
    labels = ", ".join(f'"c{c}"' for c in range(classes))
    data = ", ".join(gen_data(n, classes))
    tmpl = BASELINE_TMPL if baseline else PROGRAM_TMPL
    return tmpl.format(features=FEATURES, classes=classes, labels=labels,
                       data=data, epochs=EPOCHS)


def run_once(program, tmpdir, tag, timed):
    path = os.path.join(tmpdir, f"e2_{tag}.mlog")
    with open(path, "w") as f:
        f.write(program)
    cmd = [BIN, "run", path]
    out_path = os.path.join(tmpdir, f"e2_{tag}.out")
    err_path = os.path.join(tmpdir, f"e2_{tag}.err")
    with open(out_path, "w") as fo, open(err_path, "w") as fe:
        p = subprocess.Popen(cmd, stdout=fo, stderr=fe)
        if timed:
            # os.wait4 gives THIS child's rusage (ru_maxrss, kB on Linux)
            # — /usr/bin/time is absent in the sandbox; same measurement.
            _, status, ru = os.wait4(p.pid, 0)
            p.returncode = os.waitstatus_to_exitcode(status)
        else:
            p.wait()
            ru = None
    out = open(out_path).read()
    err = open(err_path).read()
    if p.returncode != 0:
        raise SystemExit(f"mlog run failed rc={p.returncode}:\n"
                         f"stdout: {out[:300]}\nstderr: {err[:600]}")
    m = re.search(r"E2_MS=([0-9.]+) LOSS=([0-9.eE+-]+) ACC=([0-9.eE+-]+)", out)
    if not m:
        raise SystemExit(f"no E2 marker: {out[:300]}")
    rss = ru.ru_maxrss if ru is not None else None
    return float(m.group(1)), float(m.group(2)), float(m.group(3)), rss


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--out", default="docs/research/e2_train_curve.csv")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        raise SystemExit(f"{BIN} not found — build the release binary first")

    ns = [50, 100, 200, 400, 800]
    classes_list = [2, 5, 10]
    rows = []
    rss_baseline = None

    with tempfile.TemporaryDirectory() as td:
        # the RSS baseline: the largest dataset BUILT but not trained
        prog = build_program(ns[-1], classes_list[-1], baseline=True)
        _, _, _, rss_baseline = run_once(prog, td, "rss_base", timed=True)
        print(f"RSS baseline (data built, no train): {rss_baseline} kB")
        rss_baseline = rss_baseline or 0

        for classes in classes_list:
            for n in ns:
                for rep in range(args.reps):
                    prog = build_program(n, classes, baseline=False)
                    ms, loss, acc, rss = run_once(prog, td, f"c{classes}_n{n}_{rep}",
                                                  timed=True)
                    rows.append({"n": n, "classes": classes, "rep": rep,
                                 "train_ms": round(ms, 2),
                                 "loss": loss, "accuracy": acc,
                                 "max_rss_kb": rss,
                                 "ms_per_epoch": round(ms / EPOCHS, 4)})
                    print(f"  C={classes:2d} N={n:4d} rep {rep}: {ms:9.1f} ms "
                          f"({ms / EPOCHS:6.3f} ms/epoch) loss={loss:.4f} "
                          f"acc={acc:.3f} rss={rss} kB", flush=True)

    print("\n=== E2 summary (p50 of reps) ===")
    for classes in classes_list:
        for n in ns:
            sel = [r["train_ms"] for r in rows
                   if r["classes"] == classes and r["n"] == n]
            sel.sort()
            p50 = sel[len(sel) // 2]
            rss = max(r["max_rss_kb"] for r in rows
                      if r["classes"] == classes and r["n"] == n)
            print(f"  C={classes:2d} N={n:4d}: train p50={p50:9.1f} ms "
                  f"(~{p50 / EPOCHS:6.3f} ms/epoch), max_rss={rss} kB "
                  f"(delta over baseline: {rss - rss_baseline} kB)")

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
    return 0


if __name__ == "__main__":
    sys.exit(main())
