/** Formats d’affiche supportés. */
export type PosterFormat = "a4-portrait" | "a3-landscape" | "a5-portrait";

export type PosterTemplateId = "partenariat-sportif";

/** @deprecated Conservé pour migration des anciens drafts. */
export type PosterLayoutPreset = "logos-top" | "photo-full" | "partner-right";

export type PosterBackgroundMode = "solid" | "gradient" | "image";

export type PosterTextAlign = "left" | "center" | "right";

export type PosterElementKind =
  | "title"
  | "subtitle"
  | "body"
  | "logo-school"
  | "logo-partner"
  | "image"
  | "qr"
  | "accent-bar"
  | "mention"
  | "date-place";

/** Zone normalisée (0–1), origine haut-gauche — partagée canvas HTML + PDF. */
export type PosterBox = {
  x: number;
  y: number;
  w: number;
  h: number;
};

/** Élément libre sur le canvas (Canva élémentaire). */
export type PosterElement = {
  id: string;
  kind: PosterElementKind;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Texte (title, subtitle, body, mention, date-place…). */
  text?: string;
  /** Clé S3 image (logo-partner, image). */
  imageKey?: string | null;
  align?: PosterTextAlign;
  /** Échelle relative de police (0.7–1.5), défaut 1. */
  fontScale?: number;
};

export type PosterDraft = {
  templateId: PosterTemplateId;
  format: PosterFormat;
  /** Éléments positionnés — source de vérité preview + PDF. */
  elements: PosterElement[];
  partnerName: string;
  backgroundMode: PosterBackgroundMode;
  backgroundColor: string;
  accentColor: string;
  textColor: string;
  gradientTo: string;
  backgroundImageKey: string | null;
  overlayOpacity: number;
  /**
   * Logo partenaire partagé (appliqué aux éléments logo-partner sans imageKey propre).
   */
  partnerLogoKey: string | null;
  /** URL globale pour les blocs QR sans texte dédié. */
  qrUrl: string;
  /** @deprecated migration */
  layoutPreset?: PosterLayoutPreset;
  title?: string;
  subtitle?: string;
  body?: string;
  dateLabel?: string;
  placeLabel?: string;
};

export type PosterTemplateMeta = {
  id: PosterTemplateId;
  label: string;
  description: string;
  starters: { id: string; label: string; hint: string }[];
};

export type GeneratedPoster = {
  id: string;
  templateId: PosterTemplateId;
  templateLabel: string;
  title: string;
  createdAt: string;
  createdBy?: { userId?: string; name?: string; email?: string };
  fileKey: string;
  format: PosterFormat;
  draft: PosterDraft;
};

export type GeneratedPosterIndexEntry = {
  id: string;
  templateId: PosterTemplateId;
  templateLabel: string;
  title: string;
  createdAt: string;
  format: PosterFormat;
};

export type PosterPaletteItem = {
  kind: PosterElementKind;
  label: string;
  hint: string;
};
