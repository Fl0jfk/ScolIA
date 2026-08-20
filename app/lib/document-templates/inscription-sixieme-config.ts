import type {
  InscriptionLevelCodeConfig,
  InscriptionOptionItem,
} from "@/app/lib/document-templates/types";
import { defaultAnneeScolaire } from "@/app/lib/document-templates/catalog";

export const DEFAULT_SIXIEME_OPTIONS: InscriptionOptionItem[] = [
  { id: "externat", label: "Externat" },
  { id: "demi-pension", label: "Demi-pension" },
  { id: "etude", label: "Étude surveillée" },
  { id: "garderie", label: "Garderie / accueil périscolaire" },
  { id: "bus", label: "Transport scolaire" },
  { id: "aes", label: "Activités extrascolaires" },
];

export function defaultSixiemeCodeConfig(): InscriptionLevelCodeConfig {
  return {
    schoolYear: defaultAnneeScolaire(),
    title: "Fiche d'inscription — Sixième",
    subtitle: "Document à compléter et à retourner au secrétariat",
    options: DEFAULT_SIXIEME_OPTIONS.map((o) => ({ ...o })),
  };
}

export function normalizeInscriptionOptions(raw: unknown): InscriptionOptionItem[] {
  if (!Array.isArray(raw)) return DEFAULT_SIXIEME_OPTIONS.map((o) => ({ ...o }));
  const out: InscriptionOptionItem[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const o = item as Record<string, unknown>;
    const label = String(o.label || "").trim().slice(0, 80);
    if (!label) continue;
    const id =
      String(o.id || "")
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^-|-$/g, "")
        .slice(0, 40) || `opt-${out.length + 1}`;
    out.push({ id, label });
  }
  return out.length ? out : DEFAULT_SIXIEME_OPTIONS.map((o) => ({ ...o }));
}

export function normalizeSixiemeCodeConfig(raw: unknown): InscriptionLevelCodeConfig {
  const base = defaultSixiemeCodeConfig();
  if (!raw || typeof raw !== "object") return base;
  const o = raw as Record<string, unknown>;
  return {
    schoolYear: String(o.schoolYear || base.schoolYear).trim().slice(0, 20) || base.schoolYear,
    title: String(o.title ?? base.title || "").trim().slice(0, 120) || base.title,
    subtitle: String(o.subtitle ?? base.subtitle || "").trim().slice(0, 200) || base.subtitle,
    options: normalizeInscriptionOptions(o.options),
  };
}
