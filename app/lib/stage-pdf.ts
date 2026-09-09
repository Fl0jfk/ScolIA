import { PDFDocument, StandardFonts, rgb, type PDFFont, type PDFImage, type PDFPage, type RGB } from "pdf-lib";
import { formatDaySlotLabel } from "@/app/lib/stage-schedule";
import {
  STAGE_OFFER_KIND_LABELS,
  STAGE_SIGNER_ROLE_LABELS,
  type StageConvention,
  type StageSignature,
} from "@/app/lib/stage-types";

export type StageConventionPdfLogo = {
  bytes: Uint8Array;
  format: "PNG" | "JPEG";
  width?: number;
  height?: number;
};

export type StageConventionPdfSchoolContext = {
  schoolName: string;
  schoolAddress: string;
  schoolPhone?: string;
  schoolMail?: string;
  rgpdContact: string;
  insuranceText: string;
  /** Accent tenant (#RRGGBB) — sinon teal doux. */
  accentHex?: string;
  logo?: StageConventionPdfLogo | null;
};

/** Référence assurance collège (modèle officiel) — repli si non configurée. */
const DEFAULT_INSURANCE =
  "Mutuelles Saint-Christophe assurances — Police n° 0020850051435787277 — 27 rue Saint Jacques, 75256 Paris cedex 05";

const PAGE_W = 595.28;
const PAGE_H = 841.89;

function hexToRgb(hex: string | undefined): { r: number; g: number; b: number } {
  const h = (hex || "#2F6F5E").replace("#", "").trim();
  const full =
    h.length === 3
      ? h
          .split("")
          .map((c) => c + c)
          .join("")
      : h.padEnd(6, "0").slice(0, 6);
  const n = parseInt(full, 16);
  if (!Number.isFinite(n)) return { r: 0.18, g: 0.44, b: 0.37 };
  return {
    r: ((n >> 16) & 255) / 255,
    g: ((n >> 8) & 255) / 255,
    b: (n & 255) / 255,
  };
}

function soft(c: { r: number; g: number; b: number }, amount: number): RGB {
  return rgb(c.r + (1 - c.r) * amount, c.g + (1 - c.g) * amount, c.b + (1 - c.b) * amount);
}

function sanitizePdfText(input: string): string {
  return input
    .replace(/[\u2018\u2019]/g, "'")
    .replace(/[\u201C\u201D]/g, '"')
    .replace(/[\u2013\u2014]/g, "-")
    .replace(/\u2026/g, "...")
    .replace(/\u00A0/g, " ")
    .replace(/œ/g, "oe")
    .replace(/Œ/g, "Oe")
    // Puces / symboles hors WinAnsi → ASCII (évite les « ? »)
    .replace(/[\u2022\u2023\u25E6\u2043\u2219]/g, "-")
    .replace(/[^\x00-\xFF]/g, "?");
}

function wrapText(text: string, font: PDFFont, size: number, maxWidth: number): string[] {
  const clean = sanitizePdfText(text);
  const words = clean.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let line = "";
  for (const w of words) {
    const next = line ? `${line} ${w}` : w;
    if (font.widthOfTextAtSize(next, size) > maxWidth && line) {
      lines.push(line);
      line = w;
    } else {
      line = next;
    }
  }
  if (line) lines.push(line);
  return lines.length ? lines : [""];
}

function formatFrDate(iso: string): string {
  if (!iso) return "—";
  const d = new Date(`${iso.slice(0, 10)}T12:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

function dash(value: string | undefined | null): string {
  const v = (value || "").trim();
  return v || "—";
}

function fitBox(imgW: number, imgH: number, maxW: number, maxH: number) {
  if (!imgW || !imgH) return { width: maxW, height: maxH };
  const scale = Math.min(maxW / imgW, maxH / imgH);
  return { width: imgW * scale, height: imgH * scale };
}

/** Rectangle arrondi — y = bas du rectangle (coords PDF). */
function roundedRect(
  page: PDFPage,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
  opts: { fill?: RGB; border?: RGB; borderWidth?: number },
) {
  const radius = Math.min(Math.max(r, 0), w / 2, h / 2);
  const path = [
    `M ${radius} 0`,
    `L ${w - radius} 0`,
    `Q ${w} 0 ${w} ${radius}`,
    `L ${w} ${h - radius}`,
    `Q ${w} ${h} ${w - radius} ${h}`,
    `L ${radius} ${h}`,
    `Q 0 ${h} 0 ${h - radius}`,
    `L 0 ${radius}`,
    `Q 0 0 ${radius} 0`,
    "Z",
  ].join(" ");
  page.drawSvgPath(path, {
    x,
    y: y + h,
    color: opts.fill,
    borderColor: opts.border,
    borderWidth: opts.borderWidth ?? 0,
  });
}

type Palette = {
  accent: RGB;
  accentSoft: RGB;
  accentSofter: RGB;
  accentMid: RGB;
  ink: RGB;
  muted: RGB;
  soft: RGB;
  line: RGB;
  white: RGB;
  pageWash: RGB;
  periodBg: RGB;
};

type PdfCtx = {
  doc: PDFDocument;
  page: PDFPage;
  font: PDFFont;
  bold: PDFFont;
  margin: number;
  contentW: number;
  y: number;
  pageIndex: number;
  palette: Palette;
  logoImage: PDFImage | null;
  logoSize: { width: number; height: number } | null;
  schoolName: string;
};

function makePalette(accentHex?: string): Palette {
  const a = hexToRgb(accentHex);
  return {
    accent: rgb(a.r, a.g, a.b),
    accentSoft: soft(a, 0.88),
    accentSofter: soft(a, 0.94),
    accentMid: soft(a, 0.55),
    ink: rgb(0.12, 0.14, 0.16),
    muted: rgb(0.42, 0.45, 0.48),
    soft: rgb(0.62, 0.65, 0.68),
    line: soft(a, 0.78),
    white: rgb(1, 1, 1),
    pageWash: rgb(0.985, 0.988, 0.986),
    periodBg: rgb(0.99, 0.97, 0.93),
  };
}

function newPage(ctx: PdfCtx) {
  ctx.page = ctx.doc.addPage([PAGE_W, PAGE_H]);
  ctx.pageIndex += 1;
  paintPageBackground(ctx);
  drawPageFooter(ctx);
  ctx.y = PAGE_H - 48;
}

function ensureSpace(ctx: PdfCtx, needed: number) {
  if (ctx.y - needed < 58) newPage(ctx);
}

function paintPageBackground(ctx: PdfCtx) {
  ctx.page.drawRectangle({
    x: 0,
    y: 0,
    width: PAGE_W,
    height: PAGE_H,
    color: ctx.palette.pageWash,
  });
  // Halo doux en haut à droite
  roundedRect(ctx.page, PAGE_W - 220, PAGE_H - 180, 240, 200, 80, {
    fill: ctx.palette.accentSofter,
  });
}

function drawPageFooter(ctx: PdfCtx) {
  const { page, pageIndex, palette, font, margin, schoolName } = ctx;
  roundedRect(page, margin, 18, ctx.contentW, 22, 11, {
    fill: palette.white,
    border: palette.line,
    borderWidth: 0.6,
  });
  page.drawText(sanitizePdfText(`${schoolName}  ·  Convention de stage`), {
    x: margin + 12,
    y: 25,
    size: 7,
    font,
    color: palette.muted,
  });
  const right = sanitizePdfText(`p. ${pageIndex}`);
  page.drawText(right, {
    x: PAGE_W - margin - 12 - font.widthOfTextAtSize(right, 7),
    y: 25,
    size: 7,
    font,
    color: palette.soft,
  });
}

function drawText(
  ctx: PdfCtx,
  text: string,
  opts: {
    size?: number;
    bold?: boolean;
    color?: RGB;
    indent?: number;
    maxWidth?: number;
    lineGap?: number;
  } = {},
) {
  const size = opts.size ?? 9.2;
  const font = opts.bold ? ctx.bold : ctx.font;
  const color = opts.color ?? ctx.palette.ink;
  const indent = opts.indent ?? 0;
  const maxWidth = opts.maxWidth ?? ctx.contentW - indent;
  const lineGap = opts.lineGap ?? 3.4;
  const lines = wrapText(text, font, size, maxWidth);
  for (const line of lines) {
    ensureSpace(ctx, size + lineGap + 2);
    ctx.page.drawText(sanitizePdfText(line), {
      x: ctx.margin + indent,
      y: ctx.y,
      size,
      font,
      color,
    });
    ctx.y -= size + lineGap;
  }
}

function measureLines(text: string, font: PDFFont, size: number, maxWidth: number, lineGap: number) {
  const lines = wrapText(text, font, size, maxWidth);
  return { lines, height: lines.length * (size + lineGap) };
}

async function rasterizeToPngBytes(bytes: Uint8Array): Promise<Uint8Array | null> {
  try {
    const { createCanvas, loadImage } = await import("@napi-rs/canvas");
    const img = await loadImage(Buffer.from(bytes));
    const w = Math.max(1, img.width || 512);
    const h = Math.max(1, img.height || 512);
    const max = 1024;
    const scale = Math.min(1, max / Math.max(w, h));
    const cw = Math.max(1, Math.round(w * scale));
    const ch = Math.max(1, Math.round(h * scale));
    const canvas = createCanvas(cw, ch);
    const c = canvas.getContext("2d");
    c.clearRect(0, 0, cw, ch);
    c.drawImage(img, 0, 0, cw, ch);
    return new Uint8Array(canvas.toBuffer("image/png"));
  } catch (e) {
    console.warn("[stage-pdf] rasterize logo failed", e);
    return null;
  }
}

async function embedLogo(
  doc: PDFDocument,
  logo: StageConventionPdfLogo | null | undefined,
): Promise<{ image: PDFImage; size: { width: number; height: number } } | null> {
  if (!logo?.bytes?.length) return null;
  const raw = logo.bytes;
  const isJpg = raw[0] === 0xff && raw[1] === 0xd8;
  const isPng = raw[0] === 0x89 && raw[1] === 0x50;

  try {
    let image: PDFImage;
    if (isJpg) {
      image = await doc.embedJpg(raw);
    } else if (isPng) {
      image = await doc.embedPng(raw);
    } else {
      const png = await rasterizeToPngBytes(raw);
      if (!png) return null;
      image = await doc.embedPng(png);
    }
    const size = fitBox(logo.width || image.width, logo.height || image.height, 108, 64);
    return { image, size };
  } catch (e) {
    console.warn("[stage-pdf] embed logo failed, trying rasterize", e);
    try {
      const png = await rasterizeToPngBytes(raw);
      if (!png) return null;
      const image = await doc.embedPng(png);
      const size = fitBox(logo.width || image.width, logo.height || image.height, 108, 64);
      return { image, size };
    } catch (e2) {
      console.error("[stage-pdf] embed logo impossible", e2);
      return null;
    }
  }
}

function drawHeader(ctx: PdfCtx, schoolYear: string, refId: string) {
  const { palette, margin, contentW } = ctx;
  const headerH = 104;
  ensureSpace(ctx, headerH + 16);
  const yBottom = ctx.y - headerH;

  roundedRect(ctx.page, margin, yBottom, contentW, headerH, 20, {
    fill: palette.white,
    border: palette.line,
    borderWidth: 0.8,
  });

  let textX = margin + 20;
  if (ctx.logoImage && ctx.logoSize) {
    const lx = margin + 16;
    const ly = yBottom + (headerH - ctx.logoSize.height) / 2;
    ctx.page.drawImage(ctx.logoImage, {
      x: lx,
      y: ly,
      width: ctx.logoSize.width,
      height: ctx.logoSize.height,
    });
    textX = lx + ctx.logoSize.width + 14;
  }

  roundedRect(ctx.page, textX, yBottom + 12, 48, 4, 2, {
    fill: palette.accent,
  });

  ctx.page.drawText(sanitizePdfText(ctx.schoolName), {
    x: textX,
    y: yBottom + headerH - 30,
    size: 11,
    font: ctx.bold,
    color: palette.ink,
  });
  ctx.page.drawText(sanitizePdfText("Convention de stage"), {
    x: textX,
    y: yBottom + headerH - 50,
    size: 17,
    font: ctx.bold,
    color: palette.accent,
  });
  ctx.page.drawText(
    sanitizePdfText("Séquence d'observation et de sensibilisation en entreprise"),
    {
      x: textX,
      y: yBottom + headerH - 68,
      size: 8,
      font: ctx.font,
      color: palette.muted,
    },
  );

  const badge = "Signature électronique";
  const badgeW = ctx.bold.widthOfTextAtSize(sanitizePdfText(badge), 7) + 18;
  const badgeH = 18;
  const bx = margin + contentW - badgeW - 16;
  const by = yBottom + headerH - 36;
  roundedRect(ctx.page, bx, by, badgeW, badgeH, 9, {
    fill: palette.accentSoft,
  });
  ctx.page.drawText(sanitizePdfText(badge), {
    x: bx + 9,
    y: by + 5.5,
    size: 7,
    font: ctx.bold,
    color: palette.accent,
  });

  const rightEdge = margin + contentW - 16;
  const yearLine = sanitizePdfText(`Année ${schoolYear}`);
  const yearW = ctx.font.widthOfTextAtSize(yearLine, 6.5);
  ctx.page.drawText(yearLine, {
    x: rightEdge - yearW,
    y: yBottom + 28,
    size: 6.5,
    font: ctx.font,
    color: palette.soft,
  });

  const maxRefW = Math.min(badgeW + 8, contentW * 0.42);
  let refLine = sanitizePdfText(`N° ${refId}`);
  while (ctx.font.widthOfTextAtSize(refLine, 6) > maxRefW && refLine.length > 14) {
    const keep = Math.max(8, refId.length - (refLine.length - 14));
    refLine = sanitizePdfText(`N° …${refId.slice(-keep)}`);
  }
  const refW = ctx.font.widthOfTextAtSize(refLine, 6);
  ctx.page.drawText(refLine, {
    x: rightEdge - refW,
    y: yBottom + 16,
    size: 6,
    font: ctx.font,
    color: palette.soft,
  });

  ctx.y = yBottom - 18;
}

function drawPartyCard(
  ctx: PdfCtx,
  x: number,
  yTop: number,
  w: number,
  h: number,
  title: string,
  rows: Array<{ label: string; value: string }>,
) {
  const { palette } = ctx;
  roundedRect(ctx.page, x, yTop - h, w, h, 14, {
    fill: palette.white,
    border: palette.line,
    borderWidth: 0.7,
  });
  roundedRect(ctx.page, x + 10, yTop - 22, w - 20, 14, 7, {
    fill: palette.accentSoft,
  });
  ctx.page.drawText(sanitizePdfText(title), {
    x: x + 18,
    y: yTop - 18,
    size: 8,
    font: ctx.bold,
    color: palette.accent,
  });

  let y = yTop - 38;
  for (const row of rows) {
    const label = `${row.label}  `;
    ctx.page.drawText(sanitizePdfText(label), {
      x: x + 14,
      y,
      size: 7,
      font: ctx.bold,
      color: palette.soft,
    });
    const labelW = ctx.bold.widthOfTextAtSize(sanitizePdfText(label), 7);
    const valueLines = wrapText(dash(row.value), ctx.font, 7.5, w - 28 - labelW);
    ctx.page.drawText(sanitizePdfText(valueLines[0] || "—"), {
      x: x + 14 + labelW,
      y,
      size: 7.5,
      font: ctx.font,
      color: palette.ink,
    });
    y -= 13;
    for (let i = 1; i < valueLines.length && y > yTop - h + 10; i++) {
      ctx.page.drawText(sanitizePdfText(valueLines[i]!), {
        x: x + 14,
        y,
        size: 7.5,
        font: ctx.font,
        color: palette.ink,
      });
      y -= 11;
    }
  }
}

function drawArticleBlock(ctx: PdfCtx, num: number, title: string, bodyBlocks: string[]) {
  const { palette, margin, contentW, font, bold } = ctx;
  const padX = 14;
  const padY = 12;
  const titleSize = 10;
  const bodySize = 9;
  const bodyGap = 3.2;
  const innerW = contentW - padX * 2;

  let contentH = 18; // title row
  const measured: Array<{ lines: string[]; height: number }> = [];
  for (const block of bodyBlocks) {
    const m = measureLines(block, font, bodySize, innerW, bodyGap);
    measured.push(m);
    contentH += m.height + 6;
  }
  contentH += padY;

  ensureSpace(ctx, contentH + 14);
  const yBottom = ctx.y - contentH;

  roundedRect(ctx.page, margin, yBottom, contentW, contentH, 16, {
    fill: palette.white,
    border: palette.line,
    borderWidth: 0.65,
  });

  // Pastille numéro
  const pill = 22;
  roundedRect(ctx.page, margin + padX, ctx.y - padY - pill + 4, pill, pill, 11, {
    fill: palette.accent,
  });
  const numStr = String(num);
  const numW = bold.widthOfTextAtSize(numStr, 9);
  ctx.page.drawText(numStr, {
    x: margin + padX + (pill - numW) / 2,
    y: ctx.y - padY - 11,
    size: 9,
    font: bold,
    color: palette.white,
  });

  ctx.page.drawText(sanitizePdfText(title), {
    x: margin + padX + pill + 10,
    y: ctx.y - padY - 10,
    size: titleSize,
    font: bold,
    color: palette.ink,
  });

  let ty = ctx.y - padY - 28;
  for (const m of measured) {
    for (const line of m.lines) {
      ctx.page.drawText(sanitizePdfText(line), {
        x: margin + padX,
        y: ty,
        size: bodySize,
        font,
        color: palette.ink,
      });
      ty -= bodySize + bodyGap;
    }
    ty -= 4;
  }

  ctx.y = yBottom - 12;
}

function drawBulletBlock(ctx: PdfCtx, num: number, title: string, intro: string, bullets: string[]) {
  const blocks = [intro, ...bullets.map((b) => `-  ${b}`)];
  drawArticleBlock(ctx, num, title, blocks);
}

function drawPeriodBanner(ctx: PdfCtx, convention: StageConvention) {
  const { palette, margin, contentW } = ctx;
  const h = convention.stageLabel ? 44 : 34;
  ensureSpace(ctx, h + 12);
  const yBottom = ctx.y - h;
  roundedRect(ctx.page, margin, yBottom, contentW, h, 14, {
    fill: palette.periodBg,
    border: rgb(0.9, 0.82, 0.62),
    borderWidth: 0.7,
  });
  ctx.page.drawText(
    sanitizePdfText(
      `Période  ·  du ${formatFrDate(convention.schedule.periodStart)} au ${formatFrDate(convention.schedule.periodEnd)}`,
    ),
    {
      x: margin + 16,
      y: yBottom + h - 18,
      size: 10,
      font: ctx.bold,
      color: palette.ink,
    },
  );
  if (convention.stageLabel) {
    ctx.page.drawText(sanitizePdfText(convention.stageLabel), {
      x: margin + 16,
      y: yBottom + 10,
      size: 8,
      font: ctx.font,
      color: palette.muted,
    });
  }
  ctx.y = yBottom - 12;
}

function drawScheduleTable(ctx: PdfCtx, convention: StageConvention) {
  const { palette, margin, contentW } = ctx;
  const rows = convention.schedule.days.map((d) => formatDaySlotLabel(d));
  const rowH = 16;
  const headerH = 22;
  const h = headerH + rows.length * rowH + 12;
  ensureSpace(ctx, h + 10);
  const yBottom = ctx.y - h;

  roundedRect(ctx.page, margin, yBottom, contentW, h, 14, {
    fill: palette.white,
    border: palette.line,
    borderWidth: 0.65,
  });
  roundedRect(ctx.page, margin + 10, yBottom + h - headerH - 4, contentW - 20, headerH - 2, 9, {
    fill: palette.accentSoft,
  });
  ctx.page.drawText(sanitizePdfText("Horaires de présence"), {
    x: margin + 20,
    y: yBottom + h - 18,
    size: 8,
    font: ctx.bold,
    color: palette.accent,
  });

  let y = yBottom + h - headerH - 14;
  for (const row of rows) {
    const lines = wrapText(row, ctx.font, 8, contentW - 36);
    ctx.page.drawText(sanitizePdfText(lines[0] || "—"), {
      x: margin + 18,
      y,
      size: 8,
      font: ctx.font,
      color: palette.ink,
    });
    y -= rowH;
  }
  ctx.y = yBottom - 12;
}

/** Marqueur PDF (subject) — page annexes réservée aux paraphes électroniques. */
export const STAGE_ESIGN_ANNEX_SUBJECT = "SCOLIA_ESIGN_ANNEX";

export const STAGE_ESIGN_PAGE_TITLE = "Signatures électroniques";

function signatureBoxLabel(sig: StageSignature): string {
  return STAGE_SIGNER_ROLE_LABELS[sig.role] || sig.label || sig.role;
}

/** Positions des cases signature (coords PDF, y = bas de la case) — page annexes. */
export function electronicSignatureBoxLayout(params: {
  pageWidth: number;
  pageHeight: number;
  index: number;
  total: number;
}): { x: number; y: number; width: number; height: number } {
  const margin = 40;
  const contentW = params.pageWidth - margin * 2;
  const gap = 12;
  const boxW = (contentW - gap) / 2;
  const boxH = 82;
  const headerReserve = 120;
  const col = params.index % 2;
  const row = Math.floor(params.index / 2);
  const x = margin + col * (boxW + gap);
  const yTop = params.pageHeight - headerReserve - row * (boxH + gap);
  return { x, y: yTop - boxH, width: boxW, height: boxH };
}

function drawSignatureGrid(ctx: PdfCtx, signatures: StageSignature[]) {
  const { palette, margin, contentW } = ctx;
  // Toujours une page dédiée : n'empiète pas sur d'éventuelles signatures manuscrites.
  newPage(ctx);
  ctx.doc.setSubject(STAGE_ESIGN_ANNEX_SUBJECT);

  ensureSpace(ctx, 40);
  drawText(ctx, STAGE_ESIGN_PAGE_TITLE, { size: 14, bold: true, color: palette.accent });
  drawText(
    ctx,
    "Page réservée aux signatures et paraphes électroniques. Les signatures manuscrites (document papier) restent sur les pages précédentes.",
    { size: 8.5, color: palette.muted },
  );
  ctx.y -= 10;

  const gap = 12;
  const boxW = (contentW - gap) / 2;
  const boxH = 82;
  const list = signatures.length
    ? signatures
    : ([
        { id: "a", role: "tuteur_entreprise", label: "Organisme d'accueil", status: "en_attente" },
        { id: "b", role: "parent", label: "Representant legal", status: "en_attente" },
        { id: "c", role: "eleve", label: "Eleve", status: "en_attente" },
        { id: "d", role: "direction", label: "Direction", status: "en_attente" },
      ] as StageSignature[]);

  for (let i = 0; i < list.length; i++) {
    if (i > 0 && i % 2 === 0) {
      ctx.y -= boxH + gap;
      ensureSpace(ctx, boxH + 24);
    }
    const sig = list[i]!;
    const col = i % 2;
    const x = margin + col * (boxW + gap);
    const yTop = ctx.y;

    roundedRect(ctx.page, x, yTop - boxH, boxW, boxH, 14, {
      fill: palette.white,
      border: palette.accentMid,
      borderWidth: 1,
    });
    roundedRect(ctx.page, x + 10, yTop - 22, boxW - 20, 14, 7, {
      fill: palette.accentSoft,
    });
    ctx.page.drawText(sanitizePdfText(signatureBoxLabel(sig)), {
      x: x + 18,
      y: yTop - 18,
      size: 8,
      font: ctx.bold,
      color: palette.accent,
    });

    const status =
      sig.status === "signe"
        ? `Signe le ${sig.signedAt ? new Date(sig.signedAt).toLocaleDateString("fr-FR") : "—"} — ${dash(sig.signedBy)}`
        : "En attente de signature";
    ctx.page.drawText(sanitizePdfText(status), {
      x: x + 14,
      y: yTop - 36,
      size: 7,
      font: ctx.font,
      color: palette.muted,
    });
    ctx.page.drawText(sanitizePdfText("Zone signature / paraphe électronique"), {
      x: x + 14,
      y: yTop - boxH + 14,
      size: 7,
      font: ctx.font,
      color: palette.soft,
    });
  }
  const rows = Math.ceil(list.length / 2);
  ctx.y -= rows * (boxH + gap) + 8;
}

export async function renderStageConventionPdf(
  convention: StageConvention,
  school: StageConventionPdfSchoolContext,
): Promise<Uint8Array> {
  const schoolName = school.schoolName || "Etablissement scolaire";
  const schoolAddress = school.schoolAddress || "";
  const schoolPhone = school.schoolPhone || "";
  const schoolMail = school.schoolMail || "";
  const rgpdContact = school.rgpdContact || "la direction de l'etablissement";
  const insurance = school.insuranceText?.trim() || DEFAULT_INSURANCE;

  const kindLabel =
    convention.internshipKind in STAGE_OFFER_KIND_LABELS
      ? STAGE_OFFER_KIND_LABELS[convention.internshipKind as keyof typeof STAGE_OFFER_KIND_LABELS]
      : convention.internshipKind;

  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const embedded = await embedLogo(doc, school.logo);

  const ctx: PdfCtx = {
    doc,
    page: doc.addPage([PAGE_W, PAGE_H]),
    font,
    bold,
    margin: 36,
    contentW: PAGE_W - 72,
    y: PAGE_H - 40,
    pageIndex: 1,
    palette: makePalette(school.accentHex),
    logoImage: embedded?.image ?? null,
    logoSize: embedded?.size ?? null,
    schoolName,
  };

  paintPageBackground(ctx);
  drawPageFooter(ctx);
  drawHeader(ctx, convention.schoolYear, convention.id);

  drawText(ctx, "Entre les soussignés", {
    size: 10,
    bold: true,
    color: ctx.palette.accent,
  });
  ctx.y -= 8;

  const cardH = 96;
  const cardGap = 12;
  const cardW = (ctx.contentW - cardGap) / 2;
  ensureSpace(ctx, cardH * 2 + cardGap + 20);

  const yCards = ctx.y;
  drawPartyCard(ctx, ctx.margin, yCards, cardW, cardH, "Organisme d'accueil", [
    { label: "Nom", value: convention.company.name },
    { label: "Adresse", value: convention.company.address },
    { label: "SIRET", value: convention.company.siret || "" },
    { label: "Activité", value: convention.company.activity },
  ]);
  drawPartyCard(ctx, ctx.margin + cardW + cardGap, yCards, cardW, cardH, "Tuteur / maître de stage", [
    { label: "Nom", value: convention.company.tutorName },
    { label: "E-mail", value: convention.company.tutorEmail },
    { label: "Téléphone", value: convention.company.tutorPhone || "" },
    { label: "RH", value: convention.company.rhEmail || "" },
  ]);

  const yCards2 = yCards - cardH - cardGap;
  drawPartyCard(ctx, ctx.margin, yCards2, cardW, cardH, "Élève", [
    {
      label: "Nom",
      value: `${convention.student.lastName} ${convention.student.firstName}`.trim(),
    },
    { label: "Classe", value: `${convention.student.className} — ${convention.student.level}` },
    { label: "E-mail", value: convention.student.email || "" },
    { label: "Type", value: kindLabel },
  ]);
  drawPartyCard(ctx, ctx.margin + cardW + cardGap, yCards2, cardW, cardH, "Établissement", [
    { label: "Nom", value: schoolName },
    { label: "Adresse", value: schoolAddress },
    {
      label: "Contact",
      value: [schoolPhone, schoolMail].filter(Boolean).join(" — "),
    },
    {
      label: "Référent",
      value: `${convention.teacherReferent.name} (${convention.teacherReferent.email})`,
    },
  ]);
  ctx.y = yCards2 - cardH - 18;

  drawArticleBlock(ctx, 1, "Objet de la convention", [
    "La présente convention a pour objet la mise en oeuvre, au bénéfice de l'élève désigné, d'une séquence d'observation en milieu professionnel réalisée dans le cadre de sa formation scolaire.",
  ]);

  drawBulletBlock(
    ctx,
    2,
    "Dispositions de la convention",
    "La convention doit être signée par :",
    [
      "le chef d'établissement ;",
      "le représentant de l'entreprise ou de l'organisme d'accueil ;",
      "l'élève ;",
      "s'il est mineur, son représentant légal ;",
      "ainsi que le maître de stage chargé du suivi de l'élève.",
    ],
  );

  drawArticleBlock(ctx, 3, "Finalité de la séquence d'observation", [
    "La séquence d'observation vise à sensibiliser l'élève à l'environnement technologique, économique et professionnel, en lien avec les programmes d'enseignement et l'éducation à l'orientation. Les modalités précises de la séquence sont détaillées dans l'annexe pédagogique.",
  ]);

  drawArticleBlock(ctx, 4, "Accueil et suivi de l'élève", [
    "L'organisation de la séquence est déterminée d'un commun accord entre le responsable de l'organisme d'accueil et le chef d'établissement scolaire.",
  ]);

  drawArticleBlock(ctx, 5, "Statut de l'élève", [
    "Pendant toute la séquence d'observation, l'élève conserve son statut scolaire et reste sous l'autorité du chef d'établissement. Il demeure soumis au règlement intérieur de l'établissement et ne peut prétendre à aucune rémunération.",
  ]);

  drawArticleBlock(ctx, 6, "Tâches confiées et obligations", [
    "L'élève n'a pas à concourir directement au travail de l'entreprise.",
    "Il peut réaliser des enquêtes en lien avec ses enseignements et être associé à certaines activités, à visée pédagogique uniquement.",
    "Le maître de stage et le professeur référent veillent au bon déroulement de la séquence.",
    "L'élève doit respecter les règles de sécurité, d'horaires et de discipline.",
    "Il est tenu au secret professionnel et à la discrétion : aucun renseignement confidentiel ne peut apparaître dans son rapport de stage.",
  ]);

  drawArticleBlock(ctx, 7, "Durée et horaires", [
    "La durée maximale de présence est de 30 heures hebdomadaires pour les élèves de moins de 15 ans, et de 35 heures pour les élèves de plus de 15 ans.",
    "La durée quotidienne ne peut excéder 7 heures. Une pause d'au moins 30 minutes est obligatoire au-delà de 4h30 d'activité.",
    "Repos minimal : 14 heures consécutives par tranche de 24h, et 2 jours par semaine (si possible consécutifs, incluant le dimanche).",
    "Horaires autorisés : entre 6h et 20h pour les moins de 16 ans ; entre 6h et 22h pour les 16-18 ans. Le travail de nuit est interdit aux mineurs.",
  ]);

  drawPeriodBanner(ctx, convention);
  drawScheduleTable(ctx, convention);

  drawArticleBlock(ctx, 8, "Sécurité et travaux interdits", [
    "L'élève ne peut en aucun cas accéder aux machines, appareils ou produits interdits aux mineurs par le Code du travail (articles L4153-8 et D4153-15 à D4153-37). Il peut participer à des activités pédagogiques encadrées (essais, démonstrations), avec des équipements de protection conformes à la réglementation.",
  ]);

  drawArticleBlock(ctx, 9, "Assurances et responsabilité civile", [
    "L'organisme d'accueil garantit sa responsabilité civile vis-à-vis de l'élève (attestation à remettre à l'établissement).",
    "Le chef d'établissement assure la couverture responsabilité civile de l'élève pour les dommages qu'il pourrait causer. Les parents complètent cette couverture par leur propre contrat.",
    `Référence assurance : ${insurance}`,
  ]);

  drawArticleBlock(ctx, 10, "Lieu, couverture accidents et trajets", [
    "La séquence se déroule dans les locaux de l'organisme d'accueil et, le cas échéant, dans d'autres lieux précisés en annexe.",
    "Les frais éventuels (transport, restauration, etc.) peuvent être pris en charge selon l'annexe financière.",
    "En cas d'accident, l'organisme d'accueil doit déclarer l'incident au chef d'établissement dans les 24 heures.",
    "L'élève doit emprunter l'itinéraire le plus court et utiliser des moyens de transport sûrs pour se rendre sur le lieu du stage.",
  ]);

  drawArticleBlock(ctx, 11, "Difficultés en cours de stage", [
    "Tout problème (discipline, absentéisme, santé, sécurité...) doit être signalé sans délai. Le chef d'établissement et le représentant de l'organisme d'accueil prennent, en lien avec l'équipe pédagogique, les dispositions nécessaires. En cas de danger pour l'élève, le chef d'établissement peut rompre immédiatement la convention.",
  ]);

  drawArticleBlock(ctx, 12, "Durée de validité", [
    "La présente convention est conclue uniquement pour la durée de la séquence d'observation mentionnée.",
  ]);

  drawArticleBlock(ctx, 13, "Protection des données personnelles (RGPD)", [
    "Les données personnelles recueillies pour l'organisation du stage sont traitées par l'établissement scolaire, responsable du traitement. Elles sont strictement nécessaires au suivi de la séquence d'observation et peuvent être partagées avec l'organisme d'accueil.",
    "Conformément au RGPD et à la loi Informatique et Libertés, chaque personne dispose d'un droit d'accès, de rectification, d'effacement, de limitation, d'opposition et de portabilité de ses données.",
    `Ces droits peuvent être exercés auprès de la direction de l'établissement (${rgpdContact}). Durée de conservation : pendant la scolarité, et jusqu'à 5 ans maximum. En cas de litige, une réclamation peut être adressée à la CNIL.`,
  ]);

  if (convention.adminReview) {
    const note = `Validee par ${convention.adminReview.byName} le ${new Date(convention.adminReview.at).toLocaleDateString("fr-FR")}${convention.adminReview.note ? ` — ${convention.adminReview.note}` : ""}.`;
    ensureSpace(ctx, 48);
    const h = 40;
    const yBottom = ctx.y - h;
    roundedRect(ctx.page, ctx.margin, yBottom, ctx.contentW, h, 14, {
      fill: ctx.palette.accentSoft,
      border: ctx.palette.line,
      borderWidth: 0.6,
    });
    ctx.page.drawText(sanitizePdfText("Validation administrative"), {
      x: ctx.margin + 14,
      y: yBottom + h - 16,
      size: 9,
      font: ctx.bold,
      color: ctx.palette.accent,
    });
    ctx.page.drawText(sanitizePdfText(note), {
      x: ctx.margin + 14,
      y: yBottom + 10,
      size: 8,
      font: ctx.font,
      color: ctx.palette.ink,
    });
    ctx.y = yBottom - 14;
  }

  drawSignatureGrid(ctx, convention.signatures);

  drawText(
    ctx,
    `Document genere le ${new Date().toLocaleString("fr-FR")} — ${convention.id}`,
    { size: 7, color: ctx.palette.soft },
  );

  return doc.save();
}

export async function buildStageConventionPdf(convention: StageConvention): Promise<Uint8Array> {
  const { loadAppConfig } = await import("@/app/lib/app-config");
  const { loadSchoolLogoForPdf } = await import("@/app/lib/pdf-branding");
  const [bundle, logo] = await Promise.all([loadAppConfig(), loadSchoolLogoForPdf()]);

  const schoolName = bundle.identity.name || "Établissement scolaire";
  const schoolAddress =
    bundle.identity.address?.full ||
    bundle.identity.address?.fullCompact ||
    [
      bundle.identity.address?.street,
      [bundle.identity.address?.zip, bundle.identity.address?.city].filter(Boolean).join(" "),
    ]
      .filter(Boolean)
      .join(" - ");
  const college = bundle.establishments.find((e) => e.kind === "college" && e.active !== false);
  const rgpdContact =
    college?.directorName ||
    bundle.establishments.find((e) => e.directorName)?.directorName ||
    "la direction de l'établissement";

  let logoPayload: StageConventionPdfLogo | null = null;
  if (logo?.dataUri) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      logoPayload = {
        bytes: new Uint8Array(Buffer.from(b64, "base64")),
        format: logo.format,
        width: logo.width,
        height: logo.height,
      };
    }
  }
  if (!logoPayload) {
    console.error(
      "[stage-pdf] Logo tenant introuvable — vérifier Paramètres → Identité (headerLogoUrl) / logo S3.",
    );
  }

  return renderStageConventionPdf(convention, {
    schoolName,
    schoolAddress,
    schoolPhone: bundle.identity.phone?.display || "",
    schoolMail: bundle.identity.assistanceEmail || "",
    rgpdContact,
    insuranceText: bundle.notifications.stagesInsuranceText?.trim() || DEFAULT_INSURANCE,
    accentHex: bundle.identity.dashboardAccent,
    logo: logoPayload,
  });
}

export function conventionPdfFilename(convention: StageConvention): string {
  const name = `${convention.student.lastName}_${convention.student.firstName}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9_-]/g, "_");
  return `convention_stage_${name}_${convention.id.slice(-8)}.pdf`;
}
