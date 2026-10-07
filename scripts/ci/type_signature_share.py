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

# №560 (gh#921; the audit 02.10 M-4): the PRECISE type set — a typed row
# counts as precise when its signature names the exact shape, not just the
# family. The registry's stage-0 spec strings are bare (no `List<Param>` /
# `Struct(Name)` parameterized forms exist in the source — the honest fact
# the metric records), so precise == the bare scalar shapes
# {String, Float, Bool, Unit}; a bare `List`/`Struct` is typed-but-coarse
# (the audit: ≈49% of the typed rows). The set grows when the registry
# starts carrying parameterized signatures — by a registry PR, never by
# loosening this definition.
PRECISE_TYPES = {'String', 'Float', 'Bool', 'Unit'}


def rows():
    text = open(REGISTRY, encoding='utf-8').read()
    total = SPEC_RE.findall(text)
    typed = TYPED_RE.findall(text)
    typed_types = {name: t for name, t in typed}
    return total, typed_types


def compute():
    """(total, typed, typed_bp, precise, precise_bp, ls_total, param, param_bp)
    — integer-exact bp."""
    total, typed_types = rows()
    n_total = len(total)
    n_typed = len(typed_types)
    n_precise = sum(1 for t in typed_types.values() if t in PRECISE_TYPES)
    # №623 (gh#1086): the THIRD metric — the parameterized share among the
    # List/Struct rows. The denominator: every typed row whose base type
    # (before `<`) is List or Struct (the audit t94 base: 84 bare); the
    # numerator: the rows carrying the parameterized spelling
    # (`List<T>` / `Struct<Name>`). Only-up; the Z-2 verdict (gh#1077
    # superseded) stopped the precise-share movement by scalars — THIS
    # metric is where the typed movement of the 0.30 cycle lives.
    ls_types = {
        n: t
        for n, t in typed_types.items()
        if t.split('<', 1)[0] in ('List', 'Struct')
    }
    n_ls = len(ls_types)
    n_param = sum(1 for t in ls_types.values() if '<' in t)
    typed_bp = (n_typed * 10000) // n_total if n_total else 0
    precise_bp = (n_precise * 10000) // n_total if n_total else 0
    param_bp = (n_param * 10000) // n_ls if n_ls else 0
    return n_total, n_typed, typed_bp, n_precise, precise_bp, n_ls, n_param, param_bp


def main():
    total, typed_types = rows()
    n_total, n_typed, typed_bp, n_precise, precise_bp, n_ls, n_param, param_bp = (
        compute()
    )
    args = sys.argv[1:]
    if '--list' in args:
        for name in sorted(total):
            if name not in typed_types:
                print(name)
        return
    # №560: --precise switches the METRIC (the report and the --gate value)
    # from the general typed share to the precise one. The baseline file
    # decides which floor is checked (the precise baseline carries the
    # precise floor; the general baseline the general one).
    precise = '--precise' in args
    parameterized = '--parameterized' in args
    if precise:
        n, bp = n_precise, precise_bp
        print(f'precise signatures: {n}/{n_total} ({bp / 100:.2f}%)')
        print(
            '  (the precise set: String/Float/Bool/Unit; a bare List/Struct is '
            'typed-but-coarse \u2014 the audit 02.10 M-4: the general share is '
            'reachable by coarse types, the precise one is the honest 0.29 target)'
        )
    elif parameterized:
        n, bp = n_param, param_bp
        print(
            f'parameterized signatures: {n}/{n_ls} ({bp / 100:.2f}%) '
            f'— №623: the share among the List/Struct rows'
        )
        print(
            '  (the parameterized spelling: List<T> / Struct<Name>; the '
            'stage-0 enum erases the parameter — the metric is where the '
            'typed movement of the 0.30 cycle lives, the Z-2 verdict)'
        )
    else:
        print(f'typed signatures: {n_typed}/{n_total} ({typed_bp / 100:.2f}%)')
        print(
            f'precise signatures: {n_precise}/{n_total} '
            f'({precise_bp / 100:.2f}%) \u2014 №560: the two shares side by side'
        )
        print(
            f'parameterized signatures: {n_param}/{n_ls} '
            f'({param_bp / 100:.2f}%) — №623: the third share (among List/Struct)'
        )
    # basis points keep the comparison integer-exact
    share_bp = param_bp if parameterized else (precise_bp if precise else typed_bp)
    kind = (
        'parameterized'
        if parameterized
        else ('precise' if precise else 'general')
    )
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
            kind_name = (
                'parameterized-share'
                if parameterized
                else ('precise typed-signature' if precise else 'typed-signature')
            )
            print(
                f'::error::the {kind_name} share regressed: {share_bp} bp < '
                f'{floor} bp floor (№467/№560/№623: the metric rises every '
                f'release; a typed row lost its type or an untyped row was '
                f'added — type the new rows or restore the lost paths).'
            )
            sys.exit(1)
        if share_bp > floor:
            print(
                f'note: the {kind} share rose above the floor ({floor} bp) — '
                f'№467/№560/№623 should raise the baseline floor to {share_bp} '
                f'bp in a follow-up naryad.'
            )


if __name__ == '__main__':
    main()
