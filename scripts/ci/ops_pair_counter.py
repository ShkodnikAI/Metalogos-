#!/usr/bin/env python3
"""№484 (gh#732; the audit v0.26.1 §3.3, Medium): the SECOND dedup
metric — the `*_tw` / `*_vm` function PAIRS in the shared `*_ops.rs`
modules.

WHY: the №462 metric counts NAME LITERALS — a Goodhart surface (the
60→0 drop mostly moved spellings). This metric counts the LOGIC twins:
same base name, two per-backend bodies sitting side by side in one
shared module. The goal (the audit's direction): a suffix-free function
taking the state through a trait (`DbAccess`, №484), one implementation
for both backends. Where the backends' behavior is fundamentally
different the pair STAYS — each with a docstring justification of the
divergence (the №480 rule; per №476, never in the blocked domains).

Usage:
  ops_pair_counter.py                 → prints the count report
  ops_pair_counter.py --list          → the pairs, one per line (base file)
  ops_pair_counter.py --gate BASELINE → exit 1 if pairs > baseline threshold
"""
import glob
import os
import re
import sys

ROOT = __file__.rsplit('/scripts/ci/', 1)[0]
OPS_GLOB = ROOT + '/src/*_ops.rs'

FN_RE = re.compile(r'\bfn\s+([a-z_0-9]+)_(tw|vm)\s*\(')


def pairs():
    """(base, file) per *_tw/*_vm pair — a base with BOTH suffixes in the
    same file. Private helpers (`fn dispatch_tw(` + `fn dispatch_vm(`)
    count too: they are exactly the per-backend logic the metric targets."""
    found = {}
    for path in sorted(glob.glob(OPS_GLOB)):
        text = re.sub(r'//[^\n]*', '', open(path, encoding='utf-8').read())
        bases = {}
        for m in FN_RE.finditer(text):
            base, side = m.group(1), m.group(2)
            bases.setdefault(base, set()).add(side)
        rel = os.path.basename(path)
        for base, sides in sorted(bases.items()):
            if sides == {'tw', 'vm'}:
                found[(base, rel)] = True
    return sorted(found.keys())


def main():
    args = sys.argv[1:]
    p = pairs()
    if '--list' in args:
        for base, rel in p:
            print('%s (%s)' % (base, rel))
        return
    if '--gate' in args:
        baseline = args[args.index('--gate') + 1]
        threshold = None
        for line in open(baseline, encoding='utf-8'):
            m = re.match(r'#\s*threshold:\s*(\d+)', line)
            if m:
                threshold = int(m.group(1))
                break
        if threshold is None:
            print('::error::baseline fixture has no "# threshold: N" line')
            sys.exit(2)
        print('ops *_tw/*_vm pairs: %d (threshold %d)' % (len(p), threshold))
        if len(p) > threshold:
            print(
                '::error::the ops pair count grew %d → %d (№484: the '
                'metric moves ONLY down — unify the pair over the state '
                'trait or document the fundamental divergence in both '
                'docstrings).' % (threshold, len(p))
            )
            sys.exit(1)
        if len(p) < threshold:
            print(
                'note: the count dropped below the threshold — the '
                'baseline should move down to %d (the fact) in a '
                'follow-up.' % len(p)
            )
        return
    for base, rel in p:
        print('%s (%s)' % (base, rel))
    print('ops *_tw/*_vm pairs: %d' % len(p))


if __name__ == '__main__':
    main()
