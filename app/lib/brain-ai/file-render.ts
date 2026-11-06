/**
 * Génération pure de fichiers (Word, PDF, Excel) à partir d'un contenu structuré
 * fourni par l'assistant. Aucune dépendance serveur : testable sans S3.
 */
import {
  BorderStyle,
  Document,
  HeadingLevel,
  Packer,
  Paragraph,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from "docx";
import { PDFDocument, StandardFonts, rgb, type PDFFont, type PDFPage } from "pdf-lib";
import * as XLSX from "xlsx";

export type FileFormat = "docx" | "pdf" | "xlsx";

export type FileBlock =
  | { type: "heading"; text: string; level?: number }
  | { type: "paragraph"; text: string }
  | { type: "bullets"; items: string[] }
  | { type: "table"; headers: string[]; rows: string[][] };

export type FileSpec = { format: FileFormat; title: string; blocks: FileBlock[] };

export const FILE_LIMITS = {
  maxBlocks: 200,
  maxTableRows: 2000,
  maxCharsPerBlock: 20_000,
  maxBytes: 10 * 1024 * 1024,
  maxColumns: 30,
  maxBullets: 500,
} as const;

export const FILE_MIME: Record<FileFormat, string> = {
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  pdf: "application/pdf",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
};

export class FileRenderError extends Error {}

const CONTROL_CHARS = new RegExp(
  "[\u0000-\u0009\u000B-\u001F\u007F-\u009F\u200B-\u200F\u202A-\u202E\u2028\u2029\uFEFF]",
  "g",
);

/** Retire les caractères de contrôle (sauf saut de ligne ; tabulation → espace). */
export function cleanText(v: unknown): string {
  return String(v ?? "")
    .replace(/\r\n?/g, "\n")
    .replace(/\t/g, " ")
    .replace(CONTROL_CHARS, "");
}

function cleanLine(v: unknown): string {
  return cleanText(v).replace(/\n+/g, " ").trim();
}

function checkLen(s: string) {
  if (s.length > FILE_LIMITS.maxCharsPerBlock) {
    throw new FileRenderError(`Un bloc dépasse ${FILE_LIMITS.maxCharsPerBlock} caractères.`);
  }
  return s;
}

/** Valide et nettoie la spécification brute venant du modèle. */
export function normalizeSpec(raw: { format?: unknown; title?: unknown; blocks?: unknown }): FileSpec {
  const format = String(raw.format || "").toLowerCase();
  if (format !== "docx" && format !== "pdf" && format !== "xlsx") {
    throw new FileRenderError("Format inconnu (docx, pdf ou xlsx).");
  }
  if (!Array.isArray(raw.blocks) || raw.blocks.length === 0) {
    throw new FileRenderError("Aucun contenu à écrire dans le fichier.");
  }
  if (raw.blocks.length > FILE_LIMITS.maxBlocks) {
    throw new FileRenderError(`Trop de blocs (maximum ${FILE_LIMITS.maxBlocks}).`);
  }
  const blocks: FileBlock[] = [];
  for (const b of raw.blocks) {
    if (!b || typeof b !== "object") continue;
    const o = b as Record<string, unknown>;
    const type = String(o.type || "");
    if (type === "heading") {
      const level = Math.min(2, Math.max(1, Math.round(Number(o.level)) || 1));
      blocks.push({ type, text: checkLen(cleanLine(o.text)), level });
    } else if (type === "paragraph") {
      blocks.push({ type, text: checkLen(cleanText(o.text)) });
    } else if (type === "bullets") {
      if (!Array.isArray(o.items)) continue;
      if (o.items.length > FILE_LIMITS.maxBullets) {
        throw new FileRenderError(`Trop de puces (maximum ${FILE_LIMITS.maxBullets}).`);
      }
      blocks.push({ type, items: o.items.map((i) => checkLen(cleanLine(i))) });
    } else if (type === "table") {
      if (!Array.isArray(o.headers) || !Array.isArray(o.rows)) continue;
      if (o.rows.length > FILE_LIMITS.maxTableRows) {
        throw new FileRenderError(`Tableau trop long (maximum ${FILE_LIMITS.maxTableRows} lignes).`);
      }
      if (o.headers.length === 0 || o.headers.length > FILE_LIMITS.maxColumns) {
        throw new FileRenderError(`Un tableau doit avoir entre 1 et ${FILE_LIMITS.maxColumns} colonnes.`);
      }
      const headers = o.headers.map((h) => checkLen(cleanLine(h)));
      const rows = o.rows.map((r) => {
        const cells = Array.isArray(r) ? r : [];
        return headers.map((_, i) => checkLen(cleanLine(cells[i])));
      });
      blocks.push({ type, headers, rows });
    }
  }
  if (blocks.length === 0) throw new FileRenderError("Aucun bloc valide dans le contenu fourni.");
  return { format, title: cleanLine(raw.title).slice(0, 200), blocks };
}

/** Nom de fichier sûr, sans extension. */
export function safeBaseName(name: unknown): string {
  const base = cleanLine(name)
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/\.(docx|pdf|xlsx)$/i, "")
    .replace(/[^a-zA-Z0-9_-]+/g, "_")
    .replace(/^[_-]+|[_-]+$/g, "")
    .slice(0, 80);
  return base || "document";
}

// ───────────────────────────── Word ─────────────────────────────

async function renderDocx(spec: FileSpec): Promise<Buffer> {
  const children: (Paragraph | Table)[] = [];
  if (spec.title) {
    children.push(
      new Paragraph({ heading: HeadingLevel.TITLE, spacing: { after: 240 }, children: [new TextRun(spec.title)] }),
    );
  }
  for (const b of spec.blocks) {
    if (b.type === "heading") {
      children.push(
        new Paragraph({
          heading: b.level === 2 ? HeadingLevel.HEADING_2 : HeadingLevel.HEADING_1,
          spacing: { before: 200, after: 120 },
          children: [new TextRun(b.text)],
        }),
      );
    } else if (b.type === "paragraph") {
      for (const part of b.text.split(/\n{2,}/)) {
        const runs = part.split("\n").map((l, i) => new TextRun({ text: l, break: i > 0 ? 1 : 0 }));
        children.push(new Paragraph({ spacing: { after: 140 }, children: runs }));
      }
    } else if (b.type === "bullets") {
      for (const item of b.items) {
        children.push(new Paragraph({ bullet: { level: 0 }, children: [new TextRun(item)] }));
      }
      children.push(new Paragraph({ children: [] }));
    } else {
      const border = { style: BorderStyle.SINGLE, size: 4, color: "999999" };
      const borders = { top: border, bottom: border, left: border, right: border };
      const mk = (text: string, head: boolean) =>
        new TableCell({
          borders,
          shading: head ? { fill: "E8ECEF" } : undefined,
          margins: { top: 40, bottom: 40, left: 80, right: 80 },
          children: [new Paragraph({ children: [new TextRun({ text, bold: head, size: 20 })] })],
        });
      children.push(
        new Table({
          width: { size: 100, type: WidthType.PERCENTAGE },
          rows: [
            new TableRow({ tableHeader: true, children: b.headers.map((h) => mk(h, true)) }),
            ...b.rows.map((r) => new TableRow({ children: r.map((c) => mk(c, false)) })),
          ],
        }),
        new Paragraph({ children: [] }),
      );
    }
  }
  const doc = new Document({
    title: spec.title || undefined,
    styles: { default: { document: { run: { font: "Calibri", size: 22 } } } },
    sections: [{ properties: { page: { size: { width: 11906, height: 16838 } } }, children }],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

// ───────────────────────────── PDF ─────────────────────────────

/** Remplace ce que Helvetica (WinAnsi) ne sait pas écrire ; garde accents, œ, €, guillemets. */
function pdfSafe(text: string, supported: Set<number>): string {
  let out = "";
  for (const ch of text.replace(/[  ]/g, " ")) {
    const cp = ch.codePointAt(0)!;
    if (supported.has(cp)) {
      out += ch;
      continue;
    }
    if (ch === "‑" || ch === "‐") {
      out += "-";
      continue;
    }
    const base = ch.normalize("NFKD").replace(/[̀-ͯ]/g, "");
    out += base && [...base].every((c) => supported.has(c.codePointAt(0)!)) ? base : "?";
  }
  return out;
}

function wrap(text: string, font: PDFFont, size: number, maxWidth: number): string[] {
  const lines: string[] = [];
  for (const para of text.split("\n")) {
    let line = "";
    for (let word of para.split(" ")) {
      while (font.widthOfTextAtSize(word, size) > maxWidth) {
        let n = word.length;
        while (n > 1 && font.widthOfTextAtSize(word.slice(0, n), size) > maxWidth) n -= 1;
        if (line) {
          lines.push(line);
          line = "";
        }
        lines.push(word.slice(0, n));
        word = word.slice(n);
      }
      const cand = line ? `${line} ${word}` : word;
      if (line && font.widthOfTextAtSize(cand, size) > maxWidth) {
        lines.push(line);
        line = word;
      } else {
        line = cand;
      }
    }
    lines.push(line);
  }
  return lines;
}

async function renderPdf(spec: FileSpec): Promise<Buffer> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const supported = new Set<number>([...font.getCharacterSet()]);
  const safe = (s: string) => pdfSafe(s, supported);
  if (spec.title) doc.setTitle(safe(spec.title));

  const W = 595.28;
  const H = 841.89;
  const M = 56;
  const contentW = W - 2 * M;
  const INK = rgb(0.1, 0.12, 0.16);
  let page: PDFPage = doc.addPage([W, H]);
  let y = H - M;
  const newPage = () => {
    page = doc.addPage([W, H]);
    y = H - M;
  };
  const ensure = (h: number) => {
    if (y - h < M) newPage();
  };
  const text = (t: string, o: { size: number; bold?: boolean; indent?: number; gap?: number; bullet?: boolean }) => {
    const f = o.bold ? bold : font;
    const lead = o.size * 1.35;
    const lines = wrap(safe(t), f, o.size, contentW - (o.indent ?? 0));
    lines.forEach((l, i) => {
      ensure(lead);
      y -= lead;
      if (o.bullet && i === 0) page.drawText("-", { x: M + 5, y, size: o.size, font, color: INK });
      if (l) page.drawText(l, { x: M + (o.indent ?? 0), y, size: o.size, font: f, color: INK });
    });
    y -= o.gap ?? 0;
  };

  const drawTable = (b: Extract<FileBlock, { type: "table" }>) => {
    const n = b.headers.length;
    const size = n > 6 ? 8 : 9.5;
    const pad = 4;
    const lead = size * 1.3;
    const want = b.headers.map((h, i) =>
      Math.max(
        30,
        Math.max(
          bold.widthOfTextAtSize(safe(h), size),
          ...b.rows.slice(0, 200).map((r) => font.widthOfTextAtSize(safe(r[i] ?? "").slice(0, 60), size)),
        ) +
          pad * 2,
      ),
    );
    const sum = want.reduce((a, c) => a + c, 0);
    const scale = sum > contentW ? contentW / sum : 1;
    const cols = want.map((w) => Math.max(20, w * scale));

    const drawRow = (cells: string[], head: boolean) => {
      const f = head ? bold : font;
      const wrapped = cells.map((c, i) => wrap(safe(c), f, size, cols[i]! - pad * 2));
      const rowH = Math.max(...wrapped.map((l) => l.length)) * lead + pad * 2;
      if (y - rowH < M) {
        newPage();
        if (!head) drawRow(b.headers, true);
      }
      const top = y;
      let x = M;
      cols.forEach((cw, i) => {
        if (head) page.drawRectangle({ x, y: top - rowH, width: cw, height: rowH, color: rgb(0.91, 0.93, 0.94) });
        page.drawRectangle({
          x,
          y: top - rowH,
          width: cw,
          height: rowH,
          borderColor: rgb(0.6, 0.62, 0.66),
          borderWidth: 0.5,
        });
        wrapped[i]!.forEach((l, k) => {
          if (l) page.drawText(l, { x: x + pad, y: top - pad - size - k * lead + 2, size, font: f, color: INK });
        });
        x += cw;
      });
      y = top - rowH;
    };
    drawRow(b.headers, true);
    for (const r of b.rows) drawRow(r, false);
    y -= 12;
  };

  if (spec.title) text(spec.title, { size: 20, bold: true, gap: 14 });
  for (const b of spec.blocks) {
    if (b.type === "heading") {
      ensure(40);
      y -= 6;
      text(b.text, { size: b.level === 2 ? 13 : 16, bold: true, gap: 6 });
    } else if (b.type === "paragraph") {
      text(b.text, { size: 11, gap: 8 });
    } else if (b.type === "bullets") {
      for (const item of b.items) text(item, { size: 11, indent: 16, bullet: true });
      y -= 8;
    } else {
      drawTable(b);
    }
  }
  return Buffer.from(await doc.save());
}

// ───────────────────────────── Excel ─────────────────────────────

function sheetName(raw: string, used: Set<string>, fallback: string): string {
  const base =
    raw
      .replace(/[\\/?*[\]:]/g, " ")
      .replace(/\s+/g, " ")
      .trim()
      .replace(/^'+|'+$/g, "")
      .slice(0, 31) || fallback;
  let name = base;
  let n = 2;
  while (used.has(name.toLowerCase())) {
    const suffix = ` (${n++})`;
    name = base.slice(0, 31 - suffix.length) + suffix;
  }
  used.add(name.toLowerCase());
  return name;
}

/** Feuille dont toutes les cellules sont du texte : jamais interprétées comme formule. */
function textSheet(rows: string[][]): XLSX.WorkSheet {
  const ws: XLSX.WorkSheet = {};
  let maxC = 0;
  rows.forEach((r, ri) => {
    maxC = Math.max(maxC, r.length);
    r.forEach((v, ci) => {
      ws[XLSX.utils.encode_cell({ r: ri, c: ci })] = { t: "s", v };
    });
  });
  ws["!ref"] = XLSX.utils.encode_range({
    s: { r: 0, c: 0 },
    e: { r: Math.max(0, rows.length - 1), c: Math.max(0, maxC - 1) },
  });
  ws["!cols"] = Array.from({ length: maxC }, (_, c) => ({
    wch: Math.min(60, Math.max(8, ...rows.slice(0, 300).map((r) => (r[c] ?? "").length + 2))),
  }));
  return ws;
}

function renderXlsx(spec: FileSpec): Buffer {
  const wb = XLSX.utils.book_new();
  const used = new Set<string>();
  const notes: string[][] = [];
  if (spec.title) notes.push([spec.title]);
  let lastHeading = "";
  let tableCount = 0;
  for (const b of spec.blocks) {
    if (b.type === "heading") {
      lastHeading = b.text;
      notes.push([b.text]);
    } else if (b.type === "paragraph") {
      notes.push([b.text]);
    } else if (b.type === "bullets") {
      b.items.forEach((i) => notes.push([`- ${i}`]));
    } else {
      tableCount += 1;
      const hint = lastHeading || (tableCount === 1 ? spec.title : "");
      XLSX.utils.book_append_sheet(wb, textSheet([b.headers, ...b.rows]), sheetName(hint, used, `Tableau ${tableCount}`));
      lastHeading = "";
    }
  }
  if (tableCount === 0) {
    XLSX.utils.book_append_sheet(wb, textSheet(notes.length ? notes : [[""]]), sheetName("Notes", used, "Notes"));
  }
  return Buffer.from(XLSX.write(wb, { type: "buffer", bookType: "xlsx" }) as Buffer);
}

// ───────────────────────────── Entrée ─────────────────────────────

export async function renderFile(spec: FileSpec): Promise<Buffer> {
  const buf =
    spec.format === "docx" ? await renderDocx(spec) : spec.format === "pdf" ? await renderPdf(spec) : renderXlsx(spec);
  if (buf.length === 0) throw new FileRenderError("Le fichier généré est vide.");
  if (buf.length > FILE_LIMITS.maxBytes) throw new FileRenderError("Le fichier dépasse 10 Mo : réduire le contenu.");
  return buf;
}
