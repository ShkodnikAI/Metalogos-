const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, PageNumber, NumberFormat, AlignmentType, HeadingLevel,
  WidthType, BorderStyle, ShadingType, PageBreak, TableOfContents, LevelFormat,
} = require("docx");
const fs = require("fs");

// ── Palette: GO-1 Graphite Orange (proposal / plan) ──
const P = {
  primary: "1A2330",
  body: "000000",
  secondary: "607080",
  accent: "D4875A",
  surface: "F8F0EB",
  table: {
    headerBg: "D4875A",
    headerText: "1A1A1A",
    accentLine: "D4875A",
    innerLine: "DDD0C8",
    surface: "F8F0EB",
  },
};

const c = (hex) => hex.replace("#", "");
const NB = { style: BorderStyle.NONE, size: 0, color: "FFFFFF" };
const allNoBorders = { top: NB, bottom: NB, left: NB, right: NB, insideHorizontal: NB, insideVertical: NB };
const noBordersLR = { top: NB, bottom: NB, left: NB, right: NB };

// ── Helper: horizontal table ──
function hTable(headers, rows, colWidths) {
  const totalWidth = colWidths.reduce((a, b) => a + b, 0);
  return new Table({
    width: { size: 100, type: WidthType.PERCENTAGE },
    borders: {
      top: { style: BorderStyle.SINGLE, size: 2, color: P.table.accentLine },
      bottom: { style: BorderStyle.SINGLE, size: 2, color: P.table.accentLine },
      left: { style: BorderStyle.NONE },
      right: { style: BorderStyle.NONE },
      insideHorizontal: { style: BorderStyle.SINGLE, size: 1, color: P.table.innerLine },
      insideVertical: { style: BorderStyle.NONE },
    },
    rows: [
      new TableRow({
        tableHeader: true,
        cantSplit: true,
        children: headers.map((text, i) =>
          new TableCell({
            width: { size: colWidths[i], type: WidthType.PERCENTAGE },
            shading: { type: ShadingType.CLEAR, fill: P.table.headerBg },
            margins: { top: 60, bottom: 60, left: 120, right: 120 },
            children: [new Paragraph({ children: [new TextRun({ text, bold: true, size: 21, color: P.table.headerText, font: { ascii: "Calibri", eastAsia: "SimHei" } })] })],
          })
        ),
      }),
      ...rows.map(
        (row, ri) =>
          new TableRow({
            cantSplit: true,
            children: row.map((text, i) =>
              new TableCell({
                width: { size: colWidths[i], type: WidthType.PERCENTAGE },
                shading: ri % 2 === 0 ? { type: ShadingType.CLEAR, fill: P.table.surface } : { type: ShadingType.CLEAR, fill: "FFFFFF" },
                margins: { top: 60, bottom: 60, left: 120, right: 120 },
                children: [new Paragraph({ children: [new TextRun({ text, size: 21, color: "1A1A1A", font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" } })] })],
              })
            ),
          })
      ),
    ],
  });
}

// ── Helper functions ──
function h1(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { before: 400, after: 200, line: 312 },
    children: [new TextRun({ text, bold: true, size: 32, color: P.primary, font: { ascii: "Calibri", eastAsia: "SimHei" } })],
  });
}
function h2(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_2,
    spacing: { before: 300, after: 150, line: 312 },
    children: [new TextRun({ text, bold: true, size: 28, color: P.primary, font: { ascii: "Calibri", eastAsia: "SimHei" } })],
  });
}
function h3(text) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_3,
    spacing: { before: 240, after: 100, line: 312 },
    children: [new TextRun({ text, bold: true, size: 24, color: P.primary, font: { ascii: "Calibri", eastAsia: "SimHei" } })],
  });
}
function body(text) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    spacing: { line: 312, after: 80 },
    children: [new TextRun({ text, size: 24, color: P.body, font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" } })],
  });
}
function bodyRuns(runs) {
  return new Paragraph({
    alignment: AlignmentType.JUSTIFIED,
    spacing: { line: 312, after: 80 },
    children: runs,
  });
}
function codeBlock(text) {
  return new Paragraph({
    spacing: { before: 60, after: 60, line: 280 },
    indent: { left: 400 },
    shading: { type: ShadingType.CLEAR, fill: "F0F0F0" },
    children: [new TextRun({ text, size: 20, font: { ascii: "Consolas", eastAsia: "Microsoft YaHei" }, color: "333333" })],
  });
}
function emptyLine() {
  return new Paragraph({ spacing: { after: 60 }, children: [] });
}

// ── Cover page builder (R4: Top Color Block) ──
function buildCover() {
  return {
    properties: {
      page: {
        size: { width: 11906, height: 16838 },
        margin: { top: 0, bottom: 0, left: 0, right: 0 },
      },
    },
    children: [
      // Top color block
      new Table({
        width: { size: 100, type: WidthType.PERCENTAGE },
        borders: allNoBorders,
        rows: [
          new TableRow({
            height: { value: 4200, rule: "exact" },
            verticalAlign: "top",
            children: [
              new TableCell({
                width: { size: 100, type: WidthType.PERCENTAGE },
                shading: { type: ShadingType.CLEAR, fill: P.accent },
                borders: allNoBorders,
                margins: { top: 0, bottom: 0, left: 1200, right: 1200 },
                children: [
                  new Paragraph({ spacing: { before: 1200 }, children: [] }),
                  new Paragraph({
                    spacing: { before: 200, after: 100, line: 600 },
                    children: [new TextRun({ text: "METALOGOS", size: 64, bold: true, color: "FFFFFF", font: { ascii: "Calibri" } })],
                  }),
                  new Paragraph({
                    spacing: { after: 100, line: 400 },
                    children: [new TextRun({ text: "FIX PLAN", size: 44, bold: true, color: "FFFFFF", font: { ascii: "Calibri" } })],
                  }),
                ],
              }),
            ],
          }),
        ],
      }),
      // Subtitle area
      new Paragraph({ spacing: { before: 800 }, children: [] }),
      new Paragraph({
        indent: { left: 1200, right: 1200 },
        spacing: { after: 200, line: 360 },
        children: [
          new TextRun({
            text: "\u041F\u043B\u0430\u043D \u0434\u043E\u0440\u0430\u0431\u043E\u0442\u043A\u0438 \u044F\u0437\u044B\u043A\u0430 \u0434\u043B\u044F \u043F\u043E\u043B\u043D\u043E\u0439 \u0437\u0430\u043C\u0435\u043D\u044B Python-\u043F\u0440\u043E\u043A\u0441\u0438 \u0432 FOSVED Office v2",
            size: 32, bold: true, color: P.primary, font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
          }),
        ],
      }),
      new Paragraph({
        indent: { left: 1200, right: 1200 },
        spacing: { after: 100, line: 312 },
        children: [
          new TextRun({
            text: "\u0421\u0438\u0441\u0442\u0435\u043C\u0430\u0442\u0438\u0437\u0430\u0446\u0438\u044F \u0443\u0437\u043A\u0438\u0445 \u043C\u0435\u0441\u0442, \u043F\u0440\u0438\u043E\u0440\u0438\u0442\u0435\u0442\u044B, \u043F\u043E\u0448\u0430\u0433\u043E\u0432\u0430\u044F \u0440\u0435\u0430\u043B\u0438\u0437\u0430\u0446\u0438\u044F",
            size: 24, color: P.secondary, font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" },
          }),
        ],
      }),
      // Meta info
      new Paragraph({ spacing: { before: 1600 }, children: [] }),
      new Paragraph({
        indent: { left: 1200 },
        spacing: { after: 80, line: 312 },
        children: [new TextRun({ text: "\u041D\u0430\u0440\u044F\u0434 \u2116 12 | \u0424\u0430\u0437\u0430 5: \u0414\u043E\u0440\u0430\u0431\u043E\u0442\u043A\u0430 \u044F\u0437\u044B\u043A\u0430", size: 22, color: P.secondary, font: { ascii: "Calibri" } })],
      }),
      new Paragraph({
        indent: { left: 1200 },
        spacing: { after: 80, line: 312 },
        children: [new TextRun({ text: "\u0412\u0435\u0442\u043A\u0430: fix/metalogos-runtime  |  Target: main", size: 22, color: P.secondary, font: { ascii: "Calibri" } })],
      }),
      new Paragraph({
        indent: { left: 1200 },
        spacing: { after: 80, line: 312 },
        children: [new TextRun({ text: "\u0414\u0430\u0442\u0430: 2026-06-10", size: 22, color: P.secondary, font: { ascii: "Calibri" } })],
      }),
      // Bottom accent bar
      new Paragraph({ spacing: { before: 2000 }, children: [] }),
      new Table({
        width: { size: 100, type: WidthType.PERCENTAGE },
        borders: allNoBorders,
        rows: [
          new TableRow({
            height: { value: 100, rule: "exact" },
            children: [
              new TableCell({
                width: { size: 100, type: WidthType.PERCENTAGE },
                shading: { type: ShadingType.CLEAR, fill: P.accent },
                borders: allNoBorders,
                children: [new Paragraph({ children: [] })],
              }),
            ],
          }),
        ],
      }),
    ],
  };
}

// ════════════════════════════════════════════════════════════════════
//  DOCUMENT ASSEMBLY
// ════════════════════════════════════════════════════════════════════

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: { ascii: "Calibri", eastAsia: "Microsoft YaHei" }, size: 24, color: P.body },
        paragraph: { spacing: { line: 312 } },
      },
      heading1: {
        run: { font: { ascii: "Calibri", eastAsia: "SimHei" }, size: 32, bold: true, color: P.primary },
        paragraph: { spacing: { before: 400, after: 200, line: 312 } },
      },
      heading2: {
        run: { font: { ascii: "Calibri", eastAsia: "SimHei" }, size: 28, bold: true, color: P.primary },
        paragraph: { spacing: { before: 300, after: 150, line: 312 } },
      },
      heading3: {
        run: { font: { ascii: "Calibri", eastAsia: "SimHei" }, size: 24, bold: true, color: P.primary },
        paragraph: { spacing: { before: 240, after: 100, line: 312 } },
      },
    },
  },
  numbering: {
    config: [
      {
        reference: "list-steps",
        levels: [{ level: 0, format: LevelFormat.DECIMAL, text: "%1.", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 720, hanging: 360 } } } }],
      },
      {
        reference: "list-phase",
        levels: [{ level: 0, format: LevelFormat.DECIMAL, text: "%1.", alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 720, hanging: 360 } } } }],
      },
    ],
  },
  sections: [
    // ── Section 1: Cover ──
    buildCover(),

    // ── Section 2: TOC ──
    {
      properties: {
        page: { size: { width: 11906, height: 16838 }, margin: { top: 1440, bottom: 1440, left: 1701, right: 1417 } },
        page: { pageNumbers: { start: 1, formatType: NumberFormat.UPPER_ROMAN } },
      },
      footers: {
        default: new Footer({
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ children: [PageNumber.CURRENT], size: 18, color: P.secondary })],
            }),
          ],
        }),
      },
      children: [
        new Paragraph({
          spacing: { before: 200, after: 300 },
          children: [new TextRun({ text: "\u0421\u043E\u0434\u0435\u0440\u0436\u0430\u043D\u0438\u0435", bold: true, size: 32, color: P.primary, font: { ascii: "Calibri", eastAsia: "SimHei" } })],
        }),
        new TableOfContents("\u0421\u043E\u0434\u0435\u0440\u0436\u0430\u043D\u0438\u0435", {
          hyperlink: true,
          headingStyleRange: "1-3",
        }),
        new Paragraph({
          spacing: { before: 200 },
          children: [
            new TextRun({ text: "\u0414\u043B\u044F \u043E\u0431\u043D\u043E\u0432\u043B\u0435\u043D\u0438\u044F \u043E\u0433\u043B\u0430\u0432\u043B\u0435\u043D\u0438\u044F: \u043F\u0440\u0430\u0432\u0430\u044F \u043A\u043D\u043E\u043F\u043A\u0430 \u043C\u044B\u0448\u0438 \u043D\u0430 \u043E\u0433\u043B\u0430\u0432\u043B\u0435\u043D\u0438\u0435 \u2192 \u00AB\u041E\u0431\u043D\u043E\u0432\u0438\u0442\u044C \u043F\u043E\u043B\u0435\u00BB.", size: 18, color: P.secondary, italics: true }),
          ],
        }),
        new Paragraph({ children: [new PageBreak()] }),
      ],
    },

    // ── Section 3: Body ──
    {
      properties: {
        page: { size: { width: 11906, height: 16838 }, margin: { top: 1440, bottom: 1440, left: 1701, right: 1417 } },
        page: { pageNumbers: { start: 1, formatType: NumberFormat.DECIMAL } },
      },
      footers: {
        default: new Footer({
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ children: [PageNumber.CURRENT], size: 18, color: P.secondary })],
            }),
          ],
        }),
      },
      headers: {
        default: new Header({
          children: [
            new Paragraph({
              alignment: AlignmentType.RIGHT,
              children: [new TextRun({ text: "Metalogos Fix Plan | \u041D\u0430\u0440\u044F\u0434 \u2116 12, \u0424\u0430\u0437\u0430 5", size: 18, color: P.secondary, italics: true })],
            }),
          ],
        }),
      },
      children: [
        // ═══════ 1. KONTEKST ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 1. \u041A\u043E\u043D\u0442\u0435\u043A\u0441\u0442 \u0438 \u0446\u0435\u043B\u044C"),

        h2("1.1. \u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435 \u043F\u0440\u043E\u0435\u043A\u0442\u043E\u0432"),
        body("Metalogos \u2014 \u044D\u0442\u043E AI-\u043D\u0430\u0442\u0438\u0432\u043D\u044B\u0439 \u044F\u0437\u044B\u043A \u043F\u0440\u043E\u0433\u0440\u0430\u043C\u043C\u0438\u0440\u043E\u0432\u0430\u043D\u0438\u044F, \u0440\u0435\u0430\u043B\u0438\u0437\u043E\u0432\u0430\u043D\u043D\u044B\u0439 \u043D\u0430 Rust. \u042F\u0437\u044B\u043A \u043F\u043E\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442 HTTP-\u0441\u0435\u0440\u0432\u0435\u0440 (Axum), LLM-\u0438\u043D\u0442\u0435\u0433\u0440\u0430\u0446\u0438\u044E (Anthropic, OpenAI, Ollama), SQLite-\u0445\u0440\u0430\u043D\u0438\u043B\u0438\u0449\u0435, \u0441\u0435\u0430\u043D\u0441-\u043C\u0435\u043D\u0435\u0434\u0436\u043C\u0435\u043D\u0442 \u0441 HMAC-\u043F\u043E\u0434\u043F\u0438\u0441\u044F\u043C\u0438, \u0448\u0438\u0444\u0440\u043E\u0432\u0430\u043D\u0438\u0435 (AES-256-GCM), Argon2id-\u0445\u044D\u0448\u0438\u0440\u043E\u0432\u0430\u043D\u0438\u0435 \u043F\u0430\u0440\u043E\u043B\u0435\u0439, \u0441\u0435\u043C\u0430\u043D\u0442\u0438\u0447\u0435\u0441\u043A\u0443\u044E \u043F\u0430\u043C\u044F\u0442\u044C \u0441 embeddings \u0438 \u0437\u043D\u0430\u043D\u0438\u0435\u0432\u044B\u043C \u0433\u0440\u0430\u0444\u043E\u043C. \u041A\u043E\u043C\u043F\u0438\u043B\u044F\u0446\u0438\u044F \u0432 \u0431\u0430\u0439\u0442\u043A\u043E\u0434 (.mbc), \u0438\u043D\u0442\u0435\u0440\u043F\u0440\u0435\u0442\u0430\u0446\u0438\u044F \u0438 \u0432\u044B\u043F\u043E\u043B\u043D\u0435\u043D\u0438\u0435 \u043D\u0430 \u0412\u041C. \u0422\u0435\u043A\u0443\u0449\u0430\u044F \u0432\u0435\u0440\u0441\u0438\u044F: v0.4.0, \u0431\u0438\u043D\u0430\u0440\u043D\u0438\u043A 11.2 MB ELF x86-64."),
        body("FOSVED Office v2 \u2014 Telegram-\u0431\u043E\u0442, \u0440\u0435\u0430\u043B\u0438\u0437\u043E\u0432\u0430\u043D\u043D\u044B\u0439 \u043F\u043E\u043B\u043D\u043E\u0441\u0442\u044C\u044E \u043D\u0430 Metalogos (.mlog \u0444\u0430\u0439\u043B\u044B). \u0421\u0438\u0441\u0442\u0435\u043C\u0430 \u0441\u0438\u043C\u0443\u043B\u0438\u0440\u0443\u0435\u0442 \u043A\u043E\u0440\u043F\u043E\u0440\u0430\u0442\u0438\u0432\u043D\u044B\u0439 \u043E\u0444\u0438\u0441 \u0441 12 AI-\u043E\u0442\u0434\u0435\u043B\u0430\u043C\u0438 (OSP, LZ, Expert, Dev, Design, QA, Engineering, Marketing, Finance, Legal, Visual, Kavalnya). \u041A\u0430\u0436\u0434\u044B\u0439 \u043E\u0442\u0434\u0435\u043B \u0438\u043C\u0435\u0435\u0442 \u0441\u043E\u0431\u0441\u0442\u0432\u0435\u043D\u043D\u044B\u0439 \u0441\u0438\u0441\u0442\u0435\u043C\u043D\u044B\u0439 \u043F\u0440\u043E\u043C\u043F\u0442, \u043D\u0430\u0431\u043E\u0440 \u043A\u043E\u043C\u0430\u043D\u0434 \u0438 \u043B\u043E\u0433\u0438\u043A\u0443 \u0434\u0438\u0441\u043F\u0435\u0442\u0447\u0435\u0440\u0438\u0437\u0430\u0446\u0438\u0438. \u0414\u0435\u043F\u043B\u043E\u0439 \u043D\u0430 Render \u043A\u0430\u043A Docker-\u043A\u043E\u043D\u0442\u0435\u0439\u043D\u0435\u0440 (mlog serve app.mlog, port 10000)."),

        h2("1.2. \u041F\u0440\u043E\u0431\u043B\u0435\u043C\u0430"),
        body("\u041F\u0440\u0438 \u0440\u0430\u0437\u0440\u0430\u0431\u043E\u0442\u043A\u0435 FOSVED Office v2 \u0431\u044B\u043B\u0438 \u043E\u0431\u043D\u0430\u0440\u0443\u0436\u0435\u043D\u044B \u043E\u0433\u0440\u0430\u043D\u0438\u0447\u0435\u043D\u0438\u044F \u044F\u0437\u044B\u043A\u0430 Metalogos, \u043A\u043E\u0442\u043E\u0440\u044B\u0435 \u0432\u044B\u043D\u0443\u0434\u0438\u043B\u0438 \u0441\u043E\u0437\u0434\u0430\u0442\u044C \u0434\u0432\u0430 Python-\u043C\u043E\u0434\u0443\u043B\u044F-\u043F\u0440\u043E\u043A\u0441\u0438: llm_proxy.py (\u0434\u043E\u0431\u0430\u0432\u043B\u0435\u043D\u0438\u0435 Authorization \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043A\u043E\u0432, fallback \u043F\u043E 5 \u043F\u0440\u043E\u0432\u0430\u0439\u0434\u0435\u0440\u0430\u043C) \u0438 sanitize_mlog.py (\u0442\u0440\u0430\u043D\u0441\u043B\u0438\u0442\u0435\u0440\u0430\u0446\u0438\u044F \u043A\u0438\u0440\u0438\u043B\u043B\u0438\u0446\u044B \u0432 ASCII). \u0422\u0435\u043A\u0443\u0449\u0430\u044F \u0446\u0435\u043B\u044C \u2014 \u0434\u043E\u0440\u0430\u0431\u043E\u0442\u0430\u0442\u044C Metalogos \u0434\u043E \u0443\u0440\u043E\u0432\u043D\u044F, \u043F\u043E\u0437\u0432\u043E\u043B\u044F\u044E\u0449\u0435\u0433\u043E \u043F\u0438\u0441\u0430\u0442\u044C \u0432\u0441\u0451 \u043D\u0430 \u043D\u0451\u043C, \u0431\u0435\u0437 \u043C\u043E\u0434\u0443\u043B\u0435\u0439 \u043D\u0430 \u0434\u0440\u0443\u0433\u0438\u0445 \u044F\u0437\u044B\u043A\u0430\u0445."),

        h2("1.3. \u0422\u0435\u043A\u0443\u0449\u0435\u0435 \u0441\u043E\u0441\u0442\u043E\u044F\u043D\u0438\u0435"),
        body("\u0418\u0441\u0445\u043E\u0434\u043D\u044B\u0439 \u043A\u043E\u0434 \u043D\u0430\u0445\u043E\u0434\u0438\u0442\u0441\u044F \u0432 \u0432\u0435\u0442\u043A\u0435 fix/metalogos-runtime (\u043A\u043E\u043C\u043C\u0438\u0442 67e86b6). \u0412\u0435\u0442\u043A\u0430 main \u0441\u043E\u0434\u0435\u0440\u0436\u0438\u0442 1 \u043A\u043E\u043C\u043C\u0438\u0442 (605524c) \u043E\u0442\u0441\u0443\u0442\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u0439 \u0432 fix-\u0432\u0435\u0442\u043A\u0435 (3 \u043A\u0440\u0438\u0442\u0438\u0447\u0435\u0441\u043A\u0438\u0445 \u0444\u0438\u043A\u0441\u0430 route handler + HTTP + if-then-else). 279 \u043A\u043E\u043C\u043F\u0438\u043B\u044F\u0446\u0438\u043E\u043D\u043D\u044B\u0445 \u043E\u0448\u0438\u0431\u043E\u043A \u0443\u0436\u0435 \u0438\u0441\u043F\u0440\u0430\u0432\u043B\u0435\u043D\u044B \u0434\u043E 0. \u0422\u0435\u0441\u0442\u044B: 97/104 pass, 4 \u043F\u0440\u0435\u0434\u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u0445 \u0441\u0435\u043C\u0430\u043D\u0442\u0438\u0447\u0435\u0441\u043A\u0438\u0445 \u043E\u0448\u0438\u0431\u043A\u0438, 3 ignored."),

        // ═══════ 2. KARTA UZKIH MEST ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 2. \u041A\u0430\u0440\u0442\u0430 \u0443\u0437\u043A\u0438\u0445 \u043C\u0435\u0441\u0442"),
        body("\u041D\u0438\u0436\u0435 \u043F\u0440\u0438\u0432\u0435\u0434\u0435\u043D\u0430 \u043F\u043E\u043B\u043D\u0430\u044F \u0441\u0438\u0441\u0442\u0435\u043C\u0430\u0442\u0438\u0437\u0430\u0446\u0438\u044F \u0432\u0441\u0435\u0445 \u043E\u0431\u043D\u0430\u0440\u0443\u0436\u0435\u043D\u043D\u044B\u0445 \u043E\u0433\u0440\u0430\u043D\u0438\u0447\u0435\u043D\u0438\u0439. \u0418\u0441\u0442\u043E\u0447\u043D\u0438\u043A\u0438: \u0430\u043D\u0430\u043B\u0438\u0437 \u043A\u043E\u0434\u0430 FOSVED-office-v2 (\u0432\u0441\u0435 .mlog \u0444\u0430\u0439\u043B\u044B + Python-\u043C\u043E\u0434\u0443\u043B\u0438) + \u0430\u043D\u0430\u043B\u0438\u0437 \u0438\u0441\u0445\u043E\u0434\u043D\u043E\u0433\u043E \u043A\u043E\u0434\u0430 Metalogos (\u0432\u0441\u0435 .rs \u0444\u0430\u0439\u043B\u044B). \u041A\u0430\u0436\u0434\u043E\u0435 \u043E\u0433\u0440\u0430\u043D\u0438\u0447\u0435\u043D\u0438\u0435 \u043F\u0440\u043E\u0432\u0435\u0440\u0435\u043D\u043E \u043D\u0430 \u043D\u0430\u043B\u0438\u0447\u0438\u0435 \u0432 \u0438\u0441\u0445\u043E\u0434\u043D\u043E\u043C \u043A\u043E\u0434\u0435 \u0438 \u043D\u0430 \u043D\u0430\u043B\u0438\u0447\u0438\u0435 workaround-\u0430 \u0432 FOSVED."),

        h2("2.1. \u0422\u0430\u0431\u043B\u0438\u0446\u0430 \u0443\u0437\u043A\u0438\u0445 \u043C\u0435\u0441\u0442"),

        hTable(
          ["\u2116", "\u041E\u0433\u0440\u0430\u043D\u0438\u0447\u0435\u043D\u0438\u0435", "\u041F\u0440\u0438\u043E\u0440\u0438\u0442\u0435\u0442", "\u0424\u0430\u0439\u043B\u044B", "Workaround \u0432 FOSVED"],
          [
            ["1", "\u041D\u0435\u0442 builtin query_param()", "CRITICAL", "builtins.rs, server.rs, interpreter.rs", "\u041D\u0435\u0442 \u2014 \u0431\u043B\u043E\u043A\u0438\u0440\u0443\u0435\u0442 /report \u0438 /miniapp"],
            ["2", "\u041D\u0435\u0442 builtin respond_html()", "CRITICAL", "builtins.rs", "\u041D\u0435\u0442 \u2014 \u0431\u043B\u043E\u043A\u0438\u0440\u0443\u0435\u0442 HTML-\u043E\u0442\u0432\u0435\u0442\u044B"],
            ["3", "CSP \u0431\u043B\u043E\u043A\u0438\u0440\u0443\u0435\u0442 Telegram WebApp", "HIGH", "server.rs", "\u041D\u0435\u0442 \u2014 \u043C\u0438\u043D\u0438\u0430\u043F\u043F\u044B \u043D\u0435 \u0440\u0430\u0431\u043E\u0442\u0430\u044E\u0442"],
            ["4", "call_llm() \u0432 mock-\u0440\u0435\u0436\u0438\u043C\u0435 \u043F\u043E \u0443\u043C\u043E\u043B\u0447.", "HIGH", "builtins.rs, llm.rs", "entrypoint.sh: METALOGOS_LLM_MOCK=false"],
            ["5", "\u041D\u0435\u0442 \u0446\u0435\u043F\u043E\u0447\u043A\u0438 fallback LLM", "HIGH", "llm.rs", "llm_proxy.py (DISABLED)"],
            ["6", "call_llm() \u043D\u0435 \u0440\u0430\u0437\u0434\u0435\u043B\u044F\u0435\u0442 system/user", "HIGH", "llm.rs, builtins.rs", "\u0421\u043A\u043B\u0435\u0438\u0432\u0430\u043D\u0438\u0435 prompt+input"],
            ["7", "Blocking HTTP \u043D\u0430 async tokio", "HIGH", "builtins.rs, server.rs", "\u041D\u0435\u0442 \u2014 thread starvation"],
            ["8", "http_post() \u043A\u0440\u0430\u0448\u0438\u0442 \u043F\u0440\u0438 4xx/5xx", "MEDIUM", "builtins.rs", "try/catch \u043D\u0435\u0432\u043E\u0437\u043C\u043E\u0436\u0435\u043D"],
            ["9", "\u041D\u0435\u0442 http_get() \u0441 \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043A\u0430\u043C\u0438", "MEDIUM", "builtins.rs", "\u041D\u0435\u0442"],
            ["10", "\u041D\u0435\u0442 json_encode()", "MEDIUM", "builtins.rs", "\u0420\u0443\u0447\u043D\u0430\u044F \u043A\u043E\u043D\u043A\u0430\u0442\u0435\u043D\u0430\u0446\u0438\u044F JSON"],
            ["11", "\u041D\u0435\u0442 match/switch", "LOW", "grammar.pest, parser.rs, interpreter.rs", "30+ if starts_with() \u0432 app.mlog"],
            ["12", "\u041A\u0438\u0440\u0438\u043B\u0438\u0446\u0430 \u0432 .mlog-\u0444\u0430\u0439\u043B\u0430\u0445", "P0", "parser, std", "sanitize_mlog.py (translit)"],
            ["13", "\u041D\u0435\u0441\u043E\u0433\u043B\u0430\u0441\u043E\u0432\u0430\u043D\u043D\u043E\u0441\u0442\u044C \u0432\u0435\u0442\u043E\u043A", "LOW", "git branches", "\u041C\u0435\u0440\u0433\u0435 \u043D\u0443\u0436\u0435\u043D"],
          ],
          [6, 30, 12, 28, 24]
        ),

        emptyLine(),

        // ═══════ 3. PRIORITET I ISPRavleniy ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 3. \u041F\u043E\u0448\u0430\u0433\u043E\u0432\u044B\u0439 \u043F\u043B\u0430\u043D \u0434\u043E\u0440\u0430\u0431\u043E\u0442\u043E\u043A"),
        body("\u041A\u0430\u0436\u0434\u044B\u0439 \u0448\u0430\u0433 \u0432\u043A\u043B\u044E\u0447\u0430\u0435\u0442: \u043A\u043E\u0434 \u2192 cargo build \u2192 cargo test \u2192 \u043A\u043E\u043C\u043C\u0438\u0442 \u2192 \u043F\u0443\u0448 \u2192 \u043E\u0442\u0447\u0451\u0442. \u041F\u043E\u0441\u043B\u0435 \u043A\u0430\u0436\u0434\u043E\u0433\u043E \u0448\u0430\u0433\u0430 \u2014 \u043F\u0440\u043E\u0432\u0435\u0440\u043A\u0430 \u0441\u043E\u0431\u0438\u0440\u0430\u0435\u043C\u043E\u0441\u0442\u0438 \u0438 \u0442\u0435\u0441\u0442\u043E\u0432. \u0412\u0441\u0435 \u0438\u0437\u043C\u0435\u043D\u0435\u043D\u0438\u044F \u0432 \u0432\u0435\u0442\u043A\u0435 fix/metalogos-runtime, \u0444\u0438\u043D\u0430\u043B\u044C\u043D\u044B\u0439 \u043C\u0435\u0440\u0433\u0435 \u0432 main."),

        // ── STEP 0 ──
        h2("\u0428\u0430\u0433 0. \u041F\u043E\u0434\u0433\u043E\u0442\u043E\u0432\u043A\u0430: \u043C\u0435\u0440\u0433\u0435 \u0432\u0435\u0442\u043E\u043A"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0412\u0435\u0442\u043A\u0430 main \u0441\u043E\u0434\u0435\u0440\u0436\u0438\u0442 1 \u043A\u043E\u043C\u043C\u0438\u0442 (605524c) \u043E\u0442\u0441\u0443\u0442\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u0439 \u0432 fix/metalogos-runtime: \u0444\u0438\u043A\u0441\u044B route handler (HTTP \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043A\u0438, if-then-else \u0432 parser.rs, \u0434\u043E\u0431\u0430\u0432\u043B\u0435\u043D\u0438\u0435 \u043D\u0435\u0434\u043E\u0441\u0442\u0430\u044E\u0449\u0438\u0445 match arms). \u041D\u0435\u043E\u0431\u0445\u043E\u0434\u0438\u043C\u043E \u043F\u0435\u0440\u0435\u043D\u0435\u0441\u0442\u0438 \u044D\u0442\u0438 \u0438\u0437\u043C\u0435\u043D\u0435\u043D\u0438\u044F \u0432 fix-\u0432\u0435\u0442\u043A\u0443 \u0434\u043E \u043D\u0430\u0447\u0430\u043B\u0430 \u043D\u043E\u0432\u044B\u0445 \u0440\u0430\u0431\u043E\u0442. \u0410\u043B\u044C\u0442\u0435\u0440\u043D\u0430\u0442\u0438\u0432\u043D\u043E \u2014 \u0447\u0435\u0440\u0440\u0438\u0442 (rebase) fix-\u0432\u0435\u0442\u043A\u0443 \u043D\u0430\u0434 main, \u0447\u0442\u043E\u0431\u044B \u043F\u043E\u043B\u0443\u0447\u0438\u0442\u044C \u0435\u0434\u0438\u043D\u0443\u044E \u043B\u0438\u043D\u0438\u044E \u0440\u0430\u0437\u0440\u0430\u0431\u043E\u0442\u043A\u0438."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("1. git checkout fix/metalogos-runtime && git merge main (or rebase). 2. \u0420\u0430\u0437\u0440\u0435\u0448\u0438\u0442\u044C \u043A\u043E\u043D\u0444\u043B\u0438\u043A\u0442\u044B \u0432 parser.rs, builtins.rs, grammar.pest. 3. cargo build --release. 4. cargo test --lib. 5. \u041A\u043E\u043C\u043C\u0438\u0442: chore: merge main into fix/metalogos-runtime."),
        h3("\u041A\u043E\u043D\u0442\u0440\u0430\u043A\u0442"),
        body("\u041E\u0434\u043D\u0430 \u0432\u0435\u0442\u043A\u0430 \u0441\u043E \u0432\u0441\u0435\u043C\u0438 \u0444\u0438\u043A\u0441\u0430\u043C\u0438. cargo build --release \u0443\u0441\u043F\u0435\u0448\u0435\u043D. cargo test: 97+ pass."),

        // ── STEP 1 ──
        h2("\u0428\u0430\u0433 1. Builtin query_param() [CRITICAL]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0412 app.mlog \u0438\u0441\u043F\u043E\u043B\u044C\u0437\u0443\u0435\u0442\u0441\u044F query_param(\"id\") \u0438 query_param(\"doc\") \u0434\u043B\u044F \u0440\u043E\u0443\u0442\u043E\u0432 /report \u0438 /miniapp, \u043D\u043E \u044D\u0442\u043E\u0433\u043E builtin \u043D\u0435 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u0435\u0442 \u0432 \u0438\u0441\u0445\u043E\u0434\u043D\u043E\u043C \u043A\u043E\u0434\u0435. \u0411\u0435\u0437 \u043D\u0435\u0433\u043E \u044D\u0442\u0438 \u044D\u043D\u0434\u043F\u043E\u0439\u043D\u0442\u044B \u043F\u043E\u043B\u043D\u043E\u0441\u0442\u044C\u044E \u043D\u0435\u0440\u0430\u0431\u043E\u0442\u0430\u044E\u0442 \u2014 \u043F\u043E\u043B\u044C\u0437\u043E\u0432\u0430\u0442\u0435\u043B\u044C \u043D\u0435 \u043C\u043E\u0436\u0435\u0442 \u043E\u0442\u043A\u0440\u044B\u0442\u044C \u043E\u0442\u0447\u0451\u0442 \u0438\u043B\u0438 \u043C\u0438\u043D\u0438\u0430\u043F\u043F."),
        h3("\u041A\u043E\u043D\u0442\u0440\u0430\u043A\u0442"),
        codeBlock('query_param("id") -> "abc123"     // \u0438\u0437 ?id=abc123'),
        codeBlock('query_param("missing") -> ""       // \u043F\u0443\u0441\u0442\u0430\u044F \u0441\u0442\u0440\u043E\u043A\u0430'),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 interpreter.rs: \u0434\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u043F\u043E\u043B\u0435 server_query_params: HashMap<String, String> \u0438 \u043C\u0435\u0442\u043E\u0434 set_server_query_params(). \u0412 server.rs: \u0432 execute_route_body() \u0434\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u043F\u0430\u0440\u0430\u043C\u0435\u0442\u0440 query_string: &str, \u0440\u0430\u0437\u043F\u0430\u0440\u0441\u0438\u0442\u044C URI query, \u043F\u0435\u0440\u0435\u0434\u0430\u0442\u044C \u0432 interpreter. \u0412 builtins.rs: \u0437\u0430\u0440\u0435\u0433\u0438\u0441\u0442\u0440\u0438\u0440\u043E\u0432\u0430\u0442\u044C builtin-query_param(\u0438\u043C\u044F) -> \u0437\u043D\u0430\u0447\u0435\u043D\u0438\u0435 \u0438\u0437 hashmap. \u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u0442\u0435\u0441\u0442: server_test \u0441 GET /test?foo=bar \u2192 query_param(\"foo\") == \"bar\"."),

        // ── STEP 2 ──
        h2("\u0428\u0430\u0433 2. Builtin respond_html() [CRITICAL]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0412 app.mlog \u0438\u0441\u043F\u043E\u043B\u044C\u0437\u0443\u0435\u0442\u0441\u044F respond_html(\"200\", html) \u0434\u043B\u044F \u043E\u0442\u0434\u0430\u0447\u0438 HTML-\u043A\u043E\u043D\u0442\u0435\u043D\u0442\u0430 \u0432 \u0440\u043E\u0443\u0442\u0430\u0445 /report \u0438 /miniapp. \u0422\u0438\u043F Value::Html \u0443\u0436\u0435 \u043E\u0431\u0440\u0430\u0431\u0430\u0442\u044B\u0432\u0430\u0435\u0442\u0441\u044F \u0432 server.rs (value_to_response \u043A\u043E\u043D\u0432\u0435\u0440\u0442\u0438\u0440\u0443\u0435\u0442 \u0432 AxumHtml), \u043D\u043E \u043D\u0435\u0442 builtin-\u0444\u0443\u043D\u043A\u0446\u0438\u0438 \u0434\u043B\u044F \u0435\u0433\u043E \u0441\u043E\u0437\u0434\u0430\u043D\u0438\u044F \u0438\u0437 .mlog-\u043A\u043E\u0434\u0430."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 builtins.rs: \u0434\u043E\u0431\u0430\u0432\u0438\u0442\u044C fn builtin_respond_html(args) \u2192 \u043F\u0430\u0440\u0441\u0438\u0442 status_code (info only), \u0432\u043E\u0437\u0432\u0440\u0430\u0449\u0430\u0435\u0442 Value::Html(html_content). \u0417\u0430\u0440\u0435\u0433\u0438\u0441\u0442\u0440\u0438\u0440\u043E\u0432\u0430\u0442\u044C: funcs.insert(\"respond_html\", builtin_respond_html). \u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u0442\u0435\u0441\u0442: respond_html(\"200\", \"<h1>OK</h1>\") \u2192 Value::Html."),

        // ── STEP 3 ──
        h2("\u0428\u0430\u0433 3. CSP-\u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043E\u043A \u0434\u043B\u044F Telegram WebApp [HIGH]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0422\u0435\u043A\u0443\u0449\u0438\u0439 CSP-\u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043E\u043A script-src 'self' \u0431\u043B\u043E\u043A\u0438\u0440\u0443\u0435\u0442 \u0437\u0430\u0433\u0440\u0443\u0437\u043A\u0443 telegram-web-app.js \u0441 https://telegram.org. \u041C\u0438\u043D\u0438\u0430\u043F\u043F\u044B Telegram WebApp \u043D\u0435 \u043C\u043E\u0433\u0443\u0442 \u0440\u0430\u0431\u043E\u0442\u0430\u0442\u044C, \u043F\u043E\u0441\u043A\u043E\u043B\u044C\u043A\u0443 \u0438\u0445 JS-\u043A\u043E\u0434 \u043D\u0435 \u0437\u0430\u0433\u0440\u0443\u0436\u0430\u0435\u0442\u0441\u044F. \u041A\u0440\u043E\u043C\u0435 \u0442\u043E\u0433\u043E, connect-src \u043D\u0443\u0436\u0435\u043D \u0434\u043B\u044F sendData() API \u043A Telegram."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 server.rs \u0437\u0430\u043C\u0435\u043D\u0438\u0442\u044C CSP \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043E\u043A \u043D\u0430:"),
        codeBlock("default-src 'self' https://telegram.org;"),
        codeBlock("script-src 'self' https://telegram.org;"),
        codeBlock("style-src 'self' 'unsafe-inline';"),
        codeBlock("img-src 'self' data:;"),
        codeBlock("connect-src 'self' https://api.telegram.org"),
        body("\u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u0442\u0435\u0441\u0442: \u0437\u0430\u043F\u0440\u043E\u0441 \u043D\u0430 /miniapp \u2192 \u043F\u0440\u043E\u0432\u0435\u0440\u0438\u0442\u044C Content-Security-Policy \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043E\u043A \u043E\u0442\u0432\u0435\u0442\u0430."),

        // ── STEP 4 ──
        h2("\u0428\u0430\u0433 4. Blocking HTTP \u043D\u0430 async tokio [HIGH]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("http_post(), http_get() \u0438 call_llm() \u0438\u0441\u043F\u043E\u043B\u044C\u0437\u0443\u044E\u0442 reqwest::blocking::Client \u0432\u043D\u0443\u0442\u0440\u0438 tokio async runtime. \u042D\u0442\u043E \u0431\u043B\u043E\u043A\u0438\u0440\u0443\u0435\u0442 \u043F\u043E\u0442\u043E\u043A\u0438 tokio \u0438 \u043F\u0440\u0438 concurrent-\u0437\u0430\u043F\u0440\u043E\u0441\u0430\u0445 \u043F\u0440\u0438\u0432\u043E\u0434\u0438\u0442 \u043A thread starvation \u0438 \u0442\u0430\u0439\u043C\u0430\u0443\u0442\u0430\u043C. \u041F\u0440\u0438 \u0430\u043A\u0442\u0438\u0432\u043D\u043E\u043C \u0438\u0441\u043F\u043E\u043B\u044C\u0437\u043E\u0432\u0430\u043D\u0438\u0438 LLM-\u0432\u044B\u0437\u043E\u0432\u043E\u0432 \u0438\u0437 \u043D\u0435\u0441\u043A\u043E\u043B\u044C\u043A\u0438\u0445 \u043E\u0442\u0434\u0435\u043B\u043E\u0432 \u044D\u0442\u043E \u043A\u0440\u0438\u0442\u0438\u0447\u0435\u0441\u043A\u0438 \u0432\u0430\u0436\u043D\u043E."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 server.rs \u0432 execute_route_body() \u043E\u0431\u0435\u0440\u043D\u0443\u0442\u044C \u0432\u0435\u0441\u044C \u0432\u044B\u0437\u043E\u0432 \u0438\u043D\u0442\u0435\u0440\u043F\u0440\u0435\u0442\u0430\u0442\u043E\u0440\u0430 \u0432 tokio::task::spawn_blocking(). \u042D\u0442\u043E \u0438\u0437\u043E\u043B\u0438\u0440\u0443\u0435\u0442 \u0432\u0441\u0435 blocking-\u043E\u043F\u0435\u0440\u0430\u0446\u0438\u0438 (HTTP, LLM, SQLite) \u043E\u0442 tokio runtime. \u0410\u043B\u044C\u0442\u0435\u0440\u043D\u0430\u0442\u0438\u0432\u0430: \u043F\u0435\u0440\u0435\u043F\u0438\u0441\u0430\u0442\u044C http_post \u0438 call_llm \u043D\u0430 async reqwest::Client \u0441 Handle::current().block_on(), \u043D\u043E \u044D\u0442\u043E \u0431\u043E\u043B\u0435\u0435 \u043C\u0430\u0441\u0448\u0442\u0430\u0431\u043D\u0430\u044F \u043F\u0435\u0440\u0435\u0440\u0430\u0431\u043E\u0442\u043A\u0430. spawn_blocking \u2014 \u043C\u0438\u043D\u0438\u043C\u0430\u043B\u044C\u043D\u043E \u0438\u043D\u0432\u0430\u0437\u0438\u0432\u043D\u044B\u0439 \u0432\u0430\u0440\u0438\u0430\u043D\u0442."),

        // ── STEP 5 ──
        h2("\u0428\u0430\u0433 5. \u0426\u0435\u043F\u043E\u0447\u043A\u0430 fallback LLM [HIGH]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("call_llm() \u0432\u044B\u0437\u044B\u0432\u0430\u0435\u0442 \u0442\u043E\u043B\u044C\u043A\u043E \u041E\u0414\u041D\u041E\u0413\u041E \u043F\u0440\u043E\u0432\u0430\u0439\u0434\u0435\u0440\u0430. Python-\u043F\u0440\u043E\u043A\u0441\u0438 llm_proxy.py \u0440\u0435\u0430\u043B\u0438\u0437\u043E\u0432\u044B\u0432\u0430\u043B \u0446\u0435\u043F\u043E\u0447\u043A\u0443: GLM 4.6 -> GLM 5.1 -> DeepSeek -> Groq -> Claude. \u0414\u043B\u044F \u043F\u043E\u043B\u043D\u043E\u0439 \u0437\u0430\u043C\u0435\u043D\u044B \u043F\u0440\u043E\u043A\u0441\u0438 \u043D\u0443\u0436\u043D\u0430 \u0442\u0430\u043A\u0430\u044F \u0436\u0435 \u0446\u0435\u043F\u043E\u0447\u043A\u0430 \u043D\u0430\u0442\u0438\u0432\u043D\u043E \u0432 Rust."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 llm.rs: 1. \u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u043F\u0440\u043E\u0432\u0430\u0439\u0434\u0435\u0440\u044B GLM, DeepSeek, Groq (\u0432\u0441\u0435 OpenAI-compatible). 2. \u0420\u0435\u0430\u043B\u0438\u0437\u043E\u0432\u0430\u0442\u044C FallbackLlm \u043E\u0431\u0451\u0440\u0442\u043A\u0443: \u043F\u043E\u0441\u043B\u0435\u0434\u043E\u0432\u0430\u0442\u0435\u043B\u044C\u043D\u044B\u0439 \u0432\u044B\u0437\u043E\u0432 Vec<RealLlm>, \u043F\u0435\u0440\u0432\u044B\u0439 \u0443\u0441\u043F\u0435\u0448\u043D\u044B\u0439 \u0432\u043E\u0437\u0432\u0440\u0430\u0449\u0430\u0435\u0442 \u0440\u0435\u0437\u0443\u043B\u044C\u0442\u0430\u0442. 3. \u0415\u0434\u0438\u043D\u044B\u0439 \u043C\u0435\u0442\u043E\u0434 call_openai_compatible() \u0434\u043B\u044F \u0432\u0441\u0435\u0445 OpenAI-\u043F\u0440\u043E\u0432\u0430\u0439\u0434\u0435\u0440\u043E\u0432. 4. \u041A\u043E\u043D\u0444\u0438\u0433\u0443\u0440\u0430\u0446\u0438\u044F \u0447\u0435\u0440\u0435\u0437 env: *_API_KEY, *_MODEL, *_URL. 5. create_llm_backend() \u0441\u043E\u0437\u0434\u0430\u0451\u0442 FallbackLlm \u0435\u0441\u043B\u0438 \u0437\u0430\u0434\u0430\u043D\u043E \u043D\u0435\u0441\u043A\u043E\u043B\u044C\u043A\u043E \u043A\u043B\u044E\u0447\u0435\u0439."),

        // ── STEP 6 ──
        h2("\u0428\u0430\u0433 6. \u0420\u0430\u0437\u0434\u0435\u043B\u0435\u043D\u0438\u0435 system/user \u0432 call_llm() [HIGH]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0422\u0435\u043A\u0443\u0449\u0438\u0439 call_llm(prompt, input) \u0441\u043A\u043B\u0435\u0438\u0432\u0430\u0435\u0442 \u0432 \u043E\u0434\u043D\u043E user-\u0441\u043E\u043E\u0431\u0449\u0435\u043D\u0438\u0435: \"prompt + Input: input\". \u0414\u043B\u044F FOSVED \u043A\u0440\u0438\u0442\u0438\u0447\u043D\u043E: \u043A\u0430\u0436\u0434\u044B\u0439 \u043E\u0442\u0434\u0435\u043B \u0438\u043C\u0435\u0435\u0442 \u0441\u0432\u043E\u0439 system-\u043F\u0440\u043E\u043C\u043F\u0442, \u0438 \u0440\u0430\u0437\u043C\u0435\u0448\u0435\u043D\u0438\u0435 \u0435\u0433\u043E \u0432 user-\u0441\u043E\u043E\u0431\u0449\u0435\u043D\u0438\u0435 \u0441\u043D\u0438\u0436\u0430\u0435\u0442 \u043A\u0430\u0447\u0435\u0441\u0442\u0432\u043E \u043E\u0442\u0432\u0435\u0442\u043E\u0432. \u041D\u0443\u0436\u043D\u043E \u043F\u0440\u0430\u0432\u0438\u043B\u044C\u043D\u043E\u0435 \u0440\u0430\u0437\u0434\u0435\u043B\u0435\u043D\u0438\u0435: {role: \"system\", content: ...} \u0438 {role: \"user\", content: ...} \u0432 JSON-\u0437\u0430\u043F\u0440\u043E\u0441\u0435. \u0414\u043B\u044F Anthropic \u2014 \u043E\u0442\u0434\u0435\u043B\u044C\u043D\u043E\u0435 \u043F\u043E\u043B\u0435 \"system\"."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0418\u0437\u043C\u0435\u043D\u0438\u0442\u044C \u0441\u0438\u0433\u043D\u0430\u0442\u0443\u0440\u0443: fn call(&self, system: &str, user: &str). \u0412 JSON-\u0437\u0430\u043F\u0440\u043E\u0441\u0435: messages: [{role: \"system\", content: system}, {role: \"user\", content: user}]. \u0421\u043E\u0445\u0440\u0430\u043D\u0438\u0442\u044C \u043E\u0431\u0440\u0430\u0442\u043D\u0443\u044E \u0441\u043E\u0432\u043C\u0435\u0441\u0442\u0438\u043C\u043E\u0441\u0442\u044C: call_legacy() \u0434\u043B\u044F learnable patterns (\u043E\u0434\u0438\u043D \u0430\u0440\u0433\u0443\u043C\u0435\u043D\u0442 \u0441\u043A\u043B\u0435\u0438\u0432\u0430\u0435\u0442\u0441\u044F)."),

        // ── STEP 7 ──
        h2("\u0428\u0430\u0433 7. Mock mode \u043F\u043E \u0443\u043C\u043E\u043B\u0447\u0430\u043D\u0438\u044E [HIGH]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("METALOGOS_LLM_MOCK \u043F\u043E \u0443\u043C\u043E\u043B\u0447\u0430\u043D\u0438\u044E true, \u0447\u0442\u043E \u043E\u0437\u043D\u0430\u0447\u0430\u0435\u0442 \u0447\u0442\u043E \u0432 production \u0431\u0435\u0437 \u044F\u0432\u043D\u043E\u0439 \u0443\u0441\u0442\u0430\u043D\u043E\u0432\u043A\u0438 env-\u043F\u0435\u0440\u0435\u043C\u0435\u043D\u043D\u043E\u0439 call_llm() \u0432\u043E\u0437\u0432\u0440\u0430\u0449\u0430\u0435\u0442 [MOCK: ...]. FOSVED \u043E\u0431\u0445\u043E\u0434\u0438\u0442 \u044D\u0442\u043E \u0447\u0435\u0440\u0435\u0437 entrypoint.sh (export METALOGOS_LLM_MOCK=false), \u043D\u043E \u044D\u0442\u043E \u043D\u0435 \u044F\u0432\u043D\u043E \u0434\u043E\u043A\u0443\u043C\u0435\u043D\u0442\u0438\u0440\u043E\u0432\u0430\u043D\u043D\u043E."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412\u0430\u0440\u0438\u0430\u043D\u0442 C (\u0440\u0435\u043A\u043E\u043C\u0435\u043D\u0434\u0443\u0435\u043C\u044B\u0439): \u043D\u0435 \u043C\u0435\u043D\u044F\u0442\u044C \u0434\u0435\u0444\u043E\u043B\u0442, \u043D\u043E \u0434\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u043A\u043E\u043C\u043C\u0435\u043D\u0442\u0430\u0440\u0438\u0439 \u0432 \u0438\u0441\u0445\u043E\u0434\u043D\u044B\u0439 \u043A\u043E\u0434 \u0438 \u043E\u0431\u043D\u043E\u0432\u0438\u0442\u044C entrypoint.sh FOSVED. \u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u0432 README.md Metalogos \u0441\u0435\u043A\u0446\u0438\u044E \u043E\u0431 env-\u043F\u0435\u0440\u0435\u043C\u0435\u043D\u043D\u044B\u0445."),

        // ── STEP 8 ──
        h2("\u0428\u0430\u0433 8. http_post() \u043E\u0431\u0440\u0430\u0431\u043E\u0442\u043A\u0430 \u043E\u0448\u0438\u0431\u043E\u043A [MEDIUM]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u041F\u0440\u0438 status >= 400 http_post() \u0432\u043E\u0437\u0432\u0440\u0430\u0449\u0430\u0435\u0442 Err(...), \u0447\u0442\u043E \u043A\u0440\u0430\u0448\u0438\u0442 \u0432\u0435\u0441\u044C route handler. \u041D\u0435\u043B\u044C\u0437\u044F \u043E\u0431\u0440\u0430\u0431\u043E\u0442\u0430\u0442\u044C \u043E\u0442\u0432\u0435\u0442 Telegram API gracefully. FOSVED \u0438\u0441\u043F\u043E\u043B\u044C\u0437\u0443\u0435\u0442 let _ = http_post(...), \u043D\u043E \u043E\u0448\u0438\u0431\u043A\u0430 \u0432\u0441\u0451 \u0440\u0430\u0432\u043D\u043E \u043F\u0440\u043E\u043F\u0430\u0433\u0430\u0442\u0438\u0440\u0443\u0435\u0442\u0441\u044F."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0412 builtins.rs: \u043F\u0440\u0438 status >= 400 \u0432\u043E\u0437\u0432\u0440\u0430\u0449\u0430\u0442\u044C Value::HttpResponse { status, body } \u0432\u043C\u0435\u0441\u0442\u043E Err. \u041F\u0443\u0441\u0442\u044C \u0432\u044B\u0437\u044B\u0432\u0430\u044E\u0449\u0438\u0439 \u043A\u043E\u0434 \u0441\u0430\u043C \u0440\u0435\u0448\u0430\u0435\u0442. \u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u0442\u0435\u0441\u0442: http_post \u043D\u0430 \u043D\u0435\u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u0439 URL \u2192 HttpResponse \u0432\u043C\u0435\u0441\u0442\u043E \u043A\u0440\u0430\u0448\u0430."),

        // ── STEP 9 ──
        h2("\u0428\u0430\u0433 9. http_get() \u0441 \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043A\u0430\u043C\u0438 [MEDIUM]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("http_get(url) \u043F\u0440\u0438\u043D\u0438\u043C\u0430\u0435\u0442 \u0442\u043E\u043B\u044C\u043A\u043E url, \u0431\u0435\u0437 \u0432\u043E\u0437\u043C\u043E\u0436\u043D\u043E\u0441\u0442\u0438 \u043F\u0435\u0440\u0435\u0434\u0430\u0447\u0438 \u0437\u0430\u0433\u043E\u043B\u043E\u0432\u043A\u043E\u0432 (Authorization). \u0410\u043D\u0430\u043B\u043E\u0433\u0438\u0447\u043D\u0430\u044F \u043F\u0440\u043E\u0431\u043B\u0435\u043C\u0430 \u0443\u0436\u0435 \u0440\u0435\u0448\u0435\u043D\u0430 \u0434\u043B\u044F http_post() (\u0448\u0430\u0433 4-\u0439 \u043F\u0440\u0435\u0434\u044B\u0434\u0443\u0449\u0435\u0433\u043E \u043D\u0430\u0440\u044F\u0434\u0430)."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C \u043E\u043F\u0446\u0438\u043E\u043D\u0430\u043B\u044C\u043D\u044B\u0439 2-\u0439 \u043F\u0430\u0440\u0430\u043C\u0435\u0442\u0440: http_get(url, headers_json?) \u0430\u043D\u0430\u043B\u043E\u0433\u0438\u0447\u043D\u043E http_post(). \u041F\u043E\u0434\u0434\u0435\u0440\u0436\u043A\u0430 String (JSON) \u0438 Struct."),

        // ── STEP 10 ──
        h2("\u0428\u0430\u0433 10. Builtin json_encode() [MEDIUM]"),
        h3("\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"),
        body("\u0412 FOSVED JSON \u0441\u043E\u0431\u0438\u0440\u0430\u0435\u0442\u0441\u044F \u0440\u0443\u0447\u043D\u043E\u0439 \u043A\u043E\u043D\u043A\u0430\u0442\u0435\u043D\u0430\u0446\u0438\u0435\u0439 \u0441\u0442\u0440\u043E\u043A: \"{\"chat_id\":\" + chat_id + \"}\". \u042D\u0442\u043E \u0445\u0440\u0443\u043F\u043A\u043E, \u043F\u043E\u0434\u0432\u0435\u0440\u0436\u0435\u043D\u043E \u0438\u043D\u044A\u0435\u043A\u0446\u0438\u044F\u043C (user-\u0442\u0435\u043A\u0441\u0442 \u0441 \u043A\u0430\u0432\u044B\u0447\u043A\u0430\u043C\u0438). parse_json() \u0443\u0436\u0435 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u0435\u0442, \u043D\u043E \u043E\u0431\u0440\u0430\u0442\u043D\u043E\u0433\u043E \u043F\u0440\u0435\u043E\u0431\u0440\u0430\u0437\u043E\u0432\u0430\u043D\u0438\u044F \u043D\u0435\u0442."),
        h3("\u0414\u0435\u0439\u0441\u0442\u0432\u0438\u044F"),
        body("\u0414\u043E\u0431\u0430\u0432\u0438\u0442\u044C builtin json_encode(value) -> String. \u041F\u0440\u0435\u043E\u0431\u0440\u0430\u0437\u043E\u0432\u0430\u043D\u0438\u0435 Value -> serde_json::Value -> String. \u041F\u043E\u0434\u0434\u0435\u0440\u0436\u043A\u0430 Struct, List, String, Float, Bool, null."),

        // ═══════ 4. FILE MAPPING ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 4. \u041A\u0430\u0440\u0442\u0430 \u0444\u0430\u0439\u043B\u043E\u0432"),
        body("\u0421\u0432\u044F\u0437\u043A\u0430 \u043C\u0435\u0436\u0434\u0443 \u0448\u0430\u0433\u0430\u043C\u0438 \u0438 \u0444\u0430\u0439\u043B\u0430\u043C\u0438, \u043A\u043E\u0442\u043E\u0440\u044B\u0435 \u043D\u0443\u0436\u043D\u043E \u043C\u043E\u0434\u0438\u0444\u0438\u0446\u0438\u0440\u043E\u0432\u0430\u0442\u044C:"),

        hTable(
          ["\u0424\u0430\u0439\u043B", "\u0428\u0430\u0433\u0438", "\u0421\u0443\u043C\u043C\u0430 \u0438\u0437\u043C\u0435\u043D\u0435\u043D\u0438\u0439"],
          [
            ["src/builtins.rs", "1, 2, 7, 8, 9, 10", "+150 \u0441\u0442\u0440\u043E\u043A"],
            ["src/server.rs", "1, 3, 4", "+80 \u0441\u0442\u0440\u043E\u043A"],
            ["src/interpreter.rs", "1", "+30 \u0441\u0442\u0440\u043E\u043A"],
            ["src/llm.rs", "5, 6", "+200 \u0441\u0442\u0440\u043E\u043A"],
            ["src/ast.rs", "8 (\u0435\u0441\u043B\u0438 \u043D\u0443\u0436\u0435\u043D)", "+5 \u0441\u0442\u0440\u043E\u043A"],
            ["README.md", "7", "+15 \u0441\u0442\u0440\u043E\u043A"],
            ["FOSVED/entrypoint.sh", "7", "+2 \u0441\u0442\u0440\u043E\u043A\u0438"],
          ],
          [30, 30, 40]
        ),

        emptyLine(),

        // ═══════ 5. TEIROVANIE ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 5. \u0422\u0435\u0441\u0442\u0438\u0440\u043E\u0432\u0430\u043D\u0438\u0435"),

        h2("5.1. \u041F\u043E\u0441\u043B\u0435 \u043A\u0430\u0436\u0434\u043E\u0433\u043E \u0448\u0430\u0433\u0430"),
        body("1. cargo build --release \u2014 \u043F\u0440\u043E\u0432\u0435\u0440\u043A\u0430 \u043A\u043E\u043C\u043F\u0438\u043B\u044F\u0446\u0438\u0438. 2. cargo test --lib \u2014 \u0432\u0441\u0435 \u0442\u0435\u0441\u0442\u044B \u043F\u0440\u043E\u0445\u043E\u0434\u044F\u0442. 3. cargo test -- --ignored \u2014 LLM-\u0442\u0435\u0441\u0442\u044B \u0441 \u043D\u0430\u0441\u0442\u043E\u044F\u0449\u0438\u043C API (\u043F\u043E \u0436\u0435\u043B\u0430\u043D\u0438\u044E). 4. \u041A\u043E\u043C\u043C\u0438\u0442 \u0438 \u043F\u0443\u0448."),

        h2("5.2. \u0418\u0442\u043E\u0433\u043E\u0432\u043E\u0435 \u0442\u0435\u0441\u0442\u0438\u0440\u043E\u0432\u0430\u043D\u0438\u0435"),
        body("1. \u0417\u0430\u043C\u0435\u043D\u0438\u0442\u044C bin/mlog \u0432 FOSVED-office-v2 \u043D\u0430 \u043D\u043E\u0432\u044B\u0439 \u0431\u0438\u043D\u0430\u0440\u043D\u0438\u043A. 2. \u0417\u0430\u043F\u0443\u0441\u0442\u0438\u0442\u044C \u043B\u043E\u043A\u0430\u043B\u044C\u043D\u043E: mlog serve app.mlog. 3. \u041F\u0440\u043E\u0432\u0435\u0440\u0438\u0442\u044C \u044D\u043D\u0434\u043F\u043E\u0439\u043D\u0442\u044B: GET /status, GET /health, GET /report?id=test, GET /miniapp?doc=test, POST /webhook/telegram. 4. \u041F\u0440\u043E\u0432\u0435\u0440\u0438\u0442\u044C /test-llm \u0441 \u0440\u0435\u0430\u043B\u044C\u043D\u044B\u043C LLM. 5. \u0417\u0430\u043F\u0443\u0441\u0442\u0438\u0442\u044C \u043C\u0438\u043D\u0438\u0430\u043F\u043F \u0432 Telegram, \u043F\u0440\u043E\u0432\u0435\u0440\u0438\u0442\u044C \u043A\u043D\u043E\u043F\u043A\u0438 (Deepen, Red Team, Watch)."),

        h2("5.3. \u041C\u0438\u0433\u0440\u0430\u0446\u0438\u044F"),
        body("1. \u041C\u0435\u0440\u0433\u0435 fix/metalogos-runtime \u0432 main \u0447\u0435\u0440\u0435\u0437 Pull Request. 2. \u0423\u0434\u0430\u043B\u0438\u0442\u044C llm_proxy.py.disabled \u0438\u0437 FOSVED Docker. 3. \u0423\u0434\u0430\u043B\u0438\u0442\u044C sanitize_mlog.py \u0438\u0437 FOSVED (\u0435\u0441\u043B\u0438 \u043A\u0438\u0440\u0438\u043B\u043B\u0438\u0446\u0430 \u043F\u043E\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442\u0441\u044F). 4. \u041E\u0431\u043D\u043E\u0432\u0438\u0442\u044C Dockerfile FOSVED: \u0443\u0431\u0440\u0430\u0442\u044C Python-\u0437\u0430\u0432\u0438\u0441\u0438\u043C\u043E\u0441\u0442\u0438, \u0435\u0441\u043B\u0438 \u0435\u0441\u0442\u044C."),

        // ═══════ 6. PROTOKOL OTCHYOTA ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 6. \u041F\u0440\u043E\u0442\u043E\u043A\u043E\u043B \u043E\u0442\u0447\u0451\u0442\u0430 \u043F\u043E \u043A\u0430\u0436\u0434\u043E\u043C\u0443 \u0448\u0430\u0433\u0443"),
        body("\u041F\u043E\u0441\u043B\u0435 \u0432\u044B\u043F\u043E\u043B\u043D\u0435\u043D\u0438\u044F \u043A\u0430\u0436\u0434\u043E\u0433\u043E \u0448\u0430\u0433\u0430 \u0434\u043E\u043B\u0436\u0435\u043D \u0431\u044B\u0442\u044C \u043F\u043E\u0434\u0433\u043E\u0442\u043E\u0432\u043B\u0435\u043D \u043E\u0442\u0447\u0451\u0442 \u0441\u043E \u0441\u043B\u0435\u0434\u0443\u044E\u0449\u0438\u043C \u0441\u043E\u0434\u0435\u0440\u0436\u0438\u043C\u044B\u043C:"),

        hTable(
          ["\u041F\u043E\u043B\u0435", "\u041E\u043F\u0438\u0441\u0430\u043D\u0438\u0435"],
          [
            ["\u041D\u043E\u043C\u0435\u0440 \u0448\u0430\u0433\u0430", "\u0421\u043E\u043E\u0442\u0432\u0435\u0442\u0441\u0442\u0432\u0438\u0435 \u043F\u043B\u0430\u043D\u0443 (0-10)"],
            ["\u041A\u043E\u043C\u043C\u0438\u0442 hash", "\u041F\u043E\u043B\u043D\u044B\u0439 SHA \u043A\u043E\u043C\u043C\u0438\u0442\u0430"],
            ["\u041A\u043E\u043C\u043F\u0438\u043B\u044F\u0446\u0438\u044F", "cargo build --release: \u0443\u0441\u043F\u0435\u0448\u043D\u043E / \u043E\u0448\u0438\u0431\u043A\u0438"],
            ["\u0422\u0435\u0441\u0442\u044B", "cargo test: X/104 pass, \u0434\u0435\u0442\u0430\u043B\u0438 \u043F\u0430\u0434\u0435\u043D\u0438\u0439"],
            ["\u0418\u0437\u043C\u0435\u043D\u0451\u043D\u043D\u044B\u0435 \u0444\u0430\u0439\u043B\u044B", "\u0421\u043F\u0438\u0441\u043E\u043A \u0444\u0430\u0439\u043B\u043E\u0432 \u0438 +/\u2212 \u0441\u0442\u0440\u043E\u043A"],
            ["\u041E\u0442\u043A\u0430\u0442 \u043E\u0431\u0440\u0430\u0442\u043D\u043E \u0441\u043E\u0432\u043C\u0435\u0441\u0442\u0438\u043C\u043E\u0441\u0442\u0438", "\u0427\u0442\u043E \u0441\u043B\u043E\u043C\u0430\u043B\u043E\u0441\u044C \u0438 \u043A\u0430\u043A \u0447\u0438\u043D\u0438\u0442\u044C"],
            ["\u0417\u0430\u043C\u0435\u0447\u0430\u043D\u0438\u044F", "\u041F\u0440\u043E\u0431\u043B\u0435\u043C\u044B, \u0440\u0438\u0441\u043A\u0438, \u0434\u0435\u0432\u0438\u0430\u0446\u0438\u0438 \u043E\u0442 \u043F\u043B\u0430\u043D\u0430"],
          ],
          [30, 70]
        ),

        emptyLine(),

        // ═══════ 7. RISKI ═══════
        h1("\u0420\u0430\u0437\u0434\u0435\u043B 7. \u0420\u0438\u0441\u043A\u0438 \u0438 \u043C\u0438\u0442\u0438\u0433\u0430\u0446\u0438\u044F"),

        h2("7.1. \u041A\u0440\u0438\u0442\u0438\u0447\u0435\u0441\u043A\u0438\u0435 \u0440\u0438\u0441\u043A\u0438"),
        hTable(
          ["\u0420\u0438\u0441\u043A", "\u0412\u0435\u0440\u043E\u044F\u0442\u043D\u043E\u0441\u0442\u044C", "\u041C\u0438\u0442\u0438\u0433\u0430\u0446\u0438\u044F"],
          [
            ["spawn_blocking \u043D\u0435 \u0440\u0435\u0448\u0430\u0435\u0442 thread starvation", "\u0421\u0440\u0435\u0434\u043D\u044F\u044F", "\u0422\u0435\u0441\u0442 \u0441 5+ concurrent \u0437\u0430\u043F\u0440\u043E\u0441\u0430\u043C\u0438"],
            ["Fallback LLM \u0443\u0432\u0435\u043B\u0438\u0447\u0438\u0432\u0430\u0435\u0442 \u0432\u0440\u0435\u043C\u044F \u043E\u0442\u0432\u0435\u0442\u0430", "\u0412\u044B\u0441\u043E\u043A\u0430\u044F", "\u041F\u0430\u0440\u0430\u043B\u043B\u0435\u043B\u044C\u043D\u044B\u0439 \u0437\u0430\u043F\u0440\u043E\u0441 \u043A \u043F\u0440\u043E\u0432\u0430\u0439\u0434\u0435\u0440\u0430\u043C"],
            ["\u0418\u0437\u043C\u0435\u043D\u0435\u043D\u0438\u0435 call() \u0441\u043B\u043E\u043C\u0430\u0435\u0442 learnable patterns", "\u0412\u044B\u0441\u043E\u043A\u0430\u044F", "call_legacy() \u0434\u043B\u044F backward compat"],
            ["\u041D\u0435\u0441\u043E\u0432\u043C\u0435\u0441\u0442\u0438\u043C\u043E\u0441\u0442\u044C \u0441 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u043C\u0438 .mlog", "\u041D\u0438\u0437\u043A\u0430\u044F", "\u0412\u0441\u0435 \u0438\u0437\u043C\u0435\u043D\u0435\u043D\u0438\u044F \u0434\u043E\u0431\u0430\u0432\u043B\u0435\u043D\u0438\u044F (\u043D\u0435 \u0431\u0440\u0435\u043A\u0438\u043D\u0433)"],
          ],
          [35, 15, 50]
        ),

        h2("7.2. \u041F\u0440\u0438\u043D\u0446\u0438\u043F\u044B"),
        body("\u0411\u0435\u0437\u043E\u043F\u0430\u0441\u043D\u043E\u0441\u0442\u044C \u043F\u0435\u0440\u0432\u0430\u044F: \u043D\u0438\u043A\u0430\u043A\u0438\u0445 \u0443\u0434\u0430\u043B\u0435\u043D\u0438\u0439 \u0438\u0437 FOSVED \u0431\u0435\u0437 \u043F\u0440\u043E\u0432\u0435\u0440\u043A\u0438. \u041A\u0430\u0436\u0434\u044B\u0439 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u044E\u0449\u0438\u0439 workaround \u0443\u0434\u0430\u043B\u044F\u0435\u0442\u0441\u044F \u0442\u043E\u043B\u044C\u043A\u043E \u043F\u043E\u0441\u043B\u0435 \u043F\u043E\u0434\u0442\u0432\u0435\u0440\u0436\u0434\u0435\u043D\u0438\u044F, \u0447\u0442\u043E \u043D\u0430\u0442\u0438\u0432\u043D\u0430\u044F \u0440\u0435\u0430\u043B\u0438\u0437\u0430\u0446\u0438\u044F \u0440\u0430\u0431\u043E\u0442\u0430\u0435\u0442. llm_proxy.py \u0443\u0436\u0435 DISABLED \u2014 \u043D\u043E \u043D\u0435 \u0443\u0434\u0430\u043B\u044F\u0442\u044C, \u043F\u043E\u043A\u0430 fallback \u043D\u0435 \u0432 Metalogos. sanitize_mlog.py \u2014 \u043D\u0435 \u0443\u0434\u0430\u043B\u044F\u0442\u044C, \u043F\u043E\u043A\u0430 \u043A\u0438\u0440\u0438\u043B\u043B\u0438\u0446\u0430 \u0432 .mlog \u043D\u0435 \u043F\u043E\u0434\u0434\u0435\u0440\u0436\u0438\u0432\u0430\u0435\u0442\u0441\u044F."),
      ],
    },
  ],
});

// ── Generate ──
const OUTPUT = "/home/z/my-project/download/METALOGOS_FIX_PLAN.docx";
Packer.toBuffer(doc).then((buf) => {
  fs.writeFileSync(OUTPUT, buf);
  console.log("Generated: " + OUTPUT);
});
