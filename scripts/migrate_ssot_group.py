#!/usr/bin/env python3
"""Migrate one group of spec! calls to include handlers, and remove
the corresponding funcs.insert lines from mod.rs.

Usage:
    python3 scripts/migrate_ssot_group.py core
    python3 scripts/migrate_ssot_group.py stateful
    python3 scripts/migrate_ssot_group.py svgpdf
"""

import re
import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


# Category → group mapping
CORE_CATS = {
    'string', 'std', 'math', 'stub', 'convert', 'io', 'list', 'json',
    'web', 'crypto', 'system', 'db', 'fluid', 'tokens', 'template', 'encoding',
    'llm', 'voice',
}
STATEFUL_CATS = {
    'memory', 'mtree', 'cron', 'graph', 'bot', 'time',
    'calendar', 'contacts', 'email', 'office', 'test',
}
# SVG+PDF: svg, chart, diagram, pdf (everything else)


def get_group_for_category(cat: str) -> str:
    if cat in CORE_CATS:
        return 'core'
    elif cat in STATEFUL_CATS:
        return 'stateful'
    else:
        return 'svgpdf'


def parse_handler_from_spec_line(line: str) -> str | None:
    """Extract the category from a spec! line to determine its group."""
    # Match the last string literal before the closing ) or ;
    # Pattern: spec!("name", ..., "category")
    m = re.search(r'spec!\(\s*"\w+".*?"(\w+)"\s*[);]', line)
    if m:
        return m.group(1)
    return None


def main():
    if len(sys.argv) < 2:
        print("Usage: migrate_ssot_group.py <core|stateful|svgpdf>", file=sys.stderr)
        return 1

    target_group = sys.argv[1]
    if target_group not in ('core', 'stateful', 'svgpdf'):
        print(f"Invalid group: {target_group}", file=sys.stderr)
        return 1

    root = repo_root()
    registry_path = root / 'src' / 'builtins' / 'registry.rs'
    mod_path = root / 'src' / 'builtins' / 'mod.rs'

    reg_lines = registry_path.read_text().split('\n')
    mod_content = mod_path.read_text()

    # Parse handlers from mod.rs
    pattern = re.compile(
        r'funcs\.insert\(\s*"(\w+)"\.to_string\(\)\s*,\s*(\w+)\s*(?:as\s+BuiltinFn)?\s*[,)]',
        re.DOTALL
    )
    handlers = {name: handler for name, handler in pattern.findall(mod_content)}

    # Process registry.rs: find spec! lines in the target group, add handler
    migrated_names = set()
    new_reg_lines = []
    i = 0
    while i < len(reg_lines):
        line = reg_lines[i]
        # Check if this line (or next, after #[cfg]) contains spec!
        spec_line = line
        cfg_prefix = ''
        if '#[cfg' in line.strip():
            cfg_prefix = line
            i += 1
            if i < len(reg_lines):
                spec_line = reg_lines[i]
            else:
                new_reg_lines.append(cfg_prefix)
                break

        m = re.search(r'spec!\(\s*"(\w+)"', spec_line)
        if m:
            name = m.group(1)
            # Extract category to determine group
            cat = parse_handler_from_spec_line(spec_line)
            if cat:
                group = get_group_for_category(cat)
            else:
                group = 'unknown'

            handler = handlers.get(name)

            if group == target_group and handler:
                # Check if handler is already in the spec! (already migrated)
                if f'; {handler}' in spec_line:
                    # Already migrated
                    pass
                elif ';' in spec_line and 'handler' in spec_line:
                    # Already has a handler (different pattern)
                    pass
                else:
                    # Add handler to spec! call
                    # The spec! call ends with `)` — replace with `; handler)`
                    # But we need to handle the case where it ends with `),` or just `)`
                    # Also handle `=> "layer")` pattern
                    new_spec_line = re.sub(
                        r'\)\s*$',
                        f'; {handler})',
                        spec_line.rstrip()
                    )
                    if new_spec_line != spec_line.rstrip():
                        spec_line = new_spec_line
                        migrated_names.add(name)
                        print(f"  MIGRATED: {name:30} → {handler}")
                    else:
                        # Try with trailing comma/whitespace
                        new_spec_line = re.sub(
                            r'\)\s*(,?)\s*$',
                            f'; {handler})\\1',
                            spec_line.rstrip()
                        )
                        if new_spec_line != spec_line.rstrip():
                            spec_line = new_spec_line
                            migrated_names.add(name)
                            print(f"  MIGRATED: {name:30} → {handler}")
                        else:
                            print(f"  SKIP (can't parse): {name} → {spec_line.strip()[:80]}")

            if cfg_prefix:
                new_reg_lines.append(cfg_prefix)
            new_reg_lines.append(spec_line)
        else:
            if cfg_prefix:
                new_reg_lines.append(cfg_prefix)
            new_reg_lines.append(line)
        i += 1

    registry_path.write_text('\n'.join(new_reg_lines))
    print(f"\nMigrated {len(migrated_names)} spec! calls in registry.rs")

    # Now remove corresponding funcs.insert from mod.rs
    # Build pattern to match funcs.insert calls for migrated names
    if migrated_names:
        # Remove funcs.insert calls for migrated names
        # Pattern: funcs.insert(\n  "name".to_string(),\n  handler as BuiltinFn,\n);
        mod_lines = mod_content.split('\n')
        new_mod_lines = []
        skip_until_close = False
        removed_count = 0
        for j, line in enumerate(mod_lines):
            if skip_until_close:
                if ');' in line or line.strip().endswith(')'):
                    skip_until_close = False
                continue

            # Check if this line starts a funcs.insert for a migrated name
            m = re.search(r'funcs\.insert\(\s*"(\w+)"\.to_string\(\)', line)
            if m and m.group(1) in migrated_names:
                # Check if it's a single-line or multi-line insert
                if line.strip().endswith(');') or (line.strip().endswith(')') and 'as BuiltinFn' in line):
                    # Single line — skip it
                    removed_count += 1
                    continue
                else:
                    # Multi-line — skip until we find the closing );
                    skip_until_close = True
                    removed_count += 1
                    continue

            # Also check for the pattern without .to_string() (legacy)
            m2 = re.search(r'funcs\.insert\(\s*"(\w+)"\s*,', line)
            if m2 and m2.group(1) in migrated_names:
                if line.strip().endswith(');') or (line.strip().endswith(')') and 'as BuiltinFn' in line):
                    removed_count += 1
                    continue
                else:
                    skip_until_close = True
                    removed_count += 1
                    continue

            new_mod_lines.append(line)

        mod_path.write_text('\n'.join(new_mod_lines))
        print(f"Removed {removed_count} funcs.insert calls from mod.rs")

    return 0


if __name__ == '__main__':
    sys.exit(main())
