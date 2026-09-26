#!/usr/bin/env python3
"""Generate the 100%-coverage builtin index for REFERENCE.md (Наряд №270).

SSOT is `BUILTIN_REGISTRY` (src/builtins/registry.rs, the `spec!` macros).
For every registered builtin the script emits one row into the generated
index section of REFERENCE.md:

  - name and category come straight from the registry;
  - arity (min/max) follows the ADR-0095 convention:
    `spec!("n", 0, "cat")` = variadic; `spec!("n", N, "cat")` = exactly N;
    `spec!("n", N, M, "cat")` = range N..=M;
  - the description is taken from the EXISTING curated REFERENCE rows when
    the builtin is already documented there; otherwise from the handler's
    `///` doc comment; otherwise an explicit `TODO(doc)` marker (never
    silence).

The generated block sits between explicit markers and is regenerated
in place — the curated manual sections (REFERENCE §1–§5) are untouched.

Run from the repo root:
    python3 scripts/gen_reference.py [--check]

    --check   exit 1 if regeneration would change the file (CI-style gate)
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

REGISTRY = REPO / "src" / "builtins" / "registry.rs"
REFERENCE = REPO / "REFERENCE.md"
BUILTINS_DIR = REPO / "src" / "builtins"

BEGIN = "<!-- BEGIN GENERATED BUILTIN INDEX (scripts/gen_reference.py — do not edit inside) -->"
END = "<!-- END GENERATED BUILTIN INDEX -->"

SPEC_RE = re.compile(r'spec!\(\s*"([^"]+)"\s*,\s*(\d+)\s*(?:,\s*(\d+))?\s*,\s*"([^"]+)"')
HANDLER_RE = re.compile(r';\s*([A-Za-z0-9_]+)\s*(?:,\s*"[A-Za-z][A-Za-z0-9<>]*")?\s*\)?\s*,?\s*$')  # №467: the typed tail `, "Type"` is optional
FN_RE = re.compile(r"(?:pub(?:\(crate\))?\s+)?fn\s+([a-z0-9_]+)\s*\(")
DOC_RE = re.compile(r"^\s*///\s?(.*)$")
ROW_NAME_RE = re.compile(r"^\|\s*`([a-z_][a-z0-9_]*)\(")
TODO = "TODO(doc)"

# Registry entries with NO host handler. Two verified classes (Наряд №270
# fact-check against src/vm.rs): (a) VM-native builtins — the VM implements
# them in its own call_builtin (vm.rs); the registry entry exists so VM
# bytecode arity validation and the index stay complete; (b) true stubs —
# no handler anywhere, calling the name errors on both backends.
MANUAL_DESCRIPTIONS = {
    # №415 drive-by: the reminder stubs' handlers are `cron_wrapped!` macro
    # aliases (`n_stamped` etc. in src/builtins/cron.rs), so the handler-doc
    # scan cannot resolve them and regeneration degraded the rows to
    # TODO(doc) (the pre-existing hand-maintained texts preserved verbatim).
    "remind": "`remind(message, timestamp, data?)` — one-time reminder. Returns ID.",
    "remind_recurring": "`remind_recurring(message, interval_seconds, data?)` — recurring reminder. Returns ID.",
    "cancel_remind": "`cancel_remind(id)` — cancel reminder. Returns \"ok\" or \"not_found\".",
    "check_reminders": "`check_reminders()` — get due reminders. One-shot deactivated; recurring advanced.",
    "list_reminders": "`list_reminders()` — list all active reminders.",
    "recall": "Memory recall — the front door of memory (№442): the store lane (hybrid BM25+vector RRF on the TW; the VM-native twin) plus the typed lane; a query that names gated private memory (a private container's key, no active consent grant) refuses fail-closed (MEMORY_RECALL_CONSENT_REQUIRED — the refusal is a memory.recall.denied record); typed-lane hits carry the [MEM] provenance suffix (container/subject/label/time/taint); every call records memory.recall {query hash, containers, hits, consent fact}.",
    "memory_retain_ttl": "The canon retain(memory, ttl) (№445): `memory_retain_ttl(handle, key, ttl_secs)` — gives ONE typed entry a lifetime; past the deadline the sweep auto-forgets it on the next read/keys/recall touch (the №280 v2 deferral lifted into the typed contour; the expiry is recorded as memory.ttl_expired {container, keys, hashes}). A repeat call MOVES the deadline. A POISONED entry refuses a new lifetime (MEMORY_POISONED). Records memory.retain_ttl. The deletion pin (memory_retain) and the TTL are independent.",
    "deny_event": "№392 DenyEvent — returns the typed deny event (`reason`, `sink`, `class`, `argument`, `label`, `line`, `human`) for the refusal being handled. Handler-scoped: intercepted by name inside `src/vm.rs` and `src/interpreter/execution.rs` (no host handler); outside an on_deny body it is a compile error and a loud runtime error.",
    "deny_reason": "№392 deny reason word — returns the `reason` string of the live DenyEvent (same vocabulary the audit check_ids use). Handler-scoped like deny_event; a match over it inside on_deny is checked for exhaustiveness.",
    "forget": "The forgetting front door (№445, the canon §10.3): `forget(handle, key, grant, dry_run?)` — the ADR-0155 linear action over the typed Memory<K> lane with the fail-closed refusal ladder: the consent gate (a private container requires the active memory:<subject> consent — MEMORY_FORGET_CONSENT_REQUIRED + a memory.forget.denied record), the grant ladder (GRANT_MISSING / GRANT_INACTIVE / GRANT_SCOPE_MISMATCH — scope memory:forget:<container>; the grant is consumed on SUCCESS only), the retained VETO (MEMORY_RETAIN_PROTECTED). dry_run=true returns the preview {dry_run, container, root, deleted: [], poisoned: [...], batch_id} — no state change, no grant consumption (the №280 preview discipline); apply deletes the ROOT (soft — the ledger carries the content digests) and POISONS every derived entry: a poisoned survivor stays in the container but cannot materialize into any sink — memory_read/memory_export refuse with MEMORY_POISONED and the recall lane skips the quarantine. Ledger: memory.forget {container, targets, hashes, dry_run fact} + irreversible.memory_forget on apply. The legacy 1..2-argument forget(query, days?) surface is unchanged (№72).",
    "find": "VM-native entity-store query (handled inside `src/vm.rs`, no host handler): scans globals for Struct values matching (type, field, operator, threshold).",
    "conv_start": "VM-native conversation context (handled inside `src/vm.rs`, no host handler): opens a conversation by id.",
    "conv_add": "VM-native conversation context (handled inside `src/vm.rs`, no host handler): appends a message to the conversation.",
    "conv_history": "VM-native conversation context (handled inside `src/vm.rs`, no host handler): returns the conversation message history.",
    "conv_context": "VM-native conversation context (handled inside `src/vm.rs`, no host handler): returns the compacted conversation context window.",
    "conv_end": "VM-native conversation context (handled inside `src/vm.rs`, no host handler): closes the conversation.",
    "event_count": "VM-native event analytics (handled inside `src/vm.rs`, no host handler): counts events in the event log, optionally filtered by type. (The registry's old \"planned, no handler\" comment is stale — the VM implements it.)",
    "events_since": "VM-native event analytics (handled inside `src/vm.rs`, no host handler): lists events since a sequence/timestamp marker.",
    "event_sum": "VM-native event analytics (handled inside `src/vm.rs`, no host handler): sums a numeric field across events of a type.",
    "query_scalar": "VM-native DB helper (handled inside `src/vm.rs`, no host handler): executes an SQL query and returns the first column of the first row as a scalar.",
    "query_row": "VM-native DB helper (handled inside `src/vm.rs`, no host handler): executes an SQL query and returns the first row as a Dict.",
    "resolve_skill_index": "VM-native skill resolver (handled inside `src/vm.rs`, no host handler): resolves a skill index entry for the VM execution path.",
    "fit_to_budget": "VM-native fluid-budget helper (handled inside `src/vm.rs`, no host handler): trims a List of items to fit a token budget.",
    "newline": "Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness).",
    "stdin": "Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness).",
    "split_tokens": "Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness).",
    "if_eq": "Registry-only stub — no handler on TW or VM; calling the name errors on both backends (the language's `if` comparison is an expression, not a builtin).",
    "is_string_token": "Registry-only stub — no handler on TW or VM; calling the name errors on both backends (entry kept for registry/opcode indexing completeness).",
}


def collect_registry():
    """(name, min, max, category, handler|None) per spec! line, in file order."""
    entries = []
    for line in REGISTRY.read_text(encoding="utf-8").splitlines():
        code = line.split("//")[0]
        m = SPEC_RE.search(code)
        if not m:
            continue
        name, lo, hi, cat = m.group(1), int(m.group(2)), m.group(3), m.group(4)
        hi = int(hi) if hi is not None else lo
        hm = HANDLER_RE.search(code)
        entries.append((name, lo, hi, cat, hm.group(1) if hm else None))
    return entries


def collect_handler_docs():
    """handler fn name -> raw /// lines.

    Наряд №309: the scan covers src/builtins/*.rs AND the three generative
    pillar modules (src/vision, src/voice, src/video) — their builtins'
    handlers live there (e.g. `builtin_video_render` in src/video/mod.rs,
    `builtin_tts_speak_stub` in src/voice/mod.rs). This closes the
    reference-generator pitfall where feature-gated pillar builtins always
    degraded to TODO(doc) on regeneration (the video/voice rows in the
    generated block were drifting from what the generator would emit).
    """
    docs = {}
    scan_dirs = [BUILTINS_DIR] + [
        REPO / "src" / pillar for pillar in ("vision", "voice", "video")
    ]
    seen_files = set()
    for d in scan_dirs:
        for f in sorted(d.rglob("*.rs")):
            if f.name == "registry.rs" or f in seen_files:
                continue
            seen_files.add(f)
            pending = []
            for ln in f.read_text(encoding="utf-8").splitlines():
                m = DOC_RE.match(ln)
                if m:
                    pending.append(m.group(1).strip())
                    continue
                fm = FN_RE.search(ln)
                if fm:
                    name = fm.group(1)
                    if name not in docs and pending:
                        docs[name] = list(pending)
                    pending = []
                    continue
                if ln.strip() and not ln.strip().startswith("//") and not ln.strip().startswith("#"):
                    # attribute lines (# [cfg]...) between doc and fn are fine —
                    # only real code clears the pending doc buffer
                    pending = []
    return docs


def collect_curated_rows():
    """builtin name -> (signature, description) from the CURATED reference
    (lines OUTSIDE the generated block — the block's own rows must not feed
    back into the next generation)."""
    rows = {}
    inside_generated = False
    for ln in REFERENCE.read_text(encoding="utf-8").splitlines():
        if ln.strip() == BEGIN:
            inside_generated = True
            continue
        if ln.strip() == END:
            inside_generated = False
            continue
        if inside_generated or not ln.startswith("|"):
            continue
        m = ROW_NAME_RE.match(ln)
        if not m:
            continue
        name = m.group(1)
        cells = [c.strip() for c in ln.strip().strip("|").split("|")]
        if len(cells) < 4 or name in rows:
            continue
        rows[name] = (cells[1], cells[3])
    return rows


def clean_doc(lines):
    """Raw doc lines -> one-paragraph description (Usage: prefix stripped)."""
    out = []
    for ln in lines:
        if not ln:
            if out:
                break
            continue
        if ln.startswith("#") or ln.startswith("```") or ln.startswith("- "):
            break
        out.append(ln)
    text = " ".join(out).strip()
    text = re.sub(r"^Usage:\s*", "", text)
    return text


def arity_text(lo, hi):
    if lo == 0 and hi == 0:
        return "variadic"
    if lo == hi:
        return str(lo)
    return f"{lo}..{hi}"


def split_generated(text):
    if BEGIN in text and END in text:
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        return head, tail
    return text, None


def render(entries, curated, docs):
    handler_of = {name: handler for name, _, _, _, handler in entries}
    cats = {}
    for name, lo, hi, cat, _ in entries:
        cats.setdefault(cat, []).append((name, lo, hi))

    md = [BEGIN, ""]
    md.append(f"## 6. Builtin Index — {len(entries)} registered builtins (100% of `spec!`)")
    md.append("")
    md.append(
        "> Generated from `BUILTIN_REGISTRY` (`src/builtins/registry.rs`) by "
        "`scripts/gen_reference.py` — the SSOT per `AGENTS.md` §5. Arity follows "
        "ADR-0095 (`variadic` = any count). Descriptions are imported from the "
        "curated sections above when present, otherwise from the handler's doc "
        "comment; `TODO(doc)` marks a description nobody has written yet — "
        "`tests/reference_consistency.rs` keeps the NAMES at 100%, humans keep "
        "the prose honest."
    )
    md.append("")
    todo = 0
    for cat in sorted(cats):
        rows = cats[cat]
        md.append(f"### `{cat}` — {len(rows)} builtin(s)")
        md.append("")
        md.append("| Builtin | Arity | Signature (curated) | Description |")
        md.append("|---|---|---|---|")
        for name, lo, hi in sorted(rows):
            sig, desc = curated.get(name, ("", ""))
            if not desc and name in MANUAL_DESCRIPTIONS:
                desc = MANUAL_DESCRIPTIONS[name]
            if not desc:
                h = handler_of.get(name)
                if h and h in docs:
                    desc = clean_doc(docs[h])
            if not desc:
                desc = TODO
                todo += 1
            md.append(
                f"| `{name}(...)` | {arity_text(lo, hi)} "
                f"| {(sig or '—').replace('|', '\\|')} | {desc.replace('|', '\\|')} |"
            )
        md.append("")
    md.append(END)
    return "\n".join(md), todo


def main():
    check_only = "--check" in sys.argv

    entries = collect_registry()
    curated = collect_curated_rows()
    docs = collect_handler_docs()

    text = REFERENCE.read_text(encoding="utf-8")
    head, tail = split_generated(text)
    block, todo = render(entries, curated, docs)

    if tail is None:
        new_text = text.rstrip("\n") + "\n\n" + block + "\n"
    else:
        new_text = head + block + tail

    if check_only:
        if new_text == text:
            print("OK: generated index is up to date")
            return 0
        print("STALE: generated index does not match the registry — run scripts/gen_reference.py")
        return 1

    REFERENCE.write_text(new_text, encoding="utf-8")
    doc_filled = len(entries) - len(curated) - todo
    print(
        f"registry: {len(entries)} builtins | curated rows matched: {len(curated)} | "
        f"doc-comment filled: {doc_filled} | TODO(doc) left: {todo}"
    )
    print("REFERENCE.md regenerated (curated sections untouched)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
