const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, AlignmentType, HeadingLevel, PageNumber,
  BorderStyle, WidthType, ShadingType, SectionType, TableLayoutType,
} = require("docx");

// ── Palette: DM-1 Deep Cyan (tech/AI) ──
const P = {
  bg: "162235",
  titleColor: "FFFFFF",
  subtitleColor: "B0B8C0",
  metaColor: "90989F",
  footerColor: "687078",
  accent: "37DCF2",
  primary: "0F172A",
  body: "000000",
  secondary: "5A6080",
  surface: "F8F9FF",
  // Table colors (darkened accent for white-page tables)
  table: {
    headerBg: "1B6B7A",
    headerText: "FFFFFF",
    accentLine: "1B6B7A",
    innerLine: "C8DDE2",
    surface: "EDF3F5",
  },
};
const c = (hex) => hex.replace("#", "");

const NB = { style: BorderStyle.NONE, size: 0, color: "FFFFFF" };
const noBorders = { top: NB, bottom: NB, left: NB, right: NB };
const allNoBorders = {
  top: NB, bottom: NB, left: NB, right: NB,
  insideHorizontal: NB, insideVertical: NB,
};

// ── Helper functions ──

function h1(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { before: 360, after: 200 },
    children: [
      new TextRun({
        text,
        bold: true,
        size: 32,
        color: c(P.primary),
        font: { ascii: "Calibri", eastAsia: "SimHei" },
      }),
    ],
  });
}

function h2(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    spacing: { before: 280, after: 140 },
    children: [
      new TextRun({
        text,
        bold: true,
        size: 28,
        color: c(P.primary),
        font: { ascii: "Calibri", eastAsia: "SimHei" },
      }),
    ],
  });
}

function h3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    spacing: { before: 200, after: 100 },
    children: [
      new TextRun({
        text,
        bold: true,
        size: 26,
        color: c(P.secondary),
        font: { ascii: "Calibri", eastAsia: "SimHei" },
      }),
    ],
  });
}

function body(text) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    indent: { firstLine: 480 },
    spacing: { line: 312, after: 80 },
    children: [
      new TextRun({
        text,
        size: 24,
        color: c(P.body),
        font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
      }),
    ],
  });
}

function bodyNoIndent(text) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    spacing: { line: 312, after: 80 },
    children: [
      new TextRun({
        text,
        size: 24,
        color: c(P.body),
        font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
      }),
    ],
  });
}

function code(text) {
  return new Paragraph({
    spacing: { before: 60, after: 60 },
    indent: { left: 480 },
    shading: { type: ShadingType.CLEAR, fill: c(P.table.surface) },
    children: [
      new TextRun({
        text,
        size: 21,
        color: "1A1A1A",
        font: { ascii: "Consolas", eastAsia: "Microsoft YaHei" },
      }),
    ],
  });
}

function spacer(twips) {
  return new Paragraph({ spacing: { before: twips } });
}

// ── Table builder (horizontal-only style) ──
function buildTable(headers, rows) {
  const t = P.table;
  const colCount = headers.length;
  const colWidth = Math.floor(100 / colCount);

  const headerRow = new TableRow({
    tableHeader: true,
    children: headers.map((h) =>
      new TableCell({
        width: { size: colWidth, type: WidthType.PERCENTAGE },
        shading: { type: ShadingType.CLEAR, fill: c(t.headerBg) },
        borders: {
          top: { style: BorderStyle.SINGLE, size: 2, color: c(t.accentLine) },
          bottom: { style: BorderStyle.SINGLE, size: 2, color: c(t.accentLine) },
          left: { style: BorderStyle.NONE },
          right: { style: BorderStyle.NONE },
        },
        children: [
          new Paragraph({
            spacing: { before: 60, after: 60 },
            children: [
              new TextRun({
                text: h,
                bold: true,
                size: 21,
                color: c(t.headerText),
                font: { ascii: "Calibri", eastAsia: "SimHei" },
              }),
            ],
          }),
        ],
      })
    ),
  });

  const dataRows = rows.map(
    (row, idx) =>
      new TableRow({
        children: row.map((cell) =>
          new TableCell({
            width: { size: colWidth, type: WidthType.PERCENTAGE },
            shading:
              idx % 2 === 0
                ? { type: ShadingType.CLEAR, fill: c(t.surface) }
                : { type: ShadingType.CLEAR, fill: "FFFFFF" },
            borders: {
              top: { style: BorderStyle.NONE },
              bottom: {
                style: BorderStyle.SINGLE,
                size: 1,
                color: c(t.innerLine),
              },
              left: { style: BorderStyle.NONE },
              right: { style: BorderStyle.NONE },
            },
            children: [
              new Paragraph({
                spacing: { before: 50, after: 50 },
                children: [
                  new TextRun({
                    text: cell,
                    size: 21,
                    color: c(P.body),
                    font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
                  }),
                ],
              }),
            ],
          })
        ),
      })
  );

  return new Table({
    width: { size: 100, type: WidthType.PERCENTAGE },
    layout: TableLayoutType.FIXED,
    rows: [headerRow, ...dataRows],
  });
}

// ── Cover: R1 Pure Paragraph Left ──

function calcTitleLayout(title, maxWidthTwips, preferredPt, minPt) {
  const charWidth = (pt) => pt * 20;
  const charsPerLine = (pt) => Math.floor(maxWidthTwips / charWidth(pt));
  let titlePt = preferredPt || 40;
  let lines;
  while (titlePt >= (minPt || 24)) {
    const cpl = charsPerLine(titlePt);
    if (cpl < 2) { titlePt -= 2; continue; }
    lines = splitTitleLines(title, cpl);
    if (lines.length <= 3) break;
    titlePt -= 2;
  }
  if (!lines || lines.length > 3) {
    const cpl = charsPerLine(minPt || 24);
    lines = splitTitleLines(title, cpl);
    titlePt = minPt || 24;
  }
  return { titlePt, titleLines: lines };
}

function splitTitleLines(title, charsPerLine) {
  if (title.length <= charsPerLine) return [title];
  const breakAfter = new Set([
    ..."\u0430,\u0431,\u0432,\u0433,\u0434,\u0435,\u0451,\u0436,\u0437,\u0438,\u0439,\u043a,\u043b,\u043c,\u043d,\u043e,\u043f,\u0440,\u0441,\u0442,\u0443,\u0444,\u0445,\u0446,\u0447,\u0448,\u0449,\u044a,\u044b,\u044c,\u044d,\u044e,\u044f",
    ..."-_\u2014\u2013\u00b7/",
    ..." \	",
  ]);
  // Actually, let me add proper break chars
  const breakChars = new Set([
    "\u0430","\u0438","\u043e","\u0443","\u0435","\u044f",
    "-","_","/"," ","\t",
  ]);
  const lines = [];
  let remaining = title;
  while (remaining.length > charsPerLine) {
    let breakAt = -1;
    for (let i = charsPerLine; i >= Math.floor(charsPerLine * 0.6); i--) {
      if (i < remaining.length && breakChars.has(remaining[i - 1])) {
        breakAt = i;
        break;
      }
    }
    if (breakAt === -1) breakAt = charsPerLine;
    lines.push(remaining.slice(0, breakAt).trim());
    remaining = remaining.slice(breakAt).trim();
  }
  if (remaining) lines.push(remaining);
  if (lines.length > 1 && lines[lines.length - 1].length <= 2) {
    const last = lines.pop();
    lines[lines.length - 1] += last;
  }
  return lines;
}

function calcCoverSpacing(params) {
  const {
    titleLineCount = 1, titlePt = 36, hasSubtitle = false,
    hasEnglishLabel = false, metaLineCount = 0,
    fixedHeight = 800, pageHeight = 16838,
    marginTop = 0, marginBottom = 0,
  } = params;
  const SAFETY = 1200;
  const usableHeight = pageHeight - marginTop - marginBottom - SAFETY;
  const titleHeight = titleLineCount * (titlePt * 23 + 200);
  const subtitleHeight = hasSubtitle ? (12 * 23 + 600) : 0;
  const englishLabelHeight = hasEnglishLabel ? (9 * 23 + 600) : 0;
  const metaHeight = metaLineCount * (10 * 23 + 100);
  const implicitParaHeight = 3 * 300;
  const contentHeight = titleHeight + subtitleHeight + englishLabelHeight +
                        metaHeight + fixedHeight + implicitParaHeight;
  const remainingSpace = usableHeight - contentHeight;
  const safeRemaining = Math.max(remainingSpace, 400);
  const FOOTER_MIN = 800;
  const rawTop = Math.floor(safeRemaining * 0.45);
  const rawBottom = Math.floor(safeRemaining * 0.45);
  const bottomSpacing = Math.max(rawBottom, FOOTER_MIN);
  const topSpacing = Math.max(rawTop - Math.max(0, FOOTER_MIN - rawBottom), 400);
  const midSpacing = Math.max(safeRemaining - topSpacing - bottomSpacing, 0);
  return { topSpacing, midSpacing, bottomSpacing };
}

function buildCoverR1(config) {
  const padL = 1200, padR = 800;
  const availableWidth = 11906 - padL - padR - 300;
  const { titlePt, titleLines } = calcTitleLayout(config.title, availableWidth, 40, 24);
  const titleSize = titlePt * 2;
  const spacing = calcCoverSpacing({
    titleLineCount: titleLines.length, titlePt,
    hasSubtitle: !!config.subtitle,
    hasEnglishLabel: !!config.englishLabel,
    metaLineCount: (config.metaLines || []).length,
    fixedHeight: 400,
  });
  const accentLeft = {
    style: BorderStyle.SINGLE, size: 8, color: c(P.accent), space: 12,
  };
  const children = [];
  children.push(new Paragraph({ spacing: { before: spacing.topSpacing } }));
  if (config.englishLabel) {
    children.push(
      new Paragraph({
        indent: { left: padL, right: padR },
        spacing: { after: 500 },
        border: {
          bottom: {
            style: BorderStyle.SINGLE,
            size: 6,
            color: c(P.accent),
            space: 8,
          },
        },
        children: [
          new TextRun({
            text: config.englishLabel,
            size: 18,
            color: c(P.accent),
            font: { ascii: "Calibri" },
            characterSpacing: 40,
          }),
        ],
      })
    );
  }
  for (let i = 0; i < titleLines.length; i++) {
    children.push(
      new Paragraph({
        indent: { left: padL },
        spacing: {
          after: i < titleLines.length - 1 ? 100 : 300,
          line: Math.ceil(titlePt * 23),
          lineRule: "atLeast",
        },
        children: [
          new TextRun({
            text: titleLines[i],
            size: titleSize,
            bold: true,
            color: c(P.titleColor),
            font: { eastAsia: "SimHei", ascii: "Arial" },
          }),
        ],
      })
    );
  }
  if (config.subtitle) {
    children.push(
      new Paragraph({
        indent: { left: padL },
        spacing: { after: 800 },
        children: [
          new TextRun({
            text: config.subtitle,
            size: 24,
            color: c(P.subtitleColor),
            font: { eastAsia: "Microsoft YaHei", ascii: "Arial" },
          }),
        ],
      })
    );
  }
  for (const line of config.metaLines || []) {
    children.push(
      new Paragraph({
        indent: { left: padL + 200 },
        spacing: { after: 80 },
        border: { left: accentLeft },
        children: [
          new TextRun({
            text: line,
            size: 24,
            color: c(P.metaColor),
            font: { eastAsia: "Microsoft YaHei", ascii: "Arial" },
          }),
        ],
      })
    );
  }
  children.push(new Paragraph({ spacing: { before: spacing.bottomSpacing } }));
  children.push(
    new Paragraph({
      indent: { left: padL, right: padR },
      border: {
        top: {
          style: BorderStyle.SINGLE,
          size: 2,
          color: c(P.accent),
          space: 8,
        },
      },
      spacing: { before: 200 },
      children: [
        new TextRun({
          text: config.footerLeft || "",
          size: 16,
          color: c(P.footerColor),
          font: { ascii: "Arial" },
        }),
        new TextRun({
          text: "                                        ",
          size: 16,
          color: c(P.footerColor),
        }),
        new TextRun({
          text: config.footerRight || "",
          size: 16,
          color: c(P.footerColor),
          font: { ascii: "Arial" },
        }),
      ],
    })
  );
  return [
    new Table({
      width: { size: 100, type: WidthType.PERCENTAGE },
      layout: TableLayoutType.FIXED,
      borders: allNoBorders,
      rows: [
        new TableRow({
          height: { value: 16838, rule: "exact" },
          children: [
            new TableCell({
              shading: { type: ShadingType.CLEAR, fill: c(P.bg) },
              borders: noBorders,
              children,
            }),
          ],
        }),
      ],
    }),
  ];
}

// ══════════════════════════════════════════════════════════════
// DOCUMENT CONTENT
// ══════════════════════════════════════════════════════════════

const coverConfig = {
  title: "\u041d\u0430\u0440\u044f\u0434 O-2: obsidian-mind \u2192 Metalogos",
  englishLabel: "WORK ORDER",
  subtitle: "\u0420\u0430\u0441\u0448\u0438\u0440\u0435\u043d\u0438\u0435 lifecycle hooks \u0438 load_config (YAML)",
  metaLines: [
    "\u041f\u0440\u0438\u043e\u0440\u0438\u0442\u0435\u0442: \u0432\u044b\u0441\u043e\u043a\u0438\u0439",
    "\u0412\u0435\u0440\u0441\u0438\u044f: 0.10.0 \u2192 0.11.0",
    "\u0414\u0430\u0442\u0430: 2026-07-23",
    "\u0418\u0441\u043f\u043e\u043b\u043d\u0438\u0442\u0435\u043b\u044c: \u042f\u043d\u0430 (AI-agent)",
  ],
  footerLeft: "ShkodnikAI / Metalogos",
  footerRight: "\u041a\u043e\u043d\u0444\u0438\u0434\u0435\u043d\u0446\u0438\u0430\u043b\u044c\u043d\u043e",
};

const bodyContent = [
  // ── 1. Цель ──
  h1("1. \u0426\u0435\u043b\u044c \u0438 \u043e\u0431\u043e\u0441\u043d\u043e\u0432\u0430\u043d\u0438\u0435"),

  body("\u041d\u0430\u0440\u044f\u0434 O-2 \u0437\u0430\u0434\u0430\u0451\u0442 \u043f\u043e\u0440\u0442\u0438\u0440\u043e\u0432\u0430\u043d\u0438\u0435 \u0430\u0440\u0445\u0438\u0442\u0435\u043a\u0442\u0443\u0440\u043d\u044b\u0445 \u043a\u043e\u043d\u0446\u0435\u043f\u0446\u0438\u0439 \u0438\u0437 obsidian-mind (TypeScript, 3.5k\u2605, MIT) \u0432 Metalogos. \u0412 v0.10.0 \u0443\u0436\u0435 \u0440\u0435\u0430\u043b\u0438\u0437\u043e\u0432\u0430\u043d\u044b \u0442\u0440\u0438 \u0431\u0438\u043b\u0442\u0438\u043d\u0430 (semantic_search, config_load, vault_validate) \u043a\u0430\u043a \u043f\u0435\u0440\u0432\u0430\u044f \u0432\u043e\u043b\u043d\u0430. \u041d\u0430\u0440\u044f\u0434 O-2 \u0437\u0430\u0432\u0435\u0440\u0448\u0430\u0435\u0442 \u043e\u0441\u0442\u0430\u0432\u0448\u0438\u0435\u0441\u044f \u044d\u043b\u0435\u043c\u0435\u043d\u0442\u044b \u0438 \u0440\u0430\u0441\u0448\u0438\u0440\u044f\u0435\u0442 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044e\u0449\u0438\u0435."),

  body("\u0418\u0441\u0442\u043e\u0447\u043d\u0438\u043a \u0432\u0434\u043e\u0445\u043d\u043e\u0432\u0435\u043d\u0438\u044f: obsidian-mind \u0440\u0435\u0430\u043b\u0438\u0437\u0443\u0435\u0442 5 lifecycle hooks (on_session_start, on_write, on_session_end, before_step, after_step), \u0442\u043e\u0433\u0434\u0430 \u043a\u0430\u043a Metalogos \u0438\u043c\u0435\u0435\u0442 \u0442\u043e\u043b\u044c\u043a\u043e 2 (before_pattern, after_pattern). \u041a\u0440\u043e\u043c\u0435 \u0442\u043e\u0433\u043e, obsidian-mind \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442 YAML-manifests, \u0430 config_load \u0432 Metalogos \u0440\u0430\u0431\u043e\u0442\u0430\u0435\u0442 \u0442\u043e\u043b\u044c\u043a\u043e \u0441 JSON. \u041d\u0430\u0440\u044f\u0434 \u0437\u0430\u043a\u0440\u044b\u0432\u0430\u0435\u0442 \u044d\u0442\u0438 \u0440\u0430\u0437\u0440\u044b\u0432\u044b \u043c\u0435\u0436\u0434\u0443 v0.10.0 \u0438 v0.11.0."),

  h2("1.1. \u0427\u0442\u043e \u0443\u0436\u0435 \u0441\u0434\u0435\u043b\u0430\u043d\u043e (v0.10.0)"),

  body("\u0412 v0.10.0 \u0431\u044b\u043b\u0438 \u0434\u043e\u0431\u0430\u0432\u043b\u0435\u043d\u044b \u0442\u0440\u0438 builtin-\u0444\u0443\u043d\u043a\u0446\u0438\u0438, \u0432\u0434\u043e\u0445\u043d\u043e\u0432\u043b\u0451\u043d\u043d\u044b\u0435 \u0430\u0440\u0445\u0438\u0442\u0435\u043a\u0442\u0443\u0440\u043e\u0439 obsidian-mind. semantic_search(query, documents, top_k) \u0440\u0435\u0430\u043b\u0438\u0437\u0443\u0435\u0442 QMD semantic search \u0441 \u043f\u043e\u043c\u043e\u0449\u044c\u044e EmbeddingManager (OpenAI \u0438\u043b\u0438 TF-IDF fallback). config_load(path) \u0437\u0430\u0433\u0440\u0443\u0436\u0430\u0435\u0442 JSON-\u043a\u043e\u043d\u0444\u0438\u0433 \u0432 struct (coordination point pattern). vault_validate(config, required_fields) \u043f\u0440\u043e\u0432\u0435\u0440\u044f\u0435\u0442 \u043d\u0430\u043b\u0438\u0447\u0438\u0435 \u043e\u0431\u044f\u0437\u0430\u0442\u0435\u043b\u044c\u043d\u044b\u0445 \u043f\u043e\u043b\u0435\u0439. \u0412\u0441\u0451 \u0437\u0430\u0440\u0435\u0433\u0438\u0441\u0442\u0440\u0438\u0440\u043e\u0432\u0430\u043d\u043e, \u043f\u0440\u043e\u0442\u0435\u0441\u0442\u0438\u0440\u043e\u0432\u0430\u043d\u043e, CI \u043f\u0440\u043e\u0445\u043e\u0434\u0438\u0442 (cargo build --release --bin mlog)."),

  h2("1.2. \u0427\u0442\u043e \u043e\u0441\u0442\u0430\u043b\u043e\u0441\u044c"),

  body("\u0414\u0432\u0430 \u043a\u043b\u044e\u0447\u0435\u0432\u044b\u0445 \u044d\u043b\u0435\u043c\u0435\u043d\u0442\u0430 obsidian-mind \u043d\u0435 \u0431\u044b\u043b\u0438 \u043f\u043e\u0440\u0442\u0438\u0440\u043e\u0432\u0430\u043d\u044b \u0432 v0.10.0. \u041f\u0435\u0440\u0432\u043e\u0435: \u0440\u0430\u0441\u0448\u0438\u0440\u0435\u043d\u0438\u0435 lifecycle hooks \u0441 2 \u0442\u043e\u0447\u0435\u043a \u0434\u043e 5. \u0412 \u043d\u0430\u0441\u0442\u043e\u044f\u0449\u0435\u0435 \u0432\u0440\u0435\u043c\u044f Metalogos \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442 hook before_pattern { ... } \u0438 hook after_pattern { ... } (ADR-0045), \u043d\u043e \u043d\u0435 \u0438\u043c\u0435\u0435\u0442 \u044d\u043a\u0432\u0438\u0432\u0430\u043b\u0435\u043d\u0442\u043e\u0432 on_session_start, on_write, on_session_end. \u042d\u0442\u043e \u043a\u0440\u0438\u0442\u0438\u0447\u0435\u0441\u043a\u0438\u0439 \u043f\u0440\u043e\u0431\u0435\u043b \u0434\u043b\u044f \u0430\u0433\u0435\u043d\u0442\u043d\u044b\u0445 \u0441\u0446\u0435\u043d\u0430\u0440\u0438\u0435\u0432, \u0433\u0434\u0435 \u0438\u043d\u0438\u0446\u0438\u0430\u043b\u0438\u0437\u0430\u0446\u0438\u044f \u0441\u0435\u0441\u0441\u0438\u0438 \u0438 \u0437\u0430\u043f\u0438\u0441\u044c \u0440\u0435\u0437\u0443\u043b\u044c\u0442\u0430\u0442\u043e\u0432 \u043d\u0435 \u043c\u043e\u0433\u0443\u0442 \u0431\u044b\u0442\u044c \u043f\u0435\u0440\u0435\u0445\u0432\u0430\u0447\u0435\u043d\u044b hooks."),

  body("\u0412\u0442\u043e\u0440\u043e\u0435: \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u043a\u0430 YAML \u0432 config_load. \u0421\u0435\u0439\u0447\u0430\u0441 config_load \u0440\u0430\u0431\u043e\u0442\u0430\u0435\u0442 \u0442\u043e\u043b\u044c\u043a\u043e \u0441 JSON-\u0444\u0430\u0439\u043b\u0430\u043c\u0438. Obsidian-mind \u0438\u0441\u043f\u043e\u043b\u044c\u0437\u0443\u0435\u0442 vault-manifest.json, \u043d\u043e YAML \u0448\u0438\u0440\u043e\u043a\u043e \u0438\u0441\u043f\u043e\u043b\u044c\u0437\u0443\u0435\u0442\u0441\u044f \u0432 AI/agent-\u0441\u043e\u043e\u0431\u0449\u0435\u0441\u0442\u0432\u0430\u0445 \u0434\u043b\u044f \u043a\u043e\u043d\u0444\u0438\u0433\u0443\u0440\u0430\u0446\u0438\u0438. \u0414\u043b\u044f \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u043a\u0438 YAML \u043d\u0435\u043e\u0431\u0445\u043e\u0434\u0438\u043c\u043e \u0434\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u0437\u0430\u0432\u0438\u0441\u0438\u043c\u043e\u0441\u0442\u044c serde_yaml \u0432 Cargo.toml."),

  // ── 2. Блок A: Lifecycle hooks ──
  h1("2. \u0411\u043b\u043e\u043a A: \u0420\u0430\u0441\u0448\u0438\u0440\u0435\u043d\u0438\u0435 lifecycle hooks (2 \u2192 5)"),

  h2("2.1. \u0422\u0435\u043a\u0443\u0449\u0435\u0435 \u0441\u043e\u0441\u0442\u043e\u044f\u043d\u0438\u0435"),

  body("Metalogos v0.10.0 \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442 \u0434\u0432\u0430 hook-\u0442\u043e\u0447\u043a\u0438 \u0432\u044b\u0437\u043e\u0432\u0430, \u043e\u043f\u0440\u0435\u0434\u0435\u043b\u0451\u043d\u043d\u044b\u0445 ADR-0045. \u041e\u043d\u0438 \u043e\u0431\u044a\u044f\u0432\u043b\u044f\u044e\u0442\u0441\u044f \u0432 .mlog-\u0444\u0430\u0439\u043b\u0435 \u0441\u0438\u043d\u0442\u0430\u043a\u0441\u0438\u0441\u043e\u043c hook before_pattern { ... } \u0438 hook after_pattern { ... }. \u0412 AST \u044d\u0442\u043e \u043f\u0440\u0435\u0434\u0441\u0442\u0430\u0432\u043b\u0435\u043d\u043e enum HookPhase { BeforePattern, AfterPattern } \u0438 struct HookDecl { phase, body }. \u0418\u043d\u0442\u0435\u0440\u043f\u0440\u0435\u0442\u0430\u0442\u043e\u0440 \u0445\u0440\u0430\u043d\u0438\u0442 hooks \u0432 Vec<HookDecl> \u0438 \u0432\u044b\u0437\u044b\u0432\u0430\u0435\u0442 \u0432\u0441\u0435 before-hooks \u043f\u0435\u0440\u0435\u0434 \u0438 \u0432\u0441\u0435 after-hooks \u043f\u043e\u0441\u043b\u0435 \u043a\u0430\u0436\u0434\u043e\u0433\u043e \u0432\u044b\u0437\u043e\u0432\u0430 pattern. Grammar \u0441\u043e\u0434\u0435\u0440\u0436\u0438\u0442 \u0442\u043e\u043b\u044c\u043a\u043e \u0434\u0432\u0430 \u043a\u043b\u044e\u0447\u0435\u0432\u044b\u0445 \u0441\u043b\u043e\u0432\u0430: BEFORE_PATTERN_KW \u0438 AFTER_PATTERN_KW."),

  h2("2.2. \u0426\u0435\u043b\u0435\u0432\u043e\u0435 \u0441\u043e\u0441\u0442\u043e\u044f\u043d\u0438\u0435 (5 hooks)"),

  body("\u041d\u0430\u0440\u044f\u0434 \u0440\u0430\u0441\u0448\u0438\u0440\u044f\u0435\u0442 HookPhase \u0434\u043e \u043f\u044f\u0442\u0438 \u0432\u0430\u0440\u0438\u0430\u043d\u0442\u043e\u0432, \u0430\u043d\u0430\u043b\u043e\u0433\u0438\u0447\u043d\u043e \u043f\u044f\u0442\u0438 \u043b\u0438\u0444\u0446\u0438\u043a\u043b-\u0442\u043e\u0447\u043a\u0430\u043c obsidian-mind. \u041f\u0435\u0440\u0432\u044b\u0435 \u0434\u0432\u0430 \u0443\u0436\u0435 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044e\u0442 (BeforePattern, AfterPattern \u0441\u043e\u043e\u0442\u0432\u0435\u0442\u0441\u0442\u0432\u0443\u044e\u0442 obsidian-mind before_step/after_step). \u0422\u0440\u0438 \u043d\u043e\u0432\u044b\u0445 \u0434\u043e\u0431\u0430\u0432\u043b\u044f\u044e\u0442 session-level \u0442\u043e\u0447\u043a\u0438."),

  buildTable(
    ["\u0422\u043e\u0447\u043a\u0430", "\u041e\u0431\u044a\u044f\u0432\u043b\u0435\u043d\u0438\u0435 .mlog", "\u041c\u043e\u043c\u0435\u043d\u0442 \u0432\u044b\u0437\u043e\u0432\u0430", "\u041e\u0431\u044a\u0435\u043c \u0434\u043e\u0441\u0442\u0443\u043f\u043d\u044b\u0435 \u043f\u0435\u0440\u0435\u043c\u0435\u043d\u043d\u044b\u0435"],
    [
      ["on_session_start", "hook on_session_start { ... }", "\u041f\u0440\u0438 \u0441\u0442\u0430\u0440\u0442\u0435 \u0438\u043d\u0442\u0435\u0440\u043f\u0440\u0435\u0442\u0430\u0442\u043e\u0440\u0430 / mlog serve", "session_id, config"],
      ["on_write", "hook on_write { ... }", "\u041f\u0435\u0440\u0435\u0434 \u043a\u0430\u0436\u0434\u044b\u043c \u0437\u0430\u043f\u0438\u0441\u044c\u044e \u0432 KV/DB", "key, value, target (\"kv\" / \"db\")"],
      ["on_session_end", "hook on_session_end { ... }", "\u041f\u0440\u0438 \u0437\u0430\u0432\u0435\u0440\u0448\u0435\u043d\u0438\u0438 \u0441\u0435\u0441\u0441\u0438\u0438 / shutdown", "session_id, duration_ms"],
      ["before_pattern", "hook before_pattern { ... } (\u0441\u0443\u0449\u0435\u0441\u0442\u0432.)", "\u041f\u0435\u0440\u0435\u0434 \u0432\u044b\u0437\u043e\u0432\u043e\u043c \u043b\u044e\u0431\u043e\u0433\u043e pattern", "pattern_name, args"],
      ["after_pattern", "hook after_pattern { ... } (\u0441\u0443\u0449\u0435\u0441\u0442\u0432.)", "\u041f\u043e\u0441\u043b\u0435 \u0432\u043e\u0437\u0432\u0440\u0430\u0442\u0430 pattern", "pattern_name, args, result, confidence"],
    ]
  ),

  spacer(200),

  h2("2.3. \u0418\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u044f \u043f\u043e \u0444\u0430\u0439\u043b\u0430\u043c"),

  buildTable(
    ["\u0424\u0430\u0439\u043b", "\u0418\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u0435"],
    [
      ["src/grammar.pest", "\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c ON_SESSION_START_KW, ON_WRITE_KW, ON_SESSION_END_KW \u0432 hook_kind; \u0434\u043e\u0431\u0430\u0432\u0438\u0442\u044c 3 \u043d\u043e\u0432\u044b\u0445 \u0441\u043b\u043e\u0432\u0430 \u0432 step_ident exclusions"],
      ["src/ast.rs", "\u0420\u0430\u0441\u0448\u0438\u0440\u0438\u0442\u044c HookPhase \u0434\u043e 5 \u0432\u0430\u0440\u0438\u0430\u043d\u0442\u043e\u0432"],
      ["src/parser.rs", "\u041e\u0431\u0440\u0430\u0431\u0430\u0442\u044b\u0432\u0430\u0442\u044c 3 \u043d\u043e\u0432\u044b\u0435 \u043a\u043b\u044e\u0447\u0435\u0432\u044b\u0435 \u0441\u043b\u043e\u0432\u0430 \u0432 parse_hook_decl"],
      ["src/compiler.rs", "\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u043d\u043e\u0432\u044b\u0435 HookPhase \u0432 catch-all arms (pass1 + pass2)"],
      ["src/interpreter.rs", "\u0420\u0430\u0441\u0448\u0438\u0440\u0438\u0442\u044c \u0445\u0440\u0430\u043d\u0438\u043b\u0438\u0449\u0435 hooks (hooks_before/hooks_after \u2192 hooks \u043f\u043e \u0442\u0438\u043f\u0430\u043c); \u0434\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u0432\u044b\u0437\u043e\u0432 \u0441\u0435\u0441\u0441\u0438\u043e\u043d\u043d\u044b\u0445 hooks"],
      ["src/vm.rs", "\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u043d\u043e\u0432\u044b\u0435 HookPhase \u0432 catch-all arms"],
      ["src/semantic.rs", "\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u043d\u043e\u0432\u044b\u0435 HookPhase \u0432 catch-all arms"],
    ]
  ),

  spacer(200),

  h2("2.4. \u041f\u0440\u0435\u0434\u0443\u0441\u043b\u043e\u0432\u0438\u044f"),

  body("\u0420\u0430\u0431\u043e\u0447\u0430\u044f \u043a\u043e\u043f\u0438\u044f \u043d\u0430 \u0432\u0435\u0442\u0432\u0435 main \u0438 \u0441\u0442\u0430\u0442\u0443\u0441. \u041a\u043e\u0434 \u043d\u0435 \u0438\u0437\u043c\u0435\u043d\u0435\u043d, \u0432\u0441\u0435 \u0442\u0435\u0441\u0442\u044b \u043f\u0440\u043e\u0445\u043e\u0434\u044f\u0442. CI: cargo build --release --bin mlog. \u041e\u0431\u0440\u0430\u0442\u043d\u0430\u044f \u0441\u043e\u0432\u043c\u0435\u0441\u0442\u0438\u043c\u043e\u0441\u0442\u044c: \u043d\u043e\u0432\u044b\u0435 hook \u0442\u043e\u0447\u043a\u0438 \u043d\u0435 \u0432\u044b\u0437\u044b\u0432\u0430\u044e\u0442\u0441\u044f \u0435\u0441\u043b\u0438 \u043d\u0435 \u043e\u0431\u044a\u044f\u0432\u043b\u0435\u043d\u044b \u0432 .mlog-\u0444\u0430\u0439\u043b\u0435 \u2014 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044e\u0449\u0438\u0435 .mlog-\u0444\u0430\u0439\u043b\u044b \u0440\u0430\u0431\u043e\u0442\u0430\u044e\u0442 \u0431\u0435\u0437 \u0438\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u0439."),

  h2("2.5. \u041f\u0440\u043e\u0432\u0435\u0440\u043a\u0430"),

  body("\u041f\u043e\u0441\u043b\u0435 \u0440\u0435\u0430\u043b\u0438\u0437\u0430\u0446\u0438\u0438: (1) cargo build --release --bin mlog \u2014 \u043a\u043e\u043c\u043f\u0438\u043b\u044f\u0446\u0438\u044f \u0431\u0435\u0437 \u043e\u0448\u0438\u0431\u043e\u043a; (2) \u0441\u043e\u0437\u0434\u0430\u0442\u044c test .mlog-\u0444\u0430\u0439\u043b examples/hooks_lifecycle.mlog \u0441 \u0432\u0441\u0435\u043c\u0438 5 \u0442\u043e\u0447\u043a\u0430\u043c\u0438; (3) mlog run examples/hooks_lifecycle.mlog \u2014 \u043f\u0440\u043e\u0432\u0435\u0440\u0438\u0442\u044c \u043f\u043e\u0440\u044f\u0434\u043e\u043a \u0432\u044b\u0437\u043e\u0432\u0430 \u043f\u043e \u043b\u043e\u0433\u0430\u043c; (4) \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044e\u0449\u0438\u0435 .mlog-\u0444\u0430\u0439\u043b\u044b FOSVED-office-v2 \u0441\u0442\u0430\u0440\u0442\u0443\u044e\u0442 \u0431\u0435\u0437 \u0438\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u0439 (\u043e\u0431\u0440\u0430\u0442\u043d\u0430\u044f \u0441\u043e\u0432\u043c\u0435\u0441\u0442\u0438\u043c\u043e\u0441\u0442\u044c)."),

  h2("2.6. \u041f\u0440\u0430\u0432\u0438\u043b\u0430 \u043e\u0442\u043a\u0430\u0442\u0430"),

  body("\u0415\u0441\u043b\u0438 cargo build \u043d\u0435 \u0441\u043e\u0431\u0438\u0440\u0430\u0435\u0442\u0441\u044f \u0438\u043b\u0438 CI \u043f\u0430\u0434\u0430\u0435\u0442 \u2014 git revert \u0432\u0441\u0435\u0445 \u043a\u043e\u043c\u043c\u0438\u0442\u043e\u0432 \u0411\u043b\u043e\u043a\u0430 A. \u0415\u0441\u043b\u0438 FOSVED-office-v2 \u043d\u0435 \u0441\u0442\u0430\u0440\u0442\u0443\u0435\u0442 \u043d\u0430 \u043f\u0440\u043e\u0434\u0435 \u2014 \u043e\u0442\u043a\u0430\u0442\u0438\u0442\u044c \u043d\u0430\u0440\u044f\u0434 \u0438 \u0434\u0438\u0430\u0433\u043d\u043e\u0441\u0442\u0438\u0440\u043e\u0432\u0430\u0442\u044c \u0440\u0435\u0433\u0440\u0435\u0441\u0441\u0438\u044e. \u041f\u0440\u0438 \u0447\u0438\u0441\u043b\u043e\u043c \u043e\u0442\u043a\u0430\u0442\u0435 \u2014 \u0441\u043e\u0437\u0434\u0430\u0442\u044c \u0432\u0435\u0442\u0432\u0443 \u0438\u0437 \u043f\u043e\u0441\u043b\u0435\u0434\u043d\u0435\u0433\u043e \u0440\u0430\u0431\u043e\u0447\u0435\u0433\u043e commit."),

  // ── 3. Блок B: load_config YAML ──
  h1("3. \u0411\u043b\u043e\u043a B: config_load \u2014 \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u043a\u0430 YAML"),

  h2("3.1. \u0422\u0435\u043a\u0443\u0449\u0435\u0435 \u0441\u043e\u0441\u0442\u043e\u044f\u043d\u0438\u0435"),

  body("config_load(path) \u0432 v0.10.0 \u0437\u0430\u0433\u0440\u0443\u0436\u0430\u0435\u0442 \u0444\u0430\u0439\u043b, \u043f\u0430\u0440\u0441\u0438\u0442 \u0435\u0433\u043e \u043a\u0430\u043a JSON (serde_json::from_str) \u0438 \u043a\u043e\u043d\u0432\u0435\u0440\u0442\u0438\u0440\u0443\u0435\u0442 \u0432 Metalogos struct \u0441 \u043f\u043e\u043c\u043e\u0449\u044c\u044e json_value_to_mlog_value_with_type. \u0422\u0438\u043f struct'\u0430 \u0431\u0435\u0440\u0451\u0442\u0441\u044f \u0438\u0437 \u0438\u043c\u0435\u043d\u0438 \u0444\u0430\u0439\u043b\u0430 (stem). \u0417\u0430\u0432\u0438\u0441\u0438\u043c\u043e\u0441\u0442\u044c: serde (serde_json), std::fs, std::path. \u041e\u0442\u0441\u0443\u0442\u0441\u0442\u0432\u0438\u0435 serde_yaml \u0432 Cargo.toml \u043e\u0437\u043d\u0430\u0447\u0430\u0435\u0442, \u0447\u0442\u043e YAML-\u0444\u0430\u0439\u043b\u044b \u043d\u0435 \u043f\u043e\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u044e\u0442\u0441\u044f."),

  h2("3.2. \u0426\u0435\u043b\u0435\u0432\u043e\u0435 \u0441\u043e\u0441\u0442\u043e\u044f\u043d\u0438\u0435"),

  body("\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c \u0430\u0432\u0442\u043e\u043c\u0430\u0442\u0438\u0447\u0435\u0441\u043a\u043e\u0435 \u043e\u043f\u0440\u0435\u0434\u0435\u043b\u0435\u043d\u0438\u0435 \u0444\u043e\u0440\u043c\u0430\u0442\u0430 \u043f\u043e \u0440\u0430\u0441\u0448\u0438\u0440\u0435\u043d\u0438\u044e \u0444\u0430\u0439\u043b\u0430 (.json / .yaml / .yml). \u0415\u0441\u043b\u0438 \u0444\u0430\u0439\u043b \u043e\u043a\u0430\u043d\u0447\u0438\u0432\u0430\u0435\u0442\u0441\u044f \u043d\u0430 .yaml \u0438\u043b\u0438 .yml \u2014 \u043f\u0430\u0440\u0441\u0438\u0442\u044c \u043a\u0430\u043a YAML, \u0438\u043d\u0430\u0447\u0435 \u043a\u0430\u043a JSON. \u042d\u0442\u043e \u043f\u043e\u0437\u0432\u043e\u043b\u044f\u0435\u0442 Metalogos-\u043f\u0440\u043e\u0433\u0440\u0430\u043c\u043c\u0430\u043c \u0438\u0441\u043f\u043e\u043b\u044c\u0437\u043e\u0432\u0430\u0442\u044c YAML-manifests, \u043a\u043e\u0442\u043e\u0440\u044b\u0435 \u0448\u0438\u0440\u043e\u043a\u043e \u0440\u0430\u0441\u043f\u0440\u043e\u0441\u0442\u0440\u0430\u043d\u0435\u043d\u044b \u0432 AI/agent-\u0441\u043e\u043e\u0431\u0449\u0435\u0441\u0442\u0432\u0430\u0445."),

  h2("3.3. \u0418\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u044f \u043f\u043e \u0444\u0430\u0439\u043b\u0430\u043c"),

  buildTable(
    ["\u0424\u0430\u0439\u043b", "\u0418\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u0435"],
    [
      ["Cargo.toml", "\u0414\u043e\u0431\u0430\u0432\u0438\u0442\u044c serde_yaml = \"0.9\" (\u0431\u043e\u043b\u0435\u0435 \u043d\u0435 \u0430\u043a\u0442\u0443\u0430\u043b\u044c\u043d\u0430\u044f \u0437\u0430\u0432\u0438\u0441\u0438\u043c\u043e\u0441\u0442\u044c, \u0441\u043e\u0432\u043c\u0435\u0441\u0442\u0438\u043c\u043e \u0441 rustc 1.75+)"],
      ["src/builtins.rs", "\u041c\u043e\u0434\u0438\u0444\u0438\u0446\u0438\u0440\u043e\u0432\u0430\u0442\u044c builtin_config_load: \u043e\u043fр\u0435\u0434\u0435\u043b\u0438\u0442\u044c \u0444\u043e\u0440\u043c\u0430\u0442 \u043f\u043e \u0440\u0430\u0441\u0448\u0438\u0440\u0435\u043d\u0438\u044e, \u0434\u0435\u043b\u0435\u0433\u0438\u0440\u043eв\u0430ть JSON/YAML \u0432 \u043eтдельные helper'\u044b"],
      ["examples/config_yaml.mlog", "\u0422\u0435\u0441\u0442: \u0437\u0430\u0433рузка YAML-manifest \u0438 \u0432алидация \u043fолей"],
    ]
  ),

  spacer(200),

  h2("3.4. \u041fредусловия"),

  body("serde_yaml \u0434олжна \u043aомпилироваться с текущим toolchain (rustc 1.75+, так как используется стабильная версия, не nightly). Если сериализация не нужна, можно ограничиться YAML без serde_yaml, просто парсив YAML в JSON (для простых манифестов это достаточно). Всё зависит от решения о том, нужна ли полная YAML-сериализация или достаточно конвертации YAML→JSON."),

  // ── 4. Блок D: Документация и сборка ──
  h1("4. \u0411\u043b\u043e\u043a D: \u0414\u043eк\u0443м\u0435\u043d\u0442\u0430ция, README, сборка"),

  h2("4.1. CHANGELOG.md"),

  body("\u0414обавить \u0441екци\u044e [0.11.0] \u0441 \u043eписанием \u0432сех \u0438зменени\u0439. \u0424орм\u0430т \u043f\u043e \u0430н\u0430лог\u0438\u0438 \u0441 с\u0443\u0449ес\u0442\u0432ую\u0449\u0438\u043c\u0438 \u0437а\u043fи\u0441\u044f\u043c\u0438. \u0423к\u0430з\u0430ть \u0438ст\u043e\u0447н\u0438\u043a \u0432дохновени\u044f (obsidian-mind, MIT \u2014 \u043aод \u041d\u0415 \u043aопиров\u0430л\u0441\u044f). \u041fер\u0435числить \u0432с\u0435 \u0438зм\u0435\u043d\u0451нны\u0435 \u0444айл\u044b."),

  h2("4.2. README.md"),

  body("\u041eбновить секц\u0438\u044e \u043e builtin-фун\u043aц\u0438\u044f\u0445. \u0414\u043eб\u0430\u0432\u0438\u0442\u044c \u043eпис\u0430\u043d\u0438\u0435 5 lifecycle hooks \u0441 \u043fрим\u0435р\u0430\u043c\u0438 \u0441и\u043d\u0442\u0430ксис\u0430. \u041eбнови\u0442ь config_load \u2014 \u0443к\u0430\u0437\u0430\u0442\u044c \u043fоддер\u0436\u043a\u0443 JSON \u0438 YAML. \u041eбн\u043eв\u0438\u0442\u044c верс\u0438\u044e н\u0430 0.11.0. \u0422\u0435\u043aущий README \u2014 306 стр\u043e\u043a, \u043d\u0443жн\u043e у\u0447\u0438\u0442\u044b\u0432\u0430\u0442\u044c о\u0431ъ\u0451\u043c."),

  h2("4.3. ADR"),

  body("\u0421\u043eз\u0434\u0430\u0442\u044c docs/adr/0064-obsidian-mind-lifecycle-hooks.md \u2014 \u0430р\u0445ит\u0435\u043a\u0442\u0443р\u043dо\u0435 решени\u0435 о р\u0430сшир\u0435\u043dи\u0438 hooks с 2 \u0434\u043e 5, а\u043d\u0430\u043b\u043e\u0433\u0438\u044f \u0441 obsidian-mind, о\u0431осн\u043e\u0432\u0430\u043d\u0438\u0435 \u0432\u044b\u0431\u043e\u0440\u0430 \u043f\u043e\u0434\u0445\u043e\u0434\u0430. \u0421\u043eз\u0434\u0430\u0442\u044c docs/adr/0065-config-load-yaml.md \u2014 р\u0435ш\u0435\u043d\u0438\u0435 о добавле\u043dи\u0438 YAML-под\u0434ер\u0436\u043a\u0438. Фор\u043c\u0430т \u043f\u043e \u0430\u043d\u0430\u043b\u043e\u0433\u0438\u0438 с сущ\u0435ст\u0432\u0443ю\u0449\u0438\u043cи ADR."),

  h2("4.4. Сборк\u0430 \u043aомпилят\u043eр\u0430"),

  body("cargo build --release --bin mlog. Резуль\u0442\u0438р\u0443ющ\u0438\u0439 би\u043d\u0430р\u043dы\u0439 \u0444\u0430\u0439\u043b target/release/mlog \u2014 \u043dов\u044b\u0439 ком\u043fи\u043b\u044f\u0442\u043eр v0.11.0. \u0423\u0431\u0435\u0434\u0438\u0442ь\u0441\u044f \u0447т\u043e cargo build успе\u0448\u043d\u043e \u043d\u0430 т\u0435\u043aу\u0449\u0435\u0439 коп\u0438\u0438 \u043a\u043e\u0434\u0430 (main \u043d\u0430 \u0432\u0435т\u0432\u0435). CI \u043f\u0440\u043e\u0445\u043e\u0434\u0438\u0442 а\u0432\u0442\u043e\u043c\u0430\u0442\u0438\u0447\u0435\u0441\u043a\u0438 (.github/workflows/build.yml \u043d\u0435 \u043c\u0435\u043d\u044f\u0435\u0442\u0441\u044f)."),

  // ── 5. Порядок выполнения ──
  h1("5. \u041f\u043e\u0440\u044f\u0434\u043e\u043a \u0432ы\u043fо\u043b\u043d\u0435\u043d\u0438\u044f"),

  body("\u0411\u043b\u043e\u043a\u0438 A \u0438 B \u043cог\u0443т выпо\u043b\u043d\u044f\u0442\u044c\u0441\u044f \u043fар\u0430ллельн\u043e, \u043fо\u0441\u043a\u043eль\u043aу он\u0438 \u043d\u0435 \u0437\u0430вис\u044f\u0442 \u0434р\u0443\u0433 \u043eт \u0434р\u0443\u0433\u0430. \u0411ло\u043a D (докум\u0435\u043d\u0442\u0430ци\u044f) \u2014 \u043fосл\u0435. \u041aажды\u0439 \u0431л\u043e\u043a \u043aом\u043cи\u0442\u0438\u0442с\u044f \u043eтде\u043bь\u043d\u043e, \u043fровер\u044fе\u0442с\u044f, \u0442\u043eль\u043a\u043e после \u0443с\u043f\u0435\u0448\u043dой cargo build. \u041f\u043e\u0441\u043b\u0435 \u0432с\u0435\u0445 \u0431л\u043e\u043a\u043e\u0432 \u2014 \u0435\u0434\u0438\u043d\u044b\u0439 commit, \u043eбн\u043eв\u043b\u0435\u043d\u0438\u0435 верс\u0438\u0438 \u0432 Cargo.toml, push \u043d\u0430 main."),

  buildTable(
    ["\u0428\u0430\u0433", "\u0411\u043b\u043e\u043a", "\u0420\u0435\u0437\u0443\u043b\u044c\u0442\u0430\u0442"],
    [
      ["1", "\u0411\u043b\u043e\u043a A: grammar.pest", "3 \u043dовы\u0445 к\u043b\u044e\u0447\u0435\u0432ы\u0445 с\u043bов\u0430 + step_ident exclusions"],
      ["2", "\u0411\u043b\u043e\u043a A: ast.rs", "HookPhase \u0440\u0430\u0441\u0448\u0438\u0440\u0435\u043d \u0434\u043e 5 в\u0430\u0440\u0438\u0430\u043d\u0442\u043e\u0432"],
      ["3", "\u0411\u043b\u043e\u043a A: parser.rs", "3 \u043d\u043eвы\u0445 к\u043b\u044e\u0447\u0435\u0432ы\u0445 с\u043b\u043e\u0432\u0430"],
      ["4", "\u0411\u043b\u043e\u043a A: compiler/vm/semantic.rs", "\u041dовы\u0435 HookPhase \u0432 catch-all arms"],
      ["5", "\u0411\u043b\u043e\u043a A: interpreter.rs", "\u0412ызов session hooks"],
      ["6", "\u041fровер\u043a\u0430 A", "cargo build + test .mlog"],
      ["7", "\u0411\u043b\u043e\u043a B: Cargo.toml + builtins.rs", "serde_yaml + моди\u0444ик\u0430ци\u044f config_load"],
      ["8", "\u041fровер\u043a\u0430 B", "cargo build + test YAML"],
      ["9", "\u0411\u043b\u043e\u043a D: док\u0443м\u0435н\u0442\u0430ци\u044f", "CHANGELOG, README, ADR"],
      ["10", "\u0421\u0431орк\u0430 + commit + push", "v0.11.0, cargo build --release --bin mlog"],
    ]
  ),

  // ── 6. Отчёт ──
  h1("6. \u0424ор\u043c\u0430т \u043eтч\u0451\u0442\u0430"),

  body("\u041fосл\u0435 з\u0430вершени\u044f \u043d\u0430ряд\u0430 \u043eтч\u0451\u0442 \u0434ол\u0436ен \u0441одер\u0436\u0430\u0442ь: (1) commit hash \u0438 \u0441сылк\u0430 \u043d\u0430 push; (2) список \u0438змен\u0451нных \u0444а\u0439лов; (3) результат cargo build --release; (4) нов\u0430\u044f верс\u0438\u044f. Отч\u0451т сост\u0430вляетс\u044f \u0432 т\u0435\u043aст\u043eвом сообщ\u0435ни\u0438 (\u043d\u0435 \u043a\u0430\u043a .docx \u0434ок\u0443\u043c\u0435\u043d\u0442)."),

  // ── 7. Зависимости от O-1 ──
  h1("7. \u0417\u0430в\u0438\u0441\u0438\u043c\u043e\u0441\u0442\u0438 от O-1"),

  body("\u041d\u0430р\u044f\u0434 O-2 \u043d\u0435 з\u0430висит от O-1. O-1 (п\u043e\u0440\u0442-race fix) \u043f\u043e\u0434\u0447\u0438\u043d\u044f\u043b\u0441\u044f, \u043d\u043e е\u0433\u043e \u0434\u0435п\u043b\u043e\u0439 \u0432ы\u044f\u0432\u0438л Telegram 404 \u2014 \u044d\u0442\u043e \u043e\u0442\u0434\u0435\u043bь\u043d\u0430\u044f \u043f\u0440\u043e\u0431\u043b\u0435\u043c\u0430, \u043d\u0435 \u0441\u0432\u044f\u0437\u0430\u043d\u043d\u0430\u044f \u0441 н\u0430\u0440\u044f\u0434\u043e\u043c O-2. O-2 \u0440\u0430\u0431\u043e\u0442\u0430\u0435\u0442 с \u0447ист\u043e\u0439 \u043a\u043e\u043f\u0438\u0435\u0439 main \u043d\u0430 \u0432\u0435\u0442\u0432\u0435 (7ce6c54). \u0415сл\u0438 O-1 \u0431\u0443д\u0435\u0442 о\u0442к\u0430\u0442\u0451\u043d, git revert \u043d\u0435 \u043f\u043eв\u043b\u0438\u044f\u0435т \u043d\u0430 O-2."),
];

// ══════════════════════════════════════════════════════════════
// BUILD DOCUMENT
// ══════════════════════════════════════════════════════════════

const doc = new Document({
  styles: {
    default: {
      document: {
        run: {
          font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
          size: 24,
          color: c(P.body),
        },
        paragraph: { spacing: { line: 312 } },
      },
      heading1: {
        run: {
          font: { ascii: "Calibri", eastAsia: "SimHei" },
          size: 32,
          bold: true,
          color: c(P.primary),
        },
      },
      heading2: {
        run: {
          font: { ascii: "Calibri", eastAsia: "SimHei" },
          size: 28,
          bold: true,
          color: c(P.primary),
        },
      },
      heading3: {
        run: {
          font: { ascii: "Calibri", eastAsia: "SimHei" },
          size: 26,
          bold: true,
          color: c(P.secondary),
        },
      },
    },
  },
  sections: [
    // Cover section
    {
      properties: {
        page: {
          size: { width: 11906, height: 16838 },
          margin: { top: 0, bottom: 0, left: 0, right: 0 },
        },
      },
      children: buildCoverR1(coverConfig),
    },
    // Body section
    {
      properties: {
        type: SectionType.NEXT_PAGE,
        page: {
          size: { width: 11906, height: 16838 },
          margin: { top: 1440, bottom: 1440, left: 1701, right: 1417 },
          pageNumbers: { start: 1, formatType: "decimal" },
        },
      },
      headers: {
        default: new Header({
          children: [
            new Paragraph({
              alignment: AlignmentType.RIGHT,
              children: [
                new TextRun({
                  text: "\u041d\u0430\u0440\u044f\u0434 O-2 | obsidian-mind \u2192 Metalogos",
                  size: 16,
                  color: c(P.secondary),
                  font: { ascii: "Calibri" },
                }),
              ],
            }),
          ],
        }),
      },
      footers: {
        default: new Footer({
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [
                new TextRun({
                  children: [PageNumber.CURRENT],
                  size: 18,
                  color: c(P.secondary),
                }),
              ],
            }),
          ],
        }),
      },
      children: bodyContent,
    },
  ],
});

const OUT = "/home/z/my-project/download/Naryad_O-2_obsidian-mind_Metalogos.docx";
Packer.toBuffer(doc).then((buf) => {
  fs.writeFileSync(OUT, buf);
  console.log("OK: " + OUT);
});
