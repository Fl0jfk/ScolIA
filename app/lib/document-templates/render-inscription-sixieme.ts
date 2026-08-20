import "server-only";

import {
  PDFDocument,
  StandardFonts,
  rgb,
  type PDFFont,
  type PDFForm,
  type PDFPage,
  type RGB,
} from "pdf-lib";
import {
  fitImageInBox,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { normalizeSixiemeCodeConfig } from "@/app/lib/document-templates/inscription-sixieme-config";
import { loadInscriptionTenantSettings } from "@/app/lib/document-templates/inscription-storage";
import type {
  InscriptionLevelCodeConfig,
  InscriptionPdfFontId,
} from "@/app/lib/document-templates/types";

const PAGE_W = 595.28;
const PAGE_H = 841.89;
const MARGIN = 34;
const CONTENT_RIGHT = PAGE_W - MARGIN;
const CONTENT_W = PAGE_W - MARGIN * 2;
const INK = rgb(0.08, 0.1, 0.14);
const MUTED = rgb(0.32, 0.36, 0.4);
const PAGE_BOTTOM = 28;

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
  if (!Number.isFinite(n)) return { r: 0.18, g: 0.42, b: 0.65 };
  return {
    r: ((n >> 16) & 255) / 255,
    g: ((n >> 8) & 255) / 255,
    b: (n & 255) / 255,
  };
}

function softFill(c: { r: number; g: number; b: number }, amount = 0.88): RGB {
  return rgb(
    c.r + (1 - c.r) * amount,
    c.g + (1 - c.g) * amount,
    c.b + (1 - c.b) * amount,
  );
}

function midFill(c: { r: number; g: number; b: number }, amount = 0.72): RGB {
  return rgb(
    c.r + (1 - c.r) * amount,
    c.g + (1 - c.g) * amount,
    c.b + (1 - c.b) * amount,
  );
}

function normalizePdfFont(raw: unknown): InscriptionPdfFontId {
  const v = String(raw || "")
    .trim()
    .toLowerCase();
  if (v === "helvetica" || v === "courier" || v === "times") return v;
  return "times";
}

async function embedPair(
  doc: PDFDocument,
  id: InscriptionPdfFontId,
): Promise<{ font: PDFFont; bold: PDFFont }> {
  if (id === "courier") {
    return {
      font: await doc.embedFont(StandardFonts.Courier),
      bold: await doc.embedFont(StandardFonts.CourierBold),
    };
  }
  if (id === "helvetica") {
    return {
      font: await doc.embedFont(StandardFonts.Helvetica),
      bold: await doc.embedFont(StandardFonts.HelveticaBold),
    };
  }
  return {
    font: await doc.embedFont(StandardFonts.TimesRoman),
    bold: await doc.embedFont(StandardFonts.TimesRomanBold),
  };
}

/** Rectangle arrondi (fond + bordure accent). Coords SVG locales (y vers le bas). */
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

type Ctx = {
  doc: PDFDocument;
  page: PDFPage;
  form: PDFForm;
  font: PDFFont;
  bold: PDFFont;
  y: number;
  accent: { r: number; g: number; b: number };
  accentRgb: RGB;
  soft: RGB;
  mid: RGB;
  n: number;
};

function text(
  ctx: Ctx,
  value: string,
  x: number,
  y: number,
  size: number,
  opts?: { bold?: boolean; color?: RGB; maxWidth?: number },
) {
  ctx.page.drawText(sanitize(value), {
    x,
    y,
    size,
    font: opts?.bold ? ctx.bold : ctx.font,
    color: opts?.color ?? INK,
    maxWidth: opts?.maxWidth,
  });
}

function underlineField(
  ctx: Ctx,
  name: string,
  x: number,
  y: number,
  width: number,
  height = 14,
) {
  const field = ctx.form.createTextField(`s6_${name}_${ctx.n++}`);
  field.addToPage(ctx.page, { x, y: y - 2, width, height });
  field.setFontSize(10);
  ctx.page.drawLine({
    start: { x, y: y - 2 },
    end: { x: x + width, y: y - 2 },
    thickness: 0.75,
    color: ctx.accentRgb,
  });
}

/** Radios réparties sur toute la largeur (justify-between). */
function radioJustifyBetween(
  ctx: Ctx,
  groupName: string,
  options: string[],
  left: number,
  right: number,
  fontSize = 9,
) {
  const group = ctx.form.createRadioGroup(`s6_${groupName}_${ctx.n++}`);
  const n = options.length;
  if (n === 0) return;
  const span = Math.max(right - left, 1);
  const widths = options.map(
    (opt) => 14 + ctx.font.widthOfTextAtSize(sanitize(opt), fontSize),
  );
  const totalW = widths.reduce((s, w) => s + w, 0);
  const gap = n > 1 ? Math.max((span - totalW) / (n - 1), 4) : 0;
  let x = left;
  for (let i = 0; i < n; i++) {
    const opt = options[i];
    group.addOptionToPage(opt, ctx.page, {
      x,
      y: ctx.y - 3,
      width: 11,
      height: 11,
    });
    text(ctx, opt, x + 15, ctx.y, fontSize);
    x += widths[i] + gap;
  }
}

function checkboxGrid(
  ctx: Ctx,
  options: { id: string; label: string }[],
  cols: number,
  boxBottomY: number,
  rowH: number,
) {
  if (!options.length) return ctx.y;
  const usable = CONTENT_W - 16;
  const colW = usable / cols;
  let y = ctx.y;
  for (let i = 0; i < options.length; i++) {
    const col = i % cols;
    if (col === 0 && i > 0) y -= rowH;
    if (y < boxBottomY + 10) break;
    const opt = options[i];
    const x = MARGIN + 10 + col * colW;
    const cb = ctx.form.createCheckBox(`s6_opt_${opt.id}_${ctx.n++}`);
    cb.addToPage(ctx.page, { x, y: y - 2, width: 11, height: 11 });
    text(ctx, opt.label, x + 16, y, 8.5, { maxWidth: colW - 22 });
  }
  return y - rowH;
}

/** Encadré de section ; retourne le y bas du cadre. */
function beginSection(ctx: Ctx, title: string, boxH: number): number {
  const top = ctx.y;
  const bottom = top - boxH;
  roundedRect(ctx.page, MARGIN, bottom, CONTENT_W, boxH, 8, {
    fill: ctx.soft,
    border: ctx.accentRgb,
    borderWidth: 1.35,
  });
  const titleH = 18;
  ctx.page.drawRectangle({
    x: MARGIN + 1.2,
    y: top - titleH,
    width: CONTENT_W - 2.4,
    height: titleH - 1,
    color: ctx.mid,
    borderWidth: 0,
  });
  text(ctx, title, MARGIN + 10, top - 13, 10, { bold: true, color: ctx.accentRgb });
  ctx.y = top - titleH - 12;
  return bottom;
}

type BlockSpec = { key: string; min: number; weight: number };

function distributeHeights(
  available: number,
  blocks: BlockSpec[],
  gapMin: number,
): { heights: number[]; gaps: number[] } {
  const sumMin = blocks.reduce((s, b) => s + b.min, 0);
  const gapsMin = gapMin * Math.max(blocks.length - 1, 0);
  let free = available - sumMin - gapsMin;
  if (free < 0) free = 0;
  const sumW = blocks.reduce((s, b) => s + b.weight, 0) || 1;
  // ~80 % du surplus dans les blocs, ~20 % dans les interstices
  const freeBlocks = free * 0.82;
  const freeGaps = free * 0.18;
  const heights = blocks.map((b) => b.min + (freeBlocks * b.weight) / sumW);
  const gapCount = Math.max(blocks.length - 1, 1);
  const gaps = blocks.slice(0, -1).map(() => gapMin + freeGaps / gapCount);
  return { heights, gaps };
}

/**
 * Fiche 6e : design type Providence, aéré sur pleine hauteur,
 * radios en justify-between, police configurable.
 */
export async function renderSixiemeInscriptionPdf(opts?: {
  establishmentName?: string;
  accentColor?: string;
  pdfFont?: InscriptionPdfFontId;
  config?: InscriptionLevelCodeConfig;
}): Promise<Uint8Array> {
  const settings = await loadInscriptionTenantSettings();
  const config = normalizeSixiemeCodeConfig(
    opts?.config || settings.levelConfigs?.sixieme,
  );
  const letterhead = await getSchoolLetterhead();
  const accentHex =
    opts?.accentColor?.trim() || settings.accentColor?.trim() || "#2B6CB0";
  const accent = hexToRgb(accentHex);
  const accentRgb = rgb(accent.r, accent.g, accent.b);
  const soft = softFill(accent, 0.9);
  const mid = midFill(accent, 0.78);
  const fontId = normalizePdfFont(opts?.pdfFont ?? settings.pdfFont);
  const displayName = sanitize(
    (
      opts?.establishmentName?.trim() ||
      settings.establishmentName?.trim() ||
      letterhead.name ||
      "Établissement"
    ).slice(0, 120),
  );

  const doc = await PDFDocument.create();
  const { font, bold } = await embedPair(doc, fontId);
  const form = doc.getForm();

  const ctx: Ctx = {
    doc,
    page: doc.addPage([PAGE_W, PAGE_H]),
    form,
    font,
    bold,
    y: PAGE_H - 22,
    accent,
    accentRgb,
    soft,
    mid,
    n: 1,
  };

  // ——— En-tête ———
  const headerH = 78;
  const headerBottom = ctx.y - headerH;
  roundedRect(ctx.page, MARGIN, headerBottom, CONTENT_W, headerH, 10, {
    fill: mid,
    border: accentRgb,
    borderWidth: 1.55,
  });

  const logo = await loadSchoolLogoForPdf();
  let headerTextX = MARGIN + 12;
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 52, 44);
      ctx.page.drawImage(img, {
        x: MARGIN + 12,
        y: ctx.y - fitted.height - 10,
        width: fitted.width,
        height: fitted.height,
      });
      headerTextX = MARGIN + 12 + fitted.width + 12;
    }
  }

  text(ctx, displayName.toUpperCase(), headerTextX, ctx.y - 20, 12, {
    bold: true,
    color: accentRgb,
  });
  text(
    ctx,
    `ANNÉE ${sanitize(config.schoolYear).replace(/-/g, " - ")}`,
    headerTextX,
    ctx.y - 38,
    11,
    { bold: true, color: INK },
  );
  text(
    ctx,
    sanitize(config.title || "DEMANDE D'INSCRIPTION EN SIXIÈME").toUpperCase(),
    headerTextX,
    ctx.y - 56,
    13,
    { bold: true, color: accentRgb },
  );
  ctx.y = headerBottom - 14;

  const optCount = Math.max(config.options.length, 1);
  const optRows = Math.ceil(optCount / 2);
  const page1Blocks: BlockSpec[] = [
    { key: "eleve", min: 132, weight: 1.35 },
    { key: "regime", min: 48, weight: 0.55 },
    { key: "options", min: 36 + optRows * 20, weight: 1.0 + optRows * 0.12 },
    { key: "autres", min: 62, weight: 0.75 },
    { key: "fratrie", min: 128, weight: 1.25 },
    { key: "transport", min: 102, weight: 1.05 },
  ];
  const page1Avail = ctx.y - PAGE_BOTTOM;
  const { heights: h1, gaps: g1 } = distributeHeights(page1Avail, page1Blocks, 11);
  let gi = 0;

  // ——— Élève ———
  {
    const boxH = h1[0];
    const boxBottom = beginSection(ctx, "Elève", boxH);
    const rows = 5;
    const innerSpan = ctx.y - (boxBottom + 12);
    const step = Math.max(innerSpan / rows, 16);
    text(ctx, "Nom :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "nom", MARGIN + 46, ctx.y - 1, 200);
    text(ctx, "Prénoms :", MARGIN + 268, ctx.y, 9);
    underlineField(ctx, "prenoms", MARGIN + 324, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 324) - 10);
    ctx.y -= step;
    text(ctx, "Date de naissance :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "naissance", MARGIN + 118, ctx.y - 1, 130);
    text(ctx, "Lieu :", MARGIN + 268, ctx.y, 9);
    underlineField(ctx, "lieu", MARGIN + 304, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 304) - 10);
    ctx.y -= step;
    text(ctx, "Département :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "dept", MARGIN + 92, ctx.y - 1, 150);
    text(ctx, "Nationalité :", MARGIN + 268, ctx.y, 9);
    underlineField(ctx, "nationalite", MARGIN + 340, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 340) - 10);
    ctx.y -= step;
    text(ctx, "Etablissement précédent :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "etab_prev", MARGIN + 152, ctx.y - 1, 190);
    text(ctx, "Classe :", MARGIN + 360, ctx.y, 9);
    underlineField(ctx, "classe_prev", MARGIN + 410, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 410) - 10);
    ctx.y -= step;
    text(ctx, "Adresse :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "adresse", MARGIN + 64, ctx.y - 1, 180);
    text(ctx, "Code Postal et ville :", MARGIN + 268, ctx.y, 9);
    underlineField(ctx, "cp_ville", MARGIN + 392, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 392) - 10);
    ctx.y = boxBottom - g1[gi++];
  }

  // ——— Régime (pleine largeur, justify-between) ———
  {
    const boxH = h1[1];
    const boxBottom = beginSection(ctx, "Régime", boxH);
    const midY = (ctx.y + boxBottom) / 2 + 2;
    ctx.y = midY;
    radioJustifyBetween(
      ctx,
      "regime",
      ["Internat", "Demi-pension", "Externat"],
      MARGIN + 16,
      CONTENT_RIGHT - 16,
      10,
    );
    ctx.y = boxBottom - g1[gi++];
  }

  // ——— Options ———
  {
    const boxH = h1[2];
    const boxBottom = beginSection(ctx, "Souhait classe de 6ème — Enseignements / options", boxH);
    const inner = ctx.y - (boxBottom + 10);
    const rowH = Math.max(inner / Math.max(optRows, 1), 16);
    ctx.y = checkboxGrid(ctx, config.options, 2, boxBottom, rowH);
    ctx.y = boxBottom - g1[gi++];
  }

  // ——— Autres infos ———
  {
    const boxH = h1[3];
    const boxBottom = beginSection(ctx, "Autres informations", boxH);
    const inner = ctx.y - (boxBottom + 10);
    const step = Math.max(inner / 2, 18);
    text(ctx, "Avez-vous un enfant dans un autre établissement privé :", MARGIN + 10, ctx.y, 8.5);
    radioJustifyBetween(ctx, "autre_prive", ["Non", "Oui"], MARGIN + 300, MARGIN + 400, 9);
    text(ctx, "Nb :", MARGIN + 420, ctx.y, 8.5);
    underlineField(ctx, "autre_prive_nb", MARGIN + 448, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 448) - 10, 12);
    ctx.y -= step;
    text(ctx, "Avez-vous déjà un enfant dans l'établissement :", MARGIN + 10, ctx.y, 8.5);
    radioJustifyBetween(ctx, "deja_prov", ["Non", "Oui"], MARGIN + 280, MARGIN + 380, 9);
    text(ctx, "Nb :", MARGIN + 400, ctx.y, 8.5);
    underlineField(ctx, "deja_prov_nb", MARGIN + 428, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 428) - 10, 12);
    ctx.y = boxBottom - g1[gi++];
  }

  // ——— Fratrie ———
  {
    const boxH = h1[4];
    const boxBottom = beginSection(ctx, "Composition de la famille — Frère(s) et Soeur(s)", boxH);
    const headers = ["Nom et Prénom", "Date de Naissance", "Classe", "Établissement"];
    const colWs = [158, 120, 74, CONTENT_W - 16 - 158 - 120 - 74];
    let hx = MARGIN + 10;
    for (let i = 0; i < headers.length; i++) {
      text(ctx, headers[i], hx, ctx.y, 8, { bold: true, color: accentRgb });
      hx += colWs[i];
    }
    const inner = ctx.y - 10 - (boxBottom + 10);
    const step = Math.max(inner / 4, 18);
    ctx.y -= 14;
    for (let row = 0; row < 4; row++) {
      let x = MARGIN + 10;
      for (let col = 0; col < 4; col++) {
        underlineField(ctx, `fratrie_r${row}_c${col}`, x, ctx.y - 1, colWs[col] - 8, 13);
        x += colWs[col];
      }
      ctx.y -= step;
    }
    ctx.y = boxBottom - g1[gi++];
  }

  // ——— Transport + observations ———
  {
    const boxH = h1[5];
    const boxBottom = beginSection(ctx, "Transport & observations", boxH);
    text(ctx, "Moyen(s) de transport utilisé(s) :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "transport", MARGIN + 190, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 190) - 10);
    const inner = ctx.y - 18 - (boxBottom + 10);
    const obsLines = 3;
    const step = Math.max(inner / (obsLines + 0.6), 14);
    ctx.y -= Math.max(step * 0.7, 16);
    text(
      ctx,
      "Observations particulières (santé, caractère, aptitudes, besoins particuliers, handicap ...) :",
      MARGIN + 10,
      ctx.y,
      8,
    );
    ctx.y -= 14;
    for (let i = 0; i < obsLines; i++) {
      underlineField(ctx, `obs_${i}`, MARGIN + 10, ctx.y - 1, CONTENT_W - 20, 12);
      ctx.y -= step;
    }
    ctx.y = boxBottom;
  }

  // ——— PAGE 2 ———
  ctx.page = doc.addPage([PAGE_W, PAGE_H]);
  ctx.y = PAGE_H - 24;

  const page2Blocks: BlockSpec[] = [
    { key: "r1", min: 236, weight: 1.15 },
    { key: "r2", min: 236, weight: 1.15 },
    { key: "engagement", min: 148, weight: 1.0 },
  ];
  const page2Avail = ctx.y - PAGE_BOTTOM;
  const { heights: h2, gaps: g2 } = distributeHeights(page2Avail, page2Blocks, 12);

  const drawResponsable = (title: string, prefix: string, boxH: number, gapAfter: number) => {
    const boxBottom = beginSection(ctx, title, boxH);
    const rows = 12;
    const inner = ctx.y - (boxBottom + 10);
    const step = Math.max(inner / rows, 14.5);

    text(ctx, "Civilité :", MARGIN + 10, ctx.y, 9);
    radioJustifyBetween(ctx, `${prefix}_civ`, ["Madame", "Monsieur"], MARGIN + 70, MARGIN + 280, 9);
    ctx.y -= step;

    text(ctx, "Nom :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_nom`, MARGIN + 48, ctx.y - 1, CONTENT_W - 58);
    ctx.y -= step;

    text(ctx, "Nom de jeune fille :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_njf`, MARGIN + 128, ctx.y - 1, CONTENT_W - 138);
    ctx.y -= step;

    text(ctx, "Prénom :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_prenom`, MARGIN + 62, ctx.y - 1, CONTENT_W - 72);
    ctx.y -= step;

    radioJustifyBetween(
      ctx,
      `${prefix}_situation`,
      ["marié(e)", "veuf ou veuve", "séparé(e)", "divorcé(e)", "autre"],
      MARGIN + 10,
      CONTENT_RIGHT - 10,
      8.5,
    );
    ctx.y -= step;

    text(ctx, "Lien de parenté avec l'élève :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_lien`, MARGIN + 168, ctx.y - 1, CONTENT_W - 178);
    ctx.y -= step;

    text(ctx, "Responsabilité :", MARGIN + 10, ctx.y, 9);
    radioJustifyBetween(
      ctx,
      `${prefix}_resp`,
      ["autorité parentale", "tuteur ou tutrice"],
      MARGIN + 110,
      CONTENT_RIGHT - 10,
      9,
    );
    ctx.y -= step;

    text(ctx, "Adresse :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_adresse`, MARGIN + 62, ctx.y - 1, CONTENT_W - 72);
    ctx.y -= step;

    text(ctx, "Code postal :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_cp`, MARGIN + 88, ctx.y - 1, 78);
    text(ctx, "Ville :", MARGIN + 186, ctx.y, 9);
    underlineField(ctx, `${prefix}_ville`, MARGIN + 226, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 226) - 10);
    ctx.y -= step;

    text(ctx, "Tél. Domicile :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_dom`, MARGIN + 98, ctx.y - 1, 120);
    text(ctx, "E-mail :", MARGIN + 240, ctx.y, 9);
    underlineField(ctx, `${prefix}_email`, MARGIN + 288, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 288) - 10);
    ctx.y -= step;

    radioJustifyBetween(
      ctx,
      `${prefix}_activite`,
      ["en activité", "recherche d'emploi", "retraité", "autre"],
      MARGIN + 10,
      CONTENT_RIGHT - 10,
      8.5,
    );
    ctx.y -= step;

    text(ctx, "Employeur :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, `${prefix}_employeur`, MARGIN + 78, ctx.y - 1, 110);
    text(ctx, "Tél portable :", MARGIN + 206, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_port`, MARGIN + 288, ctx.y - 1, 90);
    text(ctx, "Tél professionnel :", MARGIN + 396, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_pro`, MARGIN + 502, ctx.y - 1, CONTENT_RIGHT - (MARGIN + 502) - 10);

    ctx.y = boxBottom - gapAfter;
  };

  drawResponsable("Responsable principal", "r1", h2[0], g2[0] ?? 12);
  drawResponsable("Conjoint ou autre responsable", "r2", h2[1], g2[1] ?? 12);

  {
    const boxH = h2[2];
    const boxBottom = beginSection(ctx, "Engagement", boxH);
    const inner = ctx.y - (boxBottom + 10);
    const step = Math.max(inner / 7, 14);

    text(ctx, "Je soussigné(e) :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "soussigne", MARGIN + 108, ctx.y - 1, 190);
    text(ctx, "déclare accepter pour mon enfant le but de l'Ecole Catholique.", MARGIN + 310, ctx.y, 8, {
      maxWidth: CONTENT_RIGHT - (MARGIN + 310) - 8,
    });
    ctx.y -= step;

    text(
      ctx,
      'Celle-ci s\'efforce " de lier dans le même temps et le même acte l\'acquisition du savoir, la formation à l\'autonomie et',
      MARGIN + 10,
      ctx.y,
      8,
      { color: MUTED, maxWidth: CONTENT_W - 20 },
    );
    ctx.y -= step * 0.75;
    text(ctx, 'à la prise de responsabilités et l\'éducation de la Foi. "', MARGIN + 10, ctx.y, 8, {
      color: MUTED,
    });
    ctx.y -= step;

    text(ctx, "A :", MARGIN + 10, ctx.y, 9);
    underlineField(ctx, "fait_a", MARGIN + 32, ctx.y - 1, 190);
    text(ctx, "le :", MARGIN + 246, ctx.y, 9);
    underlineField(ctx, "fait_le", MARGIN + 272, ctx.y - 1, 190);
    ctx.y -= step;

    text(
      ctx,
      'Signature du ou des responsables (Faire précéder la signature de la mention "LU et APPROUVE")',
      MARGIN + 10,
      ctx.y,
      8,
      { color: MUTED },
    );
    ctx.y -= step * 0.85;
    const sigW = (CONTENT_W - 40) / 3;
    underlineField(ctx, "sig1", MARGIN + 10, ctx.y - 1, sigW, 28);
    underlineField(ctx, "sig2", MARGIN + 20 + sigW, ctx.y - 1, sigW, 28);
    underlineField(ctx, "sig3", MARGIN + 30 + 2 * sigW, ctx.y - 1, sigW, 28);
    ctx.y = boxBottom;
  }

  form.updateFieldAppearances(font);
  return doc.save();
}
