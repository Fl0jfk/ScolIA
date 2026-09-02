/**
 * PAP (Plan d’Accompagnement Personnalisé) — détection et libellés.
 * Fichier partagé client / serveur (pas de server-only).
 */

/** Titre proposé à l’upload (l’année peut être ajoutée côté UI). */
export const PAP_DOCUMENT_TITLE_PREFIX = "PAP";

export function isPapDocumentTitle(title: string | null | undefined): boolean {
  const n = String(title || "")
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .trim();
  if (!n) return false;
  if (/\bpap\b/.test(n)) return true;
  if (n.includes("plan d'accompagnement") || n.includes("plan d accompagnement")) return true;
  return false;
}

export function defaultPapDocumentTitle(anneeLabel?: string | null): string {
  const year = String(anneeLabel || "").trim();
  return year ? `PAP ${year}` : "PAP";
}
