import "server-only";

import type { RgpdDocumentContentPreview, RgpdWorkspace } from "@/app/lib/rgpd-types";
import { RGPD_CATALOG, evaluateDocumentApplicability } from "@/app/lib/rgpd-catalog";
import { getRgpdEstablishmentLabel, getTemplateSections } from "@/app/lib/rgpd-templates";
import { RGPD_PROCESSING_FICHES } from "@/app/lib/rgpd-processing-fiches";

export const RGPD_DOCUMENT_DISCLAIMER =
  "Document généré automatiquement — outil d'aide à la conformité, sans valeur de conseil juridique. À valider par le DPD/DPO et la direction.";

async function buildRgpdDocumentContentPreview(
  docId: string,
  workspace: RgpdWorkspace,
  establishmentLabel: string,
): Promise<RgpdDocumentContentPreview | null> {
  const entry = RGPD_CATALOG.find((c) => c.id === docId);
  if (!entry || entry.incidentTool) return null;

  const requirement = evaluateDocumentApplicability(entry, workspace.answers);
  const { title, sections } = getTemplateSections(
    docId,
    workspace.answers,
    establishmentLabel,
  );
  const state = workspace.documents[docId];

  return {
    docId,
    title,
    sections,
    disclaimer: RGPD_DOCUMENT_DISCLAIMER,
    applicable: requirement.applicable,
    reason: requirement.reason,
    hasPdf: Boolean(state?.fileKey),
    pdfFileName: state?.fileName,
    generatedAt: state?.generatedAt,
    importedAt: state?.importedAt,
    status: state?.status ?? "manquant",
  };
}

export async function buildAllRgpdDocumentContentPreviews(
  workspace: RgpdWorkspace,
): Promise<RgpdDocumentContentPreview[]> {
  const establishmentLabel = await getRgpdEstablishmentLabel();
  const out: RgpdDocumentContentPreview[] = [];

  const pushPreview = async (docId: string, titleOverride?: string) => {
    const entry = RGPD_CATALOG.find((c) => c.id === docId);
    const fiche = RGPD_PROCESSING_FICHES.find((f) => f.catalogId === docId);
    if (!entry && !fiche) return;

    const requirement = entry
      ? evaluateDocumentApplicability(entry, workspace.answers)
      : {
          applicable: fiche!.isApplicable(workspace.answers),
          reason: fiche!.isApplicable(workspace.answers)
            ? "Fiche de traitement applicable."
            : "Traitement non déclaré dans le questionnaire.",
        };

    const { title, sections } = getTemplateSections(
      docId,
      workspace.answers,
      establishmentLabel,
    );
    const state = workspace.documents[docId];

    out.push({
      docId,
      title: fiche?.getTitle?.(workspace.answers) ?? titleOverride ?? title,
      sections,
      disclaimer: RGPD_DOCUMENT_DISCLAIMER,
      applicable: requirement.applicable,
      reason: "reason" in requirement ? requirement.reason : "",
      hasPdf: Boolean(state?.fileKey),
      pdfFileName: state?.fileName,
      generatedAt: state?.generatedAt,
      importedAt: state?.importedAt,
      status: state?.status ?? "manquant",
    });
  };

  await pushPreview("registre-traitements");
  for (const fiche of RGPD_PROCESSING_FICHES) {
    await pushPreview(fiche.catalogId, fiche.title);
  }
  for (const entry of RGPD_CATALOG) {
    if (entry.incidentTool || entry.id === "registre-traitements") continue;
    await pushPreview(entry.id);
  }
  return out;
}
