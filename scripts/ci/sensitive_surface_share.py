#!/usr/bin/env python3
"""№644 (issue #1131; the unified audit of 48301708, §3 Q-2): the
SENSITIVE-SURFACE coverage metric — the fixed-denominator answer to the
fourth-order Goodhart risk (every existing typed metric is honest, but
the CHOICE of what to type stays free and drifts to the easy rows).

The definition (the audit skeleton, verbatim):
  sources    = Role::Source ∧ default_label ≠ Public   (the FIXED
               denominator — the №316 classification; never diluted;
               the definition change is a separate naryad)
  covered    = typed ∧ (scalar ∨ parameterized) ∧ (¬Struct ∨ all fields
               labeled)

  - typed          — the registry row carries a return-type string
                     (the type_signature_share.py SPEC/TYPED read);
  - scalar         — the spelling is one of the precise bare shapes
                     {String, Float, Bool, Unit} (the №560 set);
  - parameterized  — the path_is_parameterized mirror (List<T> /
                     Struct<Name>, №623/№627);
  - Struct fields  — a parameterized Struct row counts covered only
                     with a well-formed non-empty field-meta section
                     (the №627 grammar mirror — the same checker the
                     Rust fill site + from_path enforce).

The parser mirrors are REUSED from type_signature_share.py — no second
parser (the №586/№606 lesson). The classification side reads
src/builtins_classification.rs (the SSOT static map, №316; authored by
gen_classification.py — this script never edits it).

Modes:
  (default)      the report: `sensitive-surface coverage: X/N (B bp)`
                 + the label/typed breakdown;
  --gate BASE    the floor check — exit 1 if coverage_bp < the
                 baseline's `# threshold_bp:` floor (the only-up
                 ratchet, №757/№613; the raise lands in the SAME PR
                 that covers more sources);
  --list         the inventory: every uncovered sensitive source with
                 the reason (untyped / typed-coarse / struct-no-meta)
                 + the first audit-order domain groups (§6.3: db →
                 mail → contacts/calendar → request_body) — the
                 next-package candidates.

Honest postures:
  - the blocking CI status is NOT introduced in №644 (the №535 rule —
    the blocking surface grows only explicitly): the CI wiring is the
    advisory report step beside type_signature_share; the --gate mode
    serves the local read and the future owner-decided wiring;
  - the 0.31 goal is a DRAFT line in gate_030_goals.txt — the fixation
    is the OWNER's §5 path (№525: a parameter without the owner's
    verdict does not exist; ADR-0186 §5);
  - the record line `record_sensitive_surface_bp` is a RECORD (the Z-2
    shape), never re-read as an absolute gate parameter.

Exit codes: 0 = green; 1 = the floor regressed (only-up violation);
2 = the inputs unparsable (fail-closed, loud).
"""

from __future__ import annotations

import re
import sys

import type_signature_share as tss

ROOT = tss.ROOT
CLASSIFICATION = ROOT + '/src/builtins_classification.rs'

ENTRY_RE = re.compile(
    r'BuiltClassEntry \{ name: "([a-z_0-9]+)", class: BuiltClass \{ '
    r'role: Role::([A-Za-z]+), default_label: Label::([A-Za-z]+)'
)

# The first audit-order packages (§6.3: db → mail → contacts/calendar →
# request_body → memory → recognition) — the report groups and the 0.31
# draft benchmark shape (the draft line in gate_030_goals.txt).
DOMAIN_GROUPS = (
    ('db (query*)', lambda n: n.startswith('query')),
    ('mail (imap*)', lambda n: n.startswith('imap')),
    (
        'contacts/calendar (card_*/cal_*)',
        lambda n: n.startswith('card_') or n.startswith('cal_'),
    ),
    ('request_body', lambda n: n == 'request_body'),
)


def classification():
    """name → (role, label) from the SSOT static map (№316)."""
    out = {}
    for m in ENTRY_RE.finditer(open(CLASSIFICATION, encoding='utf-8').read()):
        out[m.group(1)] = (m.group(2), m.group(3))
    return out


def compute():
    """(sensitive_names dict, covered set, uncovered dict with reasons).

    sensitive: name → label (Role::Source ∧ label ≠ Public).
    uncovered: name → reason in {untyped, typed-coarse, struct-no-meta}.
    """
    cls = classification()
    _, typed_types = tss.rows()
    sensitive = {
        n: label
        for n, (role, label) in cls.items()
        if role == 'Source' and label != 'Public'
    }
    covered = {}
    uncovered = {}
    for n in sensitive:
        t = typed_types.get(n)
        if t is None:
            uncovered[n] = 'untyped'
            continue
        scalar = t in tss.PRECISE_TYPES
        param = tss.path_is_parameterized(t)
        if not (scalar or param):
            uncovered[n] = 'typed-coarse'
            continue
        if t.startswith('Struct<') and param:
            meta = tss.field_meta_of(t)
            if not (meta and tss.field_meta_well_formed(meta)):
                uncovered[n] = 'struct-no-meta'
                continue
        covered[n] = t
    return sensitive, covered, uncovered


def share_bp(n_covered, n_sensitive):
    return (n_covered * 10000) // n_sensitive if n_sensitive else 0


def report():
    cls = classification()
    sensitive, covered, uncovered = compute()
    n = len(sensitive)
    c = len(covered)
    bp = share_bp(c, n)
    public_sources = sum(
        1 for role, label in cls.values() if role == 'Source' and label == 'Public'
    )
    print(f'sensitive-surface coverage: {c}/{n} ({bp} bp) — №644: the '
          f'fixed-denominator share among Role::Source ∧ label ≠ Public')
    by_label = {}
    for label in sensitive.values():
        by_label[label] = by_label.get(label, 0) + 1
    print('  the denominator by label: '
          + ', '.join(f'{k} {v}' for k, v in sorted(by_label.items()))
          + f' (the Public sources {public_sources} excluded by the definition)')
    by_reason = {}
    for reason in uncovered.values():
        by_reason[reason] = by_reason.get(reason, 0) + 1
    print('  the uncovered by reason: '
          + ', '.join(f'{k} {v}' for k, v in sorted(by_reason.items()))
          + ' — the covered = typed ∧ (scalar ∨ parameterized) ∧ (¬Struct ∨ fields labeled)')
    return bp


def inventory():
    sensitive, covered, uncovered = compute()
    print('uncovered sensitive sources (the next-package candidates):')
    for n in sorted(uncovered):
        t = tss.rows()[1].get(n)
        print(f'  {n}: {uncovered[n]}' + (f' ({t})' if t else ''))
    print('the first audit-order groups (§6.3 order):')
    for name, match in DOMAIN_GROUPS:
        ns = [n for n in sensitive if match(n)]
        c = sum(1 for n in ns if n in covered)
        print(f'  {name}: sources {len(ns)}, covered {c}')
    report()


def gate(baseline):
    sensitive, covered, _ = compute()
    bp = share_bp(len(covered), len(sensitive))
    floor = None
    for line in open(baseline, encoding='utf-8'):
        m = re.match(r'#\s*threshold_bp:\s*(\d+)', line)
        if m:
            floor = int(m.group(1))
            break
    if floor is None:
        print('::error::baseline fixture has no "# threshold_bp: N" line')
        return 2
    if bp < floor:
        print(
            f'::error::the sensitive-surface coverage regressed: {bp} bp < '
            f'{floor} bp floor (№644: the metric rises only — a covered '
            f'source lost its type/labels or the denominator moved; '
            f'restore the coverage or re-read the definition by a '
            f'separate naryad).'
        )
        return 1
    if bp > floor:
        print(
            f'note: the sensitive-surface coverage rose above the floor '
            f'({floor} bp) — №644 should raise the baseline floor to '
            f'{bp} bp in the same-PR move (the №757 procedure).'
        )
    else:
        print(
            f'sensitive-surface floor: OK — coverage {bp} bp at the '
            f'{floor} bp floor ({len(covered)}/{len(sensitive)} sources '
            f'covered; only-up, №757/№613).'
        )
    return 0


def main(argv):
    if '--gate' in argv:
        baseline = argv[argv.index('--gate') + 1]
        return gate(baseline)
    if '--list' in argv:
        inventory()
        return 0
    report()
    return 0


if __name__ == '__main__':
    # the --list mode is piped into `head` in CI (the type_signature
    # artifact-step shape) — a closed pipe must not read as a failure
    # (the SIG_DFL convention: die silently like the coreutils do).
    try:
        import signal
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    except (ImportError, AttributeError):
        pass
    sys.exit(main(sys.argv))
