import { getTemplateMeta } from "@/app/lib/document-templates/catalog";
import { getInscriptionLevelMeta } from "@/app/lib/document-templates/inscription-levels";
import type {
  DocumentTemplateId,
  InscriptionLevelId,
} from "@/app/lib/document-templates/types";

export function documentTitle(templateId: DocumentTemplateId): string {
  const meta = getTemplateMeta(templateId);
  const label = meta?.label || "Document";
  const day = new Date().toISOString().slice(0, 10);
  return `${label} — modèle ${day}`;
}

export function inscriptionDocumentTitle(levelId: InscriptionLevelId): string {
  const level = getInscriptionLevelMeta(levelId);
  const day = new Date().toISOString().slice(0, 10);
  return `Fiche d'inscription ${level?.label || levelId} — ${day}`;
}
