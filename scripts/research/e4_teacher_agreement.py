#!/usr/bin/env python3
"""№823 E4 — the teacher's baseline agreement: the teacher labels vs the
majority baseline and vs the procedural ground truth.

The teacher is a REAL LLM-API path with a DETERMINISTIC rule-based
labeler behind it: a local HTTP server implements the OpenAI
/chat/completions contract and labels each text by the SAME rule the
ground truth comes from — except a KNOWN error pattern (texts carrying
"tricky" get the flipped label), so the teacher quality is a measured
number (~92% by construction), not a tautology. Network-free: 127.0.0.1
only.

Flow: the driver starts the teacher server, starts `mlog serve` (the
program wires llm {} to the teacher URL, memory { persist } to a
scratch db, a learnable pattern with distill_to + distill_after high
enough to stay in TEACHING mode), drives N GET /classify requests, stops
both, reads the `distill_samples` table (the №495 persistence) and
computes:
  - teacher vs ground truth (the teacher accuracy on the corpus),
  - teacher vs the majority baseline (do they agree per row),
  - the majority baseline vs ground truth (the baseline accuracy).

Reproduces:
  cargo build --release -p metalogos-server --bin mlog
  python3 scripts/research/e4_teacher_agreement.py [--out CSV]
"""
import argparse
import csv
import json
import os
import re
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from http.server import BaseHTTPRequestHandler, HTTPServer

BIN = os.environ.get("MLOG_BIN", "target/release/mlog")
CORPUS_N = 60

PROGRAM_TMPL = """\
// №823 E4 — generated measurement program (do not hand-edit)
// The teacher provider rides the ENV lane (METALOGOS_LLM_PROVIDER=
// openai + METALOGOS_OPENAI_BASE_URL → the local rule teacher): the
// serve route lane's learnable call resolves the LEGACY env backend
// (the per-request interpreter carries no smart router — the llm {{}}
// declaration is invisible there; recorded as E4's honest finding),
// so the env config is the lane that actually drives the teacher.
memory {{ persist: "{db_path}" }}

reflex TeacherHead {{
  input: embedding(4)
  layers: [dense(8, relu), dense(2, softmax)]
  labels: ["neg", "pos"]
  seed: 42
}}

learnable pattern Classify(text: String) -> String {{
  prompt: "label the request: " + text
  distill_to: TeacherHead
  distill_after: 100000
}}

mlogserver {{
  port: {serve_port}
  route "/classify" method=GET {{
    let text = query_param("text")
    let _label = Classify(text)
    return respond("200 accepted")
  }}
}}
"""


def gen_corpus(n, seed=42):
    """n texts with PROCEDURAL ground truth: 'urgent' → pos, else neg;
    every text containing 'tricky' is the teacher's KNOWN error pattern
    (the teacher flips it). Returns [(text, truth)]."""
    words = ["invoice", "refund", "status", "shipment", "password", "reset",
             "report", "meeting", "backup", "quota"]
    out, x = [], seed
    for i in range(n):
        x = (x * 1103515245 + 12345) % (2 ** 31)
        w = words[x % len(words)]
        urgent = (x >> 3) % 2 == 0
        tricky = (x >> 5) % 11 == 3  # ~1/11 of the corpus
        text = f"{w}-{i}" + (" urgent" if urgent else "")
        if tricky:
            text += " tricky"
        truth = "neg" if tricky else ("pos" if urgent else "neg")
        out.append((text, truth))
    return out


class Teacher(BaseHTTPRequestHandler):
    """The deterministic rule teacher with the KNOWN error pattern:
    'tricky' in the prompt → the flipped label."""

    corpus_rule = "tricky"  # the flip trigger

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode("utf-8", "replace")
        try:
            payload = json.loads(body)
            prompt = payload["messages"][-1]["content"]
        except Exception:
            prompt = body
        # the rule: pos if "urgent" in the text, neg otherwise; the
        # tricky-texts get the flipped label (the measured error).
        text = prompt.replace("label the request:", "").strip().lower()
        if "tricky" in text:
            label = "neg" if "urgent" in text else "pos"
        else:
            label = "pos" if "urgent" in text else "neg"
        resp = json.dumps({
            "id": "e4-teacher", "object": "chat.completion",
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": label}}],
        }).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(resp)))
        self.end_headers()
        self.wfile.write(resp)

    def log_message(self, *a):
        pass


def wait_health(port, deadline=30.0):
    import urllib.request
    t0 = time.time()
    while time.time() - t0 < deadline:
        try:
            with urllib.request.urlopen(
                    f"http://127.0.0.1:{port}/health", timeout=2) as r:
                if r.status == 200:
                    return True
        except Exception:
            time.sleep(0.5)
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=CORPUS_N)
    ap.add_argument("--out", default="docs/research/e4_teacher_agreement.csv")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        raise SystemExit(f"{BIN} not found — build the release binary first")

    corpus = gen_corpus(args.n)

    teacher = HTTPServer(("127.0.0.1", 0), Teacher)
    teacher_port = teacher.server_address[1]
    t_thread = __import__("threading").Thread(target=teacher.serve_forever,
                                              daemon=True)
    t_thread.start()
    print(f"teacher server: 127.0.0.1:{teacher_port}")

    serve_port = 18091 + (os.getpid() % 500)
    with tempfile.TemporaryDirectory() as td:
        db_path = os.path.join(td, "e4_distill.db")
        prog_path = os.path.join(td, "e4.mlog")
        with open(prog_path, "w") as f:
            f.write(PROGRAM_TMPL.format(teacher_port=teacher_port,
                                        serve_port=serve_port,
                                        db_path=db_path))
        serve = subprocess.Popen(
            [BIN, "serve", prog_path],
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            env={**os.environ,
                 "METALOGOS_LLM_PROVIDER": "openai",
                 "METALOGOS_API_KEY": "local-test",
                 "METALOGOS_OPENAI_BASE_URL": f"http://127.0.0.1:{teacher_port}/v1"})
        try:
            if not wait_health(serve_port):
                serve.kill()
                out, _ = serve.communicate(timeout=5)
                raise SystemExit(f"serve did not come up:\n{out[-1500:]}")
            print(f"serve up on :{serve_port}; driving {len(corpus)} requests")
            import urllib.request
            import urllib.parse
            answers = []
            for text, truth in corpus:
                q = urllib.parse.quote(text)
                with urllib.request.urlopen(
                        f"http://127.0.0.1:{serve_port}/classify?text={q}",
                        timeout=60) as r:
                    answers.append(r.read().decode())
            print(f"collected {len(answers)} answers")
        finally:
            serve.send_signal(signal.SIGTERM)
            try:
                serve.wait(timeout=10)
            except subprocess.TimeoutExpired:
                serve.kill()
            teacher.shutdown()

        # the distill_samples persistence (the №495 table)
        if not os.path.exists(db_path):
            raise SystemExit(f"no distill db at {db_path} — the hub did not persist")
        conn = sqlite3.connect(db_path)
        rows = conn.execute(
            "SELECT pattern_name, input, output, source FROM distill_samples "
            "ORDER BY rowid").fetchall()
        conn.close()
        print(f"distill_samples rows: {len(rows)}")

    def parse_label(out_text):
        """The label extraction: the recorded output is the teacher's
        response string — exact match or the first label token found."""
        t = out_text.strip().lower()
        if t in ("pos", "neg"):
            return t
        m = re.search(r"\b(pos|neg)\b", t)
        return m.group(1) if m else f"UNPARSED:{t[:40]}"

    records = []
    for text, truth in corpus:
        hit = [(i, o) for (p, i, o, s) in rows if text in (i or "")]
        teacher_label = parse_label(hit[0][1]) if hit else "MISSING"
        records.append({"text": text, "truth": truth,
                        "teacher": teacher_label,
                        "db_source": hit[0][1][:60] if hit else ""})

    n = len(records)
    majority = max(("pos", "neg"), key=lambda l: sum(
        1 for r in records if r["truth"] == l))
    teach_correct = sum(1 for r in records if r["teacher"] == r["truth"])
    teach_majority = sum(1 for r in records if r["teacher"] == majority)
    majority_correct = sum(1 for r in records if majority == r["truth"])
    unparsed = sum(1 for r in records if r["teacher"].startswith(("UNPARSED", "MISSING")))

    print(f"\n=== E4 summary (N={n}, majority baseline='{majority}') ===")
    print(f"  teacher vs ground truth (accuracy): {teach_correct}/{n} = {teach_correct / n:.4f}")
    print(f"  teacher vs majority baseline:       {teach_majority}/{n} = {teach_majority / n:.4f}")
    print(f"  majority vs ground truth:           {majority_correct}/{n} = {majority_correct / n:.4f}")
    if unparsed:
        print(f"  UNPARSED/MISSING outputs: {unparsed} (the raw streams are in the CSV)")

    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["text", "truth", "teacher", "db_source"])
        w.writeheader()
        w.writerows(records)
    print(f"CSV written: {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
