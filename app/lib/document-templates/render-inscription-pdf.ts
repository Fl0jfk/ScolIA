import "server-only";

import fs from "node:fs/promises";
import { PDFDocument } from "pdf-lib";
import {
  getInscriptionLevelMeta,
  inscriptionSourcePath,
} from "@/app/lib/document-templates/inscription-levels";
import {
  loadInscriptionOverrideBytes,
  loadInscriptionTenantSettings,
} from "@/app/lib/document-templates/inscription-storage";
import { renderSixiemeInscriptionPdf } from "@/app/lib/document-templates/render-inscription-sixieme";
import type { InscriptionLevelId } from "@/app/lib/document-templates/types";

export type RenderInscriptionOptions = {
  levelId: InscriptionLevelId;
  establishmentName?: string;
  accentColor?: string;
};

/**
 * Génère la fiche d’inscription.
 * - sixieme : PDF entièrement généré en code (options / textes configurables)
 * - autres niveaux : PDF source AcroForm tel quel (sans bandeau ajouté)
 */
export async function renderInscriptionFillablePdf(
  opts: RenderInscriptionOptions,
): Promise<Uint8Array> {
  const meta = getInscriptionLevelMeta(opts.levelId);
  if (!meta) throw new Error("Niveau d'inscription inconnu");

  if (opts.levelId === "sixieme") {
    return renderSixiemeInscriptionPdf({
      establishmentName: opts.establishmentName,
      accentColor: opts.accentColor,
    });
  }

  const overrideBytes = await loadInscriptionOverrideBytes(opts.levelId);
  let sourceBytes: Buffer;
  if (overrideBytes?.length) {
    sourceBytes = overrideBytes;
  } else {
    sourceBytes = await fs.readFile(inscriptionSourcePath(opts.levelId));
  }

  // Renvoie le PDF source intact (plus de bandeau / titre superposés).
  const doc = await PDFDocument.load(sourceBytes, { ignoreEncryption: true });
  // Touch settings load to keep tenant cache warm / future per-level branding.
  await loadInscriptionTenantSettings();
  return doc.save({ updateFieldAppearances: false });
}
