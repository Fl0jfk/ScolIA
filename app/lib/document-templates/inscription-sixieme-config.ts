import type {
  InscriptionLevelCodeConfig,
  InscriptionOptionItem,
} from "@/app/lib/document-templates/types";
import { defaultAnneeScolaire } from "@/app/lib/document-templates/catalog";

export const DEFAULT_SIXIEME_OPTIONS: InscriptionOptionItem[] = [
  { id: "classique", label: '6ème "Classique" (Anglais LV1)' },
  { id: "theatre", label: "Théâtre" },
  { id: "chorale", label: "Chorale" },
  { id: "bilangue", label: "6ème Bilangue (Anglais + Allemand LV1)" },
  { id: "foot", label: "USMEF Foot" },
  { id: "basket", label: "BMFB Basket" },
  {
    id: "accompagnement",
    label: "6ème avec accompagnement pour les élèves à besoin particulier",
  },
  { id: "equitation", label: "ALISA Equitation" },
];

export function defaultSixiemeCodeConfig(): InscriptionLevelCodeConfig {
  return {
    schoolYear: defaultAnneeScolaire(),
    title: "Demande d'inscription en sixième",
    subtitle: "",
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
  const schoolYearRaw = o.schoolYear != null ? String(o.schoolYear) : base.schoolYear;
  const titleRaw = o.title != null ? String(o.title) : String(base.title || "");
  const subtitleRaw = o.subtitle != null ? String(o.subtitle) : String(base.subtitle || "");
  return {
    schoolYear: schoolYearRaw.trim().slice(0, 20) || base.schoolYear,
    title: titleRaw.trim().slice(0, 120) || base.title,
    subtitle: subtitleRaw.trim().slice(0, 200) || base.subtitle,
    options: normalizeInscriptionOptions(o.options),
  };
}
