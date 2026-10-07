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
    r'spec!\("([a-z_0-9]+)",[^\n;]*;\s*[A-Za-z_0-9]+\s*,\s*"([A-Za-z][A-Za-z0-9<>{}:,_]*)"\s*\)'
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

# №627 (gh#1110): the field-label stage-0 vocabulary — the byte-exact
# mirror of the Rust `Label::as_str` tokens and the №627 grammar checkers
# in `src/builtins/sig_types.rs` (`split_field_meta`,
# `path_is_parameterized`, `field_meta_well_formed`). The two locks
# cannot drift: this script counts the SOURCE rows, the in-tree test
# counts the COMPILED specs.
FIELD_LABELS = {'internal', 'private', 'untrusted'}
FIELD_NAME_RE = re.compile(r'[A-Za-z0-9_]+')


def split_field_meta(s):
    """Mirror of the Rust const fn split_field_meta (№627): (head, meta) —
    the section exists when the string ends with `}` and there is a `{`
    whose preceding byte is `>`; meta "" = absent."""
    if len(s) < 2 or not s.endswith('}'):
        return s, ''
    o = s.rfind('{')
    if o >= 1 and s[o - 1] == '>':
        return s[:o], s[o + 1:-1]
    return s, ''


def path_is_parameterized(s):
    """Mirror of the Rust const fn (№623 + the №627 brace extension):
    the ORIGINAL rule runs on the head verbatim."""
    head, _ = split_field_meta(s)
    if not head.endswith('>'):
        return False
    if not (head.startswith('List<') or head.startswith('Struct<')):
        return False
    return not (
        head.endswith('<private>')
        or head.endswith('<internal>')
        or head.endswith('<untrusted>')
    )


def field_meta_well_formed(meta):
    """Mirror of the Rust const fn field_meta_well_formed (№627):
    entry (',' entry)*, entry = name:label, the name [A-Za-z0-9_]+ (ASCII,
    non-empty), the label in the stage-0 vocabulary, NO spaces, at least
    one entry, no trailing comma."""
    if not meta:
        return False
    for entry in meta.split(','):
        if ':' not in entry:
            return False
        name, _, label = entry.partition(':')
        if not name or not FIELD_NAME_RE.fullmatch(name):
            return False
        if label not in FIELD_LABELS:
            return False
    return True


def field_meta_of(s):
    """Mirror of the Rust fill-site extractor: the section inner when the
    head is the Struct< spelling, else "" (shape-only — the well-formedness
    armor lives in from_path and the in-tree grammar test)."""
    head, meta = split_field_meta(s)
    if head.startswith('Struct<') and head.endswith('>'):
        return meta
    return ''


def rows():
    text = open(REGISTRY, encoding='utf-8').read()
    total = SPEC_RE.findall(text)
    typed = TYPED_RE.findall(text)
    typed_types = {name: t for name, t in typed}
    return total, typed_types


def compute():
    """(total, typed, typed_bp, precise, precise_bp, ls_total, param,
    param_bp, struct_total, fieldmeta, fieldmeta_bp) — integer-exact bp."""
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
    # №627 (gh#1110): the FOURTH metric — the field-label share among the
    # parameterized Struct rows. The denominator: every typed row whose
    # spelling is the parameterized Struct form (Struct<Name>, with or
    # without the field-meta section — the path_is_parameterized mirror);
    # the numerator: the rows carrying a well-formed non-empty field-meta
    # section (the №627 grammar mirror — the same checker the Rust fill
    # site + from_path enforce). Only-up; the №757 floor procedure.
    n_struct = 0
    n_fieldmeta = 0
    for t in typed_types.values():
        if not t.startswith('Struct<') or not path_is_parameterized(t):
            continue
        n_struct += 1
        meta = field_meta_of(t)
        if meta and field_meta_well_formed(meta):
            n_fieldmeta += 1
    typed_bp = (n_typed * 10000) // n_total if n_total else 0
    precise_bp = (n_precise * 10000) // n_total if n_total else 0
    param_bp = (n_param * 10000) // n_ls if n_ls else 0
    fieldmeta_bp = (n_fieldmeta * 10000) // n_struct if n_struct else 0
    return (
        n_total,
        n_typed,
        typed_bp,
        n_precise,
        precise_bp,
        n_ls,
        n_param,
        param_bp,
        n_struct,
        n_fieldmeta,
        fieldmeta_bp,
    )


def main():
    total, typed_types = rows()
    (
        n_total,
        n_typed,
        typed_bp,
        n_precise,
        precise_bp,
        n_ls,
        n_param,
        param_bp,
        n_struct,
        n_fieldmeta,
        fieldmeta_bp,
    ) = compute()
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
    fieldmeta = '--fieldmeta' in args
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
    elif fieldmeta:
        n, bp = n_fieldmeta, fieldmeta_bp
        print(
            f'field-label signatures: {n}/{n_struct} ({bp / 100:.2f}%) '
            f'— №627: the share among the parameterized Struct rows'
        )
        print(
            '  (the field-meta form: Struct<Name>{field:label,...}; the '
            'stage-0 enum erases the metadata — it lives in the registry '
            'side-table BuiltinSpec.field_meta, the well-formedness is '
            'fail-closed via from_path + the in-tree grammar test)'
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
        print(
            f'field-label signatures: {n_fieldmeta}/{n_struct} '
            f'({fieldmeta_bp / 100:.2f}%) — №627: the fourth share (among '
            f'the parameterized Struct rows)'
        )
    # basis points keep the comparison integer-exact
    share_bp = param_bp if parameterized else (precise_bp if precise else typed_bp)
    if fieldmeta:
        share_bp = fieldmeta_bp
    kind = (
        'fieldmeta'
        if fieldmeta
        else (
            'parameterized'
            if parameterized
            else ('precise' if precise else 'general')
        )
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
                'field-label share'
                if fieldmeta
                else (
                    'parameterized-share'
                    if parameterized
                    else ('precise typed-signature' if precise else 'typed-signature')
                )
            )
            print(
                f'::error::the {kind_name} share regressed: {share_bp} bp < '
                f'{floor} bp floor (№467/№560/№623/№627: the metric rises every '
                f'release; a typed row lost its type or an untyped row was '
                f'added — type the new rows or restore the lost paths).'
            )
            sys.exit(1)
        if share_bp > floor:
            print(
                f'note: the {kind} share rose above the floor ({floor} bp) — '
                f'№467/№560/№623/№627 should raise the baseline floor to {share_bp} '
                f'bp in a follow-up naryad.'
            )


if __name__ == '__main__':
    main()
