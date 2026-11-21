#!/usr/bin/env bash
# №988 (E1–E4, ADR-0200 §5) — the documented measurement invocation.
# Runs the env-guarded measurement tests (METALOGOS_RESEARCH_E1E4=1)
# in release mode with one thread,
# captures the JSON blocks the harness prints, merges the environment
# facts, and writes the machine-readable artifacts next to the research
# reports in docs/research/.
set -euo pipefail
cd "$(dirname "$0")/../.."

export METALOGOS_EMBEDDING_PROVIDER=tfidf
export METALOGOS_MOCK_LLM=1
export METALOGOS_RESEARCH_E1E4=1

OUT=docs/research
mkdir -p "$OUT"
RAW="$OUT/e1_e4_raw_run.txt"

{
  echo "== environment =="
  date -u +"%Y-%m-%dT%H:%M:%SZ"
  uname -srm
  nproc
  grep -m1 "model name" /proc/cpuinfo || true
  grep -m1 MemTotal /proc/meminfo || true
  rustc --version
  cargo --version
  grep -m1 channel rust-toolchain.toml
} > "$RAW"

cargo test --release --test research_e1_e4 -- --ignored --nocapture --test-threads=1 2>&1 | tee -a "$RAW"

python3 - "$RAW" <<'PYEOF'
import json, re, sys
raw = open(sys.argv[1]).read()
blocks = re.findall(r'===METALOGOS-E1E4-JSON-START===\n(.*?)\n===METALOGOS-E1E4-JSON-END===', raw, re.S)
names = {
    "e1-embed-speed": "e1-embed-speed.json",
    "e2-reflex-train-time": "e2-reflex-train-time.json",
    "e3-confidence-calibration": "e3-confidence-calibration.json",
    "e4-teacher-baseline-agreement": "e4-teacher-baseline-agreement.json",
}
env_lines = raw.split("== environment ==")[1].splitlines()
# The marker line is followed by an empty line, then the facts.
env_lines = [l for l in env_lines if l.strip()]
env = {
    "captured_at": env_lines[0].strip(),
    "uname": env_lines[1].strip(),
    "cpus": int(env_lines[2].strip()),
    "cpu_model": env_lines[3].strip(),
    "mem_total": env_lines[4].strip(),
    "rustc": env_lines[5].strip(),
    "cargo": env_lines[6].strip(),
    "toolchain_channel": env_lines[7].strip(),
    "embed_provider": "tfidf",
    "mock_llm_lane": True,
}
assert len(blocks) == 4, "expected 4 JSON blocks, got %d" % len(blocks)
for b in blocks:
    obj = json.loads(b)
    obj["environment"] = env
    out = "docs/research/" + names[obj["experiment"]]
    with open(out, "w") as f:
        json.dump(obj, f, indent=2)
        f.write("\n")
    print("wrote", out)
PYEOF
echo "e1-e4 measurement run done"
