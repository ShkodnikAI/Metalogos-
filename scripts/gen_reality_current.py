#!/usr/bin/env python3
"""gen_reality_current.py — the docs/REALITY.md generated stub (№810).

The C4-a T2 owner decision (23.10; issue #1566, the registry #1497):
docs/REALITY.md carries the CURRENT TRUTH only — what works / what does
not work NOW — and stays ≤30 KB; the §6.x wave-record history lives in
docs/reality/YYYY-MM.md (the verbatim archive). This generator renders
the generated stub that replaces the old history block:

  §6. The current machine truth (generated) — the verbatim outputs of
      the scripts/ci machines at the regeneration moment (the same
      collect_machines set the wave records carry — one SSOT);
  §7. The archive map (generated) — the monthly archive files with
      their section counts (the «closed is never silently dropped»
      reconciliation: the map is checked against the archive files —
      a section dropped from an archive fails this check).

Regeneration:
    python3 scripts/gen_reality_current.py            # regenerate
    python3 scripts/gen_reality_current.py --check    # CI gate (exit 1 if stale)

The stub sits between explicit markers inside docs/REALITY.md; the
human-authored §0-§5 are untouched.
"""
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
REALITY = REPO / 'docs' / 'REALITY.md'
ARCHIVE = REPO / 'docs' / 'reality'
WAVE_REC = REPO / 'scripts' / 'gen_wave_record.py'

BEGIN = '<!-- BEGIN GENERATED REALITY STUB (scripts/gen_reality_current.py — do not edit inside) -->'
END = '<!-- END GENERATED REALITY STUB -->'

SEC_RE = re.compile(r'^### (6\.\d+)\.', re.M)


def machine_truth() -> list:
    """The verbatim machine outputs — the same set the wave records
    carry (the collect_machines call in gen_wave_record; imported, not
    duplicated — one SSOT)."""
    sys.path.insert(0, str(REPO / 'scripts'))
    import gen_wave_record  # noqa: E402
    return list(gen_wave_record.collect_machines())


def archive_map() -> list:
    """(filename, section_count, first, last) per archive file, sorted."""
    out = []
    for p in sorted(ARCHIVE.glob('*.md')):
        text = p.read_text(encoding='utf-8')
        secs = SEC_RE.findall(text)
        if secs:
            out.append((p.name, len(secs), secs[0], secs[-1]))
    return out


def render() -> str:
    md = [BEGIN, '', '## 6. The current machine truth (generated — the verbatim machine outputs)', '', '```', *machine_truth(), '```', '', '## 7. The record archive (generated map — the history lives in docs/reality/)', '', 'The §6.x wave-record sections moved to the monthly archive by №810 (the C4-a T2 owner decision): docs/REALITY.md carries the current truth only; the records transfer VERBATIM — «закрытое не удаляется молча» preserved by the transfer. New records APPEND to the archive (the record-run month).', '', '| Archive | Sections | Range |', '|---|---|---|']
    total = 0
    for name, count, first, last in archive_map():
        total += count
        md.append(f'| [docs/reality/{name}](reality/{name}) | {count} | {first} … {last} |')
    md.append(f'| **total** | **{total}** | |')
    md.append('')
    md.append(END)
    return '\n'.join(md) + '\n'


def main():
    check_only = '--check' in sys.argv
    text = REALITY.read_text(encoding='utf-8')
    rendered = render().rstrip('\n') + '\n'  # canonical: exactly one newline after END
    if BEGIN in text and END in text:
        head = text[:text.index(BEGIN)]
        tail = text[text.index(END) + len(END):]
        if tail.startswith('\n'):
            tail = tail[1:]  # the render carries its own newline after END
        new_text = head + rendered + tail
    else:
        new_text = text.rstrip('\n') + '\n\n' + rendered
    if check_only:
        if new_text == text:
            print('OK: the REALITY stub is up to date')
            return 0
        print('STALE: the REALITY stub does not match the machines/archive — run scripts/gen_reality_current.py')
        return 1
    REALITY.write_text(new_text, encoding='utf-8')
    size = REALITY.stat().st_size
    print(f'REALITY.md stub regenerated ({size} bytes = {size / 1024:.1f} KB)')
    if size > 30 * 1024:
        print(f'::error::REALITY.md is {size} bytes — over the 30 KB current-truth ceiling (№810)')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
