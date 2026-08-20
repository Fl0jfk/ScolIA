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
export type InscriptionOptionItem = {
  id: string;
  label: string;
};

/** Config d’une fiche générée en code (pilote : 6e). */
export type InscriptionLevelCodeConfig = {
  /** Année scolaire affichée, ex. 2026-2027 */
  schoolYear: string;
  /** Titre principal (sinon libellé du niveau). */
  title?: string;
  /** Sous-titre / mention libre. */
  subtitle?: string;
  /** Options cochables (demi-pension, bus…) — placement automatique. */
  options: InscriptionOptionItem[];
};

export type InscriptionPdfFontId = "times" | "helvetica" | "courier";

export type InscriptionTenantSettings = {
  /** Nom affiché sur la fiche (groupes scolaires). Vide = letterhead. */
  establishmentName: string;
  /** Couleur accent / bandeau (#rrggbb). */
  accentColor: string;
  /** Police PDF (standard pdf-lib). */
  pdfFont?: InscriptionPdfFontId;
  /** Clés S3 des PDF de remplacement par niveau (legacy AcroForm). */
  overrides: Partial<Record<InscriptionLevelId, string>>;
  /** Config code par niveau (sixieme en premier). */
  levelConfigs?: Partial<Record<InscriptionLevelId, InscriptionLevelCodeConfig>>;
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
