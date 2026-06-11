#!/usr/bin/env python3
"""Generate PDF report on 21 Metalogos Naradov (features)."""

import os, sys
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import inch, cm, mm
from reportlab.lib import colors
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, KeepTogether, HRFlowable
)
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfbase.pdfmetrics import registerFontFamily

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Font Registration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pdfmetrics.registerFont(TTFont('DejaVuSans', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSans-Bold', '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSerif', '/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuSerif-Bold', '/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf'))
pdfmetrics.registerFont(TTFont('DejaVuMono', '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'))
# Tinos not available, using Liberation fonts as fallback

registerFontFamily('DejaVuSans', normal='DejaVuSans', bold='DejaVuSans-Bold')
registerFontFamily('DejaVuSerif', normal='DejaVuSerif', bold='DejaVuSerif-Bold')

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Color Palette (auto-generated)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ACCENT       = colors.HexColor('#4f2eb2')
TEXT_PRIMARY  = colors.HexColor('#1b1918')
TEXT_MUTED    = colors.HexColor('#8a857d')
BG_SURFACE   = colors.HexColor('#e1ddd6')
BG_PAGE      = colors.HexColor('#efedeb')

TABLE_HEADER_COLOR = ACCENT
TABLE_HEADER_TEXT  = colors.white
TABLE_ROW_EVEN     = colors.white
TABLE_ROW_ODD      = BG_SURFACE

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Styles
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
W, H = A4
L_MARGIN = 1.2 * inch
R_MARGIN = 1.2 * inch
T_MARGIN = 1.0 * inch
B_MARGIN = 1.0 * inch
CONTENT_W = W - L_MARGIN - R_MARGIN

# --- Russian report styles ---
title_style = ParagraphStyle(
    name='Title', fontName='DejaVuSans-Bold', fontSize=26,
    leading=34, alignment=TA_CENTER, textColor=ACCENT,
    spaceAfter=6,
)
subtitle_style = ParagraphStyle(
    name='Subtitle', fontName='DejaVuSans', fontSize=14,
    leading=20, alignment=TA_CENTER, textColor=TEXT_MUTED,
    spaceAfter=12,
)
h1_style = ParagraphStyle(
    name='H1', fontName='DejaVuSans-Bold', fontSize=18,
    leading=24, textColor=ACCENT, spaceBefore=18, spaceAfter=10,
)
h2_style = ParagraphStyle(
    name='H2', fontName='DejaVuSans-Bold', fontSize=14,
    leading=20, textColor=TEXT_PRIMARY, spaceBefore=14, spaceAfter=8,
)
h3_style = ParagraphStyle(
    name='H3', fontName='DejaVuSans-Bold', fontSize=12,
    leading=17, textColor=TEXT_PRIMARY, spaceBefore=10, spaceAfter=6,
)
body_style = ParagraphStyle(
    name='Body', fontName='DejaVuSans', fontSize=10,
    leading=16, alignment=TA_LEFT, textColor=TEXT_PRIMARY,
    spaceAfter=6, wordWrap='CJK',
)
body_indent_style = ParagraphStyle(
    name='BodyIndent', fontName='DejaVuSans', fontSize=10,
    leading=16, alignment=TA_LEFT, textColor=TEXT_PRIMARY,
    spaceAfter=6, leftIndent=20, wordWrap='CJK',
)
code_style = ParagraphStyle(
    name='Code', fontName='DejaVuMono', fontSize=8.5,
    leading=13, alignment=TA_LEFT, textColor=colors.HexColor('#333333'),
    backColor=colors.HexColor('#f4f4f4'), leftIndent=12, rightIndent=12,
    spaceBefore=4, spaceAfter=4,
    borderPadding=6,
)
header_cell_style = ParagraphStyle(
    name='HeaderCell', fontName='DejaVuSans-Bold', fontSize=9,
    leading=13, alignment=TA_CENTER, textColor=colors.white,
)
cell_style = ParagraphStyle(
    name='Cell', fontName='DejaVuSans', fontSize=9,
    leading=13, alignment=TA_LEFT, textColor=TEXT_PRIMARY,
)
cell_center_style = ParagraphStyle(
    name='CellCenter', fontName='DejaVuSans', fontSize=9,
    leading=13, alignment=TA_CENTER, textColor=TEXT_PRIMARY,
)
caption_style = ParagraphStyle(
    name='Caption', fontName='DejaVuSans', fontSize=9,
    leading=13, alignment=TA_CENTER, textColor=TEXT_MUTED,
    spaceBefore=3, spaceAfter=6,
)

def make_table(data, col_widths):
    """Create a styled table with alternating rows."""
    table = Table(data, colWidths=col_widths, hAlign='CENTER')
    style_cmds = [
        ('BACKGROUND', (0, 0), (-1, 0), TABLE_HEADER_COLOR),
        ('TEXTCOLOR', (0, 0), (-1, 0), TABLE_HEADER_TEXT),
        ('GRID', (0, 0), (-1, -1), 0.5, TEXT_MUTED),
        ('VALIGN', (0, 0), (-1, -1), 'MIDDLE'),
        ('LEFTPADDING', (0, 0), (-1, -1), 6),
        ('RIGHTPADDING', (0, 0), (-1, -1), 6),
        ('TOPPADDING', (0, 0), (-1, -1), 5),
        ('BOTTOMPADDING', (0, 0), (-1, -1), 5),
    ]
    for i in range(1, len(data)):
        bg = TABLE_ROW_EVEN if i % 2 == 1 else TABLE_ROW_ODD
        style_cmds.append(('BACKGROUND', (0, i), (-1, i), bg))
    table.setStyle(TableStyle(style_cmds))
    return table


def heading(text, level=1):
    """Return a heading paragraph."""
    styles = {1: h1_style, 2: h2_style, 3: h3_style}
    return Paragraph(f'<b>{text}</b>', styles.get(level, h1_style))


def body(text):
    return Paragraph(text, body_style)


def body_i(text):
    return Paragraph(text, body_indent_style)


def code(text):
    return Paragraph(text.replace('\n', '<br/>'), code_style)


def hr():
    return HRFlowable(width="100%", thickness=0.5, color=TEXT_MUTED, spaceBefore=6, spaceAfter=6)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Build Document
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
output_path = '/home/z/my-project/download/metalogos_21_naradov_report.pdf'
doc = SimpleDocTemplate(
    output_path,
    pagesize=A4,
    leftMargin=L_MARGIN, rightMargin=R_MARGIN,
    topMargin=T_MARGIN, bottomMargin=B_MARGIN,
    title="Metalogos: Otchet po 21 Naradam",
    author="Z.ai",
)

story = []

# ═══════════════════════════════════════════════
# COVER PAGE
# ═══════════════════════════════════════════════
story.append(Spacer(1, 120))
story.append(Paragraph('<b>METALOGOS</b>', title_style))
story.append(Spacer(1, 12))
story.append(Paragraph('Otcet po 21 Naradam', ParagraphStyle(
    name='SubTitle2', fontName='DejaVuSans', fontSize=22,
    leading=28, alignment=TA_CENTER, textColor=TEXT_PRIMARY,
)))
story.append(Spacer(1, 24))
story.append(hr())
story.append(Spacer(1, 12))
story.append(Paragraph(
    'Implementacija 21 jazykovoj fichi v AIML-interpretere<br/>'
    'Metalogos: ot arhitektury do bezopasnogo audita',
    subtitle_style
))
story.append(Spacer(1, 60))
story.append(Paragraph('11 ijunja 2026', ParagraphStyle(
    name='DateStyle', fontName='DejaVuSans', fontSize=12,
    leading=16, alignment=TA_CENTER, textColor=TEXT_MUTED,
)))
story.append(Spacer(1, 8))
story.append(Paragraph('Z.ai', ParagraphStyle(
    name='AuthorStyle', fontName='DejaVuSans', fontSize=11,
    leading=14, alignment=TA_CENTER, textColor=TEXT_MUTED,
)))
story.append(PageBreak())

# ═══════════════════════════════════════════════
# TABLE OF CONTENTS (manual but clickable-ready)
# ═══════════════════════════════════════════════
story.append(heading('Soderzanie'))
story.append(Spacer(1, 12))

toc_entries = [
    ("1.", "Vvedenie"),
    ("2.", "Svodnaja tablica 21 narada"),
    ("3.", "Faza M1-M5: Osnovnye konstrukcii jazyka"),
    ("4.", "Faza 1: Sistema tipov"),
    ("5.", "Faza 2: Prodvinutye vozmozhnosti"),
    ("6.", "Faza 3: Instrumenty razrabotcika"),
    ("7.", "Faza 4: Bytecode VM"),
    ("8.", "Dopolnitelnye narady (ADR-0045 - ADR-0057)"),
    ("9.", "Statistika i metriki"),
    ("10.", "Zakljuchenie"),
]
for num, title in toc_entries:
    story.append(Paragraph(
        f'<b>{num}</b>  {title}',
        ParagraphStyle(name='TOCEntry', fontName='DejaVuSans', fontSize=11,
                       leading=18, leftIndent=20, textColor=TEXT_PRIMARY)
    ))
story.append(PageBreak())

# ═══════════════════════════════════════════════
# 1. INTRODUCTION
# ═══════════════════════════════════════════════
story.append(heading('1. Vvedenie'))
story.append(Spacer(1, 8))
story.append(body(
    'Metalogos eto AIML-interpreter, realizovannyj na jazyke Rust. '
    'Jazyk prednaznacen dlja sozdanija intellektualnyh prilozhenij, '
    'kotorye kombinirujut vyzovy JBesimulirujushih modelej, pravila vyvoda, '
    'pamjat s raspadom i adaptivnoe povedenie. Razrabotka vedetsja metodom '
    '"contract-first" po postroennoj lestnice milystonov (Build Ladder).'
))
story.append(body(
    'V dannom otchete sistematizirovany vse 21 narada, vypolnennye v ramkah '
    'arhitekturnyh reshenij (ADR) s nomerami ot 0001 do 0021. Kazhdyj narad '
    'predstavljaet soboj otdelnuju jazykovuju fichu ili podsistemu, '
    'proshedsiju polnyj cikl: specifikacija, implementacija, testy, dokumentacija. '
    'Otdelno rassmotreny dopolnitelnye narady (ADR-0045 po ADR-0057), '
    'vypolnennye v poslednyh sessijah.'
))
story.append(body(
    'Jazyk Metalogos ispolzuet pest 2.x dlja generacii parsera iz PEG-grammatiki, '
    'derevovidnyj interpreter dlja ispolnenija AST, trait-abstrakcii dlja LLM/ML '
    'backendov i modulnuju arhitekturu (parser, AST, interpreter, compiler, VM, audit). '
    'Vse 21 fichi integrirovany v edinyj koda vuzovu s bolee 120 testami, '
    'vkluchaja golden-testy, contract-testy i testy semanticheckogo analiza.'
))

# ═══════════════════════════════════════════════
# 2. SUMMARY TABLE
# ═══════════════════════════════════════════════
story.append(heading('2. Svodnaja tablica 21 narada'))
story.append(Spacer(1, 10))

narads_data = [
    ("ADR", "Narad", "Faza", "Ficha", "Kljucevye komponenty"),
    ("0001", "M1 Arhitektura", "M1", "Entity, Pattern, Flow",
     "pest PEG, AST, tree-walk interpreter"),
    ("0002", "M2 Pravila i vetvlenija", "M2", "Rules, Confidence, Branching",
     "priority-ordered rules, Classify{}"),
    ("0003", "M3 Learnable Patterns", "M3", "LLM-integracija",
     "LlmBackend trait, MockLlm, RealLlm"),
    ("0004", "M4 Pamjat", "M4", "Memorize, Recall, Forget",
     "Aktivacionnoe raspad, substring recall"),
    ("0005", "M5 Adapt", "M5", "Few-shot samomodifikacija",
     "adapt add_example, exact-match cache"),
    ("0006", "Fluid Types", "P1", "Tipovaja superpozicija",
     "lazy collapse, COLLAPSE_THRESHOLD"),
    ("0007", "Confidence Propagation", "P1", "Rasprostranenie uverennosti",
     "min-konfidents, confidence() builtin"),
    ("0008", "Entity Store", "P1", "Hranilishhe s identichnostju",
     "type-scoped index, find(), count()"),
    ("0009", "Semantic Analysis", "P1", "Staticheskaja proverka",
     "two-pass, AnalysisResult, error tests"),
    ("0010", "Codegen / IR", "P1", "Promezhutochnoe predstavlenie",
     "Program wrapper, semantic validation"),
    ("0011", "Type Inference", "P2", "Vyvod tipov vyrazhenij",
     "inference rules, error recovery, overlap"),
    ("0012", "Vector Recall", "P2", "Semanticheskoe podobie",
     "EmbeddingBackend, bag-of-concepts, cosine"),
    ("0013", "ML Backend", "P2", "Obuchenie patternov",
     "MlBackend trait, learn statement, MockMl"),
    ("0014", "Knowledge Graph", "P2", "Grafovaja pamjat",
     "relate, graph-aware recall, adjacency list"),
    ("0015", "Full Adapt", "P2", "Mutate, Sandbox, Rollback",
     "mutate, sandbox decl, rollback_if accuracy"),
    ("0016", "CLI + REPL", "P3", "Instrumenty razrabotcika",
     "clap subcomands, rustyline, mlog check"),
    ("0017", "Stdlib + Import", "P3", "Standartnaja biblioteka",
     "import std/X, Value::List, map/filter"),
    ("0018", "LSP Server", "P3", "Integracija v IDE",
     "tower-lsp, diagnostiki, go-to-definition"),
    ("0019", "Package Manager", "P3", "Upravlenie paketami",
     "mlogpkg, mlog.toml, mlog.lock"),
    ("0020", "Bytecode VM", "P4", "Stack-VM",
     "30+ opcodes, FlowExec, Collapse"),
    ("0021", "VM Full Coverage", "P4", "Polnaja paritetnost",
     "strict tests, rule priority, bench 1.28x"),
]

table_data = []
for i, row in enumerate(narads_data):
    if i == 0:
        table_data.append([Paragraph(f'<b>{c}</b>', header_cell_style) for c in row])
    else:
        table_data.append([
            Paragraph(row[0], cell_center_style),
            Paragraph(row[1], cell_style),
            Paragraph(row[2], cell_center_style),
            Paragraph(row[3], cell_style),
            Paragraph(row[4], cell_style),
        ])

col_w = [CONTENT_W * r for r in [0.07, 0.17, 0.07, 0.24, 0.45]]
t = make_table(table_data, col_w)
story.append(t)
story.append(Paragraph('Tablica 1. Polnyj spisok 21 narada s kljuchevymi harakteristikami', caption_style))
story.append(Spacer(1, 18))

# ═══════════════════════════════════════════════
# 3. M1-M5
# ═══════════════════════════════════════════════
story.append(heading('3. Faza M1-M5: Osnovnye konstrukcii jazyka'))
story.append(Spacer(1, 8))

story.append(body(
    'Fazy M1 po M5 sostavljajut fundamentalnyj koren Metalogosa. Kazhdaja faza '
    'demonstriruet konceptualnyj proryv: ot prostogo konvejera v M1 do '
    'adaptivnogo AI-povedenija v M5. Vse fazy strojatsja posledovatelno, '
    'kazhdaja nasleduet predydushhie i rasshirjaet im.'
))

story.append(heading('3.1. M1: Arhitektura i bazovyj cikl (ADR-0001)', level=2))
story.append(body(
    'M1 dokazal, cikl lexer - parser - AST - interpreter rabotaet skvoz do konca. '
    'Dlja parsera byl vybran pest 2.x (PEG-grammatika), dlja ispolnenija - '
    'derevovidnyj interpreter s HashMap hranilishem entity. '
    'Kontraktprogramma: <b>entity, pattern Shout, flow Main</b>. '
    'Osnovnye uroki: pest trebuet imenovannyh pravil dlja operatorov '
    '(ne inline-stroki), multi-harakternyj operator "-&gt;" trebuet '
    'otricatelnogo prosmotra v grammatike dlja razdelenija s "-" (minus).'
))

story.append(heading('3.2. M2: Pravila, uverennost, vetvlenija (ADR-0002)', level=2))
story.append(body(
    'M2 vvel struct-entity s tipizirovannymi poljami, pravila s prioritetom '
    '(first-wins strategija) i vetvlenie potoka po porogam uverennosti. '
    'Pravila sortirujutsja po ubyvaniju prioriteta, pervoe sovpadenie pobezhdaet. '
    'Flow-arhitektura razdelena na linejnyj konvejer i bloki vetvlenija: '
    'Classify { high (...) -&gt; Escalate, low (...) -&gt; Ignore }. '
    'Chetyre vyvodnyh uroka pest: neobhodimost "~" posle povtornyh operatorov, '
    'silent-rules razvertyvajut svoi vnutrennij pair, negativnyj prosmotr '
    'trebuet osobogo sintaksisa dlja alternativ.'
))

story.append(heading('3.3. M3: Learnable Patterns i LLM-integracija (ADR-0003)', level=2))
story.append(body(
    'M3 - pervyj "vauu-efekt" Metalogosa: learnable pattern otpravljaet '
    'prompt LLM i vozvrashaet otvet kak tipizirovannoe znachenie. '
    'Realizovana trait-abstrakcija LlmBackend s dvumja implementacijami: '
    'MockLlm (deterministicheskij, dlja testov) i RealLlm (HTTP POST cherez '
    'curl, podderzhka OpenAI i Ollama formatov). MockLlm pozvoljaet '
    'deteministicheskie golden-testy bez setevyh vyzovov.'
))

story.append(heading('3.4. M4: Pamjat s raspadom (ADR-0004)', level=2))
story.append(body(
    'M4 dobavil memorize/recall/forget - hranilishe faktov s aktivacionnym '
    'raspadom po formule ACT-R: activation = priority * exp(-decay * age_days). '
    'Recall ispolzuet substring-match dlja poiska i vozvrashaet luchshij '
    'rezultat po aktivacii. Pri otsutstvii sovpadenij - soft-failure '
    '(pustaja stroka vmesto oshibki). Pamjat in-process, serde-persistence '
    'otlozhena.'
))

story.append(heading('3.5. M5: Adapt - few-shot samomodifikacija (ADR-0005)', level=2))
story.append(body(
    'M5 realizuet pervuju formu samomodifikacii: adapt dobavljaet '
    '(input, output) pary v few-shot kesh learnable patterna. Pri tochnom '
    'sovpadenii vhoda s keshom LLM ne vyzivaetsja. Bezopasnost garantirovana '
    'na urovne tipov: adapt mozhno primenjat tolko k learnable patternam, '
    'ne k obyknovennym patternam, pravilam ili entity..rollback otlozhen '
    'na budushhie fazy.'
))

# ═══════════════════════════════════════════════
# 4. PHASE 1: TYPE SYSTEM
# ═══════════════════════════════════════════════
story.append(heading('4. Faza 1: Sistema tipov'))
story.append(Spacer(1, 8))
story.append(body(
    'Faza 1 vvela tipovuju systemu, semanticheckij analiz i promezhutochnoe '
    'predstavlenie. Jeti komponenty neobhodimy dlja nadezhnyh program na '
    'Metalogose, prehodyashhih ot prostyh skriptov k slozhnym AIML-prilozhenijam.'
))

story.append(heading('4.1. Fluid Types (ADR-0006)', level=2))
story.append(body(
    'Fluid Types - fundamentalnaja ficha Metalogosa: znachenija, sushestvujushhie '
    'v superpozicii konkretnyh tipov s annotacie uverennosti. Deklaracija: '
    'fluid x = Float[42.0][0.9] or String["answer"][0.1]. '
    'Kollaps proishodit lenivo - tolko v tochke ispolzovanija, kogda '
    'pattern trebuet konkretnyj tip. Porog kollapsa: COLLAPSE_THRESHOLD = 0.1. '
    'Pri neudache - soft-failure (Value::Unit).'
))

story.append(heading('4.2. Confidence Propagation (ADR-0007)', level=2))
story.append(body(
    'Rasprostranenie uverennosti cherez patterny ispolzuet pravilo min: '
    'vyhodnaja uverennost = min(urovni uverennosti vseh vhodov). Rezultat '
    'oborachivaetsja v Value::Fluid s odnim variantom, nesushim '
    'propagirovannuju uverennost. Builtin confidence(v) izvlekaet '
    'uverennost iz Fluid znachenija ili 1.0 dlja konkretnyh znachenij. '
    'Eto evristika, ne verojatnostnyj vyvod - prostaja i zashishimaja.'
))

story.append(heading('4.3. Entity Store (ADR-0008)', level=2))
story.append(body(
    'Entity Store - tipizirovannyj indeks v HashMap peremennyh. Kazhdyj struct-entity '
    'avtomaticheski registriruetsja v entity_store: HashMap&lt;String, Vec&lt;EntityRecord&gt;&gt;. '
    'Novye builtin: find(type, field, op, threshold) - linejnoe skanirovanie '
    's pervym sovpadeniem, i count(type) - kolichestvo entity dannogo tipa. '
    'Store - indeks, ne kopija: dejsvitelnye znachenija ostaetsja v variables.'
))

story.append(heading('4.4. Semantic Analysis (ADR-0009)', level=2))
story.append(body(
    'Dvuhprohodnyj analiz pered ispolneniem: prohod 1 sobiraet vse opredelenija tipov '
    'i signatury, prohod 2 validiruet ssylki. Proverjaet: neopredelennye entity, '
    'neizvestnye shagi potoka, nepozdannye funkcii, tseli pravil, tceli adapt. '
    'Rezhim "fail fast" - pervaja oshibka prekrashhaet analiz. '
    'Error-testy ispolzujut konvenciju: err_*.mlog + err_*.error.'
))

story.append(heading('4.5. Codegen / IR (ADR-0010)', level=2))
story.append(body(
    'Promezhutochnoe predstavlenie v Faze 1 - tonkij Program-wrapper, '
    'obernuvshej AST posle semanticheckogo analiza. Codegen::compile() '
    'vypolnjaet semantic::analyze() i sozdaet Program { declarations }. '
    'Eto ustanavlivaet arhitekturnuju granicu dlja budushej bytecode-VM. '
    'Pipeline: source - parse - semantic - codegen (wrap) - interpreter.'
))

story.append(heading('4.6. Type Inference (ADR-0011)', level=2))
story.append(body(
    'Dopolnenie semanticheckogo analiza: vyvod tipov vyrazhenij, '
    'vosstanovlenie posle oshibok (sborochnaja AnalysisResult s errors + warnings), '
    'obnaruzhenie peresekajushhihsja vetvlenij. Vyvod tipov napravlennym '
    'rasprostraneniem cherez potok: tip istocnika - proverka parametra shaga '
    '- tip vozvrata. Interpoljacija vetvlenij cherez intervalnuju arifmetiku '
    'na chislovoj osi.'
))

# ═══════════════════════════════════════════════
# 5. PHASE 2
# ═══════════════════════════════════════════════
story.append(heading('5. Faza 2: Prodvinutye vozmozhnosti'))
story.append(Spacer(1, 8))
story.append(body(
    'Faza 2 dobavila vektornoe vosstanovlenie pamjati, ML-obuchenie patternov, '
    'grafovuju pamjat i polnuju sistemu adaptacii s rollback. '
    'Jeti fichi prevrashhajut Metalogos iz prostogo LLM-obertki v '
    'platformu dlja adapivnyh AIML-sistem.'
))

story.append(heading('5.1. Vector Recall (ADR-0012)', level=2))
story.append(body(
    'EmbeddingBackend trait abstragiruet istochnik vektornyh predstavlenij. '
    'Faza 2.2: SimpleEmbeddingBackend s bag-of-concepts (8 grup, ~300 slov). '
    'Algoritm: tokenizacija - otozhestvlenie slov s koncept-gruppami - '
    'postroenie 8-mernogo vektora - normalizacija - cosine similarity. '
    'Rezultat: recall("food preferences") nakhodit "user likes spicy food" '
    'bez obshih slov cherez food_cuisine + preference_opinion.'
))

story.append(heading('5.2. ML Backend (ADR-0013)', level=2))
story.append(body(
    'MlBackend trait i operator learn Name with { data, epochs }. '
    'MockMlBackend vozvrashaet accuracy=0.95 dlja deterministicheskih testov. '
    'Podderzhka INT literalov v grammatike (dlja epochs: 5). '
    'Put prodvinutogo obuchenija: PyO3MlBackend s PyO3+PyTorch, '
    'eksport v ONNX dlja runtime.'
))

story.append(heading('5.3. Knowledge Graph (ADR-0014)', level=2))
story.append(body(
    'Novyj operator relate "A" to "B" as "coworker" sozdaet rebra '
    'v adjacency list (Vec&lt;Relation&gt;). Pri recall graph walk '
    'nahodit svjazannye fakty i dobavljaet ih kak [GRAPH] relation -&gt; value. '
    'Ogranichenija: string-match, netranzitivnyj obhod, linejnoe skanirovanie. '
    'Posle 3.0: Neo4j migracija.'
))

story.append(heading('5.4. Full Adapt (ADR-0015)', level=2))
story.append(body(
    'Tri novyh konstrukcii: sandbox (configuracija bezopasnosti), '
    'mutate (zamena few-shot seta s proverkoj accuracy i rollback), '
    'sandbox_decl (allowed/forbidden/timeout). '
    'Raznica adapt vs mutate: adapt dobavljaet primery, mutate zamenjaet ves set. '
    'Mock accuracy 0.95 pozvoljaet testirovat oba puti (kept i rolled back). '
    'Sandbox ne prinuzhdaetsja v runtime - otlozheno na Fazu 3.'
))

# ═══════════════════════════════════════════════
# 6. PHASE 3
# ═══════════════════════════════════════════════
story.append(heading('6. Faza 3: Instrumenty razrabotcika'))
story.append(Spacer(1, 8))
story.append(body(
    'Faza 3: CLI, REPL, standartnaja biblioteka, LSP-server i paketnyj menedzher. '
    'Jeti komponenty prevrashhajut Metalogos iz eksperimentalnogo prototipa '
    'v polnocennyj jazyk razrabotki s IDE-podderzhkoj.'
))

story.append(heading('6.1. CLI + REPL (ADR-0016)', level=2))
story.append(body(
    'Rasshiren CLI: mlog run, mlog repl, mlog check. REPL na rustyline '
    's postojannym sostojaniem interpretera mezhdu vvodami. mlog check '
    'vypolnjaet semanticheckij analiz bez ispolnenija. '
    'feed_line() - public API dlja instrumentov, LSP i notebook-ov. '
    'TTY-detection cherez libc::isatty. '
    'Istori REPL: ~/.mlog_history.'
))

story.append(heading('6.2. Standartnaja biblioteka (ADR-0017)', level=2))
story.append(body(
    'Import-mehanizm: import std/string razreshaetsja v &lt;std_root&gt;/std/string.mlog. '
    'Tri modulja: std/string (trim, replace, split, join), '
    'std/math (abs, min, max, clamp, round), '
    'std/collections (first, last, push + map/filter/reduce). '
    'Dvuhslojnoj pattern-wrapper: Rust builtins s prefiksom "__" + '
    '.mlog patterns dlja public API. Value::List tip.'
))

story.append(heading('6.3. LSP Server (ADR-0018)', level=2))
story.append(body(
    'Otdelnyj krate mlog-lsp na tower-lsp. Vozmozhnosti: diagnostiki, '
    'go-to-definition, hover, completion, text sync (FULL). '
    'Span-propagation v AST dlja pozicij. AnalysisResultDetailed s diagnostikami. '
    'Text-scanning podhod dlja go-to-definition i hover '
    '(ne trebuet spanov na urovne vyrazhenij). '
    'VS Code extension manifest.'
))

story.append(heading('6.4. Package Manager mlogpkg (ADR-0019)', level=2))
story.append(body(
    'Otdelnyj binarnyj krate mlogpkg s komandami: init, add, build, info. '
    'Manifest: mlog.toml (name, version, edition, dependencies). '
    'Lokalnyj registr: ~/.mlog/registry/. Lock-file: mlog.lock. '
    'Build workflow: read manifest - resolve deps - check sources - write lock. '
    'Net udistalennogo registr - pakety ustanavlivajutsja vruchnuju.'
))

# ═══════════════════════════════════════════════
# 7. PHASE 4
# ═══════════════════════════════════════════════
story.append(heading('7. Faza 4: Bytecode VM'))
story.append(Spacer(1, 8))
story.append(body(
    'Faza 4 vvela bytecode-kompiljator i stack-based VM, obespechivaja '
    'otdelenie kompiljacii ot ispolnenija i sozdaja bazu dlja optimizacij.'
))

story.append(heading('7.1. Bytecode VM (ADR-0020)', level=2))
story.append(body(
    'Tri novyh modulja: bytecode.rs (Instruction enum), compiler.rs (AST -&gt; Program), '
    'vm.rs (stack-based execution). 30+ opcodeov, vklyuchaja Metalogos-specific: '
    'JumpIfLow (confidence branching), Collapse (fluid to type), '
    'Memorize/Recall/Forget, Adapt/Relate/Mutate, FlowExec, ExecuteRules. '
    'Dvuhprohodnaja kompiljacija: pass 1 - sobiranie tipov, pass 2 - generacija '
    'instrukcij. Polnaja paritetnost so vsemi 11 golden-test primjerami.'
))

story.append(heading('7.2. VM Full Coverage (ADR-0021)', level=2))
story.append(body(
    'Ispravleny semanticheckie razryvy: pravilnoe sortirovanie pravil po prioritetu, '
    'propagacija collections_loaded flaga, strogie dvuhrezhimnye testy. '
    'Proizvoditelnost: 1.1x pri 10 shagah, 1.28x pri 1000 shagah, '
    '1.17x srednee na golden-testah. '
    'Skromnoe uskorenie objasnjaetsja obshim Value tipom s clone() '
    'i makro-instrukcijami (FlowExec, ExecuteRules).'
))

# ═══════════════════════════════════════════════
# 8. ADDITIONAL NARADS (ADR-0045-0057)
# ═══════════════════════════════════════════════
story.append(heading('8. Dopolnitelnye narady (ADR-0045 - ADR-0057)'))
story.append(Spacer(1, 8))
story.append(body(
    'Posle osnovnyh 21 fich byli vypolneny dopolnitelnye narady, '
    'rasshirjajushhie vozmozhnosti jazyka dlja serioznyh AIML-prilozhenij. '
    'Nizhe predstavleny kljuchevye iz nih.'
))

story.append(heading('8.1. Hooks: before_pattern / after_pattern (ADR-0045)', level=2))
story.append(body(
    'Perhvatki vyzovov patternov dlja cross-cutting concerns: '
    'logging, metriki, audit. Dva vida: before_pattern i after_pattern. '
    'V inekcirovannyh peremennyh: pattern_name, args, result, confidence. '
    '6 contract-testov, vse projdeny.'
))

story.append(heading('8.2. Context Auto-Loading (ADR-0046)', level=2))
story.append(body(
    'Avtomaticheskaja podgruzka konteksta dlja learnable patternov. '
    'Chetyre rezhima: context: auto (recall po pervomu parametru), '
    'context: none, context: recall(expr, limit=N), '
    'context: "literal string". Kontekst vstavljatsja pered system prompt '
    'v formate "Relevant context: - fact1, - fact2".'
))

story.append(heading('8.3. Session Memory (ADR-0049)', level=2))
story.append(body(
    'Vremennaja pamjat razgovora: session_set, session_get, session_clear. '
    'In-memory HashMap s izoljaciej po session_id. Net persistence '
    '- dannye terjajutsja pri restarte. 10 contract-testov.'
))

story.append(heading('8.4. Eval Harness (ADR-0050)', level=2))
story.append(body(
    'Avtomaticheskaja ocenka learnable patternov na pomeshennyh dannom. '
    'Konstrukt eval Name { dataset, metric, threshold }. '
    'Confusion matrix, adapt-suggestii dlja fail-primjerov. '
    'CLI: mlog eval. Exit code 0/1 dlja CI. 9 contract-testov.'
))

story.append(heading('8.5. inspect() Builtin (ADR-0051)', level=2))
story.append(body(
    'Builtin dlja nabljudenija: inspect("PatternName") -&gt; Struct s 8 poljami. '
    'Vozvrashaet: calls, avg_confidence, cache_hits, cache_misses, '
    'last_adapt, last_call, examples_count, is_learnable. '
    'In-memory, Mutex-protected HashMap. 8 contract-testov.'
))

story.append(heading('8.6. Event Stream (ADR-0052)', level=2))
story.append(body(
    'Edinyj strukturivannyj log vseh operacij interpretera. '
    'Event { id, timestamp, event_type, source, data, duration_ms }. '
    'Instrumentirovany: memory_store, adapt, pattern_call. '
    '4 builtin: event_count, events_since, event_sum. 9 contract-testov.'
))

story.append(heading('8.7. Conversation State (ADR-0053)', level=2))
story.append(body(
    'Upravljaemyj kontekst dialoga: conv_start, conv_add, conv_history, '
    'conv_context, conv_end. Avtoszhatie staryh soobshhenij cherez LLM. '
    'Ogranichenie max_messages s evikciej starshego. '
    'Integracija s learnable pattern cherez pole conversation. 10 contract-testov.'
))

story.append(heading('8.8. Tool Abstraction (ADR-0054)', level=2))
story.append(body(
    'Namespace-gruppirovka vneshnih servisor: tool telegram { send(), get_updates() }. '
    'Vyzov cherez qualified call: telegram.send("id", "text"). '
    'Izoljacija namespaceov cherez qualified keys. 9 contract-testov.'
))

story.append(heading('8.9. Lifecycle Control (ADR-0056)', level=2))
story.append(body(
    'Checkpoint/resume dlja dolgih potokov: checkpoint("name") v flow. '
    'Serializacija sostojanija v SQLite ili in-memory. '
    'CLI: mlog resume. 10 contract-testov. Vosstanovlenie peremennyh i znachenij.'
))

story.append(heading('8.10. Security Audit (ADR-0057)', level=2))
story.append(body(
    'Statischekij bezopasnyj analiz: mlog audit. 8 proverok: '
    'SECRETS, SQL_DYNAMIC, SANDBOX_COVERAGE, RATE_LIMIT, CSRF, '
    'HTML_INJECTION, SECRET_LEAK, OPEN_REDIRECT. '
    'Taint tracking cherez peremennye. Exit codes: 0=clean, 1=errors, '
    '2=warnings. 20/20 unit-testov.'
))

# ═══════════════════════════════════════════════
# 9. STATISTICS
# ═══════════════════════════════════════════════
story.append(heading('9. Statistika i metriki'))
story.append(Spacer(1, 10))

stats_data = [
    [Paragraph('<b>Metrika</b>', header_cell_style),
     Paragraph('<b>Znachenie</b>', header_cell_style)],
    [Paragraph('Obshee chislo ADR', cell_style),
     Paragraph('57 (iz nih 21 osnovnyh + 13 dopolnitelnyh naradov)', cell_style)],
    [Paragraph('Fazy razrabotki', cell_style),
     Paragraph('M1-M5 (core), Phase 1 (types), Phase 2 (advanced), Phase 3 (tooling), Phase 4 (VM)', cell_style)],
    [Paragraph('Contract-testy', cell_style),
     Paragraph('82/82 (vse projdeny, bez regressij)', cell_style)],
    [Paragraph('Lib-testy', cell_style),
     Paragraph('122/126 (4 predsyshestvujushih oshibki v semantic)', cell_style)],
    [Paragraph('Vozmizhnosti', cell_style),
     Paragraph('Bytecode VM: 1.17-1.28x uskorenie nad tree-walk', cell_style)],
    [Paragraph('Moduli Rust', cell_style),
     Paragraph('parser, ast, interpreter, compiler, vm, audit, semantic, codegen, ir, embedding, llm, ml, builtins', cell_style)],
    [Paragraph('Operatorov VM', cell_style),
     Paragraph('30+ (vklyuchaja Metalogos-specific: Collapse, JumpIfLow, FlowExec)', cell_style)],
    [Paragraph('Bezopasnye proverki audit', cell_style),
     Paragraph('8 (secrets, SQL, sandbox, rate_limit, CSRF, HTML, leak, redirect)', cell_style)],
    [Paragraph('Podderzhka koncepcij', cell_style),
     Paragraph('entity, pattern, flow, rule, learnable, memory, fluid, tool, conversation', cell_style)],
]

col_s = [CONTENT_W * r for r in [0.30, 0.70]]
t2 = make_table(stats_data, col_s)
story.append(t2)
story.append(Paragraph('Tablica 2. Kljuchevye metriki Metalogosa na tekushij moment', caption_style))
story.append(Spacer(1, 18))

# ═══════════════════════════════════════════════
# 10. CONCLUSION
# ═══════════════════════════════════════════════
story.append(heading('10. Zakljuchenie'))
story.append(Spacer(1, 8))
story.append(body(
    'Vse 21 osnovnyh narada Metalogosa uspeshno vypolneny i integrirovany v edinyj '
    'koda vuzovu. Jazyk proshel put ot prostogo konvejera entity-pattern-flow '
    'do polnoj AIML-platformy s bytecode VM, LSP-serverom, paketnym menedzherom, '
    'bezopasnym audito i upravljaemym kontextom dialogov.'
))
story.append(body(
    'Kluchevye dostizhenija: (1) derevovidnyj interpreter i bytecode VM s '
    'polnoj paritetnostju, (2) trait-abstrakcii dlja LLM/ML backendov s mock '
    'i realnymi implementacijami, (3) Fluid Types s lenivym kollapsom i '
    'rasprostraneniem uverennosti, (4) bezopasnyj audit s taint-tracking, '
    '(5) 82 contract-testa bez regressij.'
))
story.append(body(
    'Dopolnitelnye narady (ADR-0045 - ADR-0057) rasshirjajut osnovnyj funkcional '
    'dlja postroenija serioznyh AIML-prilozhenij: hooks dlja cross-cutting concerns, '
    'session memory dlja chat-botov, eval harness dlja CI/CD, conversation state '
    'dlja multi-turn dialogov, tool abstraction dlja integracii s vneshnimi servisa, '
    'checkpoint/resume dlja dolgih potokov i staticheskij bezopasnyj audit.'
))
story.append(body(
    'Dalnejshie napravlenija: peephole-optimizer dlja VM (Faza 4.3), '
    'realnye embeddings cherez PyO3 (Faza 2.3), remote registry dlja mlogpkg, '
    'incrementalnyj parsing dlja LSP, i primenenie vseh 21 fichej v '
    'Fosved office v2.'
))

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Build
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
doc.build(story)
print(f"Report generated: {output_path}")
