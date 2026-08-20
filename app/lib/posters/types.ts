/** Formats d’affiche supportés. */
export type PosterFormat = "a4-portrait" | "a3-landscape";

export type PosterTemplateId = "partenariat-sportif";

export type PosterLayoutPreset = "logos-top" | "photo-full" | "partner-right";

export type PosterLogoSize = "S" | "M" | "L";

export type PosterTitleSize = "S" | "M" | "L";

export type PosterBackgroundMode = "solid" | "gradient" | "image";

/** Zone normalisée (0–1), origine haut-gauche — partagée preview HTML + PDF. */
export type PosterBox = {
  x: number;
  y: number;
  w: number;
  h: number;
};

export type PosterBlocks = {
  showQr: boolean;
  showDatePlace: boolean;
  showSchoolMention: boolean;
};

/**
 * Réglages fins V2 (bornés). Absents ou 0 en V1.
 * Offsets en fraction de la hauteur/largeur de page (−0.08 … +0.08).
 */
export type PosterOffsets = {
  titleOffsetY: number;
  contentShiftX: number;
  contentShiftY: number;
  logoPartnerScale: number;
};

export type PosterDraft = {
  templateId: PosterTemplateId;
  format: PosterFormat;
  layoutPreset: PosterLayoutPreset;
  title: string;
  subtitle: string;
  body: string;
  partnerName: string;
  dateLabel: string;
  placeLabel: string;
  /** URL / clé S3 optionnelle pour QR (sinon pas de QR même si showQr). */
  qrUrl: string;
  backgroundMode: PosterBackgroundMode;
  backgroundColor: string;
  accentColor: string;
  textColor: string;
  /** Couleur secondaire du dégradé (si gradient). */
  gradientTo: string;
  /** Image de fond (clé S3 ou URL signée / data). */
  backgroundImageKey: string | null;
  overlayOpacity: number;
  partnerLogoKey: string | null;
  logoSchoolSize: PosterLogoSize;
  logoPartnerSize: PosterLogoSize;
  titleSize: PosterTitleSize;
  blocks: PosterBlocks;
  /** V2 — positions légères. */
  offsets: PosterOffsets;
};

export type PosterTemplateMeta = {
  id: PosterTemplateId;
  label: string;
  description: string;
  presets: { id: PosterLayoutPreset; label: string; hint: string }[];
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

export type ComputedPosterLayout = {
  page: { widthPt: number; heightPt: number };
  boxes: {
    background: PosterBox;
    overlay: PosterBox;
    logoSchool: PosterBox;
    logoPartner: PosterBox | null;
    title: PosterBox;
    subtitle: PosterBox;
    body: PosterBox;
    datePlace: PosterBox | null;
    schoolMention: PosterBox | null;
    qr: PosterBox | null;
    accentBar: PosterBox;
  };
  colors: {
    background: string;
    accent: string;
    text: string;
    gradientTo: string;
  };
  backgroundMode: PosterBackgroundMode;
  overlayOpacity: number;
  titleFontSize: number;
  subtitleFontSize: number;
  bodyFontSize: number;
};
