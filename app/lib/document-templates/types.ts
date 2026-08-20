export type DocumentTemplateId =
  | "certificat-scolarite"
  | "fiche-inscription"
  | "autorisation-sortie"
  | "courrier-families";

/** Niveaux de fiche d’inscription (sources AcroForm). */
export type InscriptionLevelId =
  | "sixieme"
  | "cinquieme"
  | "quatrieme"
  | "troisieme"
  | "seconde"
  | "premiere-generale"
  | "premiere-st2s"
  | "terminale-generale"
  | "terminale-st2s";

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
  /** Formats proposés dans l’UI (PDF à trous et/ou Word). */
  formats: DocumentOutputFormat[];
};

/** Réglages inscription multi-établissement (par tenant). */
export type InscriptionTenantSettings = {
  /** Nom affiché sur la fiche (groupes scolaires). Vide = letterhead. */
  establishmentName: string;
  /** Couleur accent / bandeau (#rrggbb). */
  accentColor: string;
  /** Clés S3 des PDF de remplacement par niveau. */
  overrides: Partial<Record<InscriptionLevelId, string>>;
  updatedAt?: string;
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
  /** Niveau pour fiche-inscription. */
  inscriptionLevelId?: InscriptionLevelId;
};

export type GeneratedDocumentIndexEntry = {
  id: string;
  templateId: DocumentTemplateId;
  templateLabel: string;
  title: string;
  createdAt: string;
  eleveIne?: string;
  format?: DocumentOutputFormat | "pdf";
  inscriptionLevelId?: InscriptionLevelId;
};
