#!/usr/bin/env python3
"""No529 registry-bounds ratchet (blocking CI job `registry-bounds`).

Validates scripts/ci/registry_bounds_inventory.txt against the sources:

  1. DRIFT: every `CONST=value` in the inventory row must exist in the
     named source file with that exact value (the caps cannot move
     silently).
  2. RATCHET: every process-global map-shaped static in src/ must be
     covered by an inventory row (a new unbounded registry is the C-12
     class re-opening). The scan is conservative: it flags only the
     `static NAME: ... Mutex<HashMap<...>>` / `Lazy<... Mutex<HashMap`
     shapes that the №515/№529 registries actually use, so a false
     negative is possible for exotic shapes — the inventory is the SSOT,
     and a new registry author MUST add a row (the failure message says
     exactly that).

Exit 0 = the inventory is honest; exit 1 = drift or an unlisted registry.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
INV = ROOT / "scripts/ci/registry_bounds_inventory.txt"

# Inventory key → the grep-able anchors that must exist in the source.
# The count/byte constants are checked as `NAME = value` / `NAME: usize = value`.
CONST_PATTERNS = {
    "PDF_DOCS": ["PDF_DOCS_MAX", "PDF_DOCS_MAX_BYTES", "pdf_store_eviction_metrics"],
    "VIDEO_REGISTRY": ["VIDEO_ARTIFACTS_MAX", "VIDEO_ARTIFACTS_MAX_BYTES", "video_registry_eviction_metrics"],
    "VOICE_REGISTRY#artifacts": ["VOICE_ARTIFACTS_MAX", "VOICE_ARTIFACTS_MAX_BYTES", "voice_registry_eviction_metrics"],
    "VOICE_REGISTRY#voiceprints": ["VOICE_ARTIFACTS_MAX", "VOICE_ARTIFACTS_MAX_BYTES"],
    "LLM_STREAM_REGISTRY": ["LLM_STREAM_DEFAULT_MAX", "STREAM_LIMIT_REACHED"],
    "GLOBAL_TEMPLATES": ["GLOBAL_TEMPLATES"],
}

# The static shapes the registries actually use (conservative ratchet scan).
GLOBAL_SHAPE = re.compile(
    r"static\s+([A-Z_][A-Z0-9_]{2,})\s*:\s*[^=]*Mutex<HashMap",
)


def main() -> int:
    failures: list[str] = []
    rows: dict[str, str] = {}
    for line in INV.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name = line.split("|", 1)[0].strip()
        rows[name] = line

    for name, row in sorted(rows.items()):
        parts = [p.strip() for p in row.split("|")]
        if len(parts) < 4:
            failures.append(f"{name}: the row needs name|source|count|bytes|notes")
            continue
        src = ROOT / parts[1]
        if not src.exists():
            failures.append(f"{name}: the source file {parts[1]} is gone")
            continue
        text = src.read_text()
        for anchor in CONST_PATTERNS.get(name, []):
            # Word-boundary match: a renamed anchor (anchor + suffix) is
            # still a removed anchor — the substring check would lie.
            if not re.search(rf"\b{re.escape(anchor)}\b", text):
                failures.append(f"{name}: the anchor `{anchor}` disappeared from {parts[1]}")
        # The VALUE drift: every NAME=value pair encoded in the row must
        # match the actual const definition in the source (a silently
        # moved cap is the Д-5 drift the ratchet exists to stop). The
        # source may write the cap as an arithmetic expression
        # (`16 * 1024 * 1024`) — the RHS is evaluated as pure integer
        # arithmetic before the comparison.
        for cname, cval in re.findall(r"([A-Z_][A-Z0-9_]*)=(\d+)", "|".join(parts[2:4])):
            m = re.search(rf"const\s+{cname}\s*:\s*(usize|u32)\s*=\s*([^;]+);", text)
            if not m:
                failures.append(f"{name}: the bound `{cname}` has no const definition in {parts[1]}")
                continue
            rhs = m.group(2).strip()
            if not re.fullmatch(r"[\d\s*+()]+", rhs):
                failures.append(
                    f"{name}: the bound `{cname}` in {parts[1]} is not a plain integer "
                    f"arithmetic expression (`{rhs}`) — the ratchet cannot verify it"
                )
                continue
            actual = eval(rhs, {"__builtins__": {}}, {})  # digits, * + ( ) only
            if actual != int(cval):
                failures.append(
                    f"{name}: the bound `{cname}={cval}` does not match the "
                    f"definition in {parts[1]} (= {actual}; drift — resync the "
                    f"inventory or the code)"
                )

    # The ratchet: every map-shaped global static must appear in the inventory.
    listed_names = set(rows)
    known_aliases = {
        # rows may cover several statics under one registry name
        "VOICE_REGISTRY#artifacts": {"VOICE_REGISTRY"},
        "VOICE_REGISTRY#voiceprints": {"VOICE_REGISTRY"},
    }
    covered = set(listed_names)
    for alias_set in known_aliases.values():
        covered |= alias_set

    for src in sorted((ROOT / "src").rglob("*.rs")):
        rel = src.relative_to(ROOT).as_posix()
        # The media/llm_cache/persisted lanes are NOT in-process registries
        # (the №529 boundary: persistent lanes untouched); the sandbox and
        # fs-gate statics are not entry stores. Whitelist the honest non-
        # registry map statics found in src (documented, not hidden).
        text = src.read_text()
        for m in GLOBAL_SHAPE.finditer(text):
            name = m.group(1)
            if name in covered:
                continue
            # The known honest exceptions: not request-entry stores.
            if name in {
                "GLOBAL_TEMPLATES",  # covered by its own row
                "LLM_CACHE",         # the SQLite llm_cache is the store; the in-memory map is LRU-bounded since No273
                "CARD_SESSIONS",     # documented plaintext-legacy in privacy.md (the known tail, No519)
                "CAL_SESSIONS",      # same family as CARD_SESSIONS
                "SANDBOX",           # not an entry store
                "METRICS",           # counters, not entries
            }:
                continue
            failures.append(
                f"UNLISTED REGISTRY: static `{name}` in {rel} looks like a process-global "
                f"entry store but has no row in scripts/ci/registry_bounds_inventory.txt — "
                f"add a row with BOTH bounds (count + bytes) or an explicit exception reason "
                f"(the No529 ratchet: a new unbounded registry is the C-12 class re-opening)"
            )

    if failures:
        print("registry-bounds: FAILED")
        for f in failures:
            print(" -", f)
        return 1
    print("registry-bounds: OK —", len(rows), "inventoried registries, the ratchet holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
