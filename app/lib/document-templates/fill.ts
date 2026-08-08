import { getTemplateMeta } from "@/app/lib/document-templates/catalog";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

export function documentTitle(templateId: DocumentTemplateId): string {
  const meta = getTemplateMeta(templateId);
  const label = meta?.label || "Document";
  const day = new Date().toISOString().slice(0, 10);
  return `${label} — modèle ${day}`;
}
