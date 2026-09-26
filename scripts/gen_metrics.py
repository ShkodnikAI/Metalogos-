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

BEGIN = "<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->"
END = "<!-- END GENERATED METRICS -->"


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
    type_re = re.compile(r'(;\s*[A-Za-z_0-9]+),\s*"[A-Za-z][A-Za-z0-9<>]*"\s*\)')
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
    return (
        f"| Metric | Value (generated — do not hand-edit) |\n"
        f"| ------ | ------------------------------------- |\n"
        f"| Version | {version} |\n"
        f"| Built-in Functions | {total} functions across {modules} modules |\n"
        f"| SVG/Graphics | {svg} builtins, hand-rolled in pure Rust |\n"
        f"| Grammar | {rules} rules |\n"
        f"| Architecture Decisions | {adrs} ADRs |\n"
        f"| Example Programs | {examples} .mlog programs |\n"
        f"| Reference | REFERENCE.md (~{reference_kb} KB) — 100% registry coverage |\n"
        f"| Changelog | CHANGELOG.md (~{changelog_kb} KB) — every wave documented |\n"
    )


def main() -> int:
    check = "--check" in sys.argv
    text = README.read_text(encoding="utf-8")
    if BEGIN not in text or END not in text:
        print("ERROR: README.md is missing the metrics markers", file=sys.stderr)
        return 2
    start = text.index(BEGIN) + len(BEGIN)
    end = text.index(END)
    current = text[start:end]
    generated = "\n" + block()
    if check:
        if current == generated:
            print("OK: generated metrics block is up to date")
            return 0
        print("STALE: generated metrics block does not match the repository — run scripts/gen_metrics.py", file=sys.stderr)
        return 1
    README.write_text(text[:start] + generated + text[end:], encoding="utf-8")
    print("README.md metrics block regenerated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
