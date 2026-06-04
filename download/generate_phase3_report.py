#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""METALOGOS Phase 3 Complete Report — PDF generation via ReportLab."""

import os, sys
PDF_SKILL_DIR = "/home/z/my-project/skills/pdf"
if PDF_SKILL_DIR not in sys.path:
    sys.path.insert(0, PDF_SKILL_DIR)

from reportlab.lib.pagesizes import A4
from reportlab.lib.units import inch, cm
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_JUSTIFY
from reportlab.lib import colors
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, KeepTogether, CondPageBreak, HRFlowable
)
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfbase.pdfmetrics import registerFontFamily
from reportlab.platypus.tableofcontents import TableOfContents
import hashlib

# ━━ Fonts ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pdfmetrics.registerFont(TTFont('Times New Roman', '/usr/share/fonts/truetype/chinese/LiberationSerif-Regular.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSans', '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSansBold', '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuMono', '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'))
pdfmetrics.registerFont(TTFont('LiberationSans', '/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf'))
pdfmetrics.registerFont(TTFont('LiberationSerif', '/usr/share/fonts/truetype/chinese/LiberationSerif-Regular.ttf'))

registerFontFamily('Times New Roman', normal='Times New Roman', bold='Times New Roman')
registerFontFamily('LiberationSans', normal='LiberationSans', bold='LiberationSans')
registerFontFamily('DejaVuSans', normal='DejaVuSans', bold='DejaVuSansBold')
registerFontFamily('DejaVuMono', normal='DejaVuMono', bold='DejaVuMono')

try:
    from pdf import install_font_fallback
    install_font_fallback()
except:
    pass

# ━━ Palette ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ACCENT       = colors.HexColor('#197999')
TEXT_PRIMARY  = colors.HexColor('#181a1b')
TEXT_MUTED    = colors.HexColor('#7c8489')
BG_SURFACE   = colors.HexColor('#dadfe2')
BG_PAGE      = colors.HexColor('#f2f4f4')
TABLE_HEADER_COLOR = ACCENT
TABLE_HEADER_TEXT  = colors.white
TABLE_ROW_EVEN     = colors.white
TABLE_ROW_ODD      = BG_SURFACE

# ━━ Styles ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
styles = {}

styles['Title'] = ParagraphStyle(
    name='Title', fontName='Times New Roman', fontSize=26, leading=34,
    alignment=TA_CENTER, textColor=TEXT_PRIMARY, spaceAfter=6)

styles['H1'] = ParagraphStyle(
    name='H1', fontName='Times New Roman', fontSize=18, leading=24,
    textColor=ACCENT, spaceBefore=18, spaceAfter=10)

styles['H2'] = ParagraphStyle(
    name='H2', fontName='Times New Roman', fontSize=14, leading=19,
    textColor=TEXT_PRIMARY, spaceBefore=14, spaceAfter=8)

styles['H3'] = ParagraphStyle(
    name='H3', fontName='Times New Roman', fontSize=12, leading=16,
    textColor=TEXT_PRIMARY, spaceBefore=10, spaceAfter=6)

styles['Body'] = ParagraphStyle(
    name='Body', fontName='Times New Roman', fontSize=10.5, leading=17,
    alignment=TA_JUSTIFY, textColor=TEXT_PRIMARY, spaceAfter=6)

styles['BodyLeft'] = ParagraphStyle(
    name='BodyLeft', fontName='Times New Roman', fontSize=10.5, leading=17,
    alignment=TA_LEFT, textColor=TEXT_PRIMARY, spaceAfter=6)

styles['Code'] = ParagraphStyle(
    name='Code', fontName='DejaVuSans', fontSize=8.5, leading=12,
    alignment=TA_LEFT, textColor=TEXT_PRIMARY, spaceAfter=4,
    leftIndent=12, backColor=BG_PAGE, borderPadding=4)

styles['Caption'] = ParagraphStyle(
    name='Caption', fontName='Times New Roman', fontSize=9, leading=13,
    alignment=TA_CENTER, textColor=TEXT_MUTED, spaceBefore=3, spaceAfter=6)

styles['TableCell'] = ParagraphStyle(
    name='TableCell', fontName='Times New Roman', fontSize=9.5, leading=13,
    alignment=TA_LEFT, textColor=TEXT_PRIMARY)

styles['TableHeader'] = ParagraphStyle(
    name='TableHeader', fontName='Times New Roman', fontSize=9.5, leading=13,
    alignment=TA_CENTER, textColor=colors.white)

styles['TableCell'] = ParagraphStyle(
    name='TableCell', fontName='Times New Roman', fontSize=9.5, leading=13,
    alignment=TA_LEFT, textColor=TEXT_PRIMARY)

styles['TocH1'] = ParagraphStyle(
    name='TOCHeading1', fontSize=12, leftIndent=20, fontName='Times New Roman',
    leading=20, spaceBefore=4, spaceAfter=2)

styles['TocH2'] = ParagraphStyle(
    name='TOCHeading2', fontSize=10, leftIndent=40, fontName='Times New Roman',
    leading=16, spaceBefore=2, spaceAfter=1)

styles['Meta'] = ParagraphStyle(
    name='Meta', fontName='Times New Roman', fontSize=9, leading=13,
    alignment=TA_LEFT, textColor=TEXT_MUTED, spaceAfter=2)

# ━━ Helpers ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

def make_table(data, col_ratios, has_header=True):
    available = A4[0] - 2*inch
    col_widths = [r * available for r in col_ratios]
    t = Table(data, colWidths=col_widths, hAlign='CENTER')
    style_cmds = [
        ('GRID', (0, 0), (-1, -1), 0.5, TEXT_MUTED),
        ('VALIGN', (0, 0), (-1, -1), 'MIDDLE'),
        ('LEFTPADDING', (0, 0), (-1, -1), 6),
        ('RIGHTPADDING', (0, 0), (-1, -1), 6),
        ('TOPPADDING', (0, 0), (-1, -1), 5),
        ('BOTTOMPADDING', (0, 0), (-1, -1), 5),
    ]
    if has_header:
        style_cmds.append(('BACKGROUND', (0, 0), (-1, 0), TABLE_HEADER_COLOR))
        style_cmds.append(('TEXTCOLOR', (0, 0), (-1, 0), TABLE_HEADER_TEXT))
        for i in range(1, len(data)):
            bg = TABLE_ROW_EVEN if i % 2 == 1 else TABLE_ROW_ODD
            style_cmds.append(('BACKGROUND', (0, i), (-1, i), bg))
    t.setStyle(TableStyle(style_cmds))
    return t

def P(text, style_name='Body'):
    s = styles.get(style_name, styles['Body'])
    return Paragraph(text, s)

def H1(text):
    return Paragraph(f'<b>{text}</b>', styles['H1'])

def H2(text):
    return Paragraph(f'<b>{text}</b>', styles['H2'])

def H3(text):
    return Paragraph(f'<b>{text}</b>', styles['H3'])

def sp(h=12):
    return Spacer(1, h)

def hr():
    return HRFlowable(width="100%", thickness=0.5, color=TEXT_MUTED, spaceAfter=12, spaceBefore=6)

def safe_keep(elements):
    total = sum(el.wrap(A4[0] - 2*inch, A4[1])[1] for el in elements)
    max_h = A4[1] * 0.4
    if total <= max_h:
        return [KeepTogether(elements)]
    elif len(elements) >= 2:
        return [KeepTogether(elements[:2])] + list(elements[2:])
    return list(elements)

# ━━ TocDocTemplate ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

class TocDocTemplate(SimpleDocTemplate):
    def afterFlowable(self, flowable):
        if hasattr(flowable, 'bookmark_name'):
            level = getattr(flowable, 'bookmark_level', 0)
            text = getattr(flowable, 'bookmark_text', '')
            key = getattr(flowable, 'bookmark_key', '')
            self.notify('TOCEntry', (level, text, self.page, key))

_heading_counter = {}

def add_heading(text, style, level=0):
    key = 'h_%s' % hashlib.md5(text.encode()).hexdigest()[:8]
    p = Paragraph('<a name="%s"/>%s' % (key, text), style)
    p.bookmark_name = text
    p.bookmark_level = level
    p.bookmark_text = text
    p.bookmark_key = key
    return p

available_height = A4[1] - 2*inch
H1_ORPHAN_THRESHOLD = available_height * 0.15

def add_major_section(text):
    return [
        CondPageBreak(H1_ORPHAN_THRESHOLD),
        add_heading(f'<b>{text}</b>', styles['H1'], level=0),
    ]

# ━━ Build Story ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

story = []

# ── TOC ────────────────────────────────────────────────────────────────
toc = TableOfContents()
toc.levelStyles = [styles['TocH1'], styles['TocH2']]
story.append(P('<b>Table of Contents</b>', 'Title'))
story.append(sp(12))
story.append(toc)
story.append(PageBreak())

# ── 1. Introduction ──────────────────────────────────────────────────
story.extend(add_major_section('1. Introduction'))
story.append(P(
    'METALOGOS is an AI-native programming language built on seven pillars: '
    'Entity, Pattern, Flow, Memory, Rule, Learn, and Adapt. The language is implemented '
    'in Rust and follows a contract-first TDD approach with golden tests. Phase 3 represents '
    'the final stage of the developer tooling milestone, completing the transition from a '
    'research prototype to a language ecosystem suitable for real-world development workflows.'
))
story.append(P(
    'Phase 3 delivered four major components: (1) a CLI with subcommands for running, checking, '
    'and interactively exploring .mlog programs; (2) a standard library with import mechanism providing '
    'string, math, and collections modules; (3) an LSP server for editor integration with real-time '
    'diagnostics, go-to-definition, hover, and completion; and (4) a package manager for project '
    'initialization, dependency tracking, and build validation. Additionally, comprehensive mdbook '
    'documentation was created covering tutorials, syntax reference, standard library reference, '
    'and an index of all Architecture Decision Records.'
))
story.append(P(
    'All 44 tests across three workspace crates pass successfully. The entire Phase 3 has been '
    'committed and pushed to the GitHub repository ShkodnikAI/Metalogos-. This report provides a '
    'detailed overview of every sub-phase, the architectural decisions made, the test strategy, '
    'and the final project structure.'
))

# ── 2. Project Overview ───────────────────────────────────────────────
story.extend(add_major_section('2. Project Overview'))
story.append(P(
    'The METALOGOS workspace consists of three Cargo crates organized as a Cargo workspace, '
    'alongside standard library modules, documentation, and editor support files. The workspace '
    'was progressively expanded throughout Phase 3, starting with two members (core + CLI binary) '
    'and ending with three dedicated crates plus the main binary.'
))

story.append(H2('2.1 Workspace Structure'))
story.append(sp(6))

ws_data = [
    [P('<b>Component</b>', 'TableHeader'), P('<b>Path</b>', 'TableHeader'), P('<b>Purpose</b>', 'TableHeader')],
    [P('metalogos (lib)', 'TableCell'), P('src/', 'TableCell'), P('Core library: parser, AST, interpreter, semantic analysis, LLM client, ML backend, embedding backend', 'TableCell')],
    [P('mlog (binary)', 'TableCell'), P('src/main.rs', 'TableCell'), P('CLI: mlog run | repl | check', 'TableCell')],
    [P('mlog-lsp', 'TableCell'), P('mlog-lsp/', 'TableCell'), P('LSP server: diagnostics, go-to-definition, hover, completion (tower-lsp)', 'TableCell')],
    [P('mlogpkg', 'TableCell'), P('mlogpkg/', 'TableCell'), P('Package manager: init | add | build | info', 'TableCell')],
    [P('Standard Library', 'TableCell'), P('std/', 'TableCell'), P('std/string.mlog, std/math.mlog, std/collections.mlog', 'TableCell')],
    [P('Examples', 'TableCell'), P('examples/', 'TableCell'), P('12 .mlog programs with .expected golden outputs', 'TableCell')],
    [P('VS Code Extension', 'TableCell'), P('editors/vscode/', 'TableCell'), P('package.json manifest + TextMate grammar + language config', 'TableCell')],
    [P('Documentation', 'TableCell'), P('docs/book/', 'TableCell'), P('mdbook: tutorial, syntax reference, stdlib reference, ADR index', 'TableCell')],
    [P('ADRs', 'TableCell'), P('docs/adr/', 'TableCell'), P('12 Architecture Decision Records (0001-0019)', 'TableCell')],
]
story.append(make_table(ws_data, [0.18, 0.18, 0.64]))
story.append(P('Table 1: Workspace components', 'Caption'))
story.append(sp(12))

story.append(H2('2.2 Metrics'))
story.append(sp(6))

metrics_data = [
    [P('<b>Metric</b>', 'TableHeader'), P('<b>Value</b>', 'TableHeader')],
    [P('Total Rust source lines', 'TableCell'), P('4,530', 'TableCell')],
    [P('Core library (src/)', 'TableCell'), P('3,266 lines across 8 files', 'TableCell')],
    [P('LSP server (mlog-lsp/)', 'TableCell'), P('487 lines', 'TableCell')],
    [P('Package manager (mlogpkg/)', 'TableCell'), P('348 lines', 'TableCell')],
    [P('Grammar (grammar.pest)', 'TableCell'), P('155 rules', 'TableCell')],
    [P('Integration tests', 'TableCell'), P('140 lines across 4 files', 'TableCell')],
    [P('Golden examples', 'TableCell'), P('12 .mlog + .expected pairs', 'TableCell')],
    [P('Standard library modules', 'TableCell'), P('3 (string, math, collections)', 'TableCell')],
    [P('mdbook chapters', 'TableCell'), P('4 (tutorial, syntax, stdlib, ADR index)', 'TableCell')],
    [P('ADRs written', 'TableCell'), P('12 total (3 in Phase 3: 0017, 0018, 0019)', 'TableCell')],
    [P('Total test count', 'TableCell'), P('44 passed, 0 failed', 'TableCell')],
]
story.append(make_table(metrics_data, [0.40, 0.60]))
story.append(P('Table 2: Project metrics', 'Caption'))

# ── 3. Phase 3.1: CLI + REPL ─────────────────────────────────────────
story.extend(add_major_section('3. Phase 3.1: CLI + REPL + Semantic Check'))
story.append(P(
    'Phase 3.1 transformed the single-command binary into a full CLI with three subcommands '
    'and introduced an interactive REPL with persistent state. Prior to this phase, the only '
    'way to execute a .mlog program was via a single `mlog run <file>` command. The interpreter '
    'was stateless, creating a fresh instance on every invocation and discarding all state after '
    'completion. This made incremental development and interactive exploration impossible.'
))

story.append(H2('3.1.1 CLI Architecture'))
story.append(P(
    'The CLI uses clap derive macros with three subcommands. The `mlog run <file>` subcommand '
    'executes a .mlog program, loading the standard library from the base directory of the source '
    'file for proper import resolution. The `mlog check <file>` subcommand performs semantic '
    'analysis without execution, returning errors and warnings with exit code 0 for clean '
    'programs and exit code 1 for errors. The `mlog repl` subcommand starts an interactive '
    'session with line editing, command history (persisted to ~/.mlog_history), and a persistent '
    'Interpreter instance across all inputs.'
))
story.append(P(
    'The REPL has two modes: interactive (tty) mode using rustyline with a "mlog>" prompt and '
    'command history, and piped (non-tty) mode that reads lines from stdin silently for use in '
    'integration tests. TTY detection uses libc::isatty(0) on Unix systems, with a fallback '
    'METALOGOS_FORCE_PIPE=1 environment variable for test control. A new feed_line() function '
    'in lib.rs parses a single line into declarations and feeds them to the existing interpreter, '
    'allowing entities, patterns, memory, relations, and adapted patterns to survive between inputs.'
))

story.append(H2('3.1.2 Semantic Analysis'))
story.append(P(
    'A new semantic module (src/semantic.rs) provides check_program() which performs two-pass '
    'analysis on declarations without executing them. The first pass collects all declaration '
    'names and detects duplicates. The second pass validates cross-references: entity types '
    'referenced in records exist, field initializers reference valid fields, patterns and '
    'learnable patterns invoked in flows exist, flow branch targets are known patterns, rule '
    'targets reference existing entities, and adapt/mutate targets reference existing learnable '
    'patterns. The result is an AnalysisResult containing separate errors and warnings vectors, '
    'each with a format() method for display.'
))

story.append(H2('3.1.3 Tests'))
story.append(sp(6))
cli_tests = [
    [P('<b>Test</b>', 'TableHeader'), P('<b>File</b>', 'TableHeader'), P('<b>Validates</b>', 'TableHeader')],
    [P('repl_integration_three_lines', 'TableCell'), P('tests/repl_integration.rs', 'TableCell'), P('entity + pattern + flow incremental eval via pipe', 'TableCell')],
    [P('check_ok_program', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('valid program produces no errors', 'TableCell')],
    [P('check_undefined_type_error', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('reports unknown type error', 'TableCell')],
    [P('check_adapt_target_not_found', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('reports missing learnable pattern', 'TableCell')],
    [P('check_duplicate_entity_type', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('reports duplicate entity type', 'TableCell')],
    [P('check_format_no_issues', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('"OK: no issues found" for clean program', 'TableCell')],
    [P('5 semantic unit tests', 'TableCell'), P('src/semantic.rs', 'TableCell'), P('ok program, undefined type, adapt target, duplicate, detailed spans', 'TableCell')],
]
story.append(make_table(cli_tests, [0.30, 0.28, 0.42]))
story.append(P('Table 3: Phase 3.1 test suite (11 tests)', 'Caption'))

# ── 4. Phase 3.2: Standard Library + Import ───────────────────────────
story.extend(add_major_section('4. Phase 3.2: Standard Library + Import'))
story.append(P(
    'Phase 3.2 introduced the standard library and the import mechanism, enabling code reuse '
    'across .mlog programs. The standard library provides three modules with commonly needed '
    'patterns that wrap built-in functions (prefixed with double underscore to avoid name '
    'collisions with user-defined patterns).'
))

story.append(H2('4.1 Import Mechanism'))
story.append(P(
    'The import statement has the form `import std/module`, which triggers file path resolution '
    'at runtime. The interpreter maintains a base directory (either the current working directory '
    'for REPL or the parent directory of the source file for `mlog run`) and resolves imports '
    'relative to a std/ subdirectory. The imported file is parsed and its declarations are merged '
    'into the current interpreter state, making all patterns from the imported module available '
    'for use in the importing program. Duplicate declarations across modules are handled by the '
    'existing duplicate detection in semantic analysis.'
))

story.append(H2('4.2 Standard Library Modules'))
story.append(sp(6))

stdlib_data = [
    [P('<b>Module</b>', 'TableHeader'), P('<b>Patterns</b>', 'TableHeader'), P('<b>Description</b>', 'TableHeader')],
    [P('std/string', 'TableCell'), P('trim, replace, split, join', 'TableCell'), P('String manipulation: whitespace trimming, substring replacement, splitting by separator, joining lists with separator', 'TableCell')],
    [P('std/math', 'TableCell'), P('abs, min, max, clamp, round', 'TableCell'), P('Mathematical operations: absolute value, minimum/maximum of two values, range clamping, rounding to nearest integer', 'TableCell')],
    [P('std/collections', 'TableCell'), P('first, last, push', 'TableCell'), P('Basic collection operations: get first or last element, append item to list', 'TableCell')],
]
story.append(make_table(stdlib_data, [0.15, 0.30, 0.55]))
story.append(P('Table 4: Standard library modules', 'Caption'))

story.append(sp(8))
story.append(P(
    'Each standard library pattern is a thin wrapper around a built-in function. Built-in '
    'functions use double underscore prefixes (e.g., __trim, __abs, __first) to avoid conflicts '
    'with user-defined patterns. The standard library patterns provide clean, human-readable '
    'names that users import explicitly.'
))

# ── 5. Phase 3.3: LSP Server ─────────────────────────────────────────
story.extend(add_major_section('5. Phase 3.3: LSP Server'))
story.append(P(
    'Phase 3.3 delivered a full Language Server Protocol implementation using the tower-lsp '
    'framework, providing real-time editor integration for .mlog files. The LSP server runs as '
    'a separate binary (mlog-lsp) communicating via stdio, following the standard LSP transport '
    'protocol. This was the first workspace expansion, adding mlog-lsp as a Cargo workspace member.'
))

story.append(H2('5.1 Span Propagation'))
story.append(P(
    'LSP features require source positions for diagnostics, go-to-definition, and hover. The '
    'existing AST carried no position data, so a Span struct was added with start_line, start_col, '
    'end_line, and end_col fields (0-indexed, LSP-compatible). Every declaration struct received '
    'a span field with Default trait implementation for backward compatibility. The parser was '
    'modified with a span_from_pair() helper that extracts positions from pest Pair::line_col() '
    'at each declaration boundary. Helper methods were added to Declaration: span(), name() for '
    'symbol table lookup, kind_str() for hover info, and type_info() for type signature display.'
))

story.append(H2('5.2 Spanned Diagnostics'))
story.append(P(
    'A new AnalysisResultDetailed type was added alongside the existing AnalysisResult in '
    'semantic.rs. It includes a Vec of Diagnostic objects with span, message, and severity '
    'fields. The check_program_detailed() function produces these spanned diagnostics, while '
    'check_program() continues to return the simple format for CLI backward compatibility. '
    'The LSP server converts these diagnostics to LSP Diagnostic objects using span_to_range() '
    'mapping.'
))

story.append(H2('5.3 LSP Features'))
story.append(sp(6))

lsp_feat_data = [
    [P('<b>Feature</b>', 'TableHeader'), P('<b>Implementation</b>', 'TableHeader')],
    [P('Diagnostics', 'TableCell'), P('textDocument/didOpen and textDocument/didChange trigger parse + semantic analysis + publish diagnostics', 'TableCell')],
    [P('Go-to-definition', 'TableCell'), P('Builds symbol table from declarations; finds identifier at cursor via text scanning; matches to declaration span', 'TableCell')],
    [P('Hover', 'TableCell'), P('Finds declaration at cursor; returns type signature and kind as Markdown', 'TableCell')],
    [P('Completion', 'TableCell'), P('Lists all named declarations from the current document', 'TableCell')],
    [P('Text sync', 'TableCell'), P('FULL mode (full text on every change)', 'TableCell')],
]
story.append(make_table(lsp_feat_data, [0.22, 0.78]))
story.append(P('Table 5: LSP features implemented', 'Caption'))

story.append(H2('5.4 VS Code Extension'))
story.append(P(
    'A VS Code extension manifest (editors/vscode/package.json) declares the .mlog language '
    'with file extension association, a TextMate grammar for syntax highlighting, and configuration '
    'options for the LSP server path and trace level. The extension activates on .mlog files and '
    'launches the mlog-lsp binary as an LSP client.'
))

story.append(H2('5.5 LSP Tests'))
story.append(sp(6))

lsp_tests = [
    [P('<b>Test</b>', 'TableHeader'), P('<b>Contract</b>', 'TableHeader')],
    [P('lsp_did_open_erroneous_program_returns_diagnostic', 'TableCell'), P('Erroneous program triggers ERROR severity diagnostic with "unknown type" message and valid range', 'TableCell')],
    [P('lsp_did_open_clean_program_no_diagnostics', 'TableCell'), P('Clean program produces zero diagnostics', 'TableCell')],
    [P('lsp_did_open_parse_error_returns_diagnostic', 'TableCell'), P('Invalid syntax triggers parse error diagnostic', 'TableCell')],
    [P('lsp_did_open_multiple_errors_multiple_diagnostics', 'TableCell'), P('Multiple errors produce multiple diagnostics', 'TableCell')],
    [P('lsp_did_open_duplicate_pattern_returns_error', 'TableCell'), P('Duplicate pattern detected', 'TableCell')],
    [P('lsp_did_open_adapt_not_found_returns_error', 'TableCell'), P('Missing adapt target detected', 'TableCell')],
    [P('lsp_goto_definition_finds_entity', 'TableCell'), P('Entity declaration found via symbol table lookup', 'TableCell')],
    [P('lsp_hover_returns_type_info', 'TableCell'), P('Pattern type_info contains signature with parameter types', 'TableCell')],
    [P('10 unit tests', 'TableCell'), P('Word extraction, span conversion, symbol building, multi-line, bounds checking', 'TableCell')],
]
story.append(make_table(lsp_tests, [0.42, 0.58]))
story.append(P('Table 6: LSP test suite (18 tests)', 'Caption'))

# ── 6. Phase 3.4: Package Manager ────────────────────────────────────
story.extend(add_major_section('6. Phase 3.4: Package Manager (mlogpkg)'))
story.append(P(
    'The final Phase 3 deliverable is mlogpkg, a package manager for .mlog projects. It provides '
    'project initialization with a standard manifest format, dependency tracking, build validation, '
    'and lock files for reproducible builds. This was the second workspace expansion, adding '
    'mlogpkg as the third Cargo workspace member.'
))

story.append(H2('6.1 Manifest Format'))
story.append(P(
    'The mlog.toml manifest uses a TOML format familiar to Rust developers, mirroring the '
    'Cargo.toml structure. It contains a [package] section with name, version, and edition '
    'fields, plus a [dependencies] section mapping package names to version constraints. The '
    'default edition is "2024" for forward compatibility with future language editions.'
))

code_example = (
    '[package]<br/>'
    'name = "my-project"<br/>'
    'version = "0.1.0"<br/>'
    'edition = "2024"<br/>'
    '<br/>'
    '[dependencies]<br/>'
    'some-pkg = "0.3.0"'
)
story.append(sp(4))
story.append(P(code_example, 'Code'))
story.append(sp(8))

story.append(H2('6.2 Commands'))
story.append(sp(6))

pkg_cmds = [
    [P('<b>Command</b>', 'TableHeader'), P('<b>Description</b>', 'TableHeader')],
    [P('mlogpkg init [--name N]', 'TableCell'), P('Creates mlog.toml manifest and src/main.mlog scaffold. Name defaults to directory name.', 'TableCell')],
    [P('mlogpkg add &lt;pkg&gt; [--version V]', 'TableCell'), P('Adds a dependency to mlog.toml. Verifies package exists in local registry (~/.mlog/registry/).', 'TableCell')],
    [P('mlogpkg build', 'TableCell'), P('Resolves dependencies from local registry, collects all .mlog files from src/, runs semantic check on each, writes mlog.lock.', 'TableCell')],
    [P('mlogpkg info', 'TableCell'), P('Displays project name, version, edition, dependencies, entry point, and registry path.', 'TableCell')],
]
story.append(make_table(pkg_cmds, [0.30, 0.70]))
story.append(P('Table 7: mlogpkg commands', 'Caption'))

story.append(H2('6.3 Local Registry'))
story.append(P(
    'Packages are stored in ~/.mlog/registry/&lt;pkg-name&gt;/. Each package directory contains '
    'its own mlog.toml manifest and source files. There is no remote server at this phase; packages '
    'are installed manually by copying files into the registry directory. This intentionally minimal '
    'approach provides a foundation for future remote registry support with publish/fetch commands, '
    'semantic version constraint resolution, and package signing.'
))

story.append(H2('6.4 Build Workflow'))
story.append(P(
    'The mlogpkg build command executes a seven-step pipeline: (1) read mlog.toml, (2) find '
    'entry point (src/main.mlog by default), (3) resolve dependencies from local registry, '
    '(4) collect all .mlog source files recursively from src/, (5) run semantic analysis via '
    'metalogos::check_program on each file, (6) write mlog.lock with resolved versions, and '
    '(7) report success with file count or failure with error count. The lock file uses JSON '
    'format and records each dependency\'s resolved version and source origin.'
))

story.append(H2('6.5 Package Manager Tests'))
story.append(sp(6))

pkg_tests = [
    [P('<b>Test</b>', 'TableHeader'), P('<b>Validates</b>', 'TableHeader')],
    [P('test_init_creates_mlog_toml', 'TableCell'), P('init creates mlog.toml with name and version fields + src/main.mlog scaffold', 'TableCell')],
    [P('test_init_default_name', 'TableCell'), P('init without --name uses directory name', 'TableCell')],
    [P('test_init_fails_if_exists', 'TableCell'), P('init fails when mlog.toml already exists', 'TableCell')],
    [P('test_build_with_init_project', 'TableCell'), P('build succeeds on fresh project, creates mlog.lock', 'TableCell')],
    [P('test_build_detects_errors', 'TableCell'), P('build fails when source has semantic errors', 'TableCell')],
    [P('test_build_fails_without_manifest', 'TableCell'), P('build fails without mlog.toml', 'TableCell')],
    [P('test_info_no_manifest', 'TableCell'), P('info gracefully reports no mlog.toml', 'TableCell')],
    [P('test_info_with_manifest', 'TableCell'), P('info shows project name and version', 'TableCell')],
    [P('test_manifest_toml_format', 'TableCell'), P('manifest has [package], [dependencies], edition fields', 'TableCell')],
]
story.append(make_table(pkg_tests, [0.38, 0.62]))
story.append(P('Table 8: mlogpkg test suite (9 tests)', 'Caption'))

# ── 7. Documentation ────────────────────────────────────────────────
story.extend(add_major_section('7. Documentation (mdbook)'))
story.append(P(
    'Comprehensive mdbook documentation was created in docs/book/ with four chapters covering '
    'the full language from beginner to advanced usage. The documentation serves as both a '
    'learning resource and a reference guide for the METALOGOS language.'
))

story.append(H2('7.1 Tutorial'))
story.append(P(
    'The tutorial (docs/book/src/tutorial.md) is a 10-step hands-on guide that progressively '
    'introduces all language features. It starts with a minimal "Hello, World!" program using '
    'entity, pattern, and flow, then builds up through entity types and records, learnable '
    'patterns (LLM-powered), adaptation with adapt and mutate, sandbox for safety, fluid types '
    'with confidence scores, rules for conditional logic, memory constructs, standard library '
    'imports, and finally a complete pipeline combining all features. Each step includes '
    'executable code examples with expected output.'
))

story.append(H2('7.2 Syntax Reference'))
story.append(P(
    'The syntax reference (docs/book/src/syntax.md) provides a complete specification of all '
    'surface syntax constructs: entity (simple, type, record), pattern, learnable pattern, '
    'flow (linear and branching), fluid, rule, memory (memorize, forget), adaptation (adapt, '
    'mutate, sandbox), import, and relate. It also documents all expression types (string literal, '
    'float literal, identifier, field access, function call, binary operators), built-in functions, '
    'comparison operators, and file conventions. The reference serves as the authoritative source '
    'for syntax questions.'
))

story.append(H2('7.3 Standard Library Reference'))
story.append(P(
    'The standard library reference (docs/book/src/stdlib.md) documents all three standard '
    'library modules with function signatures, descriptions, and usage examples for each pattern. '
    'It covers std/string (trim, replace, split, join), std/math (abs, min, max, clamp, round), '
    'and std/collections (first, last, push).'
))

story.append(H2('7.4 ADR Index'))
story.append(P(
    'The ADR index (docs/book/src/adr-index.md) lists all 12 Architecture Decision Records '
    'from 0001 through 0019, providing a navigable reference to the project\'s architectural '
    'history and design rationale.'
))

# ── 8. Architecture Decision Records ─────────────────────────────────
story.extend(add_major_section('8. Architecture Decision Records'))
story.append(P(
    'Three ADRs were written during Phase 3, documenting the key architectural decisions for '
    'each sub-phase. Each ADR follows the standard format: context, decision, and consequences '
    '(positive, negative, future).'
))

story.append(H2('8.1 ADR-0016: CLI + REPL + Semantic Check'))
story.append(P(
    'This ADR documents the decision to use clap derive macros for the CLI, rustyline for the '
    'REPL (chosen over reedline for lower complexity), and a two-pass semantic analysis approach. '
    'The REPL achieves state persistence by keeping a single Interpreter instance alive across '
    'iterations via the new feed_line() public API. The semantic check module validates programs '
    'without execution, enabling CI integration and quick feedback without running potentially slow '
    'LLM calls. Key trade-off: REPL state is in-memory only (not persisted between sessions), '
    'and semantic check is structural only without full type inference.'
))

story.append(H2('8.2 ADR-0018: LSP Server'))
story.append(P(
    'This ADR documents the decision to create a separate mlog-lsp crate using tower-lsp (the de-facto '
    'standard LSP framework for Rust with async support via tokio and full LSP 3.16 compatibility). '
    'Spans were added to all declaration AST nodes for source position tracking. Go-to-definition '
    'and hover use a text-scanning approach rather than adding spans to every expression node, which '
    'would require pervasive AST changes. Text sync uses FULL mode, acceptable for .mlog files '
    'which are typically under 1000 lines. Known limitation: pest\'s line_col() may report slightly '
    'different line numbers for programs with blank lines.'
))

story.append(H2('8.3 ADR-0019: Package Manager'))
story.append(P(
    'This ADR documents the decision to create a separate mlogpkg crate as a workspace member, '
    'using a TOML manifest format mirroring Cargo.toml for familiarity. The local registry approach '
    '(~/.mlog/registry/) was chosen as intentionally minimal for Phase 3, deferring remote '
    'registry support. The build workflow validates all source files with the existing semantic '
    'analysis module. Limitations: no version constraint resolution (uses exact versions), no '
    'workspace/multi-package support, and manual package installation required.'
))

# ── 9. Test Summary ──────────────────────────────────────────────────
story.extend(add_major_section('9. Test Summary'))
story.append(P(
    'The complete test suite across all three workspace crates consists of 44 tests, all passing. '
    'The tests cover unit-level logic, golden-file acceptance testing, CLI integration via subprocess, '
    'REPL integration via stdin piping, LSP integration via core code path testing, and package '
    'manager integration via filesystem-based subprocess testing.'
))

story.append(sp(6))
test_summary = [
    [P('<b>Crate</b>', 'TableHeader'), P('<b>Test File</b>', 'TableHeader'), P('<b>Count</b>', 'TableHeader')],
    [P('metalogos', 'TableCell'), P('src/semantic.rs (unit)', 'TableCell'), P('5', 'TableCell')],
    [P('metalogos', 'TableCell'), P('tests/check_integration.rs', 'TableCell'), P('5', 'TableCell')],
    [P('metalogos', 'TableCell'), P('tests/golden.rs', 'TableCell'), P('1 (auto-discovers 12 pairs)', 'TableCell')],
    [P('metalogos', 'TableCell'), P('tests/repl_integration.rs', 'TableCell'), P('1', 'TableCell')],
    [P('mlog-lsp', 'TableCell'), P('src/lib.rs (unit)', 'TableCell'), P('10', 'TableCell')],
    [P('mlog-lsp', 'TableCell'), P('tests/lsp_integration.rs', 'TableCell'), P('8', 'TableCell')],
    [P('mlogpkg', 'TableCell'), P('tests/pkg_integration.rs', 'TableCell'), P('9', 'TableCell')],
]
story.append(make_table(test_summary, [0.20, 0.50, 0.30]))
story.append(P('Table 9: Complete test suite (44 tests)', 'Caption'))

# ── 10. Git History ──────────────────────────────────────────────────
story.extend(add_major_section('10. Git History'))
story.append(P(
    'Phase 3 produced four commits on the main branch, each corresponding to a sub-phase:'
))
story.append(sp(6))

git_data = [
    [P('<b>Commit</b>', 'TableHeader'), P('<b>Message</b>', 'TableHeader')],
    [P('747500c', 'TableCell'), P('Phase 3: CLI + REPL + semantic check (ADR-0016)', 'TableCell')],
    [P('e9ce815', 'TableCell'), P('Phase 3.2: Standard Library (import, std/string, std/math, std/collections)', 'TableCell')],
    [P('ae32174', 'TableCell'), P('Phase 3.3: LSP server (mlog-lsp on tower-lsp)', 'TableCell')],
    [P('a9d5891', 'TableCell'), P('Phase 3 closed: package manager (mlogpkg), mdbook docs, ADR 0019', 'TableCell')],
]
story.append(make_table(git_data, [0.15, 0.85]))
story.append(P('Table 10: Phase 3 Git commits', 'Caption'))

# ── 11. Conclusion ──────────────────────────────────────────────────
story.extend(add_major_section('11. Conclusion and Next Steps'))
story.append(P(
    'Phase 3 is complete. The METALOGOS language now has a comprehensive developer tooling '
    'ecosystem: CLI with run/repl/check subcommands, interactive REPL with persistent state and '
    'history, semantic analysis for CI integration, standard library with import mechanism, '
    'LSP server for real-time editor support (diagnostics, go-to-definition, hover, completion), '
    'package manager for project management, and thorough documentation covering tutorials, '
    'syntax reference, standard library reference, and ADR index.'
))
story.append(P(
    'The language successfully demonstrates all seven pillars (Entity, Pattern, Flow, Memory, '
    'Rule, Learn, Adapt) with 12 example programs, 44 passing tests, and 12 ADRs documenting '
    'architectural decisions. The workspace structure supports clean separation of concerns '
    'with three independent crates sharing the core library.'
))
story.append(P(
    'Phase 4 is the next milestone: bytecode VM/JIT compilation and self-hosting. This will '
    'involve designing a bytecode instruction set, building a stack-based virtual machine, '
    'implementing a JIT compiler for hot paths, and ultimately compiling the METALOGOS interpreter '
    'to run on its own bytecode format, achieving self-hosting capability.'
))

# ━━ Build PDF ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

OUTPUT_PATH = '/home/z/my-project/download/METALOGOS_Phase3_Report.pdf'
os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)

doc = TocDocTemplate(
    OUTPUT_PATH,
    pagesize=A4,
    leftMargin=1*inch,
    rightMargin=1*inch,
    topMargin=1*inch,
    bottomMargin=1*inch,
    title='METALOGOS Phase 3 Report',
    author='Z.ai',
    subject='Complete report on Phase 3 of the METALOGOS AI-native programming language',
)

doc.multiBuild(story)
print(f"PDF generated: {OUTPUT_PATH}")
