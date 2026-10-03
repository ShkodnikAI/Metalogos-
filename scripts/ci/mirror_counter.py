#!/usr/bin/env python3
"""№502 (gh#785; the audit 28.09 §3.3): the THIRD dedup metric — the
explicit mirror MENTIONS. №564 (gh#926; the audit 02.10 §7.2 W-2)
widens the scan from src/vm.rs to the WHOLE src/ tree: the mirror
CLASS is not vm.rs-specific — html_label.rs (№544.3) carries two
taint-walk mirrors ("the parity demands the EXACT mirror, quirks
included") that the vm.rs-only scan never saw; the same class as M-8
(№558) lives wherever a port happened.

WHY: the №462 metric (name literals) and the №484 metric (the *_tw/*_vm
logic pairs) are both blind to whole-CODE-BLOCK mirrors annotated with
a mirror mark, not name twins. The distillation drift was the direct
counterexample: the №489 background training landed in the interpreter
only, while vm.rs:2970 kept training synchronously — the backends had
diverged and no metric saw it.

THE FIXED REGEX (the §3.3 narrow patterns + the §7.2 audit-sketch
markers, case-insensitive, counted over ALL src/**/*.rs comments AND
doc-comments, one hit per line):
    mirror of the … | ported verbatim | exact mirror | TW-identical | same as the TW

The metric is deliberately COARSE (the audit's words: "it is crude, but
the project honestly documents such places"): it counts MENTIONS, not
AST nodes. A false positive is fixed by rewording the comment, never by
weakening the metric (the №502 boundary).

MOVEMENT: ONLY DOWN. №564 fixates the widened fact ONCE (7 — see
src_mirror_baseline.txt); every removal of a mirror moves the baseline
in the same PR. A count above the checked-in baseline fails the gate.

Usage:
  mirror_counter.py                 → prints the count report
  mirror_counter.py --list          → each match: file, line, pattern, text
  mirror_counter.py --gate BASELINE → exit 1 if the count > baseline
"""
import glob
import re
import sys

# The whole src/ tree (№564) — every .rs file, deterministic order.
SRC_GLOB = 'src/**/*.rs'
# The fixed pattern set — the §3.3 narrow pair + the §7.2 sketch
# markers (№502 continuity: TW-identical / same as the TW stay in the
# set, zero hits at the №564 adoption).
MIRROR_RE = re.compile(
    r'mirror of the |ported verbatim|exact mirror|TW-identical|same as the TW',
    re.IGNORECASE,
)


def matches():
    """(file, line_no, matched_text, whole_line) per match in src/**/*.rs."""
    out = []
    for path in sorted(glob.glob(SRC_GLOB, recursive=True)):
        with open(path, encoding='utf-8') as fh:
            for no, line in enumerate(fh, 1):
                m = MIRROR_RE.search(line)
                if m:
                    out.append((path, no, m.group(0), line.rstrip()))
    return out


def main():
    args = sys.argv[1:]
    found = matches()
    if '--list' in args:
        for path, no, hit, text in found:
            print('%s:%5d  %-16s  %s' % (path, no, hit, text.strip()[:120]))
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
        print('src/ mirror mentions: %d (threshold %d)' % (len(found), threshold))
        if len(found) > threshold:
            print(
                '::error::the src/ mirror count grew %d → %d (№502/№564: the '
                'metric moves ONLY down — unify the block with the source '
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
    print('src/ mirror mentions (the №502 fixed regex, the №564 whole-tree scan): %d' % len(found))
    for path, no, hit, text in found:
        print('%s:%5d  %-16s  %s' % (path, no, hit, text.strip()[:120]))


if __name__ == '__main__':
    main()
