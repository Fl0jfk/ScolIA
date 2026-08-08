import "server-only";

import { PDFDocument, StandardFonts, rgb, type PDFPage, type PDFFont } from "pdf-lib";
import {
  fitImageInBox,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveDirectionSignatureBytes } from "@/app/lib/direction-signature";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

function sanitize(input: string): string {
  return String(input || "")
    .replace(/[\u2018\u2019]/g, "'")
    .replace(/[\u201C\u201D]/g, '"')
    .replace(/\u2026/g, "...")
    .replace(/[^\x00-\xFF]/g, "?");
}

function wrap(text: string, maxChars: number): string[] {
  const words = sanitize(text).split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let line = "";
  for (const w of words) {
    const next = line ? `${line} ${w}` : w;
    if (next.length > maxChars) {
      if (line) lines.push(line);
      line = w;
    } else line = next;
  }
  if (line) lines.push(line);
  return lines.length ? lines : [""];
}

function formatFrDate(iso: string): string {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) return iso || "—";
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

type Ctx = {
  doc: PDFDocument;
  page: PDFPage;
  font: PDFFont;
  bold: PDFFont;
  margin: number;
  width: number;
  y: number;
};

function ensureSpace(ctx: Ctx, needed: number) {
  if (ctx.y - needed < 56) {
    ctx.page = ctx.doc.addPage([595.28, 841.89]);
    ctx.y = 800;
  }
}

function drawText(
  ctx: Ctx,
  text: string,
  opts?: { size?: number; bold?: boolean; indent?: number; color?: ReturnType<typeof rgb> },
) {
  const size = opts?.size ?? 10;
  ensureSpace(ctx, size + 6);
  ctx.page.drawText(sanitize(text), {
    x: ctx.margin + (opts?.indent || 0),
    y: ctx.y,
    size,
    font: opts?.bold ? ctx.bold : ctx.font,
    color: opts?.color || rgb(0.1, 0.12, 0.16),
  });
  ctx.y -= size + 5;
}

function drawWrapped(
  ctx: Ctx,
  text: string,
  opts?: { size?: number; bold?: boolean; maxChars?: number; indent?: number },
) {
  const size = opts?.size ?? 10;
  for (const line of wrap(text, opts?.maxChars ?? 88)) {
    drawText(ctx, line, { size, bold: opts?.bold, indent: opts?.indent });
  }
}

async function drawHeader(ctx: Ctx) {
  const [letterhead, logo] = await Promise.all([getSchoolLetterhead(), loadSchoolLogoForPdf()]);
  const rightX = ctx.width - ctx.margin;

  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await ctx.doc.embedJpg(bytes) : await ctx.doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 90, 48);
      ctx.page.drawImage(img, {
        x: ctx.margin,
        y: ctx.y - fitted.height + 8,
        width: fitted.width,
        height: fitted.height,
      });
    }
  }

  const nameW = ctx.bold.widthOfTextAtSize(sanitize(letterhead.name), 12);
  ctx.page.drawText(sanitize(letterhead.name), {
    x: rightX - nameW,
    y: ctx.y,
    size: 12,
    font: ctx.bold,
    color: rgb(0.12, 0.16, 0.22),
  });

  let ry = ctx.y - 14;
  const sub = sanitize(letterhead.subtitle);
  const subW = ctx.font.widthOfTextAtSize(sub, 8);
  ctx.page.drawText(sub, {
    x: rightX - subW,
    y: ry,
    size: 8,
    font: ctx.font,
    color: rgb(0.4, 0.45, 0.52),
  });
  if (letterhead.addressLine) {
    ry -= 11;
    const a = sanitize(letterhead.addressLine);
    const aW = ctx.font.widthOfTextAtSize(a, 8);
    ctx.page.drawText(a, {
      x: rightX - aW,
      y: ry,
      size: 8,
      font: ctx.font,
      color: rgb(0.4, 0.45, 0.52),
    });
  }

  ctx.y -= 56;
  ctx.page.drawRectangle({
    x: 0,
    y: ctx.y,
    width: ctx.width,
    height: 2.5,
    color: rgb(0.12, 0.16, 0.22),
  });
  ctx.page.drawRectangle({
    x: 0,
    y: ctx.y - 2,
    width: ctx.width,
    height: 1.2,
    color: rgb(0.15, 0.39, 0.92),
  });
  ctx.y -= 28;
}

function str(v: Record<string, string | boolean>, key: string): string {
  const x = v[key];
  if (typeof x === "boolean") return x ? "Oui" : "Non";
  return String(x || "").trim();
}

async function renderCertificat(values: Record<string, string | boolean>): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const page = doc.addPage([595.28, 841.89]);
  const ctx: Ctx = {
    doc,
    page,
    font,
    bold,
    margin: 48,
    width: 595.28,
    y: 800,
  };
  await drawHeader(ctx);

  drawText(ctx, "CERTIFICAT DE SCOLARITE", { size: 16, bold: true });
  ctx.y -= 8;

  const letterhead = await getSchoolLetterhead();
  const prenom = str(values, "prenom");
  const nom = str(values, "nom");
  const classe = str(values, "classe");
  const annee = str(values, "anneeScolaire");
  const dateDoc = formatFrDate(str(values, "dateDocument"));
  const ville = str(values, "ville") || letterhead.cityLine || letterhead.name;
  const signataire = str(values, "signataire");
  const qualite = str(values, "qualite");

  drawWrapped(
    ctx,
    `Je soussigné(e), ${signataire || qualite}, ${qualite}, certifie que :`,
    { size: 11, maxChars: 82 },
  );
  ctx.y -= 10;

  drawText(ctx, `${prenom} ${nom}`.toUpperCase(), { size: 13, bold: true });
  ctx.y -= 4;
  drawWrapped(
    ctx,
    `est régulièrement inscrit(e) dans notre établissement en classe de ${classe} pour l'année scolaire ${annee}.`,
    { size: 11, maxChars: 82 },
  );
  ctx.y -= 12;
  drawWrapped(
    ctx,
    "Le présent certificat est délivré pour servir et valoir ce que de droit.",
    { size: 10, maxChars: 82 },
  );
  ctx.y -= 28;
  drawText(ctx, `Fait à ${ville}, le ${dateDoc}.`, { size: 10 });
  ctx.y -= 24;
  drawText(ctx, signataire || "—", { size: 11, bold: true });
  drawText(ctx, qualite || "", { size: 9, color: rgb(0.35, 0.4, 0.48) });
  ctx.y -= 6;

  // Signature direction (premier établissement configuré), si disponible
  try {
    const bundle = await loadAppConfig();
    const etabId =
      bundle.establishments.find((e) => e.active !== false && e.signatureS3Key)?.id ||
      bundle.establishments[0]?.id ||
      "college";
    const sigBytes = await resolveDirectionSignatureBytes(etabId);
    if (sigBytes?.length) {
      const isJpg = sigBytes[0] === 0xff && sigBytes[1] === 0xd8;
      const img = isJpg
        ? await ctx.doc.embedJpg(sigBytes)
        : await ctx.doc.embedPng(sigBytes);
      const fitted = fitImageInBox(img.width, img.height, 140, 56);
      ensureSpace(ctx, fitted.height + 8);
      ctx.page.drawImage(img, {
        x: ctx.margin,
        y: ctx.y - fitted.height,
        width: fitted.width,
        height: fitted.height,
      });
      ctx.y -= fitted.height + 4;
    } else {
      drawText(ctx, "Signature", { size: 8, color: rgb(0.55, 0.58, 0.62) });
    }
  } catch {
    drawText(ctx, "Signature", { size: 8, color: rgb(0.55, 0.58, 0.62) });
  }

  // footer
  const footer = sanitize(letterhead.footerLeft);
  page.drawText(footer, {
    x: 48,
    y: 36,
    size: 7,
    font,
    color: rgb(0.55, 0.6, 0.65),
  });

  return doc.save();
}

async function renderFiche(values: Record<string, string | boolean>): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const page = doc.addPage([595.28, 841.89]);
  const ctx: Ctx = {
    doc,
    page,
    font,
    bold,
    margin: 48,
    width: 595.28,
    y: 800,
  };
  await drawHeader(ctx);

  drawText(ctx, "FICHE D'INSCRIPTION", { size: 16, bold: true });
  drawText(ctx, `Année scolaire ${str(values, "anneeScolaire")}`, {
    size: 10,
    color: rgb(0.35, 0.4, 0.48),
  });
  ctx.y -= 10;

  const section = (title: string) => {
    ctx.y -= 4;
    ensureSpace(ctx, 22);
    ctx.page.drawRectangle({
      x: ctx.margin,
      y: ctx.y - 4,
      width: ctx.width - ctx.margin * 2,
      height: 18,
      color: rgb(0.93, 0.95, 0.98),
    });
    drawText(ctx, title, { size: 10, bold: true });
    ctx.y -= 2;
  };

  const row = (label: string, value: string) => {
    drawText(ctx, `${label} : ${value || "—"}`, { size: 10, indent: 4 });
  };

  section("Enfant");
  row("Nom", str(values, "nom"));
  row("Prénom", str(values, "prenom"));
  row("Date de naissance", formatFrDate(str(values, "dateNaissance")));
  row("Classe demandée", str(values, "classeDemandee"));

  section("Responsable 1");
  row("Nom", str(values, "resp1Nom"));
  row("E-mail", str(values, "resp1Email"));
  row("Téléphone", str(values, "resp1Tel"));

  if (str(values, "resp2Nom") || str(values, "resp2Email")) {
    section("Responsable 2");
    row("Nom", str(values, "resp2Nom"));
    row("E-mail", str(values, "resp2Email"));
    row("Téléphone", str(values, "resp2Tel"));
  }

  section("Coordonnées & infos");
  drawWrapped(ctx, `Adresse : ${str(values, "adresse") || "—"}`, { size: 10, maxChars: 85, indent: 4 });
  drawWrapped(ctx, `Allergies / précautions : ${str(values, "allergies") || "Néant"}`, {
    size: 10,
    maxChars: 85,
    indent: 4,
  });
  row("Droit à l'image", str(values, "droitImage"));
  if (str(values, "notes")) {
    drawWrapped(ctx, `Notes : ${str(values, "notes")}`, { size: 10, maxChars: 85, indent: 4 });
  }

  ctx.y -= 16;
  drawWrapped(
    ctx,
    "Document généré par l'établissement (Scola) — version numérique, sans réimpression nécessaire.",
    { size: 8, maxChars: 90 },
  );

  const letterhead = await getSchoolLetterhead();
  page.drawText(sanitize(letterhead.footerLeft), {
    x: 48,
    y: 36,
    size: 7,
    font,
    color: rgb(0.55, 0.6, 0.65),
  });

  return doc.save();
}

async function renderAutorisation(values: Record<string, string | boolean>): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const page = doc.addPage([595.28, 841.89]);
  const ctx: Ctx = {
    doc,
    page,
    font,
    bold,
    margin: 48,
    width: 595.28,
    y: 800,
  };
  await drawHeader(ctx);

  drawText(ctx, "AUTORISATION PARENTALE DE SORTIE", { size: 15, bold: true });
  ctx.y -= 10;

  drawWrapped(
    ctx,
    `Je soussigné(e) ${str(values, "respNom")}, responsable légal(e) de ${str(values, "prenom")} ${str(values, "nom")} (classe ${str(values, "classe")}),`,
    { size: 11, maxChars: 82 },
  );
  drawWrapped(
    ctx,
    `${str(values, "autorise") === "Oui" ? "autorise" : "n'autorise pas"} mon enfant à participer à :`,
    { size: 11, maxChars: 82 },
  );
  ctx.y -= 6;
  drawText(ctx, str(values, "sortieTitre") || "—", { size: 12, bold: true });
  ctx.y -= 4;
  drawText(ctx, `Lieu : ${str(values, "lieu") || "—"}`, { size: 10 });
  drawText(
    ctx,
    `Du ${formatFrDate(str(values, "dateDebut"))} au ${formatFrDate(str(values, "dateFin"))}` +
      (str(values, "horaireDepart") || str(values, "horaireRetour")
        ? ` — départ ${str(values, "horaireDepart") || "—"} / retour ${str(values, "horaireRetour") || "—"}`
        : ""),
    { size: 10 },
  );
  ctx.y -= 8;
  drawText(ctx, `Téléphone du responsable : ${str(values, "respTel") || "—"}`, { size: 10 });
  if (str(values, "urgenceTel")) {
    drawText(ctx, `Téléphone d'urgence : ${str(values, "urgenceTel")}`, { size: 10 });
  }
  drawText(ctx, `Soins d'urgence autorisés : ${str(values, "soins")}`, { size: 10 });
  if (str(values, "notes")) {
    drawWrapped(ctx, `Informations utiles : ${str(values, "notes")}`, {
      size: 10,
      maxChars: 85,
    });
  }
  ctx.y -= 20;
  drawText(ctx, `Date : ${formatFrDate(str(values, "dateDocument"))}`, { size: 10 });
  ctx.y -= 28;
  drawText(ctx, "Signature du responsable légal", {
    size: 9,
    color: rgb(0.45, 0.48, 0.52),
  });
  ctx.y -= 40;
  ctx.page.drawRectangle({
    x: ctx.margin,
    y: ctx.y,
    width: 200,
    height: 1,
    color: rgb(0.7, 0.72, 0.76),
  });

  const letterhead = await getSchoolLetterhead();
  page.drawText(sanitize(letterhead.footerLeft), {
    x: 48,
    y: 36,
    size: 7,
    font,
    color: rgb(0.55, 0.6, 0.65),
  });

  return doc.save();
}

async function renderCourrier(values: Record<string, string | boolean>): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const page = doc.addPage([595.28, 841.89]);
  const ctx: Ctx = {
    doc,
    page,
    font,
    bold,
    margin: 48,
    width: 595.28,
    y: 800,
  };
  await drawHeader(ctx);

  const letterhead = await getSchoolLetterhead();
  const ville = str(values, "ville") || letterhead.cityLine || letterhead.name;

  drawText(ctx, str(values, "destinataire") || "Aux familles", { size: 11 });
  ctx.y -= 8;
  drawText(ctx, `Objet : ${str(values, "objet") || "—"}`, { size: 11, bold: true });
  ctx.y -= 14;

  const corps = str(values, "corps");
  for (const block of corps.split(/\n+/).map((s) => s.trim()).filter(Boolean)) {
    drawWrapped(ctx, block, { size: 11, maxChars: 85 });
    ctx.y -= 6;
  }

  ctx.y -= 16;
  drawText(ctx, `Fait à ${ville}, le ${formatFrDate(str(values, "dateDocument"))}.`, {
    size: 10,
  });
  ctx.y -= 24;
  drawText(ctx, str(values, "signataire") || "—", { size: 11, bold: true });
  drawText(ctx, str(values, "qualite") || "", {
    size: 9,
    color: rgb(0.35, 0.4, 0.48),
  });

  page.drawText(sanitize(letterhead.footerLeft), {
    x: 48,
    y: 36,
    size: 7,
    font,
    color: rgb(0.55, 0.6, 0.65),
  });

  return doc.save();
}

export async function renderDocumentTemplatePdf(
  templateId: DocumentTemplateId,
  values: Record<string, string | boolean>,
): Promise<Uint8Array> {
  if (templateId === "certificat-scolarite") return renderCertificat(values);
  if (templateId === "fiche-inscription") return renderFiche(values);
  if (templateId === "autorisation-sortie") return renderAutorisation(values);
  if (templateId === "courrier-families") return renderCourrier(values);
  throw new Error("Modèle inconnu");
}
