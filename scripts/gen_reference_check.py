#!/usr/bin/env python3
"""Check REFERENCE.md coverage of builtins registered in src/builtins/registry.rs.

Наряд №167 Block 1: produce a list of builtin names declared in
registry.rs (via `spec!("name", ...)`) that are NOT documented in
REFERENCE.md (no `` `name(` `` occurrence).

Run from repo root:
    python3 scripts/gen_reference_check.py

Exit code 0 always — this is a reporting tool, not a CI gate. The CI
gate is the Rust test in tests/readme_consistency.rs that calls the
same logic and compares against a frozen baseline (see Block 2).
"""

import re
import sys
from pathlib import Path


def repo_root() -> Path:
    here = Path(__file__).resolve()
    return here.parent.parent


def collect_builtin_names() -> list[str]:
    """Read registry.rs and extract all builtin names from spec!() macros."""
    reg_path = repo_root() / "src" / "builtins" / "registry.rs"
    if not reg_path.is_file():
        print(f"ERROR: {reg_path} does not exist", file=sys.stderr)
        sys.exit(1)
    content = reg_path.read_text(encoding="utf-8")
    # spec!("name", 1, "category")  — first string literal is the name.
    # Use the same regex the onряд spec used.
    names = re.findall(r'spec!\("(\w+)"', content)
    # Deduplicate while preserving order (registry may have duplicates
    # under feature gates; we want unique names for documentation check).
    seen = set()
    unique = []
    for n in names:
        if n not in seen:
            seen.add(n)
            unique.append(n)
    return unique


def collect_documented_names(ref_content: str) -> set[str]:
    """Return the set of builtin names that appear as `` `name(` `` in REFERENCE.md.

    The backtick + open-paren pattern matches the signature syntax used
    throughout REFERENCE.md: e.g. `` `sha256(input: String) -> String` ``.
    Bare name mentions (e.g. in prose) don't count as documented — we
    require an actual signature with arguments.
    """
    # Match `name(` — backtick, identifier, open paren.
    pattern = re.compile(r"`(\w+)\(")
    return {m.group(1) for m in pattern.finditer(ref_content)}


def main() -> int:
    ref_path = repo_root() / "REFERENCE.md"
    if not ref_path.is_file():
        print(f"ERROR: {ref_path} does not exist", file=sys.stderr)
        sys.exit(1)
    ref_content = ref_path.read_text(encoding="utf-8")

    all_names = collect_builtin_names()
    documented = collect_documented_names(ref_content)
    missing = sorted(n for n in all_names if n not in documented)

    total = len(all_names)
    covered = total - len(missing)
    pct = (covered / total * 100) if total else 0.0

    print(f"REFERENCE.md coverage: {covered}/{total} ({pct:.1f}%)")
    print(f"Missing: {len(missing)}")
    if missing:
        print()
        for n in missing:
            print(f"  {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
