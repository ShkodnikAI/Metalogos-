#!/usr/bin/env python3
"""Migrate BUILTIN_REGISTRY spec! calls to include handler function pointers.

Наряд №170: move from two-list sync (registry.rs + funcs.insert in mod.rs)
to single source of truth (registry.rs with handler field).
"""

import re
import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def main():
    root = repo_root()
    registry_path = root / 'src' / 'builtins' / 'registry.rs'
    mod_path = root / 'src' / 'builtins' / 'mod.rs'

    reg_content = registry_path.read_text()
    mod_content = mod_path.read_text()

    # Parse handlers from mod.rs
    pattern = re.compile(
        r'funcs\.insert\(\s*"(\w+)"\.to_string\(\)\s*,\s*(\w+)\s*(?:as\s+BuiltinFn)?\s*[,)]',
        re.DOTALL
    )
    handlers = {name: handler for name, handler in pattern.findall(mod_content)}

    # Parse spec! entries from registry.rs
    specs = []
    for i, line in enumerate(reg_content.split('\n')):
        m = re.search(r'spec!\(\s*"(\w+)"', line)
        if m:
            specs.append({
                'line_idx': i,
                'name': m.group(1),
                'raw': line,
                'handler': handlers.get(m.group(1)),
            })

    stubs = [s for s in specs if s['handler'] is None]
    with_h = [s for s in specs if s['handler'] is not None]
    print(f"Total specs: {len(specs)}")
    print(f"With handler: {len(with_h)}")
    print(f"Stubs: {len(stubs)}")

    # Categorize by category field
    core_cats = {'string', 'std', 'math', 'stub', 'convert', 'io', 'list', 'json',
                 'web', 'crypto', 'system', 'db', 'fluid', 'tokens', 'template', 'encoding'}
    stateful_cats = {'memory', 'mtree', 'cron', 'graph', 'bot', 'time',
                     'calendar', 'contacts', 'email', 'office', 'test', 'voice'}

    for s in specs:
        # Extract category
        cat_match = re.search(r'spec!\(\s*"(?:\w+)".*?"(\w+)"\s*[);]', s['raw'])
        if cat_match:
            cat = cat_match.group(1)
            if cat in core_cats:
                s['group'] = 'core'
            elif cat in stateful_cats:
                s['group'] = 'stateful'
            else:
                s['group'] = 'svgpdf'
        else:
            s['group'] = 'unknown'

    from collections import Counter
    groups = Counter(s['group'] for s in specs)
    print(f"\nGroup counts: {dict(groups)}")

    # Print core group details
    core_specs = [s for s in specs if s['group'] == 'core']
    print(f"\n--- Core group ({len(core_specs)} specs) ---")
    for s in core_specs[:10]:
        print(f"  {s['name']:30} cat={s.get('cat','?')} handler={s['handler']}")

    # Print stubs by group
    for g in ['core', 'stateful', 'svgpdf']:
        g_stubs = [s for s in stubs if s['group'] == g]
        if g_stubs:
            print(f"\n{g} stubs: {[s['name'] for s in g_stubs]}")

    return 0


if __name__ == '__main__':
    sys.exit(main())
