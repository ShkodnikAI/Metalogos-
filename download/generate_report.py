# -*- coding: utf-8 -*-
"""METALOGOS Full Project Report — M1 through Phase 1 (Fluid Types)."""

import os, sys, hashlib
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import inch, mm
from reportlab.lib import colors
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_JUSTIFY
from reportlab.platypus import (
    Paragraph, Spacer, Table, TableStyle, PageBreak,
    KeepTogether, CondPageBreak, Image, HRFlowable,
)
from reportlab.platypus.tableofcontents import TableOfContents
from reportlab.platypus import SimpleDocTemplate
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfbase.pdfmetrics import registerFontFamily
from reportlab.pdfgen import canvas

# ━━ Color Palette ━━
ACCENT       = colors.HexColor('#5a30d7')
TEXT_PRIMARY  = colors.HexColor('#1b1c1e')
TEXT_MUTED    = colors.HexColor('#767b82')
BG_SURFACE   = colors.HexColor('#dce0e5')
BG_PAGE      = colors.HexColor('#eceef0')
TABLE_HEADER_COLOR = ACCENT
TABLE_HEADER_TEXT  = colors.white
TABLE_ROW_EVEN     = colors.white
TABLE_ROW_ODD      = BG_SURFACE

# ━━ Font Registration ━━
pdfmetrics.registerFont(TTFont('NotoSerifSC', '/usr/share/fonts/truetype/noto-serif-sc/NotoSerifSC-Regular.ttf'))
pdfmetrics.registerFont(TTFont('NotoSerifSCB', '/usr/share/fonts/truetype/noto-serif-sc/NotoSerifSC-Bold.ttf'))
pdfmetrics.registerFont(TTFont('SarasaMonoSC', '/usr/share/fonts/truetype/chinese/SarasaMonoSC-Regular.ttf'))
pdfmetrics.registerFont(TTFont('Carlito', '/usr/share/fonts/truetype/english/Carlito-Regular.ttf'))
pdfmetrics.registerFont(TTFont('CarlitoB', '/usr/share/fonts/truetype/english/Carlito-Bold.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSans', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'))

registerFontFamily('Carlito', normal='Carlito', bold='CarlitoB')
registerFontFamily('NotoSerifSC', normal='NotoSerifSC', bold='NotoSerifSCB')
registerFontFamily('DejaVuSans', normal='DejaVuSans', bold='DejaVuSans')

FONT_BODY = 'NotoSerifSC'
FONT_CODE = 'SarasaMonoSC'
FONT_TITLE = 'NotoSerifSC'

# ━━ Styles ━━
PAGE_W, PAGE_H = A4
MARGIN = 1.0 * inch
AVAILABLE_W = PAGE_W - 2 * MARGIN

H1_STYLE = ParagraphStyle(
    name='H1', fontName=FONT_TITLE, fontSize=20, leading=28,
    textColor=ACCENT, spaceBefore=18, spaceAfter=10,
    alignment=TA_LEFT,
)
H2_STYLE = ParagraphStyle(
    name='H2', fontName=FONT_TITLE, fontSize=15, leading=22,
    textColor=TEXT_PRIMARY, spaceBefore=14, spaceAfter=8,
    alignment=TA_LEFT,
)
H3_STYLE = ParagraphStyle(
    name='H3', fontName=FONT_TITLE, fontSize=12, leading=18,
    textColor=TEXT_PRIMARY, spaceBefore=10, spaceAfter=6,
    alignment=TA_LEFT,
)
BODY_STYLE = ParagraphStyle(
    name='Body', fontName=FONT_BODY, fontSize=10.5, leading=17,
    textColor=TEXT_PRIMARY, spaceBefore=2, spaceAfter=6,
    alignment=TA_LEFT,
)
CODE_STYLE = ParagraphStyle(
    name='Code', fontName=FONT_CODE, fontSize=9, leading=14,
    textColor=colors.HexColor('#2d2d2d'), backColor=BG_SURFACE,
    leftIndent=12, rightIndent=12, spaceBefore=6, spaceAfter=6,
    borderPadding=(6, 6, 6, 6),
)
META_STYLE = ParagraphStyle(
    name='Meta', fontName=FONT_BODY, fontSize=9, leading=14,
    textColor=TEXT_MUTED, spaceBefore=2, spaceAfter=4,
    alignment=TA_LEFT,
)
HEADER_CELL = ParagraphStyle(
    name='HeaderCell', fontName=FONT_TITLE, fontSize=10,
    textColor=colors.white, alignment=TA_CENTER,
)
CELL_STYLE = ParagraphStyle(
    name='Cell', fontName=FONT_BODY, fontSize=9.5, leading=14,
    textColor=TEXT_PRIMARY, alignment=TA_CENTER, wordWrap='CJK',
)
CELL_LEFT = ParagraphStyle(
    name='CellLeft', fontName=FONT_BODY, fontSize=9.5, leading=14,
    textColor=TEXT_PRIMARY, alignment=TA_LEFT, wordWrap='CJK',
)
TOC_H1 = ParagraphStyle(name='TOCH1', fontName=FONT_TITLE, fontSize=13, leftIndent=20, leading=22)
TOC_H2 = ParagraphStyle(name='TOCH2', fontName=FONT_BODY, fontSize=11, leftIndent=40, leading=18)

# ━━ TOC Template ━━
class TocDocTemplate(SimpleDocTemplate):
    def afterFlowable(self, flowable):
        if hasattr(flowable, 'bookmark_name'):
            level = getattr(flowable, 'bookmark_level', 0)
            text = getattr(flowable, 'bookmark_text', '')
            key = getattr(flowable, 'bookmark_key', '')
            self.notify('TOCEntry', (level, text, self.page, key))

def heading(text, style, level=0):
    key = 'h_%s' % hashlib.md5(text.encode()).hexdigest()[:8]
    p = Paragraph('<a name="%s"/><b>%s</b>' % (key, text), style)
    p.bookmark_name = text
    p.bookmark_level = level
    p.bookmark_text = text
    p.bookmark_key = key
    return p

def safe_keep(elements):
    total = sum(e.wrap(AVAILABLE_W, PAGE_H)[1] for e in elements)
    if total < PAGE_H * 0.4:
        return [KeepTogether(elements)]
    elif len(elements) >= 2:
        return [KeepTogether(elements[:2])] + list(elements[2:])
    return list(elements)

def code_block(text):
    return Paragraph(text.replace('\n', '<br/>'), CODE_STYLE)

def make_table(data, col_ratios=None):
    if col_ratios:
        cw = [r * AVAILABLE_W for r in col_ratios]
    else:
        n = len(data[0])
        cw = [AVAILABLE_W / n] * n
    t = Table(data, colWidths=cw, hAlign='CENTER')
    style_cmds = [
        ('BACKGROUND', (0, 0), (-1, 0), TABLE_HEADER_COLOR),
        ('TEXTCOLOR', (0, 0), (-1, 0), TABLE_HEADER_TEXT),
        ('GRID', (0, 0), (-1, -1), 0.5, TEXT_MUTED),
        ('VALIGN', (0, 0), (-1, -1), 'MIDDLE'),
        ('LEFTPADDING', (0, 0), (-1, -1), 8),
        ('RIGHTPADDING', (0, 0), (-1, -1), 8),
        ('TOPPADDING', (0, 0), (-1, -1), 6),
        ('BOTTOMPADDING', (0, 0), (-1, -1), 6),
    ]
    for i in range(1, len(data)):
        bg = TABLE_ROW_ODD if i % 2 == 0 else TABLE_ROW_EVEN
        style_cmds.append(('BACKGROUND', (0, i), (-1, i), bg))
    t.setStyle(TableStyle(style_cmds))
    return t

# ━━ Build Document ━━
OUTPUT = '/home/z/my-project/download/METALOGOS_Full_Report.pdf'
os.makedirs(os.path.dirname(OUTPUT), exist_ok=True)

doc = TocDocTemplate(
    OUTPUT, pagesize=A4,
    leftMargin=MARGIN, rightMargin=MARGIN,
    topMargin=MARGIN, bottomMargin=MARGIN,
    title='METALOGOS - Full Project Report',
    author='Z.ai',
    subject='METALOGOS Language Development Report M1-P1',
)

story = []

# ═══════════════════════════════════════════════
# COVER PAGE (inline, ReportLab canvas)
# ═══════════════════════════════════════════════
# We use a custom first-page template for the cover

class CoverPage(canvas.Canvas):
    """Custom cover drawn on first page."""
    pass

def draw_cover(canvas, doc):
    canvas.saveState()
    # Background
    canvas.setFillColor(colors.white)
    canvas.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)
    # Accent line at top
    canvas.setStrokeColor(ACCENT)
    canvas.setLineWidth(3)
    canvas.line(MARGIN, PAGE_H - MARGIN + 10, PAGE_W - MARGIN, PAGE_H - MARGIN + 10)
    # Title block
    canvas.setFillColor(TEXT_PRIMARY)
    canvas.setFont(FONT_TITLE, 36)
    canvas.drawString(MARGIN, PAGE_H - 200, 'METALOGOS')
    canvas.setFont(FONT_TITLE, 18)
    canvas.setFillColor(TEXT_MUTED)
    canvas.drawString(MARGIN, PAGE_H - 235, 'Full Project Report')
    canvas.setFont(FONT_BODY, 12)
    canvas.drawString(MARGIN, PAGE_H - 270, 'M1 through Phase 1: Fluid Types')
    # Accent rectangle
    canvas.setFillColor(ACCENT)
    canvas.rect(MARGIN, PAGE_H - 310, 60, 4, fill=1, stroke=0)
    # Metadata
    canvas.setFillColor(TEXT_MUTED)
    canvas.setFont(FONT_BODY, 10)
    canvas.drawString(MARGIN, PAGE_H - 370, 'Language: Rust (pest + tree-walking interpreter)')
    canvas.drawString(MARGIN, PAGE_H - 390, 'Repository: github.com/ShkodnikAI/Metalogos-')
    canvas.drawString(MARGIN, PAGE_H - 410, 'Date: 2026-05-31')
    canvas.drawString(MARGIN, PAGE_H - 430, 'Version: 0.1.0')
    # Bottom line
    canvas.setStrokeColor(TEXT_MUTED)
    canvas.setLineWidth(0.5)
    canvas.line(MARGIN, MARGIN - 10, PAGE_W - MARGIN, MARGIN - 10)
    canvas.setFont(FONT_BODY, 8)
    canvas.setFillColor(TEXT_MUTED)
    canvas.drawString(MARGIN, MARGIN - 25, 'Generated by Z.ai')
    canvas.restoreState()

def no_cover(canvas, doc):
    pass

# ═══════════════════════════════════════════════
# TOC
# ═══════════════════════════════════════════════
story.append(PageBreak())
story.append(Paragraph('<b>Table of Contents</b>', H1_STYLE))
story.append(Spacer(1, 12))
toc = TableOfContents()
toc.levelStyles = [TOC_H1, TOC_H2]
story.append(toc)
story.append(PageBreak())

# ═══════════════════════════════════════════════
# 1. INTRODUCTION
# ═══════════════════════════════════════════════
story.append(heading('1. Introduction', H1_STYLE, 0))
story.append(Paragraph(
    'METALOGOS is the first programming language designed by AI for AI. The name derives '
    'from Greek meta (beyond) + logos (reason, word, law) -- a language beyond human logic, '
    'where learning is a natural operation, data is primary and code is secondary, and '
    'self-modification is a feature, not a bug. Unlike conventional languages created by '
    'humans for humans, METALOGOS treats uncertainty as a first-class concept, memory as '
    'a semantic store with decay, and adaptation as a safe runtime operation.', BODY_STYLE))
story.append(Spacer(1, 6))
story.append(Paragraph(
    'The language is built around seven pillars: Entity (typed data with identity), '
    'Pattern (learnable transformations), Flow (declarative data pipelines), Memory '
    '(semantic storage with decay), Rule (probabilistic conditionals), Learn (LLM-backed '
    'pattern execution), and Adapt (safe self-modification via few-shot accumulation). '
    'Each pillar was implemented incrementally through a contract-first development '
    'methodology: for every feature, a contract program (.mlog + .expected) was written '
    'first, then the minimum runtime code to make it pass, followed by an Architecture '
    'Decision Record (ADR) documenting the design rationale.', BODY_STYLE))
story.append(Spacer(1, 6))
story.append(Paragraph(
    'This report covers all completed milestones: M1 (core loop), M2 (confidence and '
    'rules), M3 (LLM-backed learnable patterns), M4 (memory with decay), M5 (adapt via '
    'few-shot accumulation), and Phase 1 (Fluid Types with lazy collapse). Every milestone '
    'contributes exactly one new language capability while keeping all previous tests '
    'green. The total test suite consists of 6 golden tests that collectively validate the '
    'entire language specification implemented to date.', BODY_STYLE))

# Summary table
story.append(Spacer(1, 18))
summary_data = [
    [Paragraph('<b>Milestone</b>', HEADER_CELL),
     Paragraph('<b>Feature</b>', HEADER_CELL),
     Paragraph('<b>Contract Output</b>', HEADER_CELL),
     Paragraph('<b>Status</b>', HEADER_CELL)],
    [Paragraph('M1', CELL_STYLE), Paragraph('Entity + Pattern + Flow', CELL_LEFT),
     Paragraph('HELLO, METALOGOS!!', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
    [Paragraph('M2', CELL_STYLE), Paragraph('Rules + Branching + Struct Entities', CELL_LEFT),
     Paragraph('ESCALATE', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
    [Paragraph('M3', CELL_STYLE), Paragraph('LLM-backed Learnable Patterns', CELL_LEFT),
     Paragraph('Response: complaint', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
    [Paragraph('M4', CELL_STYLE), Paragraph('Memory: Memorize + Recall + Decay', CELL_LEFT),
     Paragraph('user likes spicy food', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
    [Paragraph('M5', CELL_STYLE), Paragraph('Adapt: Few-shot Self-modification', CELL_LEFT),
     Paragraph('Hello, world!', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
    [Paragraph('P1', CELL_STYLE), Paragraph('Fluid Types: Lazy Collapse', CELL_LEFT),
     Paragraph('84', CELL_STYLE), Paragraph('Green', CELL_STYLE)],
]
story.append(make_table(summary_data, [0.1, 0.40, 0.30, 0.20]))
story.append(Spacer(1, 6))
story.append(Paragraph('Table 1. Milestone summary and test status', META_STYLE))

# ═══════════════════════════════════════════════
# 2. M1 - CORE
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('2. M1 -- Core: Entity, Pattern, Flow', H1_STYLE, 0))
story.append(heading('2.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'M1 proved that the lexer-parser-AST-interpreter loop closes end-to-end. The contract '
    'program defines a simple entity, a pure pattern with string manipulation, and a linear '
    'flow pipeline. When executed, it prints "HELLO, METALOGOS!!" -- demonstrating that the '
    'fundamental execution model works. This milestone established the project structure, '
    'the pest PEG grammar, the AST representation, the tree-walking interpreter, and the '
    'golden test harness that all subsequent milestones rely on.', BODY_STYLE))

story.append(heading('2.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'entity greeting: String = "Hello, Metalogos!"<br/>'
    'pattern Shout(s: String) -&gt; String { return upper(s) + "!" }<br/>'
    'flow Main { input: String = greeting -&gt; Shout -&gt; output }'))

story.append(heading('2.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>Parser (pest 2.x):</b> Chosen over chumsky for its declarative grammar file that '
    'doubles as documentation. The PEG semantics are intuitive for a small language. '
    'Key lesson: inline string literals in pest do not produce pairs in into_inner(); '
    'named atomic rules do. All operators must be defined as named atomic rules for the '
    'AST converter to extract them reliably.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Interpreter (tree-walking):</b> A direct AST evaluator without bytecode or JIT. '
    'Entities stored in HashMap&lt;String, Value&gt;. Patterns compiled into CompiledPattern '
    'structs (params + body). Flows executed as linear pipelines: evaluate source, thread '
    'through pipeline steps by invoking patterns/builtins, return final value. Performance '
    'optimization explicitly deferred to Phase 4 per the Agent Charter.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Multi-character operator -&gt;:</b> Conflicts with subtraction operator "-". Solved '
    'with ARROW as an atomic rule that matches before "-", and negative lookahead in binop '
    'to prevent "-" from matching when followed by "&gt;".', BODY_STYLE))

story.append(heading('2.4 Files Modified', H2_STYLE, 1))
m1_files = [
    [Paragraph('<b>File</b>', HEADER_CELL), Paragraph('<b>Purpose</b>', HEADER_CELL)],
    [Paragraph('grammar.pest', CELL_LEFT), Paragraph('PEG grammar: entity, pattern, flow, expressions', CELL_LEFT)],
    [Paragraph('ast.rs', CELL_LEFT), Paragraph('AST types: EntityDecl, PatternDecl, FlowDecl, Expr, Statement', CELL_LEFT)],
    [Paragraph('parser.rs', CELL_LEFT), Paragraph('pest pairs to AST conversion', CELL_LEFT)],
    [Paragraph('interpreter.rs', CELL_LEFT), Paragraph('Tree-walking evaluator: env, patterns, flow execution', CELL_LEFT)],
    [Paragraph('builtins.rs', CELL_LEFT), Paragraph('Built-in functions: upper, lower, len, str, print', CELL_LEFT)],
    [Paragraph('lib.rs / main.rs', CELL_LEFT), Paragraph('Public API (run_program) + CLI (mlog run)', CELL_LEFT)],
    [Paragraph('tests/golden.rs', CELL_LEFT), Paragraph('Golden test runner: auto-discovers .mlog/.expected pairs', CELL_LEFT)],
]
story.append(make_table(m1_files, [0.25, 0.75]))
story.append(Spacer(1, 6))
story.append(Paragraph('Table 2. Files created in M1', META_STYLE))

# ═══════════════════════════════════════════════
# 3. M2 - RULES
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('3. M2 -- Confidence, Rules, Flow Branching', H1_STYLE, 0))
story.append(heading('3.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'M2 introduced three capabilities that make the language feel probabilistic: struct '
    'entities with typed fields and defaults, rules with conditions and priority ordering, '
    'and flow branching by confidence thresholds. The contract demonstrates a message '
    'triage system: a rule detects urgency keywords and elevates the urgency score, then '
    'the flow dispatches to different response patterns based on the urgency level.', BODY_STYLE))

story.append(heading('3.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'entity Message { text: String, urgency: Float = 0.0 }<br/>'
    'entity m: Message = { text: "...", urgency: 0.0 }<br/>'
    'rule If(m.text contains "...") then m.urgency = 0.9 with priority=10<br/>'
    'flow Main {<br/>'
    '  input: Message = m -&gt; Classify -&gt; output<br/>'
    '  Classify {<br/>'
    '    high (m.urgency &gt; 0.8)  -&gt; Escalate<br/>'
    '    medium (m.urgency &lt; 0.8) -&gt; Queue<br/>'
    '    low (m.urgency &lt; 0.4)  -&gt; Ignore<br/>'
    '  }<br/>'
    '}'))

story.append(heading('3.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>Rule engine (priority-ordered, first-wins):</b> Rules are sorted by priority '
    'descending; stable sort preserves declaration order for equal priority. First matching '
    'rule wins -- no chaining, no conflict detection. Grounded in production systems '
    '(Rete, CLIPS) but deliberately simplified to single-pass application. Forward-chaining '
    'to fixpoint and weighted inference (Markov Logic Networks style) are deferred to '
    'later milestones.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Flow architecture (pipeline + branch_def):</b> A fundamental restructuring where '
    'the pipeline is purely linear (sequence of step names from input to output) and '
    'branch definitions are separate named blocks that follow the pipeline. This separation '
    'provides clean PEG parsing, orthogonal extensibility, and clear semantics: the pipeline '
    'is a dispatch chain, branch definitions are pattern-matching tables.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Confidence propagation (honest):</b> M2 does not implement a full Fluid Type '
    'system. Float fields on struct entities serve as explicit confidence values, set by '
    'rules, read by branch conditions, used to route flow execution. This is documented '
    'as a deliberate simplification -- not probabilistic reasoning, but deterministic '
    'field assignment with numeric thresholds.', BODY_STYLE))

# ═══════════════════════════════════════════════
# 4. M3 - LEARNABLE
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('4. M3 -- Learnable Patterns and LLM Integration', H1_STYLE, 0))
story.append(heading('4.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'M3 introduced the first "wow" moment of METALOGOS: a learnable pattern is executed '
    'by sending its prompt to an LLM and returning the model response as a typed value. '
    'This transforms METALOGOS from a pure-functional pipeline language into an AI-native '
    'programming language where patterns can invoke external intelligence. The contract '
    'demonstrates text classification: a learnable pattern with a "complaint" prompt '
    'receives input text, classifies it, and passes the result to a response pattern.', BODY_STYLE))

story.append(heading('4.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'entity text: String = "..."<br/>'
    'learnable pattern Classify(msg: String) -&gt; String {<br/>'
    '  prompt: "complaint"<br/>'
    '}<br/>'
    'pattern Respond(category: String) -&gt; String {<br/>'
    '  return "Response: " + category<br/>'
    '}<br/>'
    'flow Main { input: String = text -&gt; Classify -&gt; Respond -&gt; output }'))

story.append(heading('4.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>LLM backend abstraction (trait-based):</b> The LlmBackend trait provides two '
    'implementations: MockLlm (test mode, returns prompt as deterministic response for '
    'golden tests) and RealLlm (production mode, HTTP POST via curl to configurable API '
    'endpoint supporting OpenAI and Ollama JSON formats). Mode selection via environment '
    'variable METALOGOS_MOCK_LLM (default: true for safety).', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Interpreter integration:</b> Learnable patterns stored in a separate HashMap with '
    'highest dispatch priority (checked before builtins and pure patterns). Arguments '
    'concatenated into input string, prompt + input sent to backend, response returned as '
    'Value::String. Confidence propagation and structured output parsing are deferred.', BODY_STYLE))

# ═══════════════════════════════════════════════
# 5. M4 - MEMORY
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('5. M4 -- Memory: Memorize, Recall, Forget, Decay', H1_STYLE, 0))
story.append(heading('5.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'M4 introduced persistent memory to METALOGOS programs. Until M4, all state was '
    'confined to entities and variables within a single execution. M4 adds the ability to '
    'store facts in memory (memorize), retrieve them by substring similarity with '
    'activation-based ranking (recall), and remove them by age (forget). The memory model '
    'is grounded in ACT-R base-level learning with exponential decay.', BODY_STYLE))

story.append(heading('5.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'memorize "user likes spicy food" with priority=0.9<br/>'
    'memorize "user hates cold soup" with priority=0.7<br/>'
    'pattern FindFood(query: String) -&gt; String {<br/>'
    '  return recall(query)<br/>'
    '}<br/>'
    'flow Main { input: String = "spicy" -&gt; FindFood -&gt; output }'))

story.append(heading('5.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>Memory model (ACT-R inspired):</b> Each entry stores value, priority, timestamp, '
    'and decay_rate. Activation = priority * exp(-decay_rate * age_in_days). Recall filters '
    'entries by substring match, ranks by activation, returns highest-activation entry above '
    'min_confidence threshold. No match returns empty string (soft-failure).', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Forget (time-based removal):</b> Removes entries matching query string with '
    'timestamp older than now - (days * 86400). Irreversible within execution. Default '
    'decay_rate = 0.01 (effectively no decay in millisecond-length golden tests). '
    'Vector similarity and embedding-based recall are deferred to Phase 2.', BODY_STYLE))

# ═══════════════════════════════════════════════
# 6. M5 - ADAPT
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('6. M5 -- Adapt: Few-Shot Self-Modification', H1_STYLE, 0))
story.append(heading('6.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'M5 introduces the first form of self-modification to METALOGOS: an adapt declaration '
    'adds few-shot examples to a learnable pattern at runtime. When the pattern is '
    'subsequently invoked, it checks the few-shot examples first (exact-match cache) and '
    'returns the cached output without calling the LLM. This is the foundation for '
    'in-context learning -- the simplest defensible form of program self-modification.', BODY_STYLE))

story.append(heading('6.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'learnable pattern Greet(name: String) -&gt; String {<br/>'
    '  prompt: "hello"<br/>'
    '}<br/>'
    'adapt Greet add_example("world", "Hello, world!")<br/>'
    'pattern RunGreet(input: String) -&gt; String {<br/>'
    '  return Greet(input)<br/>'
    '}<br/>'
    'flow Main { input: String = "world" -&gt; RunGreet -&gt; output }'))

story.append(heading('6.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>Safety invariant:</b> Adapt only modifies learnable patterns -- it cannot modify '
    'pure patterns, rules, entities, or flows. Enforced at the AST level (AdaptDecl only '
    'references a pattern name, interpreter only looks up learnable_patterns). This is the '
    'key safety invariant that distinguishes METALOGOS adapt from arbitrary self-modification.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>What M5 defers:</b> mutate pattern with rollback_if, sandbox execution with '
    'allow/forbid lists and timeout, and adapt with new_example feedback syntax. These '
    'require test-suite runners, accuracy metrics, and capability-based sandboxing -- '
    'documented in ADR 0005 as the explicit roadmap for post-M5 development.', BODY_STYLE))

# ═══════════════════════════════════════════════
# 7. PHASE 1 - FLUID TYPES
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('7. Phase 1 -- Fluid Types: Lazy Collapse', H1_STYLE, 0))
story.append(heading('7.1 Goal', H2_STYLE, 1))
story.append(Paragraph(
    'Phase 1 introduces the first type-level uncertainty to METALOGOS. A fluid value is a '
    'superposition of typed variants, each annotated with a confidence score. The '
    'superposition is materialized eagerly (all variant values computed at declaration), '
    'but the choice of which variant to use is deferred -- lazy collapse happens only at '
    'the point of use, when a typed context requires a specific type. This follows the '
    'metalogos-language-semantics skill guidance: "Fluid = tagged union of variants + '
    'confidence vector. Collapse is lazy, at the point of use."', BODY_STYLE))

story.append(heading('7.2 Contract Program', H2_STYLE, 1))
story.append(code_block(
    'fluid x = Float[42.0][0.9] or String["answer"][0.1]<br/>'
    'pattern Double(n: Float) -&gt; Float { return n + n }<br/>'
    'flow Main { input: Float = x -&gt; Double -&gt; output }'))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Execution trace:</b> x is declared as a fluid superposition with two variants: '
    'Float[42.0] at confidence 0.9 and String["answer"] at confidence 0.1. The flow '
    'passes x to Double, which requires a Float parameter. This triggers lazy collapse: '
    'the Float variant with confidence 0.9 is selected (above threshold 0.1), yielding '
    '42.0. Double computes 42.0 + 42.0 = 84. Output: "84".', BODY_STYLE))

story.append(heading('7.3 Architecture Decisions', H2_STYLE, 1))
story.append(Paragraph(
    '<b>Syntax:</b> fluid name = TypeName[value][confidence] or ... '
    'Each branch specifies type name, concrete value, and confidence (0.0..1.0). '
    'Multiple branches separated by "or".', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Runtime:</b> Value::Fluid(Vec&lt;FluidValueVariant&gt;) where each variant holds an '
    'already-evaluated concrete Value alongside its type name and confidence. Display impl '
    'shows highest-confidence variant value.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>Lazy collapse:</b> Triggered at pattern invocation via bind_and_collapse and '
    'maybe_collapse. For each (param, arg) pair, if arg is Fluid and param has a declared '
    'type, the interpreter finds the best matching variant by type + confidence. If '
    'confidence &gt;= COLLAPSE_THRESHOLD (0.1), returns concrete value; otherwise returns '
    'Unit (soft-failure). Non-Fluid values pass through unchanged -- zero overhead.', BODY_STYLE))
story.append(Spacer(1, 4))
story.append(Paragraph(
    '<b>What Phase 1 defers:</b> Confidence propagation (output confidence from input '
    'confidence), automatic type coercion between variants, probabilistic type inference, '
    'and Fluid-to-Fluid binary operations. These are documented in ADR 0006 as the growth '
    'path for subsequent Phase 1 iterations.', BODY_STYLE))

# ═══════════════════════════════════════════════
# 8. ARCHITECTURE OVERVIEW
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('8. Architecture Overview', H1_STYLE, 0))
story.append(Paragraph(
    'The current METALOGOS implementation is a single-crate Rust project with a '
    'tree-walking interpreter. The compilation pipeline is: .mlog source --&gt; pest '
    'parser --&gt; AST --&gt; interpreter --&gt; stdout. There is no bytecode, no JIT, '
    'no separate compilation phase. This is intentional for the MVP -- performance '
    'optimization is explicitly deferred to Phase 4 per the Agent Builder Charter.', BODY_STYLE))

arch_data = [
    [Paragraph('<b>Component</b>', HEADER_CELL),
     Paragraph('<b>Module</b>', HEADER_CELL),
     Paragraph('<b>Responsibility</b>', HEADER_CELL)],
    [Paragraph('Grammar', CELL_LEFT), Paragraph('grammar.pest', CELL_LEFT),
     Paragraph('PEG grammar for all language constructs', CELL_LEFT)],
    [Paragraph('AST', CELL_LEFT), Paragraph('ast.rs', CELL_LEFT),
     Paragraph('Declaration, Expr, Statement, Branch types', CELL_LEFT)],
    [Paragraph('Parser', CELL_LEFT), Paragraph('parser.rs', CELL_LEFT),
     Paragraph('pest pairs to AST conversion', CELL_LEFT)],
    [Paragraph('Interpreter', CELL_LEFT), Paragraph('interpreter.rs', CELL_LEFT),
     Paragraph('Tree-walking eval, collapse, flow engine', CELL_LEFT)],
    [Paragraph('LLM Client', CELL_LEFT), Paragraph('llm.rs', CELL_LEFT),
     Paragraph('LlmBackend trait, MockLlm, RealLlm', CELL_LEFT)],
    [Paragraph('Built-ins', CELL_LEFT), Paragraph('builtins.rs', CELL_LEFT),
     Paragraph('upper, lower, len, str, print, contains, float', CELL_LEFT)],
    [Paragraph('Public API', CELL_LEFT), Paragraph('lib.rs', CELL_LEFT),
     Paragraph('run_program() entry point', CELL_LEFT)],
    [Paragraph('CLI', CELL_LEFT), Paragraph('main.rs', CELL_LEFT),
     Paragraph('mlog run &lt;file.mlog&gt;', CELL_LEFT)],
    [Paragraph('Tests', CELL_LEFT), Paragraph('tests/golden.rs', CELL_LEFT),
     Paragraph('Auto-discovers .mlog/.expected pairs', CELL_LEFT)],
]
story.append(Spacer(1, 18))
story.append(make_table(arch_data, [0.15, 0.20, 0.65]))
story.append(Spacer(1, 6))
story.append(Paragraph('Table 3. Current module architecture', META_STYLE))

# ═══════════════════════════════════════════════
# 9. ADR INDEX
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('9. Architecture Decision Records', H1_STYLE, 0))
story.append(Paragraph(
    'Every milestone produces an ADR documenting the design rationale, prior art, '
    'decisions made, alternatives considered, and consequences. ADRs are the authoritative '
    'source for understanding why the language works the way it does.', BODY_STYLE))

adr_data = [
    [Paragraph('<b>ADR</b>', HEADER_CELL),
     Paragraph('<b>Title</b>', HEADER_CELL),
     Paragraph('<b>Milestone</b>', HEADER_CELL),
     Paragraph('<b>Key Decision</b>', HEADER_CELL)],
    [Paragraph('0001', CELL_STYLE), Paragraph('M1 Architecture', CELL_LEFT),
     Paragraph('M1', CELL_STYLE), Paragraph('pest PEG + tree-walking interpreter', CELL_LEFT)],
    [Paragraph('0002', CELL_STYLE), Paragraph('Rule Engine', CELL_LEFT),
     Paragraph('M2', CELL_STYLE), Paragraph('Priority-ordered, first-wins conflict resolution', CELL_LEFT)],
    [Paragraph('0003', CELL_STYLE), Paragraph('Learnable Semantics', CELL_LEFT),
     Paragraph('M3', CELL_STYLE), Paragraph('Trait-based LLM abstraction with mock', CELL_LEFT)],
    [Paragraph('0004', CELL_STYLE), Paragraph('Memory', CELL_LEFT),
     Paragraph('M4', CELL_STYLE), Paragraph('ACT-R decay, substring recall, soft-failure', CELL_LEFT)],
    [Paragraph('0005', CELL_STYLE), Paragraph('Adapt', CELL_LEFT),
     Paragraph('M5', CELL_STYLE), Paragraph('Few-shot only, no arbitrary code modification', CELL_LEFT)],
    [Paragraph('0006', CELL_STYLE), Paragraph('Fluid Types', CELL_LEFT),
     Paragraph('P1', CELL_STYLE), Paragraph('Lazy collapse, threshold 0.1, soft-failure', CELL_LEFT)],
]
story.append(Spacer(1, 18))
story.append(make_table(adr_data, [0.08, 0.22, 0.12, 0.58]))
story.append(Spacer(1, 6))
story.append(Paragraph('Table 4. Architecture Decision Records index', META_STYLE))

# ═══════════════════════════════════════════════
# 10. GROWTH PATH
# ═══════════════════════════════════════════════
story.append(Spacer(1, 24))
story.append(heading('10. Growth Path and Roadmap', H1_STYLE, 0))
story.append(Paragraph(
    'METALOGOS development follows a strict milestone-based approach. Each milestone '
    'adds exactly one new capability while maintaining backward compatibility. The '
    'roadmap is organized into four phases beyond the current M1-P1 milestones.', BODY_STYLE))

road_data = [
    [Paragraph('<b>Phase</b>', HEADER_CELL),
     Paragraph('<b>Scope</b>', HEADER_CELL),
     Paragraph('<b>Key Features</b>', HEADER_CELL)],
    [Paragraph('Phase 1 (current)', CELL_LEFT), Paragraph('Type System', CELL_LEFT),
     Paragraph('Fluid Types, confidence propagation, Entity Store, codegen', CELL_LEFT)],
    [Paragraph('Phase 2', CELL_LEFT), Paragraph('ML Integration', CELL_LEFT),
     Paragraph('PyO3/PyTorch/ONNX, knowledge graph, vector memory, transfer learning', CELL_LEFT)],
    [Paragraph('Phase 3', CELL_LEFT), Paragraph('Ecosystem', CELL_LEFT),
     Paragraph('CLI/REPL, LSP, mlogpkg, standard library, mdbook', CELL_LEFT)],
    [Paragraph('Phase 4', CELL_LEFT), Paragraph('Performance', CELL_LEFT),
     Paragraph('Bytecode VM/JIT, optimization passes, self-hosting compiler', CELL_LEFT)],
]
story.append(Spacer(1, 18))
story.append(make_table(road_data, [0.20, 0.20, 0.60]))
story.append(Spacer(1, 6))
story.append(Paragraph('Table 5. Development roadmap phases', META_STYLE))

# ═══════════════════════════════════════════════
# BUILD
# ═══════════════════════════════════════════════
from reportlab.platypus import PageTemplate, Frame

frame = Frame(MARGIN, MARGIN, AVAILABLE_W, PAGE_H - 2*MARGIN, id='normal')
cover_template = PageTemplate(id='Cover', frames=[frame], onPage=draw_cover)
body_template = PageTemplate(id='Body', frames=[frame], onPage=no_cover)
doc.addPageTemplates([cover_template, body_template])

# Insert template switch after cover
from reportlab.platypus.doctemplate import NextPageTemplate
story.insert(0, NextPageTemplate('Cover'))

doc.multiBuild(story)
print(f'PDF generated: {OUTPUT}')
print(f'Size: {os.path.getsize(OUTPUT)} bytes')
