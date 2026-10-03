#!/usr/bin/env python3
"""№559 (issue #920; the audit 02.10 M-2 systematic + §6.3): the call-shape
extractor — the pairs "builtin-or-call name × argc" from .mlog sources.

The corpus contract with the language is PUBLIC even when the office
corpus is private: the pairs "name × argc" carry no code and no query
strings (the privacy boundary of №559). This script extracts those pairs
from .mlog files; the snapshot it produces (scripts/ci/office_call_shapes.txt)
is the corpus the blocking registry_vs_corpus.py check validates against
the builtin registry (a warning→error change that breaks a corpus call
fails the job BEFORE a user hits it — the respond_html lesson).

Usage:
    python3 scripts/ci/extract_call_shapes.py examples/ self-host/ \
        --out scripts/ci/office_call_shapes.txt
    rg -o '<pattern>' corpus/ | python3 scripts/ci/extract_call_shapes.py - \
        --out office_snapshot.txt        # the office-side pipeline (raw text in)

Output format (sorted, deduplicated with counts):
    name|argc|count

Extraction rules (honest, conservative):
  - `//` line comments are stripped FIRST (outside string literals);
  - string literals ("…" with backslash escapes) are skipped — a call
    look-alike inside a string is not a call;
  - a call is IDENT immediately followed by `(`; the argument list is the
    balanced-paren text; argc = top-level commas + 1 (empty parens = 0);
  - DECLARATION heads (pattern/learnable/tool-method/entity/rule/…) are
    skipped — their parens are parameter lists, not calls;
  - nested calls count individually (the outer AND the inner pair).
"""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path

# The declaration-head keywords whose `name(...)` parens are parameter
# lists, not calls (the grammar's declaration heads with parameters).
DECLARATION_HEADS = {
    "pattern",
    "learnable",
    "entity",
    "rule",
    "tool",
    "template",
    "flow",
    "sandbox",
    "hook",
    "test",
    "memory",
    "conversation",
    "fluid",
    "relate",
    "adapt",
    "mutate",
    "memorize",
    "forget",
    "schema",
    "db",
    "vision",
    "reflex",
    "reflex_seq",
    "reflex_gen",
    "origin",
    "profile",
    "type",
    "skill_index",
    "context_budget",
    "llm",
    "mlogserver",
}


def strip_comments(text: str) -> str:
    """Remove `//` line comments outside string literals (the grammar's
    COMMENT rule), preserving the newlines so positions stay stable."""
    out = []
    i = 0
    n = len(text)
    in_string = False
    while i < n:
        c = text[i]
        if in_string:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if c == '"':
                in_string = False
            i += 1
            continue
        if c == '"':
            in_string = True
            out.append(c)
            i += 1
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            # the rest of the line is a comment
            j = text.find("\n", i)
            if j == -1:
                break
            out.append("\n")
            i = j + 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def iter_calls(text: str):
    """Yield (name, argc) for every call look-alike in the stripped text.

    A string-aware balance scan: IDENT immediately followed by `(`, the
    argument list runs to the MATCHING close paren, argc = top-level
    commas + 1 (0 for empty parens). Nested calls yield their own pairs.
    """
    i = 0
    n = len(text)
    in_string = False
    while i < n:
        c = text[i]
        if in_string:
            if c == "\\":
                i += 2
                continue
            if c == '"':
                in_string = False
            i += 1
            continue
        if c == '"':
            in_string = True
            i += 1
            continue
        # an identifier char run ending right before `(`
        if c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            name = text[i:j]
            k = j
            while k < n and text[k] in " \t":
                k += 1
            if k < n and text[k] == "(" and name not in DECLARATION_HEADS:
                # the balanced scan of the argument list
                depth = 0
                brace = 0  # struct literals: {..} commas are NOT separators
                bracket = 0  # list literals: [..] commas are NOT separators
                argc = 0
                has_content = False
                m = k
                while m < n:
                    ch = text[m]
                    if ch == '"':
                        # skip the string literal inside the args
                        m += 1
                        while m < n and text[m] != '"':
                            if text[m] == "\\":
                                m += 1
                            m += 1
                        m += 1
                        continue
                    if ch == "(":
                        depth += 1
                    elif ch == ")":
                        depth -= 1
                        if depth == 0:
                            break
                    elif ch == "{":
                        brace += 1
                    elif ch == "}":
                        brace -= 1
                    elif ch == "[":
                        bracket += 1
                    elif ch == "]":
                        bracket -= 1
                    elif ch == "," and depth == 1 and brace == 0 and bracket == 0:
                        argc += 1
                        m += 1
                        continue
                    if not ch.isspace():
                        has_content = True
                    m += 1
                if depth == 0:
                    if has_content:
                        yield name, argc + 1
                    else:
                        yield name, 0
                    i = k  # continue INSIDE the parens (nested calls count)
                    i += 1
                    continue
            i = j
            continue
        i += 1


def collect_shapes(sources) -> Counter:
    """sources: an iterable of (label, text) — .mlog file bodies or raw
    piped lines. Returns the Counter of (name, argc) pairs."""
    pairs: Counter = Counter()
    for _label, text in sources:
        stripped = strip_comments(text)
        for name, argc in iter_calls(stripped):
            pairs[(name, argc)] += 1
    return pairs


def format_table(pairs: Counter) -> str:
    lines = []
    for (name, argc), count in sorted(pairs.items()):
        lines.append(f"{name}|{argc}|{count}")
    return "\n".join(lines) + ("\n" if lines else "")


def main(argv: list[str]) -> int:
    args = [a for a in argv[1:] if a != "--out"]
    out_path = None
    if "--out" in argv:
        idx = argv.index("--out")
        if idx + 1 >= len(argv):
            print("error: --out requires a path", file=sys.stderr)
            return 2
        out_path = argv[idx + 1]
        args = [a for a in argv[1:idx] + argv[idx + 2:] if a != "--out"]

    sources = []
    for a in args:
        p = Path(a)
        if a == "-":
            # the office-side pipeline: raw text lines from stdin
            import io

            sources.append(("<stdin>", sys.stdin.read()))
            continue
        if p.is_dir():
            for f in sorted(p.rglob("*.mlog")):
                sources.append((str(f), f.read_text(encoding="utf-8", errors="replace")))
        elif p.is_file():
            sources.append((str(p), p.read_text(encoding="utf-8", errors="replace")))
        else:
            print(f"error: no such path: {a}", file=sys.stderr)
            return 2
    if not sources:
        print("usage: extract_call_shapes.py <file.mlog|dir|-> ... [--out FILE]",
              file=sys.stderr)
        return 2
    table = format_table(collect_shapes(sources))
    if out_path:
        Path(out_path).write_text(table, encoding="utf-8")
        print(f"extract_call_shapes: {table.count(chr(10))} pairs -> {out_path}")
    else:
        sys.stdout.write(table)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
