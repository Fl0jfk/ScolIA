import "server-only";

import {
  PDFDocument,
  StandardFonts,
  rgb,
  type PDFFont,
  type PDFForm,
  type PDFPage,
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
const MARGIN = 42;
const INK = rgb(0.08, 0.1, 0.14);
const MUTED = rgb(0.28, 0.32, 0.36);
const LINE = rgb(0.55, 0.58, 0.62);

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
  if (!Number.isFinite(n)) return { r: 0.12, g: 0.16, b: 0.22 };
  return {
    r: ((n >> 16) & 255) / 255,
    g: ((n >> 8) & 255) / 255,
    b: (n & 255) / 255,
  };
}

type Ctx = {
  doc: PDFDocument;
  page: PDFPage;
  form: PDFForm;
  font: PDFFont;
  bold: PDFFont;
  y: number;
  accent: { r: number; g: number; b: number };
  n: number;
};

function text(
  ctx: Ctx,
  value: string,
  x: number,
  y: number,
  size: number,
  opts?: { bold?: boolean; color?: ReturnType<typeof rgb>; maxWidth?: number },
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
  field.setFontSize(9);
  field.setBorderWidth(0);
  ctx.page.drawLine({
    start: { x, y: y - 2 },
    end: { x: x + width, y: y - 2 },
    thickness: 0.6,
    color: LINE,
  });
}

/** Ligne « Label : ________ » */
function labeledLine(
  ctx: Ctx,
  label: string,
  fieldName: string,
  labelWidth: number,
  fieldWidth: number,
  x = MARGIN,
) {
  text(ctx, `${label} :`, x, ctx.y, 9);
  underlineField(ctx, fieldName, x + labelWidth, ctx.y - 1, fieldWidth);
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
      width: 11,
      height: 11,
    });
    text(ctx, opt, x + 15, ctx.y, 8);
    x += gap;
  }
}

function checkboxRow(
  ctx: Ctx,
  options: { id: string; label: string }[],
  cols = 2,
) {
  if (!options.length) return;
  const usable = PAGE_W - MARGIN * 2;
  const colW = usable / cols;
  const rowH = 16;
  for (let i = 0; i < options.length; i++) {
    const col = i % cols;
    if (col === 0 && i > 0) ctx.y -= rowH;
    const opt = options[i];
    const x = MARGIN + col * colW;
    const cb = ctx.form.createCheckBox(`s6_opt_${opt.id}_${ctx.n++}`);
    cb.addToPage(ctx.page, { x, y: ctx.y - 3, width: 11, height: 11 });
    text(ctx, opt.label, x + 15, ctx.y, 8, { maxWidth: colW - 20 });
  }
  ctx.y -= rowH;
}

/**
 * Fiche 6e calquée sur le PDF d'origine Providence :
 * en-tête, élève, régime, options enseignements, famille, responsables, engagement.
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
    opts?.accentColor?.trim() || settings.accentColor?.trim() || "#1E4A32";
  const accent = hexToRgb(accentHex);
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
    y: PAGE_H - 40,
    accent,
    n: 1,
  };

  // ——— PAGE 1 : en-tête ———
  const logo = await loadSchoolLogoForPdf();
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 54, 36);
      ctx.page.drawImage(img, {
        x: MARGIN,
        y: ctx.y - fitted.height + 8,
        width: fitted.width,
        height: fitted.height,
      });
    }
  }

  const nameSize = 12;
  const nameW = bold.widthOfTextAtSize(sanitize(displayName).toUpperCase(), nameSize);
  text(ctx, displayName.toUpperCase(), (PAGE_W - nameW) / 2, ctx.y, nameSize, {
    bold: true,
    color: rgb(accent.r, accent.g, accent.b),
  });
  ctx.y -= 16;
  const yearLine = `ANNÉE ${sanitize(config.schoolYear).replace(/-/g, " - ")}`;
  const yearW = bold.widthOfTextAtSize(yearLine, 10);
  text(ctx, yearLine, (PAGE_W - yearW) / 2, ctx.y, 10, { bold: true });
  ctx.y -= 18;
  const title = sanitize(config.title || "DEMANDE D'INSCRIPTION EN SIXIÈME").toUpperCase();
  const titleW = bold.widthOfTextAtSize(title, 13);
  text(ctx, title, (PAGE_W - titleW) / 2, ctx.y, 13, { bold: true });
  if (config.subtitle?.trim()) {
    ctx.y -= 12;
    const sub = sanitize(config.subtitle);
    const subW = font.widthOfTextAtSize(sub, 8);
    text(ctx, sub, (PAGE_W - subW) / 2, ctx.y, 8, { color: MUTED });
  }
  ctx.y -= 14;
  ctx.page.drawLine({
    start: { x: MARGIN, y: ctx.y },
    end: { x: PAGE_W - MARGIN, y: ctx.y },
    thickness: 1,
    color: rgb(accent.r, accent.g, accent.b),
  });
  ctx.y -= 18;

  // ——— Élève ———
  text(ctx, "Elève", MARGIN, ctx.y, 11, { bold: true, color: rgb(accent.r, accent.g, accent.b) });
  ctx.y -= 16;

  text(ctx, "Nom :", MARGIN, ctx.y, 9);
  underlineField(ctx, "nom", MARGIN + 38, ctx.y - 1, 210);
  text(ctx, "Prénoms :", MARGIN + 270, ctx.y, 9);
  underlineField(ctx, "prenoms", MARGIN + 325, ctx.y - 1, 186);
  ctx.y -= 18;

  text(ctx, "Date de naissance :", MARGIN, ctx.y, 9);
  underlineField(ctx, "naissance", MARGIN + 105, ctx.y - 1, 145);
  text(ctx, "Lieu :", MARGIN + 270, ctx.y, 9);
  underlineField(ctx, "lieu", MARGIN + 305, ctx.y - 1, 206);
  ctx.y -= 18;

  text(ctx, "Département :", MARGIN, ctx.y, 9);
  underlineField(ctx, "dept", MARGIN + 82, ctx.y - 1, 168);
  text(ctx, "Nationalité :", MARGIN + 270, ctx.y, 9);
  underlineField(ctx, "nationalite", MARGIN + 340, ctx.y - 1, 171);
  ctx.y -= 18;

  text(ctx, "Etablissement précédent :", MARGIN, ctx.y, 9);
  underlineField(ctx, "etab_prev", MARGIN + 138, ctx.y - 1, 200);
  text(ctx, "Classe :", MARGIN + 360, ctx.y, 9);
  underlineField(ctx, "classe_prev", MARGIN + 405, ctx.y - 1, 106);
  ctx.y -= 18;

  text(ctx, "Adresse :", MARGIN, ctx.y, 9);
  underlineField(ctx, "adresse", MARGIN + 55, ctx.y - 1, 190);
  text(ctx, "Code Postal et ville :", MARGIN + 270, ctx.y, 9);
  underlineField(ctx, "cp_ville", MARGIN + 385, ctx.y - 1, 126);
  ctx.y -= 20;

  // ——— Régime (fixe comme l'original) ———
  text(ctx, "Régime", MARGIN, ctx.y, 11, { bold: true, color: rgb(accent.r, accent.g, accent.b) });
  ctx.y -= 16;
  radioRow(ctx, "regime", ["Internat", "Demi-pension", "Externat"], MARGIN, 120);
  ctx.y -= 20;

  // ——— Options / enseignements (configurables, auto-layout) ———
  text(ctx, "SOUHAIT CONCERNANT LA CLASSE DE 6ÈME — ENSEIGNEMENTS / OPTIONS", MARGIN, ctx.y, 8, {
    bold: true,
  });
  ctx.y -= 14;
  checkboxRow(ctx, config.options, 2);
  ctx.y -= 8;

  // ——— Autres informations ———
  text(ctx, "Autres informations", MARGIN, ctx.y, 11, {
    bold: true,
    color: rgb(accent.r, accent.g, accent.b),
  });
  ctx.y -= 16;
  text(ctx, "Avez-vous un enfant dans un autre établissement privé :", MARGIN, ctx.y, 8);
  radioRow(ctx, "autre_prive", ["Non", "Oui"], MARGIN + 280, 50);
  text(ctx, "Nombre :", MARGIN + 400, ctx.y, 8);
  underlineField(ctx, "autre_prive_nb", MARGIN + 450, ctx.y - 1, 60, 12);
  ctx.y -= 16;
  text(ctx, "Avez-vous déjà un enfant dans l'établissement :", MARGIN, ctx.y, 8);
  radioRow(ctx, "deja_prov", ["Non", "Oui"], MARGIN + 250, 50);
  text(ctx, "Nombre :", MARGIN + 370, ctx.y, 8);
  underlineField(ctx, "deja_prov_nb", MARGIN + 420, ctx.y - 1, 90, 12);
  ctx.y -= 20;

  // ——— Fratrie ———
  text(ctx, "Composition de la famille — Frère(s) et Soeur(s)", MARGIN, ctx.y, 11, {
    bold: true,
    color: rgb(accent.r, accent.g, accent.b),
  });
  ctx.y -= 14;
  const headers = ["Nom et Prénom", "Date de Naissance", "Classe", "Établissement"];
  const colWs = [150, 120, 70, 130];
  let hx = MARGIN;
  for (let i = 0; i < headers.length; i++) {
    text(ctx, headers[i], hx, ctx.y, 7, { bold: true, color: MUTED });
    hx += colWs[i];
  }
  ctx.y -= 12;
  for (let row = 0; row < 4; row++) {
    let x = MARGIN;
    for (let col = 0; col < 4; col++) {
      underlineField(ctx, `fratrie_r${row}_c${col}`, x, ctx.y - 1, colWs[col] - 6, 13);
      x += colWs[col];
    }
    ctx.y -= 18;
  }
  ctx.y -= 4;

  labeledLine(ctx, "Moyen(s) de transport utilisé(s)", "transport", 175, 336);
  ctx.y -= 18;

  text(ctx, "Observations particulières (santé, caractère, aptitudes, besoins particuliers, handicap ...) :", MARGIN, ctx.y, 8);
  ctx.y -= 14;
  for (let i = 0; i < 3; i++) {
    underlineField(ctx, `obs_${i}`, MARGIN, ctx.y - 1, PAGE_W - MARGIN * 2, 12);
    ctx.y -= 15;
  }

  // ——— PAGE 2 : responsables ———
  ctx.page = doc.addPage([PAGE_W, PAGE_H]);
  ctx.y = PAGE_H - 42;

  const drawResponsable = (title: string, prefix: string) => {
    text(ctx, title, MARGIN, ctx.y, 11, { bold: true, color: rgb(accent.r, accent.g, accent.b) });
    ctx.y -= 16;
    text(ctx, "Civilité :", MARGIN, ctx.y, 9);
    radioRow(ctx, `${prefix}_civ`, ["Madame", "Monsieur"], MARGIN + 55, 90);
    ctx.y -= 16;

    labeledLine(ctx, "Nom", `${prefix}_nom`, 40, 470);
    ctx.y -= 18;
    labeledLine(ctx, "Nom de jeune fille", `${prefix}_njf`, 105, 405);
    ctx.y -= 18;
    labeledLine(ctx, "Prénom", `${prefix}_prenom`, 50, 460);
    ctx.y -= 18;

    radioRow(
      ctx,
      `${prefix}_situation`,
      ["marié(e)", "veuf ou veuve", "séparé(e)", "divorcé(e)", "autre"],
      MARGIN,
      95,
    );
    ctx.y -= 16;

    labeledLine(ctx, "Lien de parenté avec l'élève", `${prefix}_lien`, 155, 355);
    ctx.y -= 16;
    text(ctx, "Responsabilité :", MARGIN, ctx.y, 9);
    radioRow(
      ctx,
      `${prefix}_resp`,
      ["autorité parentale", "tuteur ou tutrice"],
      MARGIN + 90,
      140,
    );
    ctx.y -= 16;

    labeledLine(ctx, "Adresse", `${prefix}_adresse`, 50, 460);
    ctx.y -= 18;
    text(ctx, "Code postal :", MARGIN, ctx.y, 9);
    underlineField(ctx, `${prefix}_cp`, MARGIN + 75, ctx.y - 1, 70);
    text(ctx, "Ville :", MARGIN + 165, ctx.y, 9);
    underlineField(ctx, `${prefix}_ville`, MARGIN + 205, ctx.y - 1, 305);
    ctx.y -= 18;

    text(ctx, "Tél. Domicile :", MARGIN, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_dom`, MARGIN + 80, ctx.y - 1, 110);
    text(ctx, "E-mail :", MARGIN + 210, ctx.y, 9);
    underlineField(ctx, `${prefix}_email`, MARGIN + 255, ctx.y - 1, 255);
    ctx.y -= 16;

    radioRow(
      ctx,
      `${prefix}_activite`,
      ["en activité", "recherche d'emploi", "retraité", "autre"],
      MARGIN,
      110,
    );
    ctx.y -= 16;
    labeledLine(ctx, "Profession", `${prefix}_pro`, 65, 180);
    // second field on same visual row already consumed — keep compact
    ctx.y -= 2;
    text(ctx, "Employeur :", MARGIN, ctx.y, 9);
    underlineField(ctx, `${prefix}_employeur`, MARGIN + 65, ctx.y - 1, 90);
    text(ctx, "Tél portable :", MARGIN + 175, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_port`, MARGIN + 245, ctx.y - 1, 85);
    text(ctx, "Tél professionnel :", MARGIN + 350, ctx.y, 9);
    underlineField(ctx, `${prefix}_tel_pro`, MARGIN + 450, ctx.y - 1, 60);
    ctx.y -= 22;
  };

  drawResponsable("Responsable principal", "r1");
  drawResponsable("Conjoint ou autre responsable", "r2");

  text(ctx, "Je soussigné(e) :", MARGIN, ctx.y, 9);
  underlineField(ctx, "soussigne", MARGIN + 85, ctx.y - 1, 200);
  text(ctx, "déclare accepter pour mon enfant le but de l'Ecole Catholique.", MARGIN + 295, ctx.y, 8, {
    maxWidth: 220,
  });
  ctx.y -= 16;
  text(
    ctx,
    'Celle-ci s\'efforce " de lier dans le même temps et le même acte l\'acquisition du savoir, la formation à l\'autonomie et',
    MARGIN,
    ctx.y,
    7,
    { color: MUTED, maxWidth: PAGE_W - MARGIN * 2 },
  );
  ctx.y -= 10;
  text(
    ctx,
    'à la prise de responsabilités et l\'éducation de la Foi. "',
    MARGIN,
    ctx.y,
    7,
    { color: MUTED, maxWidth: PAGE_W - MARGIN * 2 },
  );
  ctx.y -= 20;

  text(ctx, "A :", MARGIN, ctx.y, 9);
  underlineField(ctx, "fait_a", MARGIN + 25, ctx.y - 1, 180);
  text(ctx, "le :", MARGIN + 230, ctx.y, 9);
  underlineField(ctx, "fait_le", MARGIN + 255, ctx.y - 1, 180);
  ctx.y -= 24;

  text(
    ctx,
    'Signature du ou des responsables (Faire précéder la signature de la mention "LU et APPROUVE")',
    MARGIN,
    ctx.y,
    8,
    { color: MUTED },
  );
  ctx.y -= 10;
  underlineField(ctx, "sig1", MARGIN, ctx.y - 1, 150, 28);
  underlineField(ctx, "sig2", MARGIN + 170, ctx.y - 1, 150, 28);
  underlineField(ctx, "sig3", MARGIN + 340, ctx.y - 1, 150, 28);

  return doc.save();
}
