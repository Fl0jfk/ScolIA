import type {
  PosterDraft,
  PosterElement,
  PosterElementKind,
  PosterFormat,
  PosterLayoutPreset,
  PosterPaletteItem,
  PosterTemplateId,
  PosterTemplateMeta,
  PosterTextAlign,
} from "@/app/lib/posters/types";

function uid(): string {
  return `el_${Math.random().toString(36).slice(2, 10)}`;
}

export function createPosterElement(
  kind: PosterElementKind,
  partial?: Partial<PosterElement>,
): PosterElement {
  const defaults = defaultBoxForKind(kind);
  const id = partial?.id || uid();
  return clampElementBox({
    ...defaults,
    ...partial,
    id,
    kind,
  });
}

function defaultBoxForKind(kind: PosterElementKind): Omit<PosterElement, "id" | "kind"> {
  switch (kind) {
    case "logo-school":
      return { x: 0.06, y: 0.05, w: 0.14, h: 0.1, align: "center" };
    case "logo-partner":
      return { x: 0.8, y: 0.05, w: 0.14, h: 0.1, align: "center" };
    case "title":
      return {
        x: 0.08,
        y: 0.28,
        w: 0.84,
        h: 0.12,
        text: "Titre",
        align: "center",
        fontScale: 1.15,
      };
    case "subtitle":
      return {
        x: 0.1,
        y: 0.42,
        w: 0.8,
        h: 0.06,
        text: "Sous-titre",
        align: "center",
        fontScale: 1,
      };
    case "body":
      return {
        x: 0.12,
        y: 0.52,
        w: 0.76,
        h: 0.18,
        text: "Paragraphe…",
        align: "center",
        fontScale: 1,
      };
    case "date-place":
      return {
        x: 0.1,
        y: 0.78,
        w: 0.8,
        h: 0.05,
        text: "",
        align: "center",
        fontScale: 1,
      };
    case "mention":
      return { x: 0.08, y: 0.9, w: 0.6, h: 0.04, text: "", align: "left", fontScale: 0.85 };
    case "qr":
      return { x: 0.82, y: 0.82, w: 0.12, h: 0.12 };
    case "accent-bar":
      return { x: 0, y: 0.2, w: 1, h: 0.012 };
    case "image":
      return { x: 0.55, y: 0.3, w: 0.35, h: 0.35, imageKey: null };
    default:
      return { x: 0.1, y: 0.1, w: 0.3, h: 0.1 };
  }
}

export const POSTER_PALETTE: PosterPaletteItem[] = [
  { kind: "title", label: "Titre", hint: "Grand titre" },
  { kind: "subtitle", label: "Sous-titre", hint: "Ligne secondaire" },
  { kind: "body", label: "Paragraphe", hint: "Texte libre" },
  { kind: "logo-school", label: "Logo école", hint: "Logo établissement (auto)" },
  { kind: "logo-partner", label: "Logo partenaire", hint: "Upload ou placeholder" },
  { kind: "image", label: "Image", hint: "Photo / illustration" },
  { kind: "date-place", label: "Date / lieu", hint: "Période et endroit" },
  { kind: "mention", label: "Mention", hint: "École × partenaire" },
  { kind: "qr", label: "QR code", hint: "Lien scannable" },
  { kind: "accent-bar", label: "Bandeau", hint: "Barre couleur accent" },
];

export function starterElementsPartnerBand(): PosterElement[] {
  return [
    createPosterElement("logo-school", { x: 0.22, y: 0.05, w: 0.14, h: 0.1 }),
    createPosterElement("logo-partner", { x: 0.64, y: 0.05, w: 0.14, h: 0.1 }),
    createPosterElement("accent-bar", { x: 0, y: 0.2, w: 1, h: 0.012 }),
    createPosterElement("title", {
      x: 0.08,
      y: 0.26,
      w: 0.84,
      h: 0.14,
      text: "Partenariat sportif",
      align: "center",
      fontScale: 1.15,
    }),
    createPosterElement("subtitle", {
      x: 0.1,
      y: 0.42,
      w: 0.8,
      h: 0.06,
      text: "Découverte et initiation",
      align: "center",
    }),
    createPosterElement("body", {
      x: 0.12,
      y: 0.52,
      w: 0.76,
      h: 0.18,
      text: "Séances encadrées en partenariat avec notre établissement.",
      align: "center",
    }),
    createPosterElement("date-place", {
      x: 0.1,
      y: 0.78,
      w: 0.8,
      h: 0.05,
      text: "",
      align: "center",
    }),
    createPosterElement("mention", { x: 0.08, y: 0.9, w: 0.55, h: 0.04, align: "left" }),
  ];
}

export function starterElementsPartnerSides(): PosterElement[] {
  return [
    createPosterElement("logo-school", { x: 0.06, y: 0.05, w: 0.14, h: 0.1 }),
    createPosterElement("logo-partner", { x: 0.8, y: 0.05, w: 0.14, h: 0.1 }),
    createPosterElement("accent-bar", { x: 0, y: 0.2, w: 1, h: 0.012 }),
    createPosterElement("title", {
      x: 0.08,
      y: 0.28,
      w: 0.55,
      h: 0.12,
      text: "Partenariat sportif",
      align: "left",
      fontScale: 1.15,
    }),
    createPosterElement("subtitle", {
      x: 0.08,
      y: 0.42,
      w: 0.55,
      h: 0.06,
      text: "Découverte et initiation",
      align: "left",
    }),
    createPosterElement("body", {
      x: 0.08,
      y: 0.52,
      w: 0.55,
      h: 0.2,
      text: "Séances encadrées en partenariat avec notre établissement.",
      align: "left",
    }),
    createPosterElement("date-place", {
      x: 0.08,
      y: 0.78,
      w: 0.55,
      h: 0.05,
      text: "",
      align: "left",
    }),
    createPosterElement("mention", { x: 0.08, y: 0.9, w: 0.55, h: 0.04, align: "left" }),
  ];
}

export function starterElementsPhotoFull(): PosterElement[] {
  return [
    createPosterElement("logo-school", { x: 0.06, y: 0.05, w: 0.12, h: 0.09 }),
    createPosterElement("logo-partner", { x: 0.82, y: 0.05, w: 0.12, h: 0.09 }),
    createPosterElement("accent-bar", { x: 0.35, y: 0.5, w: 0.3, h: 0.008 }),
    createPosterElement("title", {
      x: 0.08,
      y: 0.36,
      w: 0.84,
      h: 0.14,
      text: "Partenariat sportif",
      align: "center",
      fontScale: 1.2,
    }),
    createPosterElement("subtitle", {
      x: 0.1,
      y: 0.54,
      w: 0.8,
      h: 0.06,
      text: "Découverte et initiation",
      align: "center",
    }),
    createPosterElement("body", {
      x: 0.12,
      y: 0.64,
      w: 0.76,
      h: 0.14,
      text: "Séances encadrées en partenariat avec notre établissement.",
      align: "center",
    }),
    createPosterElement("mention", { x: 0.08, y: 0.9, w: 0.55, h: 0.04, align: "left" }),
  ];
}

export const POSTER_TEMPLATES: PosterTemplateMeta[] = [
  {
    id: "partenariat-sportif",
    label: "Partenariat sportif",
    description:
      "Canvas libre : déposez logos, textes et images. Démarrages rapides pour partir d’une base.",
    starters: [
      {
        id: "partner-sides",
        label: "École gauche / partenaire droite",
        hint: "Deux logos en en-tête, textes à gauche",
      },
      {
        id: "logos-band",
        label: "Logos en bandeau",
        hint: "Deux logos centrés en haut",
      },
      {
        id: "photo-full",
        label: "Photo pleine page",
        hint: "Textes centrés, idéal avec fond image",
      },
    ],
  },
];

export const POSTER_FORMATS: { id: PosterFormat; label: string; hint: string }[] = [
  { id: "a4-portrait", label: "A4 portrait", hint: "Mur, impression classique" },
  { id: "a3-landscape", label: "A3 paysage", hint: "Grand format horizontal" },
  {
    id: "a5-portrait",
    label: "A5 (×4 sur A4)",
    hint: "Petit format — planche de 4 à l’impression",
  },
];

export function getPosterTemplateMeta(id: string): PosterTemplateMeta | undefined {
  return POSTER_TEMPLATES.find((t) => t.id === id);
}

export function isPosterTemplateId(id: string): id is PosterTemplateId {
  return POSTER_TEMPLATES.some((t) => t.id === id);
}

export function isPosterFormat(v: string): v is PosterFormat {
  return v === "a4-portrait" || v === "a3-landscape" || v === "a5-portrait";
}

export function isPosterLayoutPreset(v: string): v is PosterLayoutPreset {
  return v === "logos-top" || v === "photo-full" || v === "partner-right";
}

export function isPosterElementKind(v: string): v is PosterElementKind {
  return POSTER_PALETTE.some((p) => p.kind === v) || v === "accent-bar";
}

export function elementsForStarter(starterId: string): PosterElement[] {
  if (starterId === "partner-sides" || starterId === "partner-right") {
    return starterElementsPartnerSides();
  }
  if (starterId === "photo-full") return starterElementsPhotoFull();
  return starterElementsPartnerBand();
}

export function defaultPosterDraft(
  templateId: PosterTemplateId = "partenariat-sportif",
): PosterDraft {
  return {
    templateId,
    format: "a4-portrait",
    /** Page vierge — l’utilisateur ajoute ce qu’il veut. */
    elements: [],
    partnerName: "",
    backgroundMode: "solid",
    backgroundColor: "#ffffff",
    accentColor: "#0ea5e9",
    textColor: "#0f172a",
    gradientTo: "#e2e8f0",
    backgroundImageKey: null,
    overlayOpacity: 0.45,
    partnerLogoKey: null,
    qrUrl: "",
  };
}

/** Titre affiché / historique : premier élément title, sinon libellé modèle. */
export function draftDisplayTitle(draft: PosterDraft): string {
  const titleEl = draft.elements.find((e) => e.kind === "title");
  const t = (titleEl?.text || draft.title || "").trim();
  return t || getPosterTemplateMeta(draft.templateId)?.label || "Affiche";
}

export function clamp01(n: number): number {
  return Math.min(1, Math.max(0, n));
}

export function clampElementBox(el: PosterElement): PosterElement {
  const w = Math.min(1, Math.max(0.02, el.w));
  const h = Math.min(1, Math.max(0.01, el.h));
  const x = Math.min(1 - w, Math.max(0, el.x));
  const y = Math.min(1 - h, Math.max(0, el.y));
  const fontScale = el.fontScale
    ? Math.min(1.5, Math.max(0.7, el.fontScale))
    : undefined;
  const align: PosterTextAlign | undefined =
    el.align === "left" || el.align === "right" || el.align === "center" ? el.align : undefined;
  return { ...el, x, y, w, h, fontScale, align };
}
