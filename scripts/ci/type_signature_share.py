#!/usr/bin/env python3
"""№467 (gh#688): the typed-signature share metric over BUILTIN_REGISTRY.

Stage 0 of the type system (gate gh#680 decision 3-A): a builtin spec row
is TYPED when it carries the documented return-type string (the `spec!(
..., handler, "Type")` variant); its `Type::from_path` conversion happens
at the registry fill site. This script computes the share of typed rows
over the registry from the checked-in source and gates it against the
baseline floor:

  - the share may ONLY GROW (the owner's strengthening: the metric rises
    every release — a share BELOW the floor is a regression and fails CI);
  - the floor moves only up (a raise happens in the same PR that types
    more rows — the baseline header records the history).

Usage:
  type_signature_share.py                 → prints the share report
  type_signature_share.py --list          → the untyped builtin names
  type_signature_share.py --gate BASELINE → exit 1 if share < floor
"""
import re
import sys

ROOT = __file__.rsplit('/scripts/ci/', 1)[0]
REGISTRY = ROOT + '/src/builtins/registry.rs'

SPEC_RE = re.compile(r'spec!\("([a-z_0-9]+)"')
TYPED_RE = re.compile(
    r'spec!\("([a-z_0-9]+)",[^\n;]*;\s*[A-Za-z_0-9]+\s*,\s*"([A-Za-z][A-Za-z0-9<>]*)"\s*\)'
)


def rows():
    text = open(REGISTRY, encoding='utf-8').read()
    total = SPEC_RE.findall(text)
    typed = TYPED_RE.findall(text)
    typed_names = {name for name, _ in typed}
    return total, typed_names


def main():
    total, typed_names = rows()
    args = sys.argv[1:]
    if '--list' in args:
        for name in sorted(total):
            if name not in typed_names:
                print(name)
        return
    n_total = len(total)
    n_typed = len(typed_names)
    # basis points keep the comparison integer-exact
    share_bp = (n_typed * 10000) // n_total if n_total else 0
    print(f'typed signatures: {n_typed}/{n_total} ({share_bp / 100:.2f}%)')
    if '--gate' in args:
        baseline = args[args.index('--gate') + 1]
        floor = None
        for line in open(baseline, encoding='utf-8'):
            m = re.match(r'#\s*threshold_bp:\s*(\d+)', line)
            if m:
                floor = int(m.group(1))
                break
        if floor is None:
            print('::error::baseline fixture has no "# threshold_bp: N" line')
            sys.exit(2)
        if share_bp < floor:
            print(
                f'::error::the typed-signature share regressed: {share_bp} bp < '
                f'{floor} bp floor (№467: the metric rises every release; '
                f'a typed row lost its type or an untyped row was added — '
                f'type the new rows or restore the lost paths).'
            )
            sys.exit(1)
        if share_bp > floor:
            print(
                f'note: the share rose above the floor ({floor} bp) — №467 '
                f'should raise the baseline floor to {share_bp} bp in a '
                f'follow-up naryad.'
            )


if __name__ == '__main__':
    main()
