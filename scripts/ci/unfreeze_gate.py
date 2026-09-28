#!/usr/bin/env python3
"""№482 (gh#730): the unfreeze-gate summary — the machine-checkable
one-page verdict over the FOUR ADR-0177 §4 criteria.

WHAT IT IS: a collector, not a decider. The right to lift the freeze
belongs to the OWNER ONLY (ADR-0177 §4); this script never lifts or
freezes anything — it reads the evidence the criteria machinery already
produces and writes one honest summary with an explicit verdict per
criterion. The 0.27.0 release gate reads this summary ("0.27.0 is NOT
published while any §4 criterion is red" — ADR-0177 §6); a RED verdict
anywhere fails the job (blocking) so the gate cannot be missed.

The four criteria (ADR-0177 §4) and their machinery:
  1. Types   — the typed-signature share (№467) may only grow:
               scripts/ci/type_signature_share.py --gate
  2. Dedup   — the TW/VM duplicate-name count (№462) at or below the
               threshold, the threshold moving only down:
               scripts/ci/count_duplicated_names.py --gate
  3. Debt    — the ignore/dead_code counters (№468) at or below:
               scripts/ci/debt_counters.py --gate
  4. Memory  — the office E2E dogfood (office#373, FO-056):
               (a) the in-repo twin tests/naryad_429_memory_office_path
                   runs LIVE in the job (the result is passed in via
                   --office-tests pass|fail);
               (b) the office-side facts are the checked-in record
                   scripts/ci/office_e2e_baseline.txt (the office repo
                   is private — no cross-repo token; fail-closed on a
                   missing record).

Usage:
  unfreeze_gate.py --office-tests pass   → the summary, exit 0 if all GREEN
  unfreeze_gate.py --office-tests fail   → the summary, exit 1 (criterion 4 RED)
  unfreeze_gate.py --baseline-dir DIR    → the baselines root (default scripts/ci)
  unfreeze_gate.py --out PATH            → the summary file (default unfreeze_summary.md)

The summary lands in the build artifact `unfreeze-summary` and in the
GitHub step summary when GITHUB_STEP_SUMMARY is set.
"""
import os
import re
import subprocess
import sys

ROOT = __file__.rsplit('/scripts/ci/', 1)[0]
HERE = os.path.dirname(os.path.abspath(__file__))

TYPES_SCRIPT = os.path.join(HERE, 'type_signature_share.py')
DUP_SCRIPT = os.path.join(HERE, 'count_duplicated_names.py')
# №502 (gh#785): the THIRD dedup metric — the vm.rs mirror mentions
# (the fixed narrow regex; movement only down). Folded into the Dedup
# criterion 4.2: a blind spot to vm.rs's whole-block mirrors is exactly
# how the distillation drift (audit 28.09 §3.3) went unnoticed.
MIRROR_SCRIPT = os.path.join(HERE, 'mirror_counter.py')
DEBT_SCRIPT = os.path.join(HERE, 'debt_counters.py')
ADR = 'docs/adr/0177-domain-freeze-until-027.md'


def run(cmd):
    p = subprocess.run([sys.executable] + cmd, capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def baseline_values(path, key):
    """All `# key: value` records in file order (the baselines append the
    history as blocks — the FIRST is the landing fact, the LAST is the
    latest recorded fact; the gate scripts enforce the FIRST)."""
    vals = []
    for line in open(path, encoding='utf-8'):
        m = re.match(r'#\s*%s:\s*(.+?)\s*$' % re.escape(key), line)
        if m:
            vals.append(m.group(1))
    return vals


def criterion_types(baseline_dir):
    rc, out = run([TYPES_SCRIPT, '--gate', os.path.join(baseline_dir, 'type_signature_baseline.txt')])
    m = re.search(r'typed signatures:\s*(\d+)/(\d+)\s*\(([\d.]+)%\)', out)
    share = m.group(0) if m else 'n/a'
    floors = baseline_values(os.path.join(baseline_dir, 'type_signature_baseline.txt'), 'threshold_bp')
    enforced = floors[0] if floors else '?'
    latest = floors[-1] if floors else '?'
    note = ''
    if len(floors) > 1 and floors[0] != floors[-1]:
        note = (
            'note: the baseline records a raised floor (%s bp) while the gate '
            'enforces the landing floor (%s bp) — the raise procedure (№467) '
            'should converge the two in a follow-up (the summary surfaces it, '
            'the criteria thresholds are not changed here).' % (latest, enforced)
        )
    detail = 'share %s; the enforced floor %s bp; the latest recorded floor %s bp' % (
        share, enforced, latest)
    return rc, detail, out, note


def criterion_dedup(baseline_dir):
    rc, out = run([DUP_SCRIPT, '--gate', os.path.join(baseline_dir, 'tw_vm_dup_names_baseline.txt')])
    m = re.search(r'duplicated builtin names:\s*(\d+)\s*\(threshold (\d+)\)', out)
    detail = ('count %s (threshold %s)' % (m.group(1), m.group(2))) if m else 'unparsed: %s' % out.strip()
    # №502: the third metric joins the criterion — the vm.rs mirror
    # mentions (the audit 28.09 §3.3: neither №462 nor №484 saw the
    # vm.rs mirrors, and the distillation drift lived exactly there).
    rc_m, out_m = run([MIRROR_SCRIPT, '--gate', os.path.join(baseline_dir, 'vm_mirror_baseline.txt')])
    mm = re.search(r'mirror mentions:\s*(\d+)\s*\(threshold (\d+)\)', out_m)
    detail_m = ('mirrors %s (threshold %s)' % (mm.group(1), mm.group(2))) if mm else 'unparsed: %s' % out_m.strip()
    rc = rc or rc_m
    detail += '; ' + detail_m
    out = out + '\n' + out_m
    return rc, detail, out, ''


def criterion_debt(baseline_dir):
    rc, out = run([DEBT_SCRIPT, '--gate', os.path.join(baseline_dir, 'debt_baseline.txt')])
    counters = re.findall(r'^([a-z_]+):\s*(\d+)\s*\(threshold (\d+)\)', out, re.M)
    detail = '; '.join('%s %s/%s' % (n, v, t) for n, v, t in counters) if counters else 'unparsed'
    return rc, detail, out, ''


def criterion_memory(baseline_dir, office_tests):
    notes = []
    rc = 0
    if office_tests == 'pass':
        twin = 'the in-repo twin (naryad_429_memory_office_path) GREEN on this commit'
    else:
        twin = 'the in-repo twin (naryad_429_memory_office_path) FAILED on this commit'
        rc = 1
    rec_path = os.path.join(baseline_dir, 'office_e2e_baseline.txt')
    if not os.path.isfile(rec_path):
        rec = 'the office evidence record is MISSING (fail-closed)'
        rc = 1
    else:
        rec = open(rec_path, encoding='utf-8').read()
        m = re.search(r'^verdict:\s*(.+)$', rec, re.M)
        if not m:
            rec += '\nthe record carries no verdict line (fail-closed)'
            rc = 1
        else:
            notes.append('the office record verdict: %s' % m.group(1).strip())
    notes.append(twin)
    notes.append(
        'the office repo is private — the live office CI status is not '
        'queryable from the Metalogos CI (no cross-repo token); the '
        'checked-in record is the machine-readable evidence, refreshed by '
        'the office-side naryads')
    return rc, ' | '.join(notes[:2]), rec, '\n'.join('- ' + n for n in notes[2:])


def main():
    args = sys.argv[1:]
    baseline_dir = args[args.index('--baseline-dir') + 1] if '--baseline-dir' in args else os.path.join(ROOT, 'scripts', 'ci')
    office_tests = args[args.index('--office-tests') + 1] if '--office-tests' in args else 'fail'
    out_path = args[args.index('--out') + 1] if '--out' in args else 'unfreeze_summary.md'
    if office_tests not in ('pass', 'fail'):
        print('--office-tests must be pass|fail')
        return 2

    c1 = criterion_types(baseline_dir)
    c2 = criterion_dedup(baseline_dir)
    c3 = criterion_debt(baseline_dir)
    c4 = criterion_memory(baseline_dir, office_tests)

    rows = []
    lines = []
    lines.append('# ADR-0177 §4 — the unfreeze-criteria summary (№482 machine check)')
    lines.append('')
    lines.append('The one-page machine verdict over the four unfreeze criteria. '
                 'The right to lift the freeze belongs to the OWNER ONLY '
                 '(ADR-0177 §4) — this summary collects evidence, it decides '
                 'nothing. Release gate (ADR-0177 §6): **0.27.0 is NOT '
                 'published while any §4 criterion is red.**')
    lines.append('')
    lines.append('| § | Criterion | Verdict | Evidence |')
    lines.append('|---|-----------|---------|----------|')
    for num, name, (rc, detail, raw, note) in (
        ('4.1', 'Types — the typed-signature share grows (№467)', c1),
        ('4.2', 'Dedup — the TW/VM duplicate names at/below the threshold (№462)', c2),
        ('4.3', 'Debt — the ignore/dead_code counters green (№468)', c3),
        ('4.4', 'Memory — the office E2E dogfood (office#373, FO-056)', c4),
    ):
        verdict = 'GREEN' if rc == 0 else 'RED'
        lines.append('| %s | %s | **%s** | %s |' % (num, name, verdict, detail))
        rows.append((num, verdict))
        if note:
            lines.append('')
            lines.append('> %s (%s): %s' % (num, name.split(' — ')[0], note))
    overall = 'GREEN' if all(v == 'GREEN' for _, v in rows) else 'RED'
    lines.append('')
    lines.append('**Overall: %s.**' % overall)
    if overall == 'GREEN':
        lines.append('The 0.27.0 release gate: **SATISFIED** (release-blocking '
                     'label — a RED anywhere in this summary blocks the release '
                     'read; the summary is the artifact `unfreeze-summary`).')
    else:
        lines.append('The 0.27.0 release gate: **BLOCKED** — the release is not '
                     'published while any §4 criterion is red (ADR-0177 §6).')
    lines.append('')
    lines.append('<details><summary>the raw gate outputs</summary>')
    lines.append('')
    for num, name, (rc, detail, raw, note) in (
        ('4.1', 'types', c1), ('4.2', 'dedup', c2), ('4.3', 'debt', c3), ('4.4', 'memory', c4),
    ):
        lines.append('**%s (%s), gate exit %d:**' % (num, name, rc))
        lines.append('')
        lines.append('```')
        lines.append(raw.strip()[:2000])
        lines.append('```')
        lines.append('')
    lines.append('</details>')
    lines.append('')
    lines.append('ADR: %s · generated by `scripts/ci/unfreeze_gate.py` (№482, '
                 'gh#730) — the summary is machine-checkable, the lift is not '
                 'a machine act.' % ADR)
    lines.append('')

    text = '\n'.join(lines)
    with open(out_path, 'w', encoding='utf-8') as f:
        f.write(text)
    print(text)
    step = os.environ.get('GITHUB_STEP_SUMMARY')
    if step:
        with open(step, 'a', encoding='utf-8') as f:
            f.write(text)

    if overall != 'GREEN':
        print('::error::the unfreeze summary is RED — the 0.27.0 release gate reads this verdict (ADR-0177 §6)')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
