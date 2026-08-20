import "server-only";

import fs from "node:fs/promises";
import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import {
  fitImageInBox,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import {
  getInscriptionLevelMeta,
  inscriptionSourcePath,
} from "@/app/lib/document-templates/inscription-levels";
import {
  loadInscriptionOverrideBytes,
  loadInscriptionTenantSettings,
} from "@/app/lib/document-templates/inscription-storage";
import type { InscriptionLevelId } from "@/app/lib/document-templates/types";

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

export type RenderInscriptionOptions = {
  levelId: InscriptionLevelId;
  /** Surcharge nom (sinon settings tenant puis letterhead). */
  establishmentName?: string;
  /** Surcharge couleur accent. */
  accentColor?: string;
};

/**
 * Charge le PDF AcroForm du niveau (override tenant ou source repo),
 * conserve les champs, ajoute logo + nom + bandeau couleur en tête de la 1re page.
 */
export async function renderInscriptionFillablePdf(
  opts: RenderInscriptionOptions,
): Promise<Uint8Array> {
  const meta = getInscriptionLevelMeta(opts.levelId);
  if (!meta) throw new Error("Niveau d'inscription inconnu");

  const settings = await loadInscriptionTenantSettings();
  const overrideBytes = await loadInscriptionOverrideBytes(opts.levelId);

  let sourceBytes: Buffer;
  if (overrideBytes?.length) {
    sourceBytes = overrideBytes;
  } else {
    sourceBytes = await fs.readFile(inscriptionSourcePath(opts.levelId));
  }

  const doc = await PDFDocument.load(sourceBytes, { ignoreEncryption: true });
  const pages = doc.getPages();
  if (!pages.length) throw new Error("PDF inscription vide");
  const page = pages[0];
  const { width, height } = page.getSize();

  const letterhead = await getSchoolLetterhead();
  const displayName = sanitize(
    (opts.establishmentName?.trim() ||
      settings.establishmentName?.trim() ||
      letterhead.name ||
      "Établissement").slice(0, 120),
  );
  const accentHex =
    opts.accentColor?.trim() || settings.accentColor?.trim() || "#0f172a";
  const accent = hexToRgb(accentHex);

  const bandH = 36;
  // Bandeau haut (sous le contenu éventuel — on dessine en overlay)
  page.drawRectangle({
    x: 0,
    y: height - bandH,
    width,
    height: bandH,
    color: rgb(accent.r, accent.g, accent.b),
  });

  const logo = await loadSchoolLogoForPdf();
  let logoRight = 12;
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 70, 28);
      const logoY = height - bandH + (bandH - fitted.height) / 2;
      page.drawImage(img, {
        x: 10,
        y: logoY,
        width: fitted.width,
        height: fitted.height,
      });
      logoRight = 10 + fitted.width + 10;
    }
  }

  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const nameSize = 11;
  const nameWidth = bold.widthOfTextAtSize(displayName, nameSize);
  page.drawText(displayName, {
    x: Math.max(logoRight, width - 14 - nameWidth),
    y: height - bandH / 2 - nameSize / 3,
    size: nameSize,
    font: bold,
    color: rgb(1, 1, 1),
  });

  const levelLabel = sanitize(`Fiche d'inscription — ${meta.label}`);
  page.drawText(levelLabel, {
    x: 12,
    y: height - bandH - 12,
    size: 8,
    font,
    color: rgb(accent.r, accent.g, accent.b),
  });

  return doc.save({ updateFieldAppearances: false });
}
