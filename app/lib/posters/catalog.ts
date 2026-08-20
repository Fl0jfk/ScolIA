import type {
  PosterDraft,
  PosterFormat,
  PosterLayoutPreset,
  PosterOffsets,
  PosterTemplateId,
  PosterTemplateMeta,
} from "@/app/lib/posters/types";

export const POSTER_TEMPLATES: PosterTemplateMeta[] = [
  {
    id: "partenariat-sportif",
    label: "Partenariat sportif",
    description:
      "Affiche partenariat (club, centre équestre, association…) — logos établissement + partenaire, fond personnalisable.",
    presets: [
      {
        id: "logos-top",
        label: "Logos en haut",
        hint: "Deux logos en bandeau, titre centré, texte bas",
      },
      {
        id: "photo-full",
        label: "Photo pleine page",
        hint: "Fond image dominant, textes sur voile",
      },
      {
        id: "partner-right",
        label: "Partenaire à droite",
        hint: "Logo école à gauche, partenaire à droite, colonne texte",
      },
    ],
  },
];

export const POSTER_FORMATS: { id: PosterFormat; label: string; hint: string }[] = [
  { id: "a4-portrait", label: "A4 portrait", hint: "Mur, impression classique" },
  { id: "a3-landscape", label: "A3 paysage", hint: "Deux logos + photo large" },
];

export function getPosterTemplateMeta(id: string): PosterTemplateMeta | undefined {
  return POSTER_TEMPLATES.find((t) => t.id === id);
}

export function isPosterTemplateId(id: string): id is PosterTemplateId {
  return POSTER_TEMPLATES.some((t) => t.id === id);
}

export function isPosterFormat(v: string): v is PosterFormat {
  return v === "a4-portrait" || v === "a3-landscape";
}

export function isPosterLayoutPreset(v: string): v is PosterLayoutPreset {
  return v === "logos-top" || v === "photo-full" || v === "partner-right";
}

export function defaultPosterOffsets(): PosterOffsets {
  return {
    titleOffsetY: 0,
    contentShiftX: 0,
    contentShiftY: 0,
    logoPartnerScale: 1,
  };
}

export function defaultPosterDraft(
  templateId: PosterTemplateId = "partenariat-sportif",
): PosterDraft {
  return {
    templateId,
    format: "a4-portrait",
    layoutPreset: "logos-top",
    title: "Partenariat sportif",
    subtitle: "Découverte et initiation",
    body: "Séances encadrées en partenariat avec notre établissement.",
    partnerName: "",
    dateLabel: "",
    placeLabel: "",
    qrUrl: "",
    backgroundMode: "gradient",
    backgroundColor: "#0f172a",
    accentColor: "#0ea5e9",
    textColor: "#ffffff",
    gradientTo: "#1e3a5f",
    backgroundImageKey: null,
    overlayOpacity: 0.45,
    partnerLogoKey: null,
    logoSchoolSize: "M",
    logoPartnerSize: "M",
    titleSize: "L",
    blocks: {
      showQr: false,
      showDatePlace: true,
      showSchoolMention: true,
    },
    offsets: defaultPosterOffsets(),
  };
}

/** Clamp offsets V2. */
export function clampPosterOffsets(raw: Partial<PosterOffsets> | undefined): PosterOffsets {
  const base = defaultPosterOffsets();
  if (!raw) return base;
  const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));
  return {
    titleOffsetY: clamp(Number(raw.titleOffsetY) || 0, -0.08, 0.08),
    contentShiftX: clamp(Number(raw.contentShiftX) || 0, -0.08, 0.08),
    contentShiftY: clamp(Number(raw.contentShiftY) || 0, -0.08, 0.08),
    logoPartnerScale: clamp(Number(raw.logoPartnerScale) || 1, 0.6, 1.6),
  };
}
