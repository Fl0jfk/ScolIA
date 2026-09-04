/**
 * Dispositifs d’accompagnement scolaire (PAP / PAI / PPS / GEVASCO).
 * Détection par titre de document — partagé client / serveur.
 */

export type AccompagnementKind = "pap" | "pai" | "pps" | "gevasco";

export type AccompagnementKindDef = {
  kind: AccompagnementKind;
  /** Sigle affiché (badge, select). */
  code: string;
  /** Libellé court. */
  label: string;
  /** Libellé long (aide UI / mails). */
  fullLabel: string;
};

export const ACCOMPAGNEMENT_KINDS: readonly AccompagnementKindDef[] = [
  {
    kind: "pap",
    code: "PAP",
    label: "PAP",
    fullLabel: "Plan d’accompagnement personnalisé",
  },
  {
    kind: "pai",
    code: "PAI",
    label: "PAI",
    fullLabel: "Projet d’accueil individualisé",
  },
  {
    kind: "pps",
    code: "PPS",
    label: "PPS",
    fullLabel: "Projet personnalisé de scolarisation",
  },
  {
    kind: "gevasco",
    code: "GEVASCO",
    label: "GEVASCO",
    fullLabel:
      "Guide d’évaluation des besoins de compensation en matière de scolarisation",
  },
] as const;

export const PAP_DOCUMENT_TITLE_PREFIX = "PAP";

function foldTitle(title: string | null | undefined): string {
  return String(title || "")
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .trim();
}

export function accompagnementKindDef(kind: AccompagnementKind): AccompagnementKindDef {
  return ACCOMPAGNEMENT_KINDS.find((k) => k.kind === kind) ?? ACCOMPAGNEMENT_KINDS[0]!;
}

/** Détecte PAP / PAI / PPS / GEVASCO d’après le titre du document. */
export function detectAccompagnementKind(
  title: string | null | undefined,
): AccompagnementKind | null {
  const n = foldTitle(title);
  if (!n) return null;
  // Ordre : sigles exacts d’abord, puis libellés longs.
  if (
    /\bgevasco\b/.test(n) ||
    /\bgevas[\s._-]?co\b/.test(n) ||
    n.includes("guide d'evaluation des besoins de compensation") ||
    n.includes("guide d evaluation des besoins de compensation")
  ) {
    return "gevasco";
  }
  if (/\bpps\b/.test(n) || n.includes("projet personnalise de scolarisation")) return "pps";
  if (
    /\bpai\b/.test(n) ||
    n.includes("projet d'accueil individualise") ||
    n.includes("projet d accueil individualise") ||
    n.includes("projet daccueil individualise")
  ) {
    return "pai";
  }
  if (
    /\bpap\b/.test(n) ||
    n.includes("plan d'accompagnement") ||
    n.includes("plan d accompagnement")
  ) {
    return "pap";
  }
  return null;
}

/** Tout document PAP, PAI, PPS ou GEVASCO (même circuit d’accès pédagogique). */
export function isAccompagnementDocumentTitle(title: string | null | undefined): boolean {
  return detectAccompagnementKind(title) !== null;
}

/** @deprecated Préférer `isAccompagnementDocumentTitle` / `detectAccompagnementKind`. */
export function isPapDocumentTitle(title: string | null | undefined): boolean {
  return isAccompagnementDocumentTitle(title);
}

export function defaultAccompagnementDocumentTitle(
  kind: AccompagnementKind,
  anneeLabel?: string | null,
): string {
  const code = accompagnementKindDef(kind).code;
  const year = String(anneeLabel || "").trim();
  return year ? `${code} ${year}` : code;
}

/** @deprecated Préférer `defaultAccompagnementDocumentTitle("pap", …)`. */
export function defaultPapDocumentTitle(anneeLabel?: string | null): string {
  return defaultAccompagnementDocumentTitle("pap", anneeLabel);
}
