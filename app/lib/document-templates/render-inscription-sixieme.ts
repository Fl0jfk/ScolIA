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
import type { InscriptionLevelCodeConfig } from "@/app/lib/document-templates/types";

const PAGE_W = 595.28;
const PAGE_H = 841.89;
const MARGIN = 36;
const INK = rgb(0.08, 0.1, 0.14);
const MUTED = rgb(0.32, 0.36, 0.4);

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
  // Origine SVG en haut-gauche du rectangle PDF (pdf-lib inverse l’axe Y).
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
  height = 13,
) {
  const field = ctx.form.createTextField(`s6_${name}_${ctx.n++}`);
  field.addToPage(ctx.page, { x, y: y - 2, width, height });
  field.setFontSize(9);
  ctx.page.drawLine({
    start: { x, y: y - 2 },
    end: { x: x + width, y: y - 2 },
    thickness: 0.7,
    color: ctx.accentRgb,
  });
}

function radioRow(
  ctx: Ctx,
  groupName: string,
  options: string[],
  startX: number,
  gap = 95,
) {
  const group = ctx.form.createRadioGroup(`s6_${groupName}_${ctx.n++}`);
  let x = startX;
  for (const opt of options) {
    group.addOptionToPage(opt, ctx.page, {
      x,
      y: ctx.y - 3,
      width: 10,
      height: 10,
    });
    text(ctx, opt, x + 14, ctx.y, 8);
    x += gap;
  }
}

function checkboxGrid(
  ctx: Ctx,
  options: { id: string; label: string }[],
  cols = 2,
  boxBottomY: number,
) {
  if (!options.length) return ctx.y;
  const usable = PAGE_W - MARGIN * 2 - 16;
  const colW = usable / cols;
  const rowH = 15;
  let y = ctx.y;
  for (let i = 0; i < options.length; i++) {
    const col = i % cols;
    if (col === 0 && i > 0) y -= rowH;
    if (y < boxBottomY + 8) break;
    const opt = options[i];
    const x = MARGIN + 8 + col * colW;
    const cb = ctx.form.createCheckBox(`s6_opt_${opt.id}_${ctx.n++}`);
    cb.addToPage(ctx.page, { x, y: y - 2, width: 10, height: 10 });
    text(ctx, opt.label, x + 14, y, 7.5, { maxWidth: colW - 18 });
  }
  return y - rowH;
}

/** Encadré de section : fond soft + bordure accent + titre sur bandeau. */
function beginSection(ctx: Ctx, title: string, estimatedHeight: number): number {
  const top = ctx.y + 6;
  const boxH = estimatedHeight;
  const bottom = top - boxH;
  roundedRect(ctx.page, MARGIN, bottom, PAGE_W - MARGIN * 2, boxH, 8, {
    fill: ctx.soft,
    border: ctx.accentRgb,
    borderWidth: 1.4,
  });
  // bandeau titre (plein largeur, coins hauts arrondis via rect simple)
  ctx.page.drawRectangle({
    x: MARGIN + 1.2,
    y: top - 17,
    width: PAGE_W - MARGIN * 2 - 2.4,
    height: 15.5,
    color: ctx.mid,
    borderWidth: 0,
  });
  text(ctx, title, MARGIN + 8, top - 12, 9, { bold: true, color: ctx.accentRgb });
  ctx.y = top - 28;
  return bottom;
}

/**
 * Fiche 6e : design type fiche Providence (fonds teintés, encadrés arrondis,
 * bordures / lignes dans la couleur d'accent configurable).
 */
export async function renderSixiemeInscriptionPdf(opts?: {
  establishmentName?: string;
  accentColor?: string;
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
  const displayName = sanitize(
    (
      opts?.establishmentName?.trim() ||
      settings.establishmentName?.trim() ||
      letterhead.name ||
      "Établissement"
    ).slice(0, 120),
  );

  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const form = doc.getForm();

  const ctx: Ctx = {
    doc,
    page: doc.addPage([PAGE_W, PAGE_H]),
    form,
    font,
    bold,
    y: PAGE_H - 28,
    accent,
    accentRgb,
    soft,
    mid,
    n: 1,
  };

  // ——— En-tête avec fond couleur (comme la fiche d'origine) ———
  const headerH = 72;
  roundedRect(ctx.page, MARGIN - 2, ctx.y - headerH + 8, PAGE_W - MARGIN * 2 + 4, headerH, 10, {
    fill: mid,
    border: accentRgb,
    borderWidth: 1.6,
  });

  const logo = await loadSchoolLogoForPdf();
  let headerTextX = MARGIN + 10;
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 48, 40);
      ctx.page.drawImage(img, {
        x: MARGIN + 10,
        y: ctx.y - fitted.height - 6,
        width: fitted.width,
        height: fitted.height,
      });
      headerTextX = MARGIN + 10 + fitted.width + 10;
    }
  }

  const nameUpper = displayName.toUpperCase();
  text(ctx, nameUpper, headerTextX, ctx.y - 18, 11, { bold: true, color: accentRgb });
  const yearLine = `ANNÉE ${sanitize(config.schoolYear).replace(/-/g, " - ")}`;
  text(ctx, yearLine, headerTextX, ctx.y - 34, 10, { bold: true, color: INK });
  const title = sanitize(config.title || "DEMANDE D'INSCRIPTION EN SIXIÈME").toUpperCase();
  text(ctx, title, headerTextX, ctx.y - 50, 12, { bold: true, color: accentRgb });
  ctx.y -= headerH + 10;

  // ——— Bloc Élève ———
  let boxBottom = beginSection(ctx, "Elève", 118);
  text(ctx, "Nom :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "nom", MARGIN + 42, ctx.y - 1, 200);
  text(ctx, "Prénoms :", MARGIN + 260, ctx.y, 8);
  underlineField(ctx, "prenoms", MARGIN + 312, ctx.y - 1, 200);
  ctx.y -= 16;
  text(ctx, "Date de naissance :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "naissance", MARGIN + 108, ctx.y - 1, 130);
  text(ctx, "Lieu :", MARGIN + 260, ctx.y, 8);
  underlineField(ctx, "lieu", MARGIN + 292, ctx.y - 1, 220);
  ctx.y -= 16;
  text(ctx, "Département :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "dept", MARGIN + 85, ctx.y - 1, 150);
  text(ctx, "Nationalité :", MARGIN + 260, ctx.y, 8);
  underlineField(ctx, "nationalite", MARGIN + 328, ctx.y - 1, 184);
  ctx.y -= 16;
  text(ctx, "Etablissement précédent :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "etab_prev", MARGIN + 140, ctx.y - 1, 190);
  text(ctx, "Classe :", MARGIN + 350, ctx.y, 8);
  underlineField(ctx, "classe_prev", MARGIN + 395, ctx.y - 1, 117);
  ctx.y -= 16;
  text(ctx, "Adresse :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "adresse", MARGIN + 58, ctx.y - 1, 180);
  text(ctx, "Code Postal et ville :", MARGIN + 260, ctx.y, 8);
  underlineField(ctx, "cp_ville", MARGIN + 375, ctx.y - 1, 137);
  ctx.y = boxBottom - 12;

  // ——— Régime ———
  boxBottom = beginSection(ctx, "Régime", 36);
  radioRow(ctx, "regime", ["Internat", "Demi-pension", "Externat"], MARGIN + 10, 130);
  ctx.y = boxBottom - 12;

  // ——— Options enseignements ———
  const optCount = Math.max(config.options.length, 1);
  const optRows = Math.ceil(optCount / 2);
  const optBoxH = 28 + optRows * 15 + 8;
  boxBottom = beginSection(ctx, "Souhait classe de 6ème — Enseignements / options", optBoxH);
  ctx.y = checkboxGrid(ctx, config.options, 2, boxBottom);
  ctx.y = boxBottom - 12;

  // ——— Autres infos ———
  boxBottom = beginSection(ctx, "Autres informations", 48);
  text(ctx, "Avez-vous un enfant dans un autre établissement privé :", MARGIN + 8, ctx.y, 7.5);
  radioRow(ctx, "autre_prive", ["Non", "Oui"], MARGIN + 290, 48);
  text(ctx, "Nb :", MARGIN + 410, ctx.y, 7.5);
  underlineField(ctx, "autre_prive_nb", MARGIN + 435, ctx.y - 1, 70, 11);
  ctx.y -= 15;
  text(ctx, "Avez-vous déjà un enfant dans l'établissement :", MARGIN + 8, ctx.y, 7.5);
  radioRow(ctx, "deja_prov", ["Non", "Oui"], MARGIN + 260, 48);
  text(ctx, "Nb :", MARGIN + 380, ctx.y, 7.5);
  underlineField(ctx, "deja_prov_nb", MARGIN + 405, ctx.y - 1, 100, 11);
  ctx.y = boxBottom - 12;

  // ——— Fratrie ———
  boxBottom = beginSection(ctx, "Composition de la famille — Frère(s) et Soeur(s)", 100);
  const headers = ["Nom et Prénom", "Date de Naissance", "Classe", "Établissement"];
  const colWs = [150, 115, 70, 140];
  let hx = MARGIN + 8;
  for (let i = 0; i < headers.length; i++) {
    text(ctx, headers[i], hx, ctx.y, 7, { bold: true, color: accentRgb });
    hx += colWs[i];
  }
  ctx.y -= 12;
  for (let row = 0; row < 4; row++) {
    let x = MARGIN + 8;
    for (let col = 0; col < 4; col++) {
      underlineField(ctx, `fratrie_r${row}_c${col}`, x, ctx.y - 1, colWs[col] - 6, 12);
      x += colWs[col];
    }
    ctx.y -= 16;
  }
  ctx.y = boxBottom - 12;

  // ——— Transport + observations ———
  boxBottom = beginSection(ctx, "Transport & observations", 78);
  text(ctx, "Moyen(s) de transport utilisé(s) :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "transport", MARGIN + 175, ctx.y - 1, 340);
  ctx.y -= 16;
  text(
    ctx,
    "Observations particulières (santé, caractère, aptitudes, besoins particuliers, handicap ...) :",
    MARGIN + 8,
    ctx.y,
    7,
  );
  ctx.y -= 12;
  for (let i = 0; i < 3; i++) {
    underlineField(ctx, `obs_${i}`, MARGIN + 8, ctx.y - 1, PAGE_W - MARGIN * 2 - 16, 11);
    ctx.y -= 13;
  }
  ctx.y = boxBottom - 8;

  // ——— PAGE 2 ———
  ctx.page = doc.addPage([PAGE_W, PAGE_H]);
  ctx.y = PAGE_H - 36;

  const drawResponsable = (title: string, prefix: string) => {
    const h = 210;
    boxBottom = beginSection(ctx, title, h);
    text(ctx, "Civilité :", MARGIN + 8, ctx.y, 8);
    radioRow(ctx, `${prefix}_civ`, ["Madame", "Monsieur"], MARGIN + 55, 90);
    ctx.y -= 15;
    text(ctx, "Nom :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_nom`, MARGIN + 42, ctx.y - 1, 470);
    ctx.y -= 15;
    text(ctx, "Nom de jeune fille :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_njf`, MARGIN + 112, ctx.y - 1, 400);
    ctx.y -= 15;
    text(ctx, "Prénom :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_prenom`, MARGIN + 55, ctx.y - 1, 457);
    ctx.y -= 15;
    radioRow(
      ctx,
      `${prefix}_situation`,
      ["marié(e)", "veuf ou veuve", "séparé(e)", "divorcé(e)", "autre"],
      MARGIN + 8,
      95,
    );
    ctx.y -= 15;
    text(ctx, "Lien de parenté avec l'élève :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_lien`, MARGIN + 155, ctx.y - 1, 357);
    ctx.y -= 15;
    text(ctx, "Responsabilité :", MARGIN + 8, ctx.y, 8);
    radioRow(
      ctx,
      `${prefix}_resp`,
      ["autorité parentale", "tuteur ou tutrice"],
      MARGIN + 95,
      140,
    );
    ctx.y -= 15;
    text(ctx, "Adresse :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_adresse`, MARGIN + 55, ctx.y - 1, 457);
    ctx.y -= 15;
    text(ctx, "Code postal :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_cp`, MARGIN + 80, ctx.y - 1, 70);
    text(ctx, "Ville :", MARGIN + 170, ctx.y, 8);
    underlineField(ctx, `${prefix}_ville`, MARGIN + 210, ctx.y - 1, 302);
    ctx.y -= 15;
    text(ctx, "Tél. Domicile :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_tel_dom`, MARGIN + 85, ctx.y - 1, 110);
    text(ctx, "E-mail :", MARGIN + 215, ctx.y, 8);
    underlineField(ctx, `${prefix}_email`, MARGIN + 260, ctx.y - 1, 252);
    ctx.y -= 15;
    radioRow(
      ctx,
      `${prefix}_activite`,
      ["en activité", "recherche d'emploi", "retraité", "autre"],
      MARGIN + 8,
      110,
    );
    ctx.y -= 15;
    text(ctx, "Employeur :", MARGIN + 8, ctx.y, 8);
    underlineField(ctx, `${prefix}_employeur`, MARGIN + 70, ctx.y - 1, 95);
    text(ctx, "Tél portable :", MARGIN + 185, ctx.y, 8);
    underlineField(ctx, `${prefix}_tel_port`, MARGIN + 260, ctx.y - 1, 85);
    text(ctx, "Tél professionnel :", MARGIN + 365, ctx.y, 8);
    underlineField(ctx, `${prefix}_tel_pro`, MARGIN + 465, ctx.y - 1, 47);
    ctx.y = boxBottom - 12;
  };

  drawResponsable("Responsable principal", "r1");
  drawResponsable("Conjoint ou autre responsable", "r2");

  boxBottom = beginSection(ctx, "Engagement", 110);
  text(ctx, "Je soussigné(e) :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "soussigne", MARGIN + 95, ctx.y - 1, 180);
  text(ctx, "déclare accepter pour mon enfant le but de l'Ecole Catholique.", MARGIN + 285, ctx.y, 7, {
    maxWidth: 230,
  });
  ctx.y -= 14;
  text(
    ctx,
    'Celle-ci s\'efforce " de lier dans le même temps et le même acte l\'acquisition du savoir, la formation à l\'autonomie et',
    MARGIN + 8,
    ctx.y,
    7,
    { color: MUTED, maxWidth: PAGE_W - MARGIN * 2 - 16 },
  );
  ctx.y -= 10;
  text(
    ctx,
    'à la prise de responsabilités et l\'éducation de la Foi. "',
    MARGIN + 8,
    ctx.y,
    7,
    { color: MUTED },
  );
  ctx.y -= 16;
  text(ctx, "A :", MARGIN + 8, ctx.y, 8);
  underlineField(ctx, "fait_a", MARGIN + 28, ctx.y - 1, 180);
  text(ctx, "le :", MARGIN + 230, ctx.y, 8);
  underlineField(ctx, "fait_le", MARGIN + 255, ctx.y - 1, 180);
  ctx.y -= 18;
  text(
    ctx,
    'Signature du ou des responsables (Faire précéder la signature de la mention "LU et APPROUVE")',
    MARGIN + 8,
    ctx.y,
    7,
    { color: MUTED },
  );
  ctx.y -= 10;
  underlineField(ctx, "sig1", MARGIN + 8, ctx.y - 1, 150, 26);
  underlineField(ctx, "sig2", MARGIN + 175, ctx.y - 1, 150, 26);
  underlineField(ctx, "sig3", MARGIN + 342, ctx.y - 1, 150, 26);

  return doc.save();
}
