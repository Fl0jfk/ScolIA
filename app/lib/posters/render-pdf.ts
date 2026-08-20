import "server-only";

import { PDFDocument, StandardFonts, rgb, type PDFPage, type PDFFont } from "pdf-lib";
import QRCode from "qrcode";
import {
  fitImageInBox,
  getSchoolLetterhead,
  loadImageForPdfFromRef,
  loadSchoolLogoForPdf,
  type PdfLogo,
} from "@/app/lib/pdf-branding";
import { draftDisplayTitle, getPosterTemplateMeta } from "@/app/lib/posters/catalog";
import { exportSheetSizePt, pageSizePt } from "@/app/lib/posters/poster-layout";
import type { PosterBox, PosterDraft, PosterElement } from "@/app/lib/posters/types";

function sanitize(input: string): string {
  return String(input || "")
    .replace(/[\u2018\u2019]/g, "'")
    .replace(/[\u201C\u201D]/g, '"')
    .replace(/\u2026/g, "...")
    .replace(/[^\x00-\xFF]/g, "?");
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const h = hex.replace("#", "").trim();
  const full =
    h.length === 3
      ? h
          .split("")
          .map((c) => c + c)
          .join("")
      : h.padEnd(6, "0").slice(0, 6);
  const n = parseInt(full, 16);
  if (!Number.isFinite(n)) return { r: 0.06, g: 0.09, b: 0.16 };
  return {
    r: ((n >> 16) & 255) / 255,
    g: ((n >> 8) & 255) / 255,
    b: (n & 255) / 255,
  };
}

/** Box normalisée (haut-gauche) → rect PDF (bas-gauche) dans un tile. */
function boxToPdf(
  box: PosterBox,
  pageW: number,
  pageH: number,
  ox = 0,
  oy = 0,
): { x: number; y: number; w: number; h: number } {
  const w = box.w * pageW;
  const h = box.h * pageH;
  const x = ox + box.x * pageW;
  const y = oy + pageH - box.y * pageH - h;
  return { x, y, w, h };
}

function wrapLines(font: PDFFont, text: string, size: number, maxWidth: number): string[] {
  const words = sanitize(text).split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const trial = current ? `${current} ${word}` : word;
    if (font.widthOfTextAtSize(trial, size) <= maxWidth) {
      current = trial;
    } else {
      if (current) lines.push(current);
      current = word;
    }
  }
  if (current) lines.push(current);
  return lines.length ? lines : [""];
}

async function embedLogo(
  doc: PDFDocument,
  logo: PdfLogo | null,
): Promise<{ img: Awaited<ReturnType<PDFDocument["embedPng"]>>; w: number; h: number } | null> {
  if (!logo) return null;
  const b64 = logo.dataUri.split(",")[1];
  if (!b64) return null;
  const bytes = Buffer.from(b64, "base64");
  const img =
    logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
  return { img, w: logo.width || img.width, h: logo.height || img.height };
}

function drawTextBlock(
  page: PDFPage,
  font: PDFFont,
  text: string,
  box: PosterBox,
  tileW: number,
  tileH: number,
  ox: number,
  oy: number,
  size: number,
  color: { r: number; g: number; b: number },
  align: "left" | "center" | "right" = "center",
) {
  const rect = boxToPdf(box, tileW, tileH, ox, oy);
  const lines = wrapLines(font, text, size, rect.w);
  const lineH = size * 1.2;
  let yTop = rect.y + rect.h - size;
  for (const line of lines) {
    if (yTop < rect.y) break;
    const tw = font.widthOfTextAtSize(line, size);
    let x = rect.x;
    if (align === "center") x = rect.x + (rect.w - tw) / 2;
    if (align === "right") x = rect.x + rect.w - tw;
    page.drawText(line, {
      x,
      y: yTop,
      size,
      font,
      color: rgb(color.r, color.g, color.b),
    });
    yTop -= lineH;
  }
}

function baseFontSize(kind: PosterElement["kind"], tileH: number, fontScale = 1): number {
  const scale = fontScale || 1;
  if (kind === "title") return Math.max(14, tileH * 0.045) * scale;
  if (kind === "subtitle") return Math.max(10, tileH * 0.022) * scale;
  if (kind === "body") return Math.max(9, tileH * 0.018) * scale;
  if (kind === "date-place") return Math.max(9, tileH * 0.02) * scale;
  if (kind === "mention") return Math.max(8, tileH * 0.015) * scale;
  return Math.max(9, tileH * 0.018) * scale;
}

type Embedded = {
  img: Awaited<ReturnType<PDFDocument["embedPng"]>>;
  w: number;
  h: number;
};

async function paintPosterTile(
  page: PDFPage,
  doc: PDFDocument,
  draft: PosterDraft,
  font: PDFFont,
  bold: PDFFont,
  tileW: number,
  tileH: number,
  ox: number,
  oy: number,
  assets: {
    schoolEmb: Embedded | null;
    partnerEmb: Embedded | null;
    bgEmb: Embedded | null;
    imageCache: Map<string, Embedded | null>;
    qrCache: Map<string, Embedded | null>;
    schoolName: string;
  },
) {
  const bg = hexToRgb(draft.backgroundColor);
  const accent = hexToRgb(draft.accentColor);
  const text = hexToRgb(draft.textColor);
  const gradTo = hexToRgb(draft.gradientTo);

  if (draft.backgroundMode === "image" && assets.bgEmb) {
    page.drawImage(assets.bgEmb.img, { x: ox, y: oy, width: tileW, height: tileH });
    if (draft.overlayOpacity > 0) {
      page.drawRectangle({
        x: ox,
        y: oy,
        width: tileW,
        height: tileH,
        color: rgb(0.02, 0.04, 0.08),
        opacity: draft.overlayOpacity,
      });
    }
  } else if (draft.backgroundMode === "gradient") {
    const bands = 40;
    for (let i = 0; i < bands; i++) {
      const t = i / (bands - 1);
      const r = bg.r + (gradTo.r - bg.r) * t;
      const g = bg.g + (gradTo.g - bg.g) * t;
      const b = bg.b + (gradTo.b - bg.b) * t;
      const h = tileH / bands;
      page.drawRectangle({
        x: ox,
        y: oy + tileH - (i + 1) * h,
        width: tileW,
        height: h + 0.5,
        color: rgb(r, g, b),
      });
    }
  } else {
    page.drawRectangle({
      x: ox,
      y: oy,
      width: tileW,
      height: tileH,
      color: rgb(bg.r, bg.g, bg.b),
    });
  }

  for (const el of draft.elements) {
    const box = { x: el.x, y: el.y, w: el.w, h: el.h };
    const align = el.align || "center";

    if (el.kind === "accent-bar") {
      const rect = boxToPdf(box, tileW, tileH, ox, oy);
      page.drawRectangle({
        x: rect.x,
        y: rect.y,
        width: rect.w,
        height: Math.max(2, rect.h),
        color: rgb(accent.r, accent.g, accent.b),
      });
      continue;
    }

    if (el.kind === "logo-school" && assets.schoolEmb) {
      const rect = boxToPdf(box, tileW, tileH, ox, oy);
      const fitted = fitImageInBox(assets.schoolEmb.w, assets.schoolEmb.h, rect.w, rect.h);
      page.drawImage(assets.schoolEmb.img, {
        x: rect.x + (rect.w - fitted.width) / 2,
        y: rect.y + (rect.h - fitted.height) / 2,
        width: fitted.width,
        height: fitted.height,
      });
      continue;
    }

    if (el.kind === "logo-partner") {
      const key = el.imageKey || draft.partnerLogoKey;
      let emb = assets.partnerEmb;
      if (key && key !== draft.partnerLogoKey) {
        if (!assets.imageCache.has(key)) {
          const logo = await loadImageForPdfFromRef(key);
          assets.imageCache.set(key, await embedLogo(doc, logo));
        }
        emb = assets.imageCache.get(key) || null;
      }
      if (emb) {
        const rect = boxToPdf(box, tileW, tileH, ox, oy);
        const fitted = fitImageInBox(emb.w, emb.h, rect.w, rect.h);
        page.drawImage(emb.img, {
          x: rect.x + (rect.w - fitted.width) / 2,
          y: rect.y + (rect.h - fitted.height) / 2,
          width: fitted.width,
          height: fitted.height,
        });
      }
      continue;
    }

    if (el.kind === "image" && el.imageKey) {
      if (!assets.imageCache.has(el.imageKey)) {
        const logo = await loadImageForPdfFromRef(el.imageKey);
        assets.imageCache.set(el.imageKey, await embedLogo(doc, logo));
      }
      const emb = assets.imageCache.get(el.imageKey);
      if (emb) {
        const rect = boxToPdf(box, tileW, tileH, ox, oy);
        const fitted = fitImageInBox(emb.w, emb.h, rect.w, rect.h);
        page.drawImage(emb.img, {
          x: rect.x + (rect.w - fitted.width) / 2,
          y: rect.y + (rect.h - fitted.height) / 2,
          width: fitted.width,
          height: fitted.height,
        });
      }
      continue;
    }

    if (el.kind === "qr") {
      const url = (el.text || draft.qrUrl || "").trim();
      if (!url) continue;
      if (!assets.qrCache.has(url)) {
        try {
          const png = await QRCode.toBuffer(url, {
            type: "png",
            width: 256,
            margin: 1,
            errorCorrectionLevel: "M",
          });
          const qrImg = await doc.embedPng(png);
          assets.qrCache.set(url, { img: qrImg, w: qrImg.width, h: qrImg.height });
        } catch {
          assets.qrCache.set(url, null);
        }
      }
      const emb = assets.qrCache.get(url);
      if (emb) {
        const rect = boxToPdf(box, tileW, tileH, ox, oy);
        page.drawImage(emb.img, {
          x: rect.x,
          y: rect.y,
          width: rect.w,
          height: rect.h,
        });
      }
      continue;
    }

    if (
      el.kind === "title" ||
      el.kind === "subtitle" ||
      el.kind === "body" ||
      el.kind === "date-place" ||
      el.kind === "mention"
    ) {
      let content = (el.text || "").trim();
      if (el.kind === "mention" && !content) {
        content = draft.partnerName.trim()
          ? `${assets.schoolName}  ×  ${draft.partnerName.trim()}`
          : assets.schoolName;
      }
      if (!content && el.kind === "title") content = getPosterTemplateMeta(draft.templateId)?.label || "Affiche";
      if (!content) continue;
      const size = baseFontSize(el.kind, tileH, el.fontScale);
      const color = el.kind === "date-place" ? accent : text;
      const useBold = el.kind === "title" || el.kind === "date-place";
      drawTextBlock(
        page,
        useBold ? bold : font,
        content,
        box,
        tileW,
        tileH,
        ox,
        oy,
        size,
        color,
        align,
      );
    }
  }
}

export async function renderPosterPdf(draft: PosterDraft): Promise<Uint8Array> {
  const meta = getPosterTemplateMeta(draft.templateId);
  if (!meta) throw new Error("Modèle d'affiche inconnu");

  const tile = pageSizePt(draft.format);
  const sheet = exportSheetSizePt(draft.format);
  const doc = await PDFDocument.create();
  const page = doc.addPage([sheet.widthPt, sheet.heightPt]);
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);

  const letterhead = await getSchoolLetterhead();
  const schoolLogo = await loadSchoolLogoForPdf();
  const schoolEmb = await embedLogo(doc, schoolLogo);

  let partnerEmb: Embedded | null = null;
  if (draft.partnerLogoKey) {
    partnerEmb = await embedLogo(doc, await loadImageForPdfFromRef(draft.partnerLogoKey));
  }

  let bgEmb: Embedded | null = null;
  if (draft.backgroundMode === "image" && draft.backgroundImageKey) {
    bgEmb = await embedLogo(doc, await loadImageForPdfFromRef(draft.backgroundImageKey));
  }

  const assets = {
    schoolEmb,
    partnerEmb,
    bgEmb,
    imageCache: new Map<string, Embedded | null>(),
    qrCache: new Map<string, Embedded | null>(),
    schoolName: letterhead.name,
  };

  const tileW = tile.widthPt;
  const tileH = tile.heightPt;

  if (sheet.tiles === 4) {
    const positions = [
      { ox: 0, oy: sheet.heightPt / 2 },
      { ox: sheet.widthPt / 2, oy: sheet.heightPt / 2 },
      { ox: 0, oy: 0 },
      { ox: sheet.widthPt / 2, oy: 0 },
    ];
    for (const pos of positions) {
      await paintPosterTile(
        page,
        doc,
        draft,
        font,
        bold,
        tileW,
        tileH,
        pos.ox,
        pos.oy,
        assets,
      );
    }
    // Traits de coupe légers
    const midX = sheet.widthPt / 2;
    const midY = sheet.heightPt / 2;
    const mark = 8;
    page.drawLine({
      start: { x: midX, y: sheet.heightPt - mark },
      end: { x: midX, y: sheet.heightPt },
      thickness: 0.5,
      color: rgb(0.6, 0.6, 0.6),
    });
    page.drawLine({
      start: { x: midX, y: 0 },
      end: { x: midX, y: mark },
      thickness: 0.5,
      color: rgb(0.6, 0.6, 0.6),
    });
    page.drawLine({
      start: { x: 0, y: midY },
      end: { x: mark, y: midY },
      thickness: 0.5,
      color: rgb(0.6, 0.6, 0.6),
    });
    page.drawLine({
      start: { x: sheet.widthPt - mark, y: midY },
      end: { x: sheet.widthPt, y: midY },
      thickness: 0.5,
      color: rgb(0.6, 0.6, 0.6),
    });
  } else {
    await paintPosterTile(page, doc, draft, font, bold, tileW, tileH, 0, 0, assets);
  }

  return doc.save();
}

export function posterTitleFromDraft(draft: PosterDraft): string {
  const t = draftDisplayTitle(draft);
  const partner = draft.partnerName.trim();
  return partner ? `${t} — ${partner}` : t;
}
