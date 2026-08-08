export type DocumentTemplateId =
  | "certificat-scolarite"
  | "fiche-inscription"
  | "autorisation-sortie"
  | "courrier-families";

export type DocumentOutputFormat = "pdf" | "docx" | "fillable-pdf";

export type DocumentFieldType = "text" | "date" | "textarea" | "checkbox";

export type DocumentFieldDef = {
  key: string;
  label: string;
  type: DocumentFieldType;
  required?: boolean;
  placeholder?: string;
  /** Prérempli depuis l’élève si clé connue. */
  fromEleve?: "nom" | "prenom" | "classe" | "nomComplet";
  /** Bouton assistant IA (reformulation) dans l’UI. */
  aiAssist?: boolean;
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
  format: DocumentOutputFormat;
  title: string;
};

export type GeneratedDocumentIndexEntry = {
  id: string;
  templateId: DocumentTemplateId;
  templateLabel: string;
  title: string;
  createdAt: string;
  eleveIne?: string;
  format?: DocumentOutputFormat;
};
