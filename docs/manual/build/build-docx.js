// Builds the Word manual from content.<lang>.js, figures/<lang>/ and art/.
// Usage: node build-docx.js <lang> <outDir>
const fs = require('fs');
const path = require('path');
const {
  AlignmentType,
  BorderStyle,
  Document,
  ExternalHyperlink,
  FrameAnchorType,
  Footer,
  Header,
  HeadingLevel,
  HorizontalPositionRelativeFrom,
  ImageRun,
  LevelFormat,
  NumberFormat,
  Packer,
  PageNumber,
  Paragraph,
  SectionType,
  ShadingType,
  Table,
  TableCell,
  TableLayoutType,
  TableOfContents,
  TableRow,
  TextRun,
  TextWrappingType,
  VerticalAlign,
  VerticalPositionRelativeFrom,
  WidthType,
} = require('docx');

const [lang, outDir] = process.argv.slice(2);
const C = require(`./content.${lang}.js`);
const FIG = path.join(__dirname, 'figures', lang);
const ART = path.join(__dirname, 'art');
const zh = lang.startsWith('zh');
// The app version, from the workspace manifest.
const VERSION = fs.readFileSync(path.join(__dirname, '..', '..', '..', 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)[1];

// ── design tokens ──
const BLUE = '1D4FFF';
const DEEP = '1638B8';
const TEXT = '1B1B1B';
const MUTED = '5B6170';
const RULE = 'D5DAE3';
const TINT = 'EEF2FF';
const FONT = zh ? { ascii: 'Segoe UI', hAnsi: 'Segoe UI', eastAsia: C.font, cs: 'Segoe UI' } : { ascii: C.font, hAnsi: C.font, eastAsia: C.font, cs: C.font };
const MONO = { ascii: C.monoFont, hAnsi: C.monoFont, eastAsia: zh ? C.font : C.monoFont, cs: C.monoFont };
const SEP = zh ? '　' : ' '; // between number and title
const PAGE = { w: 11906, h: 16838 }; // A4 in twips
const MARGIN = { top: 1474, bottom: 1361, left: 1304, right: 1304 };
const CONTENT_TW = PAGE.w - MARGIN.left - MARGIN.right;
const CM = 37.795; // px per cm (docx images use 96 dpi pixels)
const MAX_IMG_CM = (CONTENT_TW / 1440) * 2.54;

// ── inline markup: **bold**, [[Key]], `code` ──
function runs(text, base = {}) {
  const out = [];
  for (const part of String(text).split(/(\*\*[^*]+\*\*|\[\[[^\]]+\]\]|`[^`]+`)/)) {
    if (!part) continue;
    if (part.startsWith('**')) out.push(new TextRun({ ...base, text: part.slice(2, -2), bold: true }));
    else if (part.startsWith('[['))
      out.push(
        new TextRun({
          ...base,
          text: ` ${part.slice(2, -2)} `,
          bold: true,
          size: Math.round((base.size ?? 21) * 0.86),
          color: '2B3140',
          shading: { type: ShadingType.CLEAR, fill: 'F1F3F7', color: 'auto' },
          border: { style: BorderStyle.SINGLE, size: 4, color: 'C3C9D4', space: 0 },
        }),
      );
    else if (part.startsWith('`'))
      out.push(new TextRun({ ...base, text: part.slice(1, -1), font: MONO, size: Math.round((base.size ?? 21) * 0.9), color: '1F2A44', shading: { type: ShadingType.CLEAR, fill: 'F3F5F9', color: 'auto' } }));
    else out.push(new TextRun({ ...base, text: part }));
  }
  return out;
}

const para = (text, opts = {}, runOpts = {}) => new Paragraph({ ...opts, children: runs(text, runOpts) });

// ── figures ──
function jpegSize(buf) {
  let i = 2;
  while (i < buf.length) {
    const marker = buf.readUInt16BE(i);
    const len = buf.readUInt16BE(i + 2);
    if (marker >= 0xffc0 && marker <= 0xffc3) return { height: buf.readUInt16BE(i + 5), width: buf.readUInt16BE(i + 7) };
    i += 2 + len;
  }
  throw new Error('bad jpeg');
}
function image(name, widthCm) {
  const data = fs.readFileSync(path.join(FIG, `${name}.jpg`));
  const { width, height } = jpegSize(data);
  const w = Math.min(widthCm ?? MAX_IMG_CM, MAX_IMG_CM) * CM;
  return new ImageRun({ type: 'jpg', data, transformation: { width: Math.round(w), height: Math.round((w * height) / width) }, altText: { name, title: name, description: name } });
}

let chapter = 0;
let appendix = 0;
let section = 0;
let figNo = 0;
let numberingInstance = 0;
const chapterId = () => (appendix ? String.fromCharCode(64 + appendix) : String(chapter));

function caption(text) {
  figNo++;
  return new Paragraph({
    alignment: AlignmentType.CENTER,
    spacing: { before: 60, after: 260 },
    keepLines: true,
    children: [
      new TextRun({ text: `${C.labels.figure(`${chapterId()}-${figNo}`)}${SEP}`, bold: true, color: DEEP, size: 18 }),
      ...runs(text, { color: MUTED, size: 18 }),
    ],
  });
}

// ── tables ──
const thin = { style: BorderStyle.SINGLE, size: 4, color: RULE };
const none = { style: BorderStyle.NONE, size: 0, color: 'FFFFFF' };
function cellParas(text, runOpts) {
  return String(text)
    .split('\n')
    .map((line) => new Paragraph({ spacing: { before: 0, after: 0, line: 300 }, children: runs(line, runOpts) }));
}
function table({ cols, head, rows }) {
  const widths = cols.map((p) => Math.round((CONTENT_TW * p) / 100));
  const mk = (cells, isHead) =>
    new TableRow({
      tableHeader: isHead,
      cantSplit: true,
      children: cells.map(
        (t, i) =>
          new TableCell({
            width: { size: widths[i], type: WidthType.DXA },
            margins: { top: 90, bottom: 90, left: 140, right: 140 },
            verticalAlign: VerticalAlign.CENTER,
            shading: isHead ? { type: ShadingType.CLEAR, fill: TINT, color: 'auto' } : undefined,
            children: cellParas(t, isHead ? { bold: true, color: DEEP, size: 19 } : { size: 19, color: TEXT }),
          }),
      ),
    });
  return new Table({
    width: { size: CONTENT_TW, type: WidthType.DXA },
    columnWidths: widths,
    layout: TableLayoutType.FIXED,
    borders: { top: thin, bottom: thin, left: none, right: none, insideHorizontal: thin, insideVertical: none },
    rows: [mk(head, true), ...rows.map((r) => mk(r, false))],
  });
}
const gap = (after = 160) => new Paragraph({ spacing: { before: 0, after }, children: [] });

function callout(kind, text) {
  const color = kind === 'tip' ? BLUE : 'C27C0E';
  const fill = kind === 'tip' ? 'F0F4FF' : 'FFF7E6';
  return new Table({
    width: { size: CONTENT_TW, type: WidthType.DXA },
    columnWidths: [CONTENT_TW],
    borders: { top: none, bottom: none, right: none, insideHorizontal: none, insideVertical: none, left: { style: BorderStyle.SINGLE, size: 24, color } },
    rows: [
      new TableRow({
        cantSplit: true,
        children: [
          new TableCell({
            width: { size: CONTENT_TW, type: WidthType.DXA },
            margins: { top: 120, bottom: 120, left: 220, right: 200 },
            shading: { type: ShadingType.CLEAR, fill, color: 'auto' },
            children: [
              new Paragraph({
                spacing: { before: 0, after: 0, line: 312 },
                children: [new TextRun({ text: `${C.labels[kind]}${SEP}`, bold: true, color }), ...runs(text, { color: TEXT })],
              }),
            ],
          }),
        ],
      }),
    ],
  });
}

function codeBlock(text) {
  return new Table({
    width: { size: CONTENT_TW, type: WidthType.DXA },
    columnWidths: [CONTENT_TW],
    borders: { top: thin, bottom: thin, left: thin, right: thin, insideHorizontal: none, insideVertical: none },
    rows: [
      new TableRow({
        children: [
          new TableCell({
            margins: { top: 100, bottom: 100, left: 180, right: 180 },
            shading: { type: ShadingType.CLEAR, fill: 'F6F8FB', color: 'auto' },
            children: [new Paragraph({ spacing: { before: 0, after: 0 }, children: [new TextRun({ text, font: MONO, size: 18, color: '1F2A44' })] })],
          }),
        ],
      }),
    ],
  });
}

function figRow(items, cap) {
  const half = Math.floor(CONTENT_TW / 2);
  const cell = ([name, sub]) =>
    new TableCell({
      width: { size: half, type: WidthType.DXA },
      margins: { top: 0, bottom: 0, left: 80, right: 80 },
      children: [
        new Paragraph({ alignment: AlignmentType.CENTER, keepNext: true, children: [image(name, MAX_IMG_CM / 2 - 0.4)] }),
        new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 40, after: 0 }, children: runs(sub, { size: 17, color: MUTED }) }),
      ],
    });
  return [
    new Table({
      width: { size: CONTENT_TW, type: WidthType.DXA },
      columnWidths: [half, half],
      borders: { top: none, bottom: none, left: none, right: none, insideHorizontal: none, insideVertical: none },
      rows: [new TableRow({ cantSplit: true, children: items.map(cell) })],
    }),
    caption(cap),
  ];
}

// ── body ──
function body() {
  const out = [];
  for (const b of C.blocks) {
    if (b.h1) {
      if (b.appendix) appendix++;
      else chapter++;
      section = 0;
      figNo = 0;
      const label = b.appendix ? C.labels.appendix(chapterId()) : C.labels.chapter(chapter);
      out.push(
        new Paragraph({
          pageBreakBefore: true,
          spacing: { before: 0, after: 80 },
          children: [new TextRun({ text: label.toUpperCase(), color: BLUE, bold: true, size: 20, characterSpacing: zh ? 40 : 60 })],
        }),
      );
      out.push(
        new Paragraph({
          heading: HeadingLevel.HEADING_1,
          children: [new TextRun({ text: b.appendix ? `${chapterId()}${SEP}${b.h1}` : `${chapter}${SEP}${b.h1}` })],
        }),
      );
    } else if (b.h2) {
      section++;
      out.push(new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun({ text: `${chapterId()}.${section}${SEP}${b.h2}` })] }));
    } else if (b.h3) out.push(new Paragraph({ heading: HeadingLevel.HEADING_3, children: [new TextRun({ text: b.h3 })] }));
    else if (b.p) out.push(para(b.p));
    else if (b.ul) for (const t of b.ul) out.push(new Paragraph({ numbering: { reference: 'bullets', level: 0 }, spacing: { after: 80 }, children: runs(t) }));
    else if (b.ol) {
      numberingInstance++;
      for (const t of b.ol) out.push(new Paragraph({ numbering: { reference: 'steps', level: 0, instance: numberingInstance }, spacing: { after: 80 }, children: runs(t) }));
      out.push(gap(60));
    } else if (b.fig) {
      out.push(new Paragraph({ alignment: AlignmentType.CENTER, keepNext: true, spacing: { before: 160, after: 0 }, children: [image(b.fig, b.width)] }));
      out.push(caption(b.cap));
    } else if (b.figs) out.push(...figRow(b.figs, b.cap));
    else if (b.table) {
      out.push(table(b.table));
      out.push(gap(200));
    } else if (b.tip || b.note) {
      out.push(callout(b.tip ? 'tip' : 'note', b.tip ?? b.note));
      out.push(gap(200));
    } else if (b.code) {
      out.push(codeBlock(b.code));
      out.push(gap(200));
    } else if (b.qa) {
      for (const [q, a] of b.qa) {
        out.push(
          new Paragraph({
            keepNext: true,
            spacing: { before: 200, after: 60 },
            children: [new TextRun({ text: `Q${SEP}`, bold: true, color: BLUE }), ...runs(q, { bold: true })],
          }),
        );
        out.push(new Paragraph({ indent: { left: 440 }, spacing: { after: 120 }, children: runs(a) }));
      }
    } else throw new Error(`unknown block ${JSON.stringify(b)}`);
  }
  return out;
}

// ── cover and back cover ──
function art(name) {
  return new ImageRun({
    type: 'svg',
    data: fs.readFileSync(path.join(ART, `${name}.svg`)),
    fallback: { type: 'png', data: fs.readFileSync(path.join(ART, `${name}.png`)) },
    transformation: { width: 794, height: 1123 },
    floating: {
      horizontalPosition: { relative: HorizontalPositionRelativeFrom.PAGE, offset: 0 },
      verticalPosition: { relative: VerticalPositionRelativeFrom.PAGE, offset: 0 },
      behindDocument: true,
      allowOverlap: true,
      wrap: { type: TextWrappingType.NONE },
    },
    altText: { name, title: name, description: C.docTitle },
  });
}
const mm = (v) => Math.round(v * 56.7);
// A paragraph placed at an absolute position on the page (x, y, width in mm).
function placed(x, y, w, h, children, align = AlignmentType.LEFT) {
  return new Paragraph({
    frame: { type: 'absolute', position: { x: mm(x), y: mm(y) }, width: mm(w), height: mm(h), anchor: { horizontal: FrameAnchorType.PAGE, vertical: FrameAnchorType.PAGE } },
    alignment: align,
    spacing: { before: 0, after: 0, line: 240 },
    children,
  });
}

function coverPage() {
  const c = C.cover;
  return [
    new Paragraph({ children: [art('cover')] }),
    placed(19, 64, 160, 22, [new TextRun({ text: c.title, bold: true, size: 96, color: 'FFFFFF' })]),
    placed(19, 88, 120, 16, [new TextRun({ text: c.subtitle, size: 60, color: 'FFFFFF' })]),
    placed(19, 108, 125, 20, [new TextRun({ text: c.tagline, size: 26, color: 'FFFFFF' })]),
    placed(19, 238, 150, 8, [new TextRun({ text: c.edition, size: 24, bold: true, color: TEXT })]),
    placed(19, 247, 150, 14, [new TextRun({ text: c.platforms, size: 20, color: MUTED })]),
    placed(19, 284, 172, 6, [new TextRun({ text: 'github.com/Lrniey666/qrcode-tool', size: 17, color: MUTED })]),
  ];
}

function backPage() {
  const k = C.back;
  const out = [new Paragraph({ children: [art('back')] })];
  out.push(placed(19, 34, 172, 14, [new TextRun({ text: k.heading, bold: true, size: 40, color: TEXT })]));
  let y = 54;
  for (const t of k.body) {
    out.push(placed(19, y, 172, 22, runs(t, { size: 21, color: TEXT })));
    y += 24;
  }
  y += 4;
  for (const [label, link] of k.links) {
    out.push(
      placed(19, y, 172, 7, [
        new TextRun({ text: `${label}${SEP}`, bold: true, size: 20, color: DEEP }),
        new ExternalHyperlink({ link, children: [new TextRun({ text: link, size: 20, color: BLUE, underline: {} })] }),
      ]),
    );
    y += 8;
  }
  y += 8;
  out.push(placed(19, y, 172, 14, runs(k.license, { size: 17, color: MUTED })));
  out.push(placed(19, y + 15, 172, 14, runs(k.trademark, { size: 17, color: MUTED })));
  out.push(placed(19, 282, 172, 8, [new TextRun({ text: `QRcode tool ${VERSION}`, bold: true, size: 20, color: 'FFFFFF' })], AlignmentType.CENTER));
  return out;
}

// ── headers / footers ──
const blankHeader = () => new Header({ children: [new Paragraph({ children: [] })] });
const blankFooter = () => new Footer({ children: [new Paragraph({ children: [] })] });
const header = () =>
  new Header({
    children: [
      new Paragraph({
        alignment: AlignmentType.RIGHT,
        border: { bottom: { style: BorderStyle.SINGLE, size: 4, color: RULE, space: 4 } },
        children: [new TextRun({ text: C.labels.header, size: 16, color: MUTED })],
      }),
    ],
  });
const footer = () =>
  new Footer({
    children: [new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ children: [PageNumber.CURRENT], size: 18, color: MUTED })] })],
  });

const pageProps = (extra = {}) => ({ page: { size: { width: PAGE.w, height: PAGE.h }, margin: { ...MARGIN, header: 709, footer: 567 }, ...extra } });

const doc = new Document({
  creator: 'QRcode tool project',
  title: C.docTitle,
  description: C.cover.tagline,
  features: { updateFields: true },
  styles: {
    default: {
      document: { run: { font: FONT, size: 21, color: TEXT }, paragraph: { spacing: { after: 140, line: zh ? 336 : 312 } } },
      heading1: {
        run: { font: FONT, size: 44, bold: true, color: TEXT },
        paragraph: { spacing: { before: 0, after: 360 }, keepNext: true, border: { bottom: { style: BorderStyle.SINGLE, size: 12, color: BLUE, space: 8 } } },
      },
      heading2: { run: { font: FONT, size: 28, bold: true, color: DEEP }, paragraph: { spacing: { before: 360, after: 140 }, keepNext: true } },
      // Not in the table of contents (it lists levels 1–2).
      heading3: { run: { font: FONT, size: 23, bold: true, color: TEXT }, paragraph: { spacing: { before: 240, after: 100 }, keepNext: true } },
    },
    // Compact table of contents (Word applies "toc 1" / "toc 2" to the generated entries).
    paragraphStyles: [
      { id: 'TOC1', name: 'toc 1', basedOn: 'Normal', next: 'Normal', run: { bold: true, size: 21, color: TEXT }, paragraph: { spacing: { before: 150, after: 30, line: 264 } } },
      { id: 'TOC2', name: 'toc 2', basedOn: 'Normal', next: 'Normal', run: { size: 19, color: MUTED }, paragraph: { indent: { left: 440 }, spacing: { before: 0, after: 20, line: 264 } } },
    ],
  },
  numbering: {
    config: [
      {
        reference: 'bullets',
        levels: [{ level: 0, format: LevelFormat.BULLET, text: '•', alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 440, hanging: 260 } }, run: { color: BLUE } } }],
      },
      {
        reference: 'steps',
        levels: [{ level: 0, format: LevelFormat.DECIMAL, text: '%1.', alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 440, hanging: 340 } }, run: { color: BLUE, bold: true } } }],
      },
    ],
  },
  sections: [
    {
      properties: { ...pageProps(), titlePage: false },
      headers: { default: blankHeader() },
      footers: { default: blankFooter() },
      children: coverPage(),
    },
    {
      properties: { type: SectionType.NEXT_PAGE, ...pageProps({ pageNumbers: { start: 1, formatType: NumberFormat.LOWER_ROMAN } }) },
      headers: { default: header() },
      footers: { default: footer() },
      children: [
        new Paragraph({ spacing: { before: 0, after: 360 }, border: { bottom: { style: BorderStyle.SINGLE, size: 12, color: BLUE, space: 8 } }, children: [new TextRun({ text: C.labels.toc, bold: true, size: 44 })] }),
        new TableOfContents(C.labels.toc, { hyperlink: true, headingStyleRange: '1-2' }),
      ],
    },
    {
      properties: { type: SectionType.NEXT_PAGE, ...pageProps({ pageNumbers: { start: 1, formatType: NumberFormat.DECIMAL } }) },
      headers: { default: header() },
      footers: { default: footer() },
      children: body(),
    },
    {
      properties: { type: SectionType.NEXT_PAGE, ...pageProps() },
      headers: { default: blankHeader() },
      footers: { default: blankFooter() },
      children: backPage(),
    },
  ],
});

fs.mkdirSync(outDir, { recursive: true });
Packer.toBuffer(doc).then((buf) => {
  const file = path.join(outDir, `${C.file}.docx`);
  fs.writeFileSync(file, buf);
  console.log(`${file} ${(buf.length / 1024 / 1024).toFixed(1)} MB, ${chapter} chapters, ${appendix} appendices`);
});
