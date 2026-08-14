import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";

export const DEFAULT_ESTABLISHMENT_KIND_COLORS: Record<EstablishmentKind, string> = {
  ecole: "#F59E0B",
  college: "#0EA5E9",
  lycee: "#F43F5E",
  custom: "#8B5CF6",
};

export const ESTABLISHMENT_KIND_OPTIONS: { value: EstablishmentKind; label: string }[] = [
  { value: "ecole", label: "École" },
  { value: "college", label: "Collège" },
  { value: "lycee", label: "Lycée" },
  { value: "custom", label: "Autre" },
];

export const ESTABLISHMENT_KIND_PRESETS: {
  kind: EstablishmentKind;
  id: string;
  label: string;
  grades: string;
}[] = [
  { kind: "ecole", id: "ecole", label: "École", grades: "Maternelle & Élémentaire" },
  { kind: "college", id: "college", label: "Collège", grades: "6ème · 5ème · 4ème · 3ème" },
  { kind: "lycee", id: "lycee", label: "Lycée", grades: "2nde · 1ère · Terminale" },
  { kind: "custom", id: "principal", label: "Établissement", grades: "" },
];

export const ESTABLISHMENT_COLOR_SWATCHES = [
  { id: "ecole", label: "Ambre (école)", hex: DEFAULT_ESTABLISHMENT_KIND_COLORS.ecole },
  { id: "college", label: "Bleu (collège)", hex: DEFAULT_ESTABLISHMENT_KIND_COLORS.college },
  { id: "lycee", label: "Rose (lycée)", hex: DEFAULT_ESTABLISHMENT_KIND_COLORS.lycee },
  { id: "custom", label: "Violet (groupe)", hex: DEFAULT_ESTABLISHMENT_KIND_COLORS.custom },
  { id: "emerald", label: "Vert", hex: "#10B981" },
  { id: "slate", label: "Ardoise", hex: "#64748B" },
] as const;

export function normalizeColorHex(raw: string | undefined | null): string | undefined {
  const v = String(raw || "").trim();
  if (/^#[0-9a-fA-F]{6}$/.test(v)) return v.toUpperCase();
  if (/^[0-9a-fA-F]{6}$/.test(v)) return `#${v.toUpperCase()}`;
  return undefined;
}

export function inferEstablishmentKind(
  est: { kind?: string; id?: string; label?: string },
): EstablishmentKind {
  const kind = String(est.kind || "").toLowerCase();
  if (kind === "ecole" || kind === "college" || kind === "lycee" || kind === "custom") return kind;
  const blob = `${est.id || ""} ${est.label || ""}`
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
  if (blob.includes("ecole") || blob.includes("primaire") || blob.includes("elementaire") || blob.includes("maternelle")) {
    return "ecole";
  }
  if (blob.includes("college")) return "college";
  if (blob.includes("lycee")) return "lycee";
  return "custom";
}

export function resolveEstablishmentColorHex(est: {
  colorHex?: string;
  kind?: string;
  id?: string;
  label?: string;
}): string {
  return (
    normalizeColorHex(est.colorHex) ||
    DEFAULT_ESTABLISHMENT_KIND_COLORS[inferEstablishmentKind(est)]
  );
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const h = hex.replace("#", "");
  return {
    r: parseInt(h.slice(0, 2), 16),
    g: parseInt(h.slice(2, 4), 16),
    b: parseInt(h.slice(4, 6), 16),
  };
}

export function hexToRgba(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function mixToward(hex: string, toward: number, amount: number): string {
  const { r, g, b } = hexToRgb(hex);
  const mix = (c: number) => Math.round(c + (toward - c) * amount);
  const toHex = (n: number) => n.toString(16).padStart(2, "0");
  return `#${toHex(mix(r))}${toHex(mix(g))}${toHex(mix(b))}`;
}

export type EstablishmentVisual = {
  hex: string;
  washBg: string;
  orbBg: string;
  borderColor: string;
  badgeBg: string;
  textColor: string;
};

export function establishmentVisualFromHex(hex: string): EstablishmentVisual {
  const color = normalizeColorHex(hex) || DEFAULT_ESTABLISHMENT_KIND_COLORS.custom;
  return {
    hex: color,
    washBg: hexToRgba(color, 0.045),
    orbBg: hexToRgba(color, 0.16),
    borderColor: hexToRgba(color, 0.28),
    badgeBg: hexToRgba(color, 0.72),
    textColor: mixToward(color, 0, 0.45),
  };
}

export function visualForEstablishmentLabel(
  label: string,
  establishments: Establishment[],
  groupeLabel?: string,
): EstablishmentVisual {
  const hit = establishments.find(
    (e) => e.active !== false && (e.label === label || e.id === label),
  );
  if (hit) return establishmentVisualFromHex(resolveEstablishmentColorHex(hit));
  if (groupeLabel && label === groupeLabel) {
    return establishmentVisualFromHex(DEFAULT_ESTABLISHMENT_KIND_COLORS.custom);
  }
  return establishmentVisualFromHex(resolveEstablishmentColorHex({ label }));
}

export function establishmentKindEmoji(kind: EstablishmentKind | string | undefined): string {
  if (kind === "ecole") return "🏫";
  if (kind === "college") return "📚";
  if (kind === "lycee") return "🎓";
  return "🏛";
}
