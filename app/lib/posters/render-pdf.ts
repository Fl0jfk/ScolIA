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
import { getPosterTemplateMeta } from "@/app/lib/posters/catalog";
import { computePosterLayout } from "@/app/lib/posters/poster-layout";
import type { PosterBox, PosterDraft } from "@/app/lib/posters/types";

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

/** Box normalisée (haut-gauche) → rect PDF (bas-gauche). */
function boxToPdf(
  box: PosterBox,
  pageW: number,
  pageH: number,
): { x: number; y: number; w: number; h: number } {
  const w = box.w * pageW;
  const h = box.h * pageH;
  const x = box.x * pageW;
  const y = pageH - box.y * pageH - h;
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
  pageW: number,
  pageH: number,
  size: number,
  color: { r: number; g: number; b: number },
  align: "left" | "center" = "center",
) {
  const rect = boxToPdf(box, pageW, pageH);
  const lines = wrapLines(font, text, size, rect.w);
  const lineH = size * 1.2;
  let yTop = rect.y + rect.h - size;
  for (const line of lines) {
    if (yTop < rect.y) break;
    const tw = font.widthOfTextAtSize(line, size);
    const x = align === "center" ? rect.x + (rect.w - tw) / 2 : rect.x;
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

export async function renderPosterPdf(draft: PosterDraft): Promise<Uint8Array> {
  const meta = getPosterTemplateMeta(draft.templateId);
  if (!meta) throw new Error("Modèle d'affiche inconnu");

  const layout = computePosterLayout(draft);
  const { widthPt: pageW, heightPt: pageH } = layout.page;

  const doc = await PDFDocument.create();
  const page = doc.addPage([pageW, pageH]);
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);

  const bg = hexToRgb(layout.colors.background);
  const accent = hexToRgb(layout.colors.accent);
  const text = hexToRgb(layout.colors.text);
  const gradTo = hexToRgb(layout.colors.gradientTo);

  // Fond
  if (draft.backgroundMode === "image" && draft.backgroundImageKey) {
    const bgLogo = await loadImageForPdfFromRef(draft.backgroundImageKey);
    const embedded = await embedLogo(doc, bgLogo);
    if (embedded) {
      page.drawImage(embedded.img, { x: 0, y: 0, width: pageW, height: pageH });
    } else {
      page.drawRectangle({
        x: 0,
        y: 0,
        width: pageW,
        height: pageH,
        color: rgb(bg.r, bg.g, bg.b),
      });
    }
    if (layout.overlayOpacity > 0) {
      page.drawRectangle({
        x: 0,
        y: 0,
        width: pageW,
        height: pageH,
        color: rgb(0.02, 0.04, 0.08),
        opacity: layout.overlayOpacity,
      });
    }
  } else if (draft.backgroundMode === "gradient") {
    const bands = 48;
    for (let i = 0; i < bands; i++) {
      const t = i / (bands - 1);
      const r = bg.r + (gradTo.r - bg.r) * t;
      const g = bg.g + (gradTo.g - bg.g) * t;
      const b = bg.b + (gradTo.b - bg.b) * t;
      const h = pageH / bands;
      page.drawRectangle({
        x: 0,
        y: pageH - (i + 1) * h,
        width: pageW,
        height: h + 0.5,
        color: rgb(r, g, b),
      });
    }
  } else {
    page.drawRectangle({
      x: 0,
      y: 0,
      width: pageW,
      height: pageH,
      color: rgb(bg.r, bg.g, bg.b),
    });
  }

  // Bandeau accent
  const bar = boxToPdf(layout.boxes.accentBar, pageW, pageH);
  page.drawRectangle({
    x: bar.x,
    y: bar.y,
    width: bar.w,
    height: Math.max(2, bar.h),
    color: rgb(accent.r, accent.g, accent.b),
  });

  // Logos
  const schoolLogo = await loadSchoolLogoForPdf();
  const schoolEmb = await embedLogo(doc, schoolLogo);
  if (schoolEmb) {
    const box = boxToPdf(layout.boxes.logoSchool, pageW, pageH);
    const fitted = fitImageInBox(schoolEmb.w, schoolEmb.h, box.w, box.h);
    page.drawImage(schoolEmb.img, {
      x: box.x + (box.w - fitted.width) / 2,
      y: box.y + (box.h - fitted.height) / 2,
      width: fitted.width,
      height: fitted.height,
    });
  }

  if (layout.boxes.logoPartner && draft.partnerLogoKey) {
    const partnerLogo = await loadImageForPdfFromRef(draft.partnerLogoKey);
    const partnerEmb = await embedLogo(doc, partnerLogo);
    if (partnerEmb) {
      const box = boxToPdf(layout.boxes.logoPartner, pageW, pageH);
      const fitted = fitImageInBox(partnerEmb.w, partnerEmb.h, box.w, box.h);
      page.drawImage(partnerEmb.img, {
        x: box.x + (box.w - fitted.width) / 2,
        y: box.y + (box.h - fitted.height) / 2,
        width: fitted.width,
        height: fitted.height,
      });
    }
  }

  drawTextBlock(
    page,
    bold,
    draft.title || meta.label,
    layout.boxes.title,
    pageW,
    pageH,
    layout.titleFontSize,
    text,
    "center",
  );

  if (draft.subtitle.trim()) {
    drawTextBlock(
      page,
      font,
      draft.subtitle,
      layout.boxes.subtitle,
      pageW,
      pageH,
      layout.subtitleFontSize,
      text,
      "center",
    );
  }

  if (draft.body.trim()) {
    drawTextBlock(
      page,
      font,
      draft.body,
      layout.boxes.body,
      pageW,
      pageH,
      layout.bodyFontSize,
      text,
      "center",
    );
  }

  if (layout.boxes.datePlace) {
    const bits = [draft.dateLabel, draft.placeLabel].map((s) => s.trim()).filter(Boolean);
    if (bits.length) {
      drawTextBlock(
        page,
        bold,
        bits.join("  ·  "),
        layout.boxes.datePlace,
        pageW,
        pageH,
        layout.bodyFontSize,
        accent,
        "center",
      );
    }
  }

  if (layout.boxes.schoolMention) {
    const letterhead = await getSchoolLetterhead();
    const mention = draft.partnerName.trim()
      ? `${letterhead.name}  ×  ${draft.partnerName.trim()}`
      : letterhead.name;
    drawTextBlock(
      page,
      font,
      mention,
      layout.boxes.schoolMention,
      pageW,
      pageH,
      Math.max(9, layout.bodyFontSize * 0.75),
      text,
      "left",
    );
  }

  if (layout.boxes.qr && draft.qrUrl.trim()) {
    try {
      const png = await QRCode.toBuffer(draft.qrUrl.trim(), {
        type: "png",
        width: 256,
        margin: 1,
        errorCorrectionLevel: "M",
      });
      const qrImg = await doc.embedPng(png);
      const box = boxToPdf(layout.boxes.qr, pageW, pageH);
      page.drawImage(qrImg, {
        x: box.x,
        y: box.y,
        width: box.w,
        height: box.h,
      });
    } catch {
      /* QR optionnel */
    }
  }

  return doc.save();
}

export function posterTitleFromDraft(draft: PosterDraft): string {
  const t = draft.title.trim() || getPosterTemplateMeta(draft.templateId)?.label || "Affiche";
  const partner = draft.partnerName.trim();
  return partner ? `${t} — ${partner}` : t;
}
