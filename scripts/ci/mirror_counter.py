#!/usr/bin/env python3
"""№502 (gh#785; the audit 28.09 §3.3): the THIRD dedup metric — the
explicit TW-mirror MENTIONS in `src/vm.rs`.

WHY: the №462 metric (name literals) and the №484 metric (the *_tw/*_vm
logic pairs) are both blind to vm.rs's own duplication — the mirrors
there are whole CODE BLOCKS annotated with a mirror mark, not name
twins. The distillation drift was the direct counterexample: the №489
background training landed in the interpreter only, while vm.rs:2970
kept training synchronously — the backends had diverged and no metric
saw it.

THE FIXED REGEX (the audit's narrow pattern, case-insensitive, counted
over vm.rs comments AND doc-comments):
    mirror of the TW | ported verbatim | TW-identical | same as the TW

The metric is deliberately COARSE (the audit's words: "it is crude, but
the project honestly documents such places"): it counts MENTIONS, not
AST nodes. A false positive is fixed by rewording the comment, never by
weakening the metric (the №502 boundary).

MOVEMENT: ONLY DOWN. A count above the checked-in baseline fails the
gate; a drop asks for a baseline move in a follow-up.

Usage:
  mirror_counter.py                 → prints the count report
  mirror_counter.py --list          → each match: line, pattern, text
  mirror_counter.py --gate BASELINE → exit 1 if the count > baseline
"""
import re
import sys

VM = 'src/vm.rs'
# The fixed, narrow pattern — the audit's §3.3 inventory heuristic.
MIRROR_RE = re.compile(
    r'mirror of the TW|ported verbatim|TW-identical|same as the TW',
    re.IGNORECASE,
)


def matches():
    """(line_no, matched_text, whole_line) per match in src/vm.rs."""
    out = []
    with open(VM, encoding='utf-8') as fh:
        for no, line in enumerate(fh, 1):
            m = MIRROR_RE.search(line)
            if m:
                out.append((no, m.group(0), line.rstrip()))
    return out


def main():
    args = sys.argv[1:]
    found = matches()
    if '--list' in args:
        for no, hit, text in found:
            print('%5d  %-16s  %s' % (no, hit, text.strip()[:120]))
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
        print('vm.rs mirror mentions: %d (threshold %d)' % (len(found), threshold))
        if len(found) > threshold:
            print(
                '::error::the vm.rs mirror count grew %d → %d (№502: the '
                'metric moves ONLY down — unify the block with the TW '
                'side (the trait/state path) or leave a deliberate '
                'mirror mark with a divergence justification; never copy '
                'silently).' % (threshold, len(found))
            )
            sys.exit(1)
        if len(found) < threshold:
            print(
                'note: the count dropped below the threshold — the '
                'baseline should move down to %d (the fact) in a '
                'follow-up.' % len(found)
            )
        return
    print('vm.rs mirror mentions (the №502 fixed regex): %d' % len(found))
    for no, hit, text in found:
        print('%5d  %-16s  %s' % (no, hit, text.strip()[:120]))


if __name__ == '__main__':
    main()
