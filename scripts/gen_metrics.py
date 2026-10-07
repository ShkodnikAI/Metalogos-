#!/usr/bin/env python3
"""Generate the metrics block for README.md (Naryad #460, audit 25.09 §7.1).

The problem the naryad closed: README carried hand-written, contradictory
numbers ("~59 000 LOC" vs "src/ ~93 000 LOC" vs the real thing) that drifted
on every merge. The fix: the numbers are GENERATED from the repository state
by this script and embedded into README.md between explicit markers — the
same posture as `gen_reference.py` (the curated text stays hand-written; the
counts are computed).

Mirrors the independent recomputation in `tests/readme_consistency.rs`:
the test derives the same numbers from the same sources and compares them
against the claims the README makes. If the generated block and the test
ever disagree, CI is red — by construction.

Run from the repo root:
    python3 scripts/gen_metrics.py [--check]

    --check   exit 1 if regeneration would change the file (CI-style gate)
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
README = REPO / "README.md"
PLAN_SUMMARY = REPO / "docs" / "PLAN-SUMMARY.md"

BEGIN = "<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->"
END = "<!-- END GENERATED METRICS -->"

# №566 (gh#928; the audit 02.10 §7.2 W-4): the SECOND generated target —
# the machine facts the public digest (docs/PLAN-SUMMARY.md) quotes as
# CURRENT. The audit's finding: the digest carried a hand-written
# "now 2110 bp" while the fact was 3195 bp — the grantors' digest lagged
# the machine metrics. The fix mirrors the №460 posture: ONE generator,
# the numbers between explicit markers, the narrative stays hand-written
# (the №566 boundary: the digest's prose is the publisher's voice).
PS_BEGIN = "<!-- BEGIN GENERATED NUMBERS (scripts/gen_metrics.py — do not edit inside) -->"
PS_END = "<!-- END GENERATED NUMBERS -->"


def count_total_builtins() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    return len(re.findall(r"spec!\(", content))


def count_builtin_modules() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    string_re = re.compile(r'"([^"]+)"')
    layer_re = re.compile(r'=>\s*"[^"]+"')
    # №467: the typed rows end with the signature string —
    # `spec!(..., handler, "Type")` — strip it so the last remaining
    # string is the category again (the type is not a module).
    type_re = re.compile(r'(;\s*[A-Za-z_0-9]+),\s*"[A-Za-z][A-Za-z0-9<>{}:,_]*"\s*\)')
    categories = set()
    for line in content.splitlines():
        if "spec!(" not in line:
            continue
        code = line.split("//")[0]
        clean = layer_re.sub("", code)
        clean = type_re.sub(r"\1)", clean)
        strings = string_re.findall(clean)
        if strings:
            categories.add(strings[-1])
    return len(categories)


def count_typed_signatures() -> tuple[int, int, int]:
    """№467/№560: the general and PRECISE typed-signature counts over
    BUILTIN_REGISTRY (the same regex contract as
    scripts/ci/type_signature_share.py; precise = the bare scalar shapes
    String/Float/Bool/Unit)."""
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    spec_re = re.compile(r'spec!\("([a-z_0-9]+)"')
    typed_re = re.compile(
        r'spec!\("([a-z_0-9]+)",[^\n;]*;\s*[A-Za-z_0-9]+\s*,\s*"([A-Za-z][A-Za-z0-9<>{}:,_]*)"\s*\)'
    )
    precise_types = {"String", "Float", "Bool", "Unit"}
    total = spec_re.findall(content)
    typed = typed_re.findall(content)
    precise = sum(1 for _, t in typed if t in precise_types)
    return len(typed), len(total), precise


def count_svg_builtins() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    re_svg = re.compile(r'spec!\("(svg_|chart_|diagram_|color_palette|template_render|html_render)')
    return sum(1 for line in content.splitlines() if re_svg.search(line))


def count_parser_rules() -> int:
    content = (REPO / "src" / "grammar.pest").read_text(encoding="utf-8")
    re_rule = re.compile(r"^[a-zA-Z_][a-zA-Z_0-9]*\s*=")
    return sum(1 for line in content.splitlines() if re_rule.match(line))


def count_adrs() -> int:
    adr = REPO / "docs" / "adr"
    return sum(
        1
        for p in adr.iterdir()
        if p.suffix == ".md" and p.name != "README.md"
    )


def count_examples() -> int:
    examples = REPO / "examples"
    return sum(
        1
        for p in examples.iterdir()
        if p.is_file() and p.suffix == ".mlog"
    )


def file_kb(rel: str) -> int:
    return (REPO / rel).stat().st_size // 1024


def cargo_version() -> str:
    content = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([^"]+)"', content, re.M)
    return m.group(1)


def block() -> str:
    total = count_total_builtins()
    modules = count_builtin_modules()
    svg = count_svg_builtins()
    rules = count_parser_rules()
    adrs = count_adrs()
    examples = count_examples()
    version = cargo_version()
    changelog_kb = file_kb("CHANGELOG.md")
    reference_kb = file_kb("REFERENCE.md")
    typed, total_fns, precise = count_typed_signatures()
    typed_bp = (typed * 10000) // total_fns if total_fns else 0
    precise_bp = (precise * 10000) // total_fns if total_fns else 0
    return (
        f"| Metric | Value (generated — do not hand-edit) |\n"
        f"| ------ | ------------------------------------- |\n"
        f"| Version | {version} |\n"
        f"| Built-in Functions | {total} functions across {modules} modules |\n"
        f"| Typed Signatures | {typed}/{total_fns} ({typed_bp / 100:.2f}%) — precise {precise}/{total_fns} ({precise_bp / 100:.2f}%) (№467/№560) |\n"
        f"| SVG/Graphics | {svg} builtins, hand-rolled in pure Rust |\n"
        f"| Grammar | {rules} rules |\n"
        f"| Architecture Decisions | {adrs} ADRs |\n"
        f"| Example Programs | {examples} .mlog programs |\n"
        f"| Reference | REFERENCE.md (~{reference_kb} KB) — 100% registry coverage |\n"
        f"| Changelog | CHANGELOG.md (~{changelog_kb} KB) — every wave documented |\n"
    )


def plan_summary_block() -> str:
    """№566: the digest's CURRENT machine facts — the same sources as the
    README's Typed Signatures row (one SSOT; the two can never disagree
    because one process computes both)."""
    typed, total_fns, precise = count_typed_signatures()
    typed_bp = (typed * 10000) // total_fns if total_fns else 0
    precise_bp = (precise * 10000) // total_fns if total_fns else 0
    return (
        f"| Machine fact (generated — do not hand-edit) | Value |\n"
        f"| --- | --- |\n"
        f"| Typed-signature floor — the 0.28-gate line (ADR-0179) "
        f"| {typed_bp} bp — {typed}/{total_fns} = {typed_bp / 100:.2f}% "
        f"(precise {precise}/{total_fns} = {precise_bp / 100:.2f}%, №560) |\n"
        f"| BUILTIN_REGISTRY rows | {total_fns} |\n"
    )


def upsert_block(path: Path, begin: str, end: str, generated: str, label: str) -> tuple[bool, str]:
    """Replace (or verify, in --check mode) the block between the markers.
    Returns (ok, current_block) — ok=False means stale in check mode."""
    text = path.read_text(encoding="utf-8")
    if begin not in text or end not in text:
        print(f"ERROR: {path.name} is missing the {label} markers", file=sys.stderr)
        sys.exit(2)
    start = text.index(begin) + len(begin)
    end_pos = text.index(end, start)
    current = text[start:end_pos]
    fresh = "\n" + generated
    return current == fresh, current


def main() -> int:
    check = "--check" in sys.argv
    stale = []
    ok, current = upsert_block(README, BEGIN, END, block(), "metrics")
    if check:
        if ok:
            print("OK: README.md generated metrics block is up to date")
        else:
            stale.append("README.md")
    else:
        if not ok:
            text = README.read_text(encoding="utf-8")
            start = text.index(BEGIN) + len(BEGIN)
            end_pos = text.index(END, start)
            README.write_text(text[:start] + "\n" + block() + text[end_pos:], encoding="utf-8")
            print("README.md metrics block regenerated")

    if not PLAN_SUMMARY.exists():
        print("ERROR: docs/PLAN-SUMMARY.md is missing", file=sys.stderr)
        return 2
    ok, current = upsert_block(PLAN_SUMMARY, PS_BEGIN, PS_END, plan_summary_block(), "numbers")
    if check:
        if ok:
            print("OK: PLAN-SUMMARY.md generated numbers block is up to date")
        else:
            stale.append("docs/PLAN-SUMMARY.md")
    else:
        if not ok:
            text = PLAN_SUMMARY.read_text(encoding="utf-8")
            start = text.index(PS_BEGIN) + len(PS_BEGIN)
            end_pos = text.index(PS_END, start)
            PLAN_SUMMARY.write_text(
                text[:start] + "\n" + plan_summary_block() + text[end_pos:], encoding="utf-8"
            )
            print("docs/PLAN-SUMMARY.md numbers block regenerated")

    if check and stale:
        print(
            "STALE: generated blocks do not match the repository: "
            + ", ".join(stale)
            + " — run scripts/gen_metrics.py",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
