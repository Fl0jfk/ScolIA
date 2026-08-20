import "server-only";

import { PDFDocument, StandardFonts, rgb, type PDFFont, type PDFPage } from "pdf-lib";
import {
  fitImageInBox,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { normalizeSixiemeCodeConfig } from "@/app/lib/document-templates/inscription-sixieme-config";
import { loadInscriptionTenantSettings } from "@/app/lib/document-templates/inscription-storage";
import type { InscriptionLevelCodeConfig } from "@/app/lib/document-templates/types";

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
  form: ReturnType<PDFDocument["getForm"]>;
  font: PDFFont;
  bold: PDFFont;
  margin: number;
  width: number;
  y: number;
  accent: { r: number; g: number; b: number };
  fieldIndex: number;
};

function ensureSpace(ctx: Ctx, needed: number) {
  if (ctx.y - needed >= 48) return;
  ctx.page = ctx.doc.addPage([595.28, 841.89]);
  ctx.y = 800;
}

function sectionTitle(ctx: Ctx, title: string) {
  ensureSpace(ctx, 28);
  ctx.page.drawRectangle({
    x: ctx.margin,
    y: ctx.y - 4,
    width: ctx.width - ctx.margin * 2,
    height: 18,
    color: rgb(ctx.accent.r, ctx.accent.g, ctx.accent.b),
  });
  ctx.page.drawText(sanitize(title), {
    x: ctx.margin + 6,
    y: ctx.y,
    size: 9,
    font: ctx.bold,
    color: rgb(1, 1, 1),
  });
  ctx.y -= 26;
}

function labeledField(
  ctx: Ctx,
  label: string,
  key: string,
  opts?: { width?: number; height?: number; multiline?: boolean },
) {
  const boxH = opts?.height ?? 18;
  const boxW = opts?.width ?? ctx.width - ctx.margin * 2;
  ensureSpace(ctx, 14 + boxH + 8);
  ctx.page.drawText(sanitize(label), {
    x: ctx.margin,
    y: ctx.y,
    size: 8,
    font: ctx.font,
    color: rgb(0.25, 0.3, 0.35),
  });
  ctx.y -= 12;
  const field = ctx.form.createTextField(`s6_${key}_${ctx.fieldIndex++}`);
  field.addToPage(ctx.page, {
    x: ctx.margin,
    y: ctx.y - boxH + 4,
    width: boxW,
    height: boxH,
  });
  field.setFontSize(9);
  if (opts?.multiline) field.enableMultiline();
  ctx.y -= boxH + 8;
}

function twoColFields(
  ctx: Ctx,
  left: { label: string; key: string },
  right: { label: string; key: string },
) {
  const gap = 12;
  const colW = (ctx.width - ctx.margin * 2 - gap) / 2;
  const boxH = 18;
  ensureSpace(ctx, 14 + boxH + 8);
  ctx.page.drawText(sanitize(left.label), {
    x: ctx.margin,
    y: ctx.y,
    size: 8,
    font: ctx.font,
    color: rgb(0.25, 0.3, 0.35),
  });
  ctx.page.drawText(sanitize(right.label), {
    x: ctx.margin + colW + gap,
    y: ctx.y,
    size: 8,
    font: ctx.font,
    color: rgb(0.25, 0.3, 0.35),
  });
  ctx.y -= 12;
  const f1 = ctx.form.createTextField(`s6_${left.key}_${ctx.fieldIndex++}`);
  f1.addToPage(ctx.page, {
    x: ctx.margin,
    y: ctx.y - boxH + 4,
    width: colW,
    height: boxH,
  });
  f1.setFontSize(9);
  const f2 = ctx.form.createTextField(`s6_${right.key}_${ctx.fieldIndex++}`);
  f2.addToPage(ctx.page, {
    x: ctx.margin + colW + gap,
    y: ctx.y - boxH + 4,
    width: colW,
    height: boxH,
  });
  f2.setFontSize(9);
  ctx.y -= boxH + 8;
}

/** Options cochables en grille — s’adaptent au nombre d’items. */
function drawOptionsGrid(ctx: Ctx, options: { id: string; label: string }[]) {
  if (!options.length) return;
  sectionTitle(ctx, "Options demandées");
  const cols = options.length <= 4 ? 2 : 3;
  const gapX = 10;
  const gapY = 8;
  const colW = (ctx.width - ctx.margin * 2 - gapX * (cols - 1)) / cols;
  const rowH = 22;

  for (let i = 0; i < options.length; i++) {
    const col = i % cols;
    if (col === 0) ensureSpace(ctx, rowH + gapY);
    const opt = options[i];
    const x = ctx.margin + col * (colW + gapX);
    const y = ctx.y - 14;
    const cb = ctx.form.createCheckBox(`s6_opt_${opt.id}_${ctx.fieldIndex++}`);
    cb.addToPage(ctx.page, { x, y, width: 12, height: 12 });
    ctx.page.drawText(sanitize(opt.label), {
      x: x + 16,
      y: y + 2,
      size: 8,
      font: ctx.font,
      color: rgb(0.15, 0.18, 0.22),
      maxWidth: colW - 20,
    });
    if (col === cols - 1 || i === options.length - 1) {
      ctx.y -= rowH + gapY;
    }
  }
}

function drawFratrieTable(ctx: Ctx) {
  sectionTitle(ctx, "Frères et sœurs déjà scolarisés dans l'établissement");
  const headers = ["Nom et prénom", "Date de naissance", "Classe", "Établissement"];
  const colWs = [150, 90, 70, 140];
  const rowH = 18;
  ensureSpace(ctx, 16 + rowH * 5);
  let x = ctx.margin;
  for (let i = 0; i < headers.length; i++) {
    ctx.page.drawText(sanitize(headers[i]), {
      x,
      y: ctx.y,
      size: 7,
      font: ctx.bold,
      color: rgb(0.3, 0.35, 0.4),
    });
    x += colWs[i];
  }
  ctx.y -= 12;
  for (let row = 0; row < 4; row++) {
    x = ctx.margin;
    for (let col = 0; col < 4; col++) {
      const f = ctx.form.createTextField(`s6_fratrie_r${row}_c${col}_${ctx.fieldIndex++}`);
      f.addToPage(ctx.page, {
        x,
        y: ctx.y - rowH + 4,
        width: colWs[col] - 4,
        height: rowH,
      });
      f.setFontSize(8);
      x += colWs[col];
    }
    ctx.y -= rowH + 4;
  }
}

function drawResponsable(ctx: Ctx, n: 1 | 2) {
  sectionTitle(ctx, `Responsable légal ${n}`);
  twoColFields(
    ctx,
    { label: "Nom", key: `resp${n}_nom` },
    { label: "Nom de jeune fille", key: `resp${n}_njf` },
  );
  twoColFields(
    ctx,
    { label: "Prénom", key: `resp${n}_prenom` },
    { label: "Lien de parenté", key: `resp${n}_lien` },
  );
  labeledField(ctx, "Adresse", `resp${n}_adresse`);
  twoColFields(
    ctx,
    { label: "Code postal", key: `resp${n}_cp` },
    { label: "Ville", key: `resp${n}_ville` },
  );
  twoColFields(
    ctx,
    { label: "Tél. domicile", key: `resp${n}_tel_dom` },
    { label: "Tél. portable", key: `resp${n}_tel_port` },
  );
  twoColFields(
    ctx,
    { label: "E-mail", key: `resp${n}_email` },
    { label: "Tél. professionnel", key: `resp${n}_tel_pro` },
  );
  twoColFields(
    ctx,
    { label: "Profession", key: `resp${n}_pro` },
    { label: "Employeur", key: `resp${n}_employeur` },
  );
}

/**
 * Fiche d’inscription 6e générée entièrement en code (AcroForm).
 * Les options se placent automatiquement selon la liste configurée.
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
  const page = doc.addPage([595.28, 841.89]);
  const form = doc.getForm();
  const margin = 40;
  const width = 595.28;

  const ctx: Ctx = {
    doc,
    page,
    form,
    font,
    bold,
    margin,
    width,
    y: 800,
    accent,
    fieldIndex: 1,
  };

  // En-tête intégré (pas de surcharge)
  const logo = await loadSchoolLogoForPdf();
  let headerLeft = margin;
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 72, 42);
      page.drawImage(img, {
        x: margin,
        y: ctx.y - fitted.height + 10,
        width: fitted.width,
        height: fitted.height,
      });
      headerLeft = margin + fitted.width + 12;
    }
  }

  page.drawText(displayName, {
    x: headerLeft,
    y: ctx.y,
    size: 13,
    font: bold,
    color: rgb(accent.r, accent.g, accent.b),
    maxWidth: width - headerLeft - margin,
  });
  if (letterhead.addressLine) {
    page.drawText(sanitize(letterhead.addressLine), {
      x: headerLeft,
      y: ctx.y - 14,
      size: 8,
      font,
      color: rgb(0.35, 0.4, 0.45),
      maxWidth: width - headerLeft - margin,
    });
  }
  const yearLabel = `Année scolaire ${sanitize(config.schoolYear)}`;
  page.drawText(yearLabel, {
    x: width - margin - bold.widthOfTextAtSize(yearLabel, 9),
    y: ctx.y - 28,
    size: 9,
    font: bold,
    color: rgb(0.2, 0.25, 0.3),
  });
  ctx.y -= 48;

  page.drawRectangle({
    x: margin,
    y: ctx.y,
    width: width - margin * 2,
    height: 1.5,
    color: rgb(accent.r, accent.g, accent.b),
  });
  ctx.y -= 20;

  page.drawText(sanitize(config.title || "Fiche d'inscription — Sixième"), {
    x: margin,
    y: ctx.y,
    size: 14,
    font: bold,
    color: rgb(0.1, 0.12, 0.16),
  });
  ctx.y -= 14;
  if (config.subtitle) {
    page.drawText(sanitize(config.subtitle), {
      x: margin,
      y: ctx.y,
      size: 8,
      font,
      color: rgb(0.4, 0.45, 0.5),
      maxWidth: width - margin * 2,
    });
    ctx.y -= 18;
  } else {
    ctx.y -= 8;
  }

  sectionTitle(ctx, "Identité de l'élève");
  twoColFields(ctx, { label: "Nom", key: "nom" }, { label: "Prénom(s)", key: "prenoms" });
  twoColFields(
    ctx,
    { label: "Date de naissance", key: "naissance" },
    { label: "Lieu de naissance", key: "lieu" },
  );
  twoColFields(
    ctx,
    { label: "Département", key: "dept" },
    { label: "Nationalité", key: "nationalite" },
  );

  sectionTitle(ctx, "Scolarité précédente");
  twoColFields(
    ctx,
    { label: "Établissement précédent", key: "etab_prev" },
    { label: "Classe", key: "classe_prev" },
  );

  sectionTitle(ctx, "Adresse familiale");
  labeledField(ctx, "Adresse", "adresse");
  labeledField(ctx, "Code postal et ville", "cp_ville");

  drawFratrieTable(ctx);
  drawOptionsGrid(ctx, config.options);

  sectionTitle(ctx, "Moyens de transport");
  labeledField(ctx, "Précisez les moyens de transport utilisés", "transport");

  sectionTitle(ctx, "Observations (santé, besoins particuliers…)");
  labeledField(ctx, "Observations", "observations", { height: 48, multiline: true });

  // Page responsables
  ctx.page = doc.addPage([595.28, 841.89]);
  ctx.y = 800;
  drawResponsable(ctx, 1);
  drawResponsable(ctx, 2);

  sectionTitle(ctx, "Engagement");
  labeledField(ctx, "Je soussigné(e)", "soussigne");
  twoColFields(ctx, { label: "Fait à", key: "fait_a" }, { label: "Le", key: "fait_le" });
  ensureSpace(ctx, 50);
  ctx.page.drawText("Signature du responsable légal", {
    x: margin,
    y: ctx.y,
    size: 8,
    font,
    color: rgb(0.3, 0.35, 0.4),
  });
  ctx.y -= 8;
  const sig = form.createTextField(`s6_signature_${ctx.fieldIndex++}`);
  sig.addToPage(ctx.page, {
    x: margin,
    y: ctx.y - 40,
    width: 220,
    height: 44,
  });

  return doc.save();
}
