#!/usr/bin/env python3
"""Generate docs/adr/README.md index from actual ADR files.

Наряд №166 Block 1: the ADR README index was stale (truncated at 0094
while 113 ADR files exist). This script regenerates the index from
the actual files in docs/adr/, extracting:
  - Number (from filename prefix `NNNN-`)
  - Title (from the `# ADR-NNNN: ...` H1 line in each file)
  - Status (from the `**Status:** ...` line in each file)

Run from repo root:
    python3 scripts/gen_adr_index.py

The script preserves the existing intro/numbering-rule/reserved-numbers
sections (everything before `## Index`) and regenerates only the table.
"""

import re
import sys
from pathlib import Path


def repo_root() -> Path:
    here = Path(__file__).resolve()
    return here.parent.parent


def extract_title(content: str, number: str) -> str:
    """Pull the title from `# ADR-NNNN: Title here` H1 line.

    Handles both `ADR-NNNN` (hyphen, newer) and `ADR NNNN` (space, older)
    formats — the codebase has both.
    """
    for line in content.splitlines():
        if not line.startswith("#"):
            continue
        h1 = line.lstrip("#").strip()
        # Match "ADR-NNNN: Title" or "ADR NNNN: Title" or with em-dash/regular dash
        patterns = [
            rf"^ADR[-\s]{number}\s*:\s*(.+)$",
            rf"^ADR[-\s]{number}\s*[—–-]\s*(.+)$",
        ]
        for pat in patterns:
            m = re.match(pat, h1)
            if m:
                return m.group(1).strip()
        # Fallback: strip "ADR[-\s]NNNN" prefix
        m = re.match(rf"^ADR[-\s]{number}\s*[:—–-]?\s*(.+)$", h1)
        if m:
            return m.group(1).strip()
    return None


def extract_status(content: str) -> str:
    """Pull the status from `**Status:** ...` line.

    Returns only the first sentence/clause (up to the first `—` em-dash
    or `;` semicolon, whichever comes first). Long statuses would break
    the table layout; the index shows the short form and the reader can
    follow the ADR link for the full status.

    Handles both formats:
      - `**Status:** Accepted` (plain line)
      - `> **Status:** Accepted` (blockquote, newer ADRs)
    """
    # Try single-line `**Status:** value` / `> **Status:** value` / `## Status: value` first.
    # Then fall back to H2 header `## Status` / `## Статус` followed by a paragraph.
    lines = content.splitlines()
    for i, line in enumerate(lines):
        stripped = line.strip()
        candidate = stripped.lstrip(">").strip()
        # Single-line formats:
        #   `**Status:** Accepted` (English)
        #   `**Status**: Accepted` (colon outside bold)
        #   `**Статус**: Accepted` (Russian)
        m = re.match(r"^\*\*(Status|Статус)\*?\*?\s*:\s*(.+)$", candidate)
        if m:
            value = m.group(2).strip().strip("*`").strip()
            for sep in [" — ", " — ", " — ", " ; ", "; "]:
                if sep in value:
                    value = value.split(sep)[0].strip()
                    break
            return value
        # H2 header format: `## Status` or `## Статус` followed by a paragraph
        m_h2 = re.match(r"^##\s+(Status|Статус)\s*:?\s*(.*)$", candidate)
        if m_h2:
            inline = m_h2.group(2).strip()
            if inline:
                # `## Status: Accepted` (inline)
                value = inline.strip("*`").strip()
                for sep in [" — ", " — ", " — ", " ; ", "; "]:
                    if sep in value:
                        value = value.split(sep)[0].strip()
                        break
                return value
            # `## Status\n\nAccepted.` (next non-empty line)
            for j in range(i + 1, min(i + 5, len(lines))):
                next_line = lines[j].strip()
                if next_line and not next_line.startswith("#"):
                    value = next_line.strip("*`").strip()
                    # Cut at first period for Russian «Принято. Phase 5.2.» style
                    if ". " in value:
                        value = value.split(". ")[0].strip()
                    for sep in [" — ", " — ", " — ", " ; ", "; "]:
                        if sep in value:
                            value = value.split(sep)[0].strip()
                            break
                    return value
    return "unknown"


def main() -> int:
    adr_dir = repo_root().joinpath("docs", "adr")
    readme_path = adr_dir.joinpath("README.md")

    if not adr_dir.is_dir():
        print(f"ERROR: {adr_dir} does not exist", file=sys.stderr)
        return 1

    entries = []
    for md_file in sorted(adr_dir.glob("*.md")):
        if md_file.name == "README.md":
            continue
        m = re.match(r"^(\d{4})-", md_file.name)
        if not m:
            print(f"WARN: {md_file.name} does not match NNNN- pattern, skipping", file=sys.stderr)
            continue
        number = m.group(1)
        content = md_file.read_text(encoding="utf-8")
        title = extract_title(content, number) or md_file.stem
        status = extract_status(content)
        entries.append((number, title, status))

    if not entries:
        print("ERROR: no ADR files found", file=sys.stderr)
        return 1

    existing = readme_path.read_text(encoding="utf-8") if readme_path.exists() else ""
    idx_marker = "## Index"
    if idx_marker in existing:
        header = existing.split(idx_marker)[0]
    else:
        header = (
            "# ADR Index — Architecture Decision Records\n\n"
            "## Numbering rule\n\n"
            "Before creating a new ADR, check for duplicates:\n\n"
            "```bash\nls docs/adr/ | sed 's/-.*//' | sort | uniq -d\n```\n\n"
            "Must be empty. If not empty, resolve collisions before proceeding.\n\n"
            f"Numbers are assigned sequentially. The current maximum is in `{entries[-1][0]}-*`.\n\n"
            f"## Index\n\n"
        )

    lines = [header.rstrip(), "", "## Index", "", "| # | Title | Status |", "|---|-------|--------|"]
    for number, title, status in entries:
        title_clean = title.replace("|", "\\|")
        status_clean = status.replace("|", "\\|")
        lines.append(f"| {number} | {title_clean} | {status_clean} |")

    new_content = "\n".join(lines) + "\n"
    readme_path.write_text(new_content, encoding="utf-8")
    print(f"Generated {readme_path.relative_to(repo_root())} with {len(entries)} ADR entries.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
