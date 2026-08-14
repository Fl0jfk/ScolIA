export type DocumentTemplateId =
  | "certificat-scolarite"
  | "fiche-inscription"
  | "autorisation-sortie"
  | "courrier-families";

/** Formats de sortie V2 : modèles vierges uniquement. */
export type DocumentOutputFormat = "docx" | "fillable-pdf";

type DocumentFieldType = "text" | "date" | "textarea" | "checkbox";

/** Champ du modèle = zone à trous (PDF) ou placeholder `$$key$$` (Word). */
export type DocumentFieldDef = {
  key: string;
  label: string;
  type: DocumentFieldType;
};

export type DocumentPlaceholderDef = {
  token: string;
  label: string;
};

export type DocumentTemplateMeta = {
  id: DocumentTemplateId;
  label: string;
  description: string;
  fields: DocumentFieldDef[];
};

export type GeneratedDocument = {
  id: string;
  templateId: DocumentTemplateId;
  templateLabel: string;
  createdAt: string;
  createdBy?: { userId?: string; name?: string; email?: string };
  values: Record<string, string | boolean>;
  eleveIne?: string;
  /** @deprecated garder pour docs déjà générés — préférer fileKey */
  pdfKey?: string;
  fileKey: string;
  format: DocumentOutputFormat | "pdf";
  title: string;
};

export type GeneratedDocumentIndexEntry = {
  id: string;
  templateId: DocumentTemplateId;
  templateLabel: string;
  title: string;
  createdAt: string;
  eleveIne?: string;
  format?: DocumentOutputFormat | "pdf";
};
